use crate::{
    Combinator, CompoundSelector, CssError, Declaration, ParseResult, Selector, SimpleSelector,
    Specificity, StyleRule, Stylesheet, Token, TokenKind, tokenize,
};

pub fn parse_stylesheet(input: &str) -> ParseResult<Stylesheet> {
    let tokenized = tokenize(input);
    let mut parser = Parser {
        tokens: &tokenized.tokens,
        errors: tokenized.errors,
    };
    let rules = parser.parse_rule_list();
    ParseResult {
        value: Stylesheet { rules },
        errors: parser.errors,
    }
}

pub fn parse_declaration_list(input: &str) -> ParseResult<Vec<Declaration>> {
    let tokenized = tokenize(input);
    let mut parser = Parser {
        tokens: &tokenized.tokens,
        errors: tokenized.errors,
    };
    let declarations = parser.parse_declarations(&tokenized.tokens);
    ParseResult {
        value: declarations,
        errors: parser.errors,
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    errors: Vec<CssError>,
}

impl Parser<'_> {
    fn parse_rule_list(&mut self) -> Vec<StyleRule> {
        let mut rules = Vec::new();
        let mut index = 0;
        while index < self.tokens.len() {
            while index < self.tokens.len()
                && matches!(
                    self.tokens[index].kind,
                    TokenKind::Whitespace | TokenKind::Semicolon
                )
            {
                index += 1;
            }
            if index >= self.tokens.len() {
                break;
            }

            if matches!(self.tokens[index].kind, TokenKind::AtKeyword(_)) {
                let start = self.tokens[index].start;
                index = skip_at_rule(self.tokens, index);
                self.errors.push(CssError {
                    offset: start,
                    message: "at-rules are not supported yet and were ignored".into(),
                });
                continue;
            }

            let prelude_start = index;
            let Some(open) = find_rule_block_start(self.tokens, index) else {
                self.errors.push(CssError {
                    offset: self.tokens[index].start,
                    message: "qualified rule is missing an opening block".into(),
                });
                break;
            };
            let Some(close) = find_matching_close_curly(self.tokens, open) else {
                self.errors.push(CssError {
                    offset: self.tokens[open].start,
                    message: "unterminated CSS rule block".into(),
                });
                break;
            };

            let prelude = trim_whitespace(&self.tokens[prelude_start..open]);
            let body = &self.tokens[open + 1..close];
            match parse_selector_list(prelude) {
                Ok(selectors) if !selectors.is_empty() => {
                    let declarations = self.parse_declarations(body);
                    rules.push(StyleRule {
                        selectors,
                        declarations,
                    });
                }
                Ok(_) => self.errors.push(CssError {
                    offset: self.tokens[open].start,
                    message: "empty selector list".into(),
                }),
                Err(error) => self.errors.push(error),
            }

            index = close + 1;
        }
        rules
    }

    fn parse_declarations(&mut self, tokens: &[Token]) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        let mut start = 0;
        while start < tokens.len() {
            while start < tokens.len()
                && matches!(
                    tokens[start].kind,
                    TokenKind::Whitespace | TokenKind::Semicolon
                )
            {
                start += 1;
            }
            if start >= tokens.len() {
                break;
            }

            let end = find_declaration_end(tokens, start);
            let segment = trim_whitespace(&tokens[start..end]);
            if !segment.is_empty() {
                match parse_declaration(segment) {
                    Ok(declaration) => declarations.push(declaration),
                    Err(error) => self.errors.push(error),
                }
            }
            start = if end < tokens.len() { end + 1 } else { end };
        }
        declarations
    }
}

fn parse_declaration(tokens: &[Token]) -> Result<Declaration, CssError> {
    let Some(first) = tokens.first() else {
        unreachable!("empty declarations are filtered by caller");
    };
    let TokenKind::Ident(name) = &first.kind else {
        return Err(CssError {
            offset: first.start,
            message: "declaration must start with a property name".into(),
        });
    };

    let mut colon = 1;
    while colon < tokens.len() && matches!(tokens[colon].kind, TokenKind::Whitespace) {
        colon += 1;
    }
    if colon >= tokens.len() || !matches!(tokens[colon].kind, TokenKind::Colon) {
        return Err(CssError {
            offset: first.start,
            message: format!("property {name} is missing ':'"),
        });
    }

    let value_tokens = trim_whitespace(&tokens[colon + 1..]);
    if value_tokens.is_empty() {
        return Err(CssError {
            offset: tokens[colon].end,
            message: format!("property {name} has an empty value"),
        });
    }

    let (value_tokens, important) = strip_important(value_tokens);
    if value_tokens.is_empty() {
        return Err(CssError {
            offset: tokens[colon].end,
            message: format!("property {name} has an empty value"),
        });
    }
    let normalized_name = if name.starts_with("--") {
        name.clone()
    } else {
        name.to_ascii_lowercase()
    };

    Ok(Declaration {
        name: normalized_name,
        value: value_tokens
            .iter()
            .map(|token| token.kind.clone())
            .collect(),
        important,
    })
}

fn strip_important(tokens: &[Token]) -> (&[Token], bool) {
    let mut end = tokens.len();
    while end > 0 && matches!(tokens[end - 1].kind, TokenKind::Whitespace) {
        end -= 1;
    }
    if end == 0 {
        return (&tokens[..end], false);
    }
    let TokenKind::Ident(keyword) = &tokens[end - 1].kind else {
        return (&tokens[..end], false);
    };
    if !keyword.eq_ignore_ascii_case("important") {
        return (&tokens[..end], false);
    }
    let mut bang = end - 1;
    while bang > 0 && matches!(tokens[bang - 1].kind, TokenKind::Whitespace) {
        bang -= 1;
    }
    if bang == 0 || tokens[bang - 1].kind != TokenKind::Delim('!') {
        return (&tokens[..end], false);
    }
    let mut value_end = bang - 1;
    while value_end > 0 && matches!(tokens[value_end - 1].kind, TokenKind::Whitespace) {
        value_end -= 1;
    }
    (&tokens[..value_end], true)
}

fn parse_selector_list(tokens: &[Token]) -> Result<Vec<Selector>, CssError> {
    if tokens.is_empty() {
        return Ok(Vec::new());
    }
    let mut selectors = Vec::new();
    let mut start = 0;
    for index in 0..=tokens.len() {
        if index == tokens.len() || matches!(tokens[index].kind, TokenKind::Comma) {
            let group = trim_whitespace(&tokens[start..index]);
            if group.is_empty() {
                let offset = tokens
                    .get(index)
                    .or_else(|| tokens.last())
                    .map_or(0, |token| token.start);
                return Err(CssError {
                    offset,
                    message: "empty selector in selector list".into(),
                });
            }
            selectors.push(parse_selector(group)?);
            start = index + 1;
        }
    }
    Ok(selectors)
}

fn parse_selector(tokens: &[Token]) -> Result<Selector, CssError> {
    let mut compounds = Vec::new();
    let mut combinators = Vec::new();
    let mut specificity = Specificity::default();
    let mut index = 0;

    loop {
        let (compound, next) = parse_compound(tokens, index, &mut specificity)?;
        compounds.push(compound);
        index = next;

        let whitespace_start = index;
        while index < tokens.len() && matches!(tokens[index].kind, TokenKind::Whitespace) {
            index += 1;
        }
        let had_whitespace = index > whitespace_start;
        if index >= tokens.len() {
            break;
        }

        if tokens[index].kind == TokenKind::Delim('>') {
            combinators.push(Combinator::Child);
            index += 1;
            while index < tokens.len() && matches!(tokens[index].kind, TokenKind::Whitespace) {
                index += 1;
            }
            if index >= tokens.len() {
                return Err(CssError {
                    offset: tokens.last().map_or(0, |token| token.end),
                    message: "selector cannot end with '>'".into(),
                });
            }
        } else if had_whitespace {
            combinators.push(Combinator::Descendant);
        } else {
            return Err(CssError {
                offset: tokens[index].start,
                message: "unsupported or malformed selector syntax".into(),
            });
        }
    }

    debug_assert_eq!(compounds.len(), combinators.len() + 1);
    Ok(Selector {
        compounds,
        combinators,
        specificity,
    })
}

fn parse_compound(
    tokens: &[Token],
    start: usize,
    specificity: &mut Specificity,
) -> Result<(CompoundSelector, usize), CssError> {
    let mut index = start;
    let mut simple = Vec::new();

    if let Some(token) = tokens.get(index) {
        match &token.kind {
            TokenKind::Ident(name) => {
                let selector = SimpleSelector::Type(name.to_ascii_lowercase());
                specificity.add_simple(&selector);
                simple.push(selector);
                index += 1;
            }
            TokenKind::Delim('*') => {
                simple.push(SimpleSelector::Universal);
                index += 1;
            }
            _ => {}
        }
    }

    while let Some(token) = tokens.get(index) {
        match &token.kind {
            TokenKind::Hash(name) => {
                let selector = SimpleSelector::Id(name.clone());
                specificity.add_simple(&selector);
                simple.push(selector);
                index += 1;
            }
            TokenKind::Delim('.') => {
                let Some(next) = tokens.get(index + 1) else {
                    return Err(CssError {
                        offset: token.start,
                        message: "class selector is missing a class name".into(),
                    });
                };
                let TokenKind::Ident(name) = &next.kind else {
                    return Err(CssError {
                        offset: next.start,
                        message: "class selector requires an identifier".into(),
                    });
                };
                let selector = SimpleSelector::Class(name.clone());
                specificity.add_simple(&selector);
                simple.push(selector);
                index += 2;
            }
            _ => break,
        }
    }

    if simple.is_empty() {
        let token = tokens.get(start).expect("selector group is non-empty");
        return Err(CssError {
            offset: token.start,
            message: "unsupported or malformed selector syntax".into(),
        });
    }
    Ok((CompoundSelector { simple }, index))
}

fn find_rule_block_start(tokens: &[Token], start: usize) -> Option<usize> {
    let mut parens = 0_u32;
    let mut squares = 0_u32;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.kind {
            TokenKind::OpenParen | TokenKind::Function(_) => parens += 1,
            TokenKind::CloseParen => parens = parens.saturating_sub(1),
            TokenKind::OpenSquare => squares += 1,
            TokenKind::CloseSquare => squares = squares.saturating_sub(1),
            TokenKind::OpenCurly if parens == 0 && squares == 0 => return Some(index),
            TokenKind::CloseCurly if parens == 0 && squares == 0 => return None,
            _ => {}
        }
    }
    None
}

fn find_matching_close_curly(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 1_u32;
    for (index, token) in tokens.iter().enumerate().skip(open + 1) {
        match token.kind {
            TokenKind::OpenCurly => depth += 1,
            TokenKind::CloseCurly => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_declaration_end(tokens: &[Token], start: usize) -> usize {
    let mut parens = 0_u32;
    let mut squares = 0_u32;
    let mut curlies = 0_u32;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.kind {
            TokenKind::OpenParen | TokenKind::Function(_) => parens += 1,
            TokenKind::CloseParen => parens = parens.saturating_sub(1),
            TokenKind::OpenSquare => squares += 1,
            TokenKind::CloseSquare => squares = squares.saturating_sub(1),
            TokenKind::OpenCurly => curlies += 1,
            TokenKind::CloseCurly => curlies = curlies.saturating_sub(1),
            TokenKind::Semicolon if parens == 0 && squares == 0 && curlies == 0 => return index,
            _ => {}
        }
    }
    tokens.len()
}

fn skip_at_rule(tokens: &[Token], start: usize) -> usize {
    let mut index = start + 1;
    let mut block_depth = 0_u32;
    while index < tokens.len() {
        match tokens[index].kind {
            TokenKind::Semicolon if block_depth == 0 => return index + 1,
            TokenKind::OpenCurly => block_depth += 1,
            TokenKind::CloseCurly => {
                if block_depth == 0 {
                    return index;
                }
                block_depth -= 1;
                if block_depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    index
}

fn trim_whitespace(mut tokens: &[Token]) -> &[Token] {
    while tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Whitespace))
    {
        tokens = &tokens[1..];
    }
    while tokens
        .last()
        .is_some_and(|token| matches!(token.kind, TokenKind::Whitespace))
    {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_selector_groups_combinators_specificity_and_declarations() {
        let parsed = parse_stylesheet(
            "article.card > p.note, #hero { Color: red; margin: 10px 20px !important; }",
        );
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        assert_eq!(parsed.value.rules.len(), 1);
        let rule = &parsed.value.rules[0];
        assert_eq!(rule.selectors.len(), 2);
        assert_eq!(rule.selectors[0].combinators, vec![Combinator::Child]);
        assert_eq!(
            rule.selectors[0].specificity,
            Specificity {
                ids: 0,
                classes: 2,
                types: 2,
            }
        );
        assert_eq!(
            rule.selectors[1].specificity,
            Specificity {
                ids: 1,
                classes: 0,
                types: 0,
            }
        );
        assert_eq!(rule.declarations[0].name, "color");
        assert_eq!(rule.declarations[1].name, "margin");
        assert!(rule.declarations[1].important);
    }

    #[test]
    fn parses_inline_declaration_lists_and_preserves_custom_property_case() {
        let parsed = parse_declaration_list("--Brand: #fff; FONT-SIZE: 16px; broken; color: blue");
        assert_eq!(parsed.value.len(), 3);
        assert_eq!(parsed.value[0].name, "--Brand");
        assert_eq!(parsed.value[1].name, "font-size");
        assert_eq!(parsed.value[2].name, "color");
        assert_eq!(parsed.errors.len(), 1);
        assert!(parsed.errors[0].message.contains("missing ':'"));
    }

    #[test]
    fn malformed_rule_does_not_poison_following_rule() {
        let parsed = parse_stylesheet(
            ".ok { color: red; bad } :hover { color: black } p.note { width: 20px }",
        );
        assert_eq!(parsed.value.rules.len(), 2);
        assert_eq!(parsed.value.rules[0].selectors[0].specificity.classes, 1);
        assert_eq!(parsed.value.rules[1].selectors[0].specificity.types, 1);
        assert_eq!(parsed.value.rules[1].selectors[0].specificity.classes, 1);
        assert_eq!(parsed.errors.len(), 2);
    }

    #[test]
    fn ignores_at_rules_as_an_explicit_initial_limitation() {
        let parsed = parse_stylesheet("@media screen { p { color:red } } h1 { color: blue }");
        assert_eq!(parsed.value.rules.len(), 1);
        assert_eq!(parsed.errors.len(), 1);
        assert!(parsed.errors[0].message.contains("at-rules"));
    }

    #[test]
    fn descendant_selector_is_distinct_from_compound_selector() {
        let parsed = parse_stylesheet("main .card.feature span { color: red }");
        assert!(parsed.errors.is_empty());
        let selector = &parsed.value.rules[0].selectors[0];
        assert_eq!(
            selector.combinators,
            vec![Combinator::Descendant, Combinator::Descendant]
        );
        assert_eq!(selector.compounds.len(), 3);
        assert_eq!(
            selector.specificity,
            Specificity {
                ids: 0,
                classes: 2,
                types: 2,
            }
        );
    }
}
