use crate::{
    Combinator, CssError, Declaration, Selector, SimpleSelector, Specificity, StyleRule,
    parse_declaration_list, parse_stylesheet,
};
use op_dom::{Document, NodeId, NodeKind};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleSource {
    Stylesheet,
    Inline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedDeclaration {
    pub declaration: Declaration,
    pub specificity: Specificity,
    pub source_order: usize,
    pub source: StyleSource,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleMap {
    entries: HashMap<NodeId, Vec<MatchedDeclaration>>,
}

impl StyleMap {
    pub fn declarations_for(&self, node: NodeId) -> &[MatchedDeclaration] {
        self.entries
            .get(&node)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleError {
    pub node: NodeId,
    pub error: CssError,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleCollection {
    pub styles: StyleMap,
    pub errors: Vec<StyleError>,
}

pub fn collect_author_styles(document: &Document) -> StyleCollection {
    let mut errors = Vec::new();
    let mut rules = Vec::new();
    let mut next_source_order = 0;

    collect_embedded_rules(
        document,
        document.root(),
        &mut rules,
        &mut errors,
        &mut next_source_order,
    );

    let mut styles = StyleMap::default();
    apply_author_styles(
        document,
        document.root(),
        &rules,
        &mut styles,
        &mut errors,
        &mut next_source_order,
    );

    StyleCollection { styles, errors }
}

pub fn selector_matches(document: &Document, node: NodeId, selector: &Selector) -> bool {
    if selector.compounds.is_empty() {
        return false;
    }
    matches_selector_at(
        document,
        node,
        selector,
        selector.compounds.len().saturating_sub(1),
    )
}

#[derive(Debug)]
struct CollectedRule {
    rule: StyleRule,
    declaration_orders: Vec<usize>,
}

fn collect_embedded_rules(
    document: &Document,
    node: NodeId,
    rules: &mut Vec<CollectedRule>,
    errors: &mut Vec<StyleError>,
    next_source_order: &mut usize,
) {
    if let Some(element) = document.element(node)
        && element.tag_name.eq_ignore_ascii_case("style")
        && is_css_style_element(element)
    {
        let text = descendant_text(document, node);
        let parsed = parse_stylesheet(&text);
        errors.extend(
            parsed
                .errors
                .into_iter()
                .map(|error| StyleError { node, error }),
        );

        for rule in parsed.value.rules {
            let declaration_orders = (0..rule.declarations.len())
                .map(|_| {
                    let order = *next_source_order;
                    *next_source_order = next_source_order.saturating_add(1);
                    order
                })
                .collect();
            rules.push(CollectedRule {
                rule,
                declaration_orders,
            });
        }
    }

    for child in document.children(node) {
        collect_embedded_rules(document, *child, rules, errors, next_source_order);
    }
}

fn apply_author_styles(
    document: &Document,
    node: NodeId,
    rules: &[CollectedRule],
    styles: &mut StyleMap,
    errors: &mut Vec<StyleError>,
    next_source_order: &mut usize,
) {
    if let Some(element) = document.element(node) {
        for collected in rules {
            let specificity = collected
                .rule
                .selectors
                .iter()
                .filter(|selector| selector_matches(document, node, selector))
                .map(|selector| selector.specificity)
                .max();

            if let Some(specificity) = specificity {
                for (declaration, source_order) in collected
                    .rule
                    .declarations
                    .iter()
                    .cloned()
                    .zip(collected.declaration_orders.iter().copied())
                {
                    styles
                        .entries
                        .entry(node)
                        .or_default()
                        .push(MatchedDeclaration {
                            declaration,
                            specificity,
                            source_order,
                            source: StyleSource::Stylesheet,
                        });
                }
            }
        }

        if let Some(style) = attribute_value(element, "style") {
            let parsed = parse_declaration_list(style);
            errors.extend(
                parsed
                    .errors
                    .into_iter()
                    .map(|error| StyleError { node, error }),
            );

            for declaration in parsed.value {
                let source_order = *next_source_order;
                *next_source_order = next_source_order.saturating_add(1);
                styles
                    .entries
                    .entry(node)
                    .or_default()
                    .push(MatchedDeclaration {
                        declaration,
                        specificity: Specificity::default(),
                        source_order,
                        source: StyleSource::Inline,
                    });
            }
        }
    }

    for child in document.children(node) {
        apply_author_styles(document, *child, rules, styles, errors, next_source_order);
    }
}

fn matches_selector_at(
    document: &Document,
    node: NodeId,
    selector: &Selector,
    compound_index: usize,
) -> bool {
    if !compound_matches(document, node, &selector.compounds[compound_index]) {
        return false;
    }
    if compound_index == 0 {
        return true;
    }

    match selector.combinators[compound_index - 1] {
        Combinator::Child => document
            .node(node)
            .and_then(|current| current.parent)
            .is_some_and(|parent| {
                document.element(parent).is_some()
                    && matches_selector_at(document, parent, selector, compound_index - 1)
            }),
        Combinator::Descendant => {
            let mut ancestor = document.node(node).and_then(|current| current.parent);
            while let Some(candidate) = ancestor {
                if document.element(candidate).is_some()
                    && matches_selector_at(document, candidate, selector, compound_index - 1)
                {
                    return true;
                }
                ancestor = document.node(candidate).and_then(|current| current.parent);
            }
            false
        }
    }
}

fn compound_matches(document: &Document, node: NodeId, compound: &crate::CompoundSelector) -> bool {
    let Some(element) = document.element(node) else {
        return false;
    };

    compound.simple.iter().all(|selector| match selector {
        SimpleSelector::Type(name) => element.tag_name.eq_ignore_ascii_case(name),
        SimpleSelector::Universal => true,
        SimpleSelector::Class(name) => attribute_value(element, "class")
            .is_some_and(|classes| classes.split_ascii_whitespace().any(|class| class == name)),
        SimpleSelector::Id(name) => attribute_value(element, "id").is_some_and(|id| id == name),
    })
}

fn is_css_style_element(element: &op_dom::ElementData) -> bool {
    attribute_value(element, "type").is_none_or(|value| {
        value.trim().is_empty() || value.trim().eq_ignore_ascii_case("text/css")
    })
}

fn attribute_value<'a>(element: &'a op_dom::ElementData, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
        .map(|attribute| attribute.value.as_str())
}

fn descendant_text(document: &Document, node: NodeId) -> String {
    let mut text = String::new();
    append_descendant_text(document, node, &mut text);
    text
}

fn append_descendant_text(document: &Document, node: NodeId, output: &mut String) {
    for child in document.children(node) {
        if let Some(child_node) = document.node(*child) {
            match &child_node.kind {
                NodeKind::Text(value) => output.push_str(value),
                _ => append_descendant_text(document, *child, output),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_html::parse_document;

    fn first_element_by_tag(document: &Document, tag: &str) -> NodeId {
        fn find(document: &Document, node: NodeId, tag: &str) -> Option<NodeId> {
            if document
                .element(node)
                .is_some_and(|element| element.tag_name == tag)
            {
                return Some(node);
            }
            document
                .children(node)
                .iter()
                .find_map(|child| find(document, *child, tag))
        }

        find(document, document.root(), tag).expect("expected element")
    }

    #[test]
    fn matches_supported_compounds_child_and_descendant_combinators() {
        let document = parse_document(
            "<main id='app'><section class='card featured'><span class='label'>x</span></section></main>",
        );
        let span = first_element_by_tag(&document, "span");

        let child = parse_stylesheet("section.card > span.label { color: red }");
        let descendant = parse_stylesheet("#app .featured span { color: red }");
        let miss = parse_stylesheet("main > span { color: red }");

        assert!(selector_matches(
            &document,
            span,
            &child.value.rules[0].selectors[0]
        ));
        assert!(selector_matches(
            &document,
            span,
            &descendant.value.rules[0].selectors[0]
        ));
        assert!(!selector_matches(
            &document,
            span,
            &miss.value.rules[0].selectors[0]
        ));
    }

    #[test]
    fn collects_embedded_rules_globally_and_inline_declarations_separately() {
        let document = parse_document(
            "<div id='hero' class='card' style='color: blue; padding: 2px !important'><span class='label'>x</span></div>
             <style>.card { color: red; margin: 1px } #hero.card, .missing { color: green } .card > .label { font-weight: bold }</style>",
        );
        let hero = first_element_by_tag(&document, "div");
        let span = first_element_by_tag(&document, "span");
        let collected = collect_author_styles(&document);

        assert!(collected.errors.is_empty(), "{:?}", collected.errors);

        let hero_declarations = collected.styles.declarations_for(hero);
        assert_eq!(hero_declarations.len(), 5);
        assert!(hero_declarations.iter().any(|matched| {
            matched.declaration.name == "color"
                && matched.source == StyleSource::Stylesheet
                && matched.specificity
                    == Specificity {
                        ids: 1,
                        classes: 1,
                        types: 0,
                    }
        }));
        assert!(hero_declarations.iter().any(|matched| {
            matched.declaration.name == "color" && matched.source == StyleSource::Inline
        }));
        assert!(hero_declarations.iter().any(|matched| {
            matched.declaration.name == "padding"
                && matched.source == StyleSource::Inline
                && matched.declaration.important
        }));

        let span_declarations = collected.styles.declarations_for(span);
        assert_eq!(span_declarations.len(), 1);
        assert_eq!(span_declarations[0].declaration.name, "font-weight");
        assert_eq!(
            span_declarations[0].specificity,
            Specificity {
                ids: 0,
                classes: 2,
                types: 0,
            }
        );
    }

    #[test]
    fn uses_highest_matching_specificity_once_for_selector_lists() {
        let document = parse_document(
            "<style>.card, #hero { color: red }</style><div id='hero' class='card'>x</div>",
        );
        let div = first_element_by_tag(&document, "div");
        let collected = collect_author_styles(&document);
        let declarations = collected.styles.declarations_for(div);

        assert_eq!(declarations.len(), 1);
        assert_eq!(
            declarations[0].specificity,
            Specificity {
                ids: 1,
                classes: 0,
                types: 0,
            }
        );
    }

    #[test]
    fn reports_css_errors_without_losing_valid_author_styles() {
        let document = parse_document(
            "<style>.ok { color: red; broken } :hover { color: black } p { margin: 1px }</style>
             <p class='ok' style='padding: ; width: 20px'>x</p>",
        );
        let p = first_element_by_tag(&document, "p");
        let collected = collect_author_styles(&document);

        assert_eq!(collected.errors.len(), 3);
        let declarations = collected.styles.declarations_for(p);
        assert!(
            declarations
                .iter()
                .any(|matched| matched.declaration.name == "color")
        );
        assert!(
            declarations
                .iter()
                .any(|matched| matched.declaration.name == "margin")
        );
        assert!(
            declarations
                .iter()
                .any(|matched| matched.declaration.name == "width")
        );
    }

    #[test]
    fn skips_non_css_style_types() {
        let document =
            parse_document("<style type='text/not-css'>p { color: red }</style><p>x</p>");
        let p = first_element_by_tag(&document, "p");
        let collected = collect_author_styles(&document);

        assert!(collected.errors.is_empty());
        assert!(collected.styles.declarations_for(p).is_empty());
    }
}
