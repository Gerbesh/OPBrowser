use crate::{
    AttributeMatcher, AttributeSelector, Combinator, CssError, Declaration, NthExpression,
    PseudoClass, Selector, SimpleSelector, Specificity, StyleRule, parse_declaration_list,
    parse_stylesheet,
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
    collect_author_styles_with_linked(document, &HashMap::new())
}

pub fn collect_author_styles_with_linked(
    document: &Document,
    linked_stylesheets: &HashMap<NodeId, String>,
) -> StyleCollection {
    let mut errors = Vec::new();
    let mut rules = Vec::new();
    let mut next_source_order = 0;

    collect_stylesheet_rules(
        document,
        document.root(),
        linked_stylesheets,
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

fn collect_stylesheet_rules(
    document: &Document,
    node: NodeId,
    linked_stylesheets: &HashMap<NodeId, String>,
    rules: &mut Vec<CollectedRule>,
    errors: &mut Vec<StyleError>,
    next_source_order: &mut usize,
) {
    let css = document.element(node).and_then(|element| {
        if element.tag_name.eq_ignore_ascii_case("style") && is_css_style_element(element) {
            Some(descendant_text(document, node))
        } else {
            linked_stylesheets.get(&node).cloned()
        }
    });

    if let Some(css) = css {
        append_parsed_rules(node, &css, rules, errors, next_source_order);
    }

    for child in document.children(node) {
        collect_stylesheet_rules(
            document,
            *child,
            linked_stylesheets,
            rules,
            errors,
            next_source_order,
        );
    }
}

fn append_parsed_rules(
    node: NodeId,
    css: &str,
    rules: &mut Vec<CollectedRule>,
    errors: &mut Vec<StyleError>,
    next_source_order: &mut usize,
) {
    let parsed = parse_stylesheet(css);
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
        Combinator::AdjacentSibling => {
            previous_element_sibling(document, node).is_some_and(|sibling| {
                matches_selector_at(document, sibling, selector, compound_index - 1)
            })
        }
        Combinator::GeneralSibling => preceding_element_siblings(document, node)
            .any(|sibling| matches_selector_at(document, sibling, selector, compound_index - 1)),
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
        SimpleSelector::Attribute(attribute) => attribute_matches(element, attribute),
        SimpleSelector::PseudoClass(pseudo) => pseudo_class_matches(document, node, *pseudo),
        SimpleSelector::Is(selectors) | SimpleSelector::Where(selectors) => selectors
            .iter()
            .any(|selector| selector_matches(document, node, selector)),
        SimpleSelector::Not(selectors) => selectors
            .iter()
            .all(|selector| !selector_matches(document, node, selector)),
        SimpleSelector::NthChild(expression) => nth_child_matches(document, node, *expression),
    })
}

fn attribute_matches(element: &op_dom::ElementData, selector: &AttributeSelector) -> bool {
    let Some(actual) = attribute_value(element, &selector.name) else {
        return false;
    };
    if selector.matcher == AttributeMatcher::Exists {
        return true;
    }
    let Some(expected) = selector.value.as_deref() else {
        return false;
    };
    let equals = |left: &str, right: &str| {
        if selector.case_insensitive {
            left.eq_ignore_ascii_case(right)
        } else {
            left == right
        }
    };
    let normalize = |value: &str| {
        if selector.case_insensitive {
            value.to_ascii_lowercase()
        } else {
            value.to_owned()
        }
    };

    match selector.matcher {
        AttributeMatcher::Exists => true,
        AttributeMatcher::Exact => equals(actual, expected),
        AttributeMatcher::Includes => {
            !expected.is_empty()
                && actual
                    .split_ascii_whitespace()
                    .any(|word| equals(word, expected))
        }
        AttributeMatcher::DashMatch => {
            equals(actual, expected)
                || (!expected.is_empty()
                    && normalize(actual).starts_with(&(normalize(expected) + "-")))
        }
        AttributeMatcher::Prefix => {
            !expected.is_empty() && normalize(actual).starts_with(&normalize(expected))
        }
        AttributeMatcher::Suffix => {
            !expected.is_empty() && normalize(actual).ends_with(&normalize(expected))
        }
        AttributeMatcher::Substring => {
            !expected.is_empty() && normalize(actual).contains(&normalize(expected))
        }
    }
}

fn pseudo_class_matches(document: &Document, node: NodeId, pseudo: PseudoClass) -> bool {
    match pseudo {
        PseudoClass::Root => document
            .node(node)
            .and_then(|current| current.parent)
            .is_some_and(|parent| parent == document.root()),
        PseudoClass::FirstChild => {
            element_siblings(document, node).and_then(|siblings| siblings.first().copied())
                == Some(node)
        }
        PseudoClass::LastChild => {
            element_siblings(document, node).and_then(|siblings| siblings.last().copied())
                == Some(node)
        }
        PseudoClass::OnlyChild => {
            element_siblings(document, node).is_some_and(|siblings| siblings.as_slice() == [node])
        }
        PseudoClass::Empty => document.children(node).iter().all(|child| {
            document.node(*child).is_none_or(|child| match &child.kind {
                NodeKind::Element(_) => false,
                NodeKind::Text(text) => text.is_empty(),
                NodeKind::Document => true,
            })
        }),
        PseudoClass::Link => document.element(node).is_some_and(|element| {
            element.tag_name.eq_ignore_ascii_case("a") && attribute_value(element, "href").is_some()
        }),
    }
}

fn nth_child_matches(document: &Document, node: NodeId, expression: NthExpression) -> bool {
    let Some(siblings) = element_siblings(document, node) else {
        return false;
    };
    let Some(position) = siblings.iter().position(|candidate| *candidate == node) else {
        return false;
    };
    let index = position as i64 + 1;
    let a = i64::from(expression.a);
    let b = i64::from(expression.b);
    if a == 0 {
        return index == b;
    }
    let difference = index - b;
    difference % a == 0 && difference / a >= 0
}

fn element_siblings(document: &Document, node: NodeId) -> Option<Vec<NodeId>> {
    let parent = document.node(node)?.parent?;
    Some(
        document
            .children(parent)
            .iter()
            .copied()
            .filter(|candidate| document.element(*candidate).is_some())
            .collect(),
    )
}

fn previous_element_sibling(document: &Document, node: NodeId) -> Option<NodeId> {
    preceding_element_siblings(document, node).next()
}

fn preceding_element_siblings(
    document: &Document,
    node: NodeId,
) -> impl Iterator<Item = NodeId> + '_ {
    let siblings = document
        .node(node)
        .and_then(|current| current.parent)
        .map(|parent| document.children(parent))
        .unwrap_or_default();
    let before = siblings
        .iter()
        .position(|candidate| *candidate == node)
        .unwrap_or(0);
    siblings[..before]
        .iter()
        .rev()
        .copied()
        .filter(|candidate| document.element(*candidate).is_some())
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

    fn element_by_id(document: &Document, id: &str) -> NodeId {
        fn find(document: &Document, node: NodeId, id: &str) -> Option<NodeId> {
            if document.element(node).is_some_and(|element| {
                attribute_value(element, "id").is_some_and(|value| value == id)
            }) {
                return Some(node);
            }
            document
                .children(node)
                .iter()
                .find_map(|child| find(document, *child, id))
        }
        find(document, document.root(), id).expect("expected id")
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
    fn matches_attribute_sibling_and_structural_pseudo_selectors() {
        let document = parse_document(
            "<html><body><main id='main' data-mode='Dark'>
                <a id='a' href='https://x' rel='external noopener'></a>
                text
                <span id='s' data-lang='en-US' title='HelloWorld'></span>
                <em id='e'></em>
                <i><b id='only'></b></i>
                <div id='empty'></div>
                <div id='notempty'> </div>
             </main></body></html>",
        );
        let html = first_element_by_tag(&document, "html");
        let main = element_by_id(&document, "main");
        let a = element_by_id(&document, "a");
        let span = element_by_id(&document, "s");
        let em = element_by_id(&document, "e");
        let only = element_by_id(&document, "only");
        let empty = element_by_id(&document, "empty");
        let notempty = element_by_id(&document, "notempty");

        let matches = |node, source: &str| {
            let parsed = parse_stylesheet(&format!("{source} {{ color:red }}"));
            assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
            selector_matches(&document, node, &parsed.value.rules[0].selectors[0])
        };

        assert!(matches(html, ":root"));
        assert!(matches(main, "[data-mode=dark i]"));
        assert!(matches(
            a,
            "a:link:first-child[rel~=external][href^=https][href$=x][href*='://']"
        ));
        assert!(matches(span, "a + span[data-lang|=en][title='HelloWorld']"));
        assert!(matches(em, "a ~ em"));
        assert!(matches(only, "b:only-child:first-child:last-child"));
        assert!(matches(empty, "div:empty"));
        assert!(!matches(notempty, "div:empty"));
        assert!(!matches(span, "a + em"));
        assert!(!matches(span, "[data-mode=dark]"));
    }

    #[test]
    fn matches_functional_pseudos_and_nth_child_against_element_siblings() {
        let document = parse_document(
            "<ul><li id='one' class='hot'></li>text<li id='two' class='hot skip'></li><li id='three' class='warm target'></li><li id='four' class='cold'></li></ul>",
        );
        let one = element_by_id(&document, "one");
        let two = element_by_id(&document, "two");
        let three = element_by_id(&document, "three");
        let four = element_by_id(&document, "four");
        let matches = |node, source: &str| {
            let parsed = parse_stylesheet(&format!("{source} {{ color:red }}"));
            assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
            selector_matches(&document, node, &parsed.value.rules[0].selectors[0])
        };

        assert!(matches(one, "li:is(.hot,.warm):not(.skip):nth-child(odd)"));
        assert!(!matches(two, "li:is(.hot,.warm):not(.skip)"));
        assert!(matches(three, "li:where(.target,#never):nth-child(2n+1)"));
        assert!(!matches(four, "li:is(.hot,.warm)"));
        assert!(matches(two, "li:nth-child(2)"));
        assert!(matches(four, "li:nth-child(-n+4)"));
        assert!(!matches(four, "li:nth-child(2n+1)"));
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
    fn interleaves_linked_and_embedded_stylesheets_in_document_order() {
        let document = parse_document(
            "<link rel='stylesheet' href='first.css'><style>p { color: green }</style><link rel='stylesheet' href='last.css'><p>x</p>",
        );
        let mut links = Vec::new();
        let mut stack = vec![document.root()];
        while let Some(node) = stack.pop() {
            if document
                .element(node)
                .is_some_and(|element| element.tag_name == "link")
            {
                links.push(node);
            }
            stack.extend(document.children(node).iter().rev());
        }
        let mut linked = HashMap::new();
        linked.insert(links[0], "p { color: red }".to_owned());
        linked.insert(links[1], "p { color: blue }".to_owned());
        let p = first_element_by_tag(&document, "p");
        let collected = collect_author_styles_with_linked(&document, &linked);
        let colors: Vec<_> = collected
            .styles
            .declarations_for(p)
            .iter()
            .filter(|matched| matched.declaration.name == "color")
            .collect();

        assert_eq!(colors.len(), 3);
        assert!(colors[0].source_order < colors[1].source_order);
        assert!(colors[1].source_order < colors[2].source_order);
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
