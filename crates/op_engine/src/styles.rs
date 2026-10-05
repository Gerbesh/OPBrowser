//! Bounded external author stylesheet discovery/loading for one prepared document.

use op_dom::{Document, NodeId};
use op_net::{NetworkContext, resolve_stylesheet_source};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MAX_STYLESHEET_NODES: usize = 32;
const MAX_REQUESTS: usize = 8;
const TEXT_BUDGET: usize = 2 * 1024 * 1024;

pub(super) fn load(
    network: &NetworkContext,
    document: &Document,
    base: &str,
) -> HashMap<NodeId, String> {
    let started = Instant::now();
    let mut linked = HashMap::new();
    let mut cache: HashMap<String, Option<String>> = HashMap::new();
    let mut nodes = 0usize;
    let mut requests = 0usize;
    let mut text_bytes = 0usize;

    let mut stack = vec![document.root()];
    while let Some(node) = stack.pop() {
        if let Some(element) = document.element(node)
            && element.tag_name.eq_ignore_ascii_case("link")
            && is_active_stylesheet_link(element)
        {
            nodes += 1;
            if nodes > MAX_STYLESHEET_NODES {
                break;
            }

            let Some(href) = attribute(element, "href") else {
                stack.extend(document.children(node).iter().rev());
                continue;
            };
            let Ok(source) = resolve_stylesheet_source(Some(base), href) else {
                stack.extend(document.children(node).iter().rev());
                continue;
            };

            if let Some(Some(css)) = cache.get(&source) {
                linked.insert(node, css.clone());
                stack.extend(document.children(node).iter().rev());
                continue;
            }
            if cache.contains_key(&source)
                || requests >= MAX_REQUESTS
                || text_bytes >= TEXT_BUDGET
                || started.elapsed() > Duration::from_secs(10)
            {
                stack.extend(document.children(node).iter().rev());
                continue;
            }

            requests += 1;
            let css = network
                .load_stylesheet(&source, TEXT_BUDGET - text_bytes)
                .ok()
                .map(|loaded| loaded.text)
                .filter(|text| {
                    text_bytes = text_bytes.saturating_add(text.len());
                    text_bytes <= TEXT_BUDGET
                });

            if let Some(css) = &css {
                linked.insert(node, css.clone());
            }
            cache.insert(source, css);
        }

        stack.extend(document.children(node).iter().rev());
    }

    linked
}

fn is_active_stylesheet_link(element: &op_dom::ElementData) -> bool {
    if attribute(element, "disabled").is_some() {
        return false;
    }

    let Some(rel) = attribute(element, "rel") else {
        return false;
    };
    let mut stylesheet = false;
    for token in rel.split_ascii_whitespace() {
        if token.eq_ignore_ascii_case("alternate") {
            return false;
        }
        if token.eq_ignore_ascii_case("stylesheet") {
            stylesheet = true;
        }
    }
    if !stylesheet {
        return false;
    }

    if attribute(element, "type").is_some_and(|value| {
        !value.trim().is_empty() && !value.trim().eq_ignore_ascii_case("text/css")
    }) {
        return false;
    }

    attribute(element, "media").is_none_or(|value| {
        let value = value.trim();
        value.is_empty()
            || value.eq_ignore_ascii_case("all")
            || value.eq_ignore_ascii_case("screen")
    })
}

fn attribute<'a>(element: &'a op_dom::ElementData, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
        .map(|attribute| attribute.value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_html::parse_document;

    fn first_link(document: &Document) -> NodeId {
        let mut stack = vec![document.root()];
        while let Some(node) = stack.pop() {
            if document
                .element(node)
                .is_some_and(|element| element.tag_name == "link")
            {
                return node;
            }
            stack.extend(document.children(node).iter().rev());
        }
        panic!("expected link");
    }

    #[test]
    fn recognizes_only_active_screen_stylesheet_links() {
        for (attributes, expected) in [
            ("rel='stylesheet' href='a.css'", true),
            ("rel='STYLESHEET' media='screen' href='a.css'", true),
            ("rel='stylesheet' media='all' href='a.css'", true),
            ("rel='alternate stylesheet' href='a.css'", false),
            ("rel='stylesheet' disabled href='a.css'", false),
            ("rel='stylesheet' media='print' href='a.css'", false),
            ("rel='icon' href='a.css'", false),
            ("rel='stylesheet' type='text/plain' href='a.css'", false),
        ] {
            let document = parse_document(&format!("<link {attributes}><p>x</p>"));
            let link = first_link(&document);
            assert_eq!(
                is_active_stylesheet_link(document.element(link).unwrap()),
                expected,
                "{attributes}"
            );
        }
    }
}
