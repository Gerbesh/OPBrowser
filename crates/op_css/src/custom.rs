//! Owned custom-property dependency resolution and bounded token substitution.
use crate::{CustomPropertyMap, TokenKind};
use std::collections::HashMap;

const MAX_VALUE_TOKENS: usize = 16_384;
const MAX_VALUE_BYTES: usize = 256 * 1024;
const MAX_CUSTOM_BYTES: usize = 2 * 1024 * 1024;
const MAX_FALLBACK_DEPTH: usize = 64;

pub(super) fn resolve_custom_values(raw: &CustomPropertyMap) -> CustomPropertyMap {
    let mut names: Vec<&str> = raw.keys().map(String::as_str).collect();
    names.sort_unstable();
    let indices: HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (*name, index))
        .collect();
    let mut edges = vec![Vec::new(); names.len()];
    let mut reverse = vec![Vec::new(); names.len()];
    for (index, name) in names.iter().enumerate() {
        let tokens = &raw[*name];
        for (start, token) in tokens.iter().enumerate() {
            if matches!(token, TokenKind::Function(name) if name.eq_ignore_ascii_case("var")) {
                // Scan every var(), including references in unused fallback branches.
                let first = tokens[start + 1..]
                    .iter()
                    .find(|token| !matches!(token, TokenKind::Whitespace));
                if let Some(TokenKind::Ident(dependency)) = first
                    && let Some(&target) = indices.get(dependency.as_str())
                {
                    edges[index].push(target);
                }
            }
        }
        edges[index].sort_unstable();
        edges[index].dedup();
        for &target in &edges[index] {
            reverse[target].push(index);
        }
    }

    // Iterative Kosaraju traversal avoids native-stack growth on long chains.
    let order = dependency_order(&edges);
    let mut assigned = vec![false; names.len()];
    let mut cyclic = vec![false; names.len()];
    for &root in order.iter().rev() {
        if assigned[root] {
            continue;
        }
        assigned[root] = true;
        let mut stack = vec![root];
        let mut component = Vec::new();
        while let Some(node) = stack.pop() {
            component.push(node);
            for &next in &reverse[node] {
                if !assigned[next] {
                    assigned[next] = true;
                    stack.push(next);
                }
            }
        }
        if component.len() > 1 || edges[root].contains(&root) {
            for node in component {
                cyclic[node] = true;
            }
        }
    }

    let mut resolved = CustomPropertyMap::new();
    let mut total_bytes = 0_usize;
    for node in order {
        if cyclic[node] {
            continue;
        }
        let name = names[node];
        if let Some(value) =
            substitute_vars(&raw[name], |name| resolved.get(name).map(Vec::as_slice))
        {
            let bytes = value.iter().map(token_bytes).sum::<usize>();
            if bytes <= MAX_CUSTOM_BYTES.saturating_sub(total_bytes) {
                total_bytes += bytes;
                resolved.insert(name.to_owned(), value);
            }
        }
    }
    resolved
}

fn dependency_order(edges: &[Vec<usize>]) -> Vec<usize> {
    let mut visited = vec![false; edges.len()];
    let mut order = Vec::with_capacity(edges.len());
    for root in 0..edges.len() {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut stack = vec![(root, 0)];
        while let Some((node, next_edge)) = stack.last_mut() {
            if let Some(&next) = edges[*node].get(*next_edge) {
                *next_edge += 1;
                if !visited[next] {
                    visited[next] = true;
                    stack.push((next, 0));
                }
            } else {
                order.push(*node);
                stack.pop();
            }
        }
    }
    order
}

fn token_bytes(token: &TokenKind) -> usize {
    let payload = match token {
        TokenKind::Ident(value)
        | TokenKind::AtKeyword(value)
        | TokenKind::String(value)
        | TokenKind::Number(value)
        | TokenKind::Percentage(value)
        | TokenKind::Function(value)
        | TokenKind::Hash { value, .. } => value.len(),
        TokenKind::Dimension { number, unit } => number.len().saturating_add(unit.len()),
        _ => 0,
    };
    std::mem::size_of::<TokenKind>().saturating_add(payload)
}

#[derive(Default)]
struct ValueBudget {
    tokens: usize,
    bytes: usize,
}

impl ValueBudget {
    fn append(&mut self, output: &mut Vec<TokenKind>, tokens: &[TokenKind]) -> Option<()> {
        let bytes = tokens.iter().map(token_bytes).sum::<usize>();
        if tokens.len() > MAX_VALUE_TOKENS.saturating_sub(self.tokens)
            || bytes > MAX_VALUE_BYTES.saturating_sub(self.bytes)
        {
            return None;
        }
        self.tokens += tokens.len();
        self.bytes += bytes;
        output.extend_from_slice(tokens);
        Some(())
    }
}

pub(super) fn substitute_vars<'a, F>(tokens: &[TokenKind], mut lookup: F) -> Option<Vec<TokenKind>>
where
    F: FnMut(&str) -> Option<&'a [TokenKind]>,
{
    let mut output = Vec::new();
    substitute_inner(
        tokens,
        &mut lookup,
        &mut ValueBudget::default(),
        &mut output,
        0,
    )?;
    Some(output)
}

fn substitute_inner<'a, F>(
    tokens: &[TokenKind],
    lookup: &mut F,
    budget: &mut ValueBudget,
    output: &mut Vec<TokenKind>,
    depth: usize,
) -> Option<()>
where
    F: FnMut(&str) -> Option<&'a [TokenKind]>,
{
    if depth > MAX_FALLBACK_DEPTH {
        return None;
    }
    let mut index = 0;
    while index < tokens.len() {
        if matches!(&tokens[index], TokenKind::Function(name) if name.eq_ignore_ascii_case("var")) {
            let (end, name, fallback) = parse_var_function(tokens, index)?;
            if let Some(value) = lookup(name) {
                budget.append(output, value)?;
            } else {
                substitute_inner(fallback?, lookup, budget, output, depth + 1)?;
            }
            index = end + 1;
        } else {
            budget.append(output, &tokens[index..index + 1])?;
            index += 1;
        }
    }
    Some(())
}

fn parse_var_function(
    tokens: &[TokenKind],
    start: usize,
) -> Option<(usize, &str, Option<&[TokenKind]>)> {
    let mut depth = 1_u32;
    let mut comma = None;
    let mut end = None;
    for (index, token) in tokens.iter().enumerate().skip(start + 1) {
        match token {
            TokenKind::Function(_) | TokenKind::OpenParen => depth += 1,
            TokenKind::CloseParen => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end = Some(index);
                    break;
                }
            }
            TokenKind::Comma if depth == 1 && comma.is_none() => comma = Some(index),
            _ => {}
        }
    }
    let end = end?;
    let name_end = comma.unwrap_or(end);
    let significant: Vec<&TokenKind> = tokens[start + 1..name_end]
        .iter()
        .filter(|token| !matches!(token, TokenKind::Whitespace))
        .collect();
    let [TokenKind::Ident(name)] = significant.as_slice() else {
        return None;
    };
    if name == "--" || !name.starts_with("--") {
        return None;
    }
    Some((
        end,
        name.as_str(),
        comma.map(|comma| &tokens[comma + 1..end]),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &str) -> Vec<TokenKind> {
        crate::tokenize(source)
            .tokens
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    fn values(entries: &[(&str, &str)]) -> CustomPropertyMap {
        entries
            .iter()
            .map(|(name, value)| ((*name).to_owned(), tokens(value)))
            .collect()
    }

    #[test]
    fn invalidates_exact_cycle_components_including_unused_fallback_edges() {
        let raw = values(&[
            ("--good", "green"),
            ("--self", "var(--self, red)"),
            ("--a", "var(--good, var(--b))"),
            ("--b", "var(--a, blue)"),
            ("--x", "var(--y) var(--z)"),
            ("--y", "var(--x)"),
            ("--z", "var(--y)"),
            ("--rescued", "var(--a, var(--self, red))"),
            ("--invalid", "var(--x)"),
        ]);
        let resolved = resolve_custom_values(&raw);
        for name in ["--self", "--a", "--b", "--x", "--y", "--z", "--invalid"] {
            assert!(!resolved.contains_key(name), "{name} must be invalid");
        }
        assert_eq!(resolved["--good"], tokens("green"));
        assert_eq!(
            resolved["--rescued"],
            vec![
                TokenKind::Whitespace,
                TokenKind::Whitespace,
                TokenKind::Ident("red".into())
            ]
        );
    }

    #[test]
    fn resolves_shared_acyclic_dependencies_and_empty_values_without_false_cycles() {
        let raw = values(&[
            ("--a", "var(--b)"),
            ("--b", "var(--c, var(--a-missing))"),
            ("--c", "green"),
            ("--d", "var(--c) var(--b)"),
            ("--empty", ""),
            ("--use-empty", "var(--empty, red)"),
            ("--C", "blue"),
        ]);
        let resolved = resolve_custom_values(&raw);
        assert_eq!(resolved["--a"], tokens("green"));
        assert_eq!(resolved["--d"], tokens("green green"));
        assert_eq!(resolved["--C"], tokens("blue"));
        assert_eq!(resolved["--use-empty"], Vec::<TokenKind>::new());
    }

    #[test]
    fn long_dependency_chains_resolve_without_recursive_native_stack_growth() {
        let mut raw = CustomPropertyMap::new();
        for index in 0..10_000 {
            raw.insert(
                format!("--v{index}"),
                tokens(&format!("var(--v{})", index + 1)),
            );
        }
        raw.insert("--v10000".into(), tokens("green"));
        let resolved = resolve_custom_values(&raw);
        assert_eq!(resolved.len(), 10_001);
        assert_eq!(resolved["--v0"], tokens("green"));
    }

    #[test]
    fn bounds_exponential_expansion_payload_bytes_and_fallback_recursion() {
        let mut raw = values(&[("--v0", "word")]);
        for index in 1..32 {
            let previous = index - 1;
            raw.insert(
                format!("--v{index}"),
                tokens(&format!("var(--v{previous}) var(--v{previous})")),
            );
        }
        let resolved = resolve_custom_values(&raw);
        assert!(!resolved.contains_key("--v31"));
        assert!(
            resolved
                .values()
                .all(|value| value.len() <= MAX_VALUE_TOKENS
                    && value.iter().map(token_bytes).sum::<usize>() <= MAX_VALUE_BYTES)
        );
        assert!(
            substitute_vars(&tokens("var(--v31, red)"), |name| resolved
                .get(name)
                .map(Vec::as_slice))
            .is_some()
        );
        let huge = vec![TokenKind::String("x".repeat(MAX_VALUE_BYTES))];
        assert!(substitute_vars(&huge, |_| None).is_none());
        let mut fallback = "green".to_owned();
        for _ in 0..MAX_FALLBACK_DEPTH + 2 {
            fallback = format!("var(--missing,{fallback})");
        }
        assert!(substitute_vars(&tokens(&fallback), |_| None).is_none());
    }

    #[test]
    fn limits_total_resolved_storage_deterministically() {
        let mut raw = CustomPropertyMap::new();
        for index in 0..40 {
            raw.insert(
                format!("--v{index:02}"),
                vec![TokenKind::String("x".repeat(128 * 1024))],
            );
        }
        let resolved = resolve_custom_values(&raw);
        assert!(resolved.len() < raw.len());
        assert!(resolved.values().flatten().map(token_bytes).sum::<usize>() <= MAX_CUSTOM_BYTES);
        assert_eq!(resolved, resolve_custom_values(&raw));
    }
}
