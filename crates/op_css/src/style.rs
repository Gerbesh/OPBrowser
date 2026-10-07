use crate::{
    AttributeMatcher, AttributeSelector, Combinator, CssError, Declaration, NthSelector,
    PseudoClass, PseudoElement, RelativeSelector, Selector, SimpleSelector, Specificity, StyleRule,
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
    /// DOM style/link node, or the element carrying an inline style attribute.
    pub style_node: NodeId,
    /// Computed copies retain pending-substitution priority after var() expansion.
    pub value_from_var: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleMap {
    entries: HashMap<NodeId, Vec<MatchedDeclaration>>,
    pseudo_entries: HashMap<(NodeId, PseudoElement), Vec<MatchedDeclaration>>,
}

impl StyleMap {
    pub fn declarations_for(&self, node: NodeId) -> &[MatchedDeclaration] {
        self.entries
            .get(&node)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn declarations_for_pseudo(
        &self,
        node: NodeId,
        pseudo: PseudoElement,
    ) -> &[MatchedDeclaration] {
        self.pseudo_entries
            .get(&(node, pseudo))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.entries.len().saturating_add(self.pseudo_entries.len())
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.pseudo_entries.is_empty()
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
    style_node: NodeId,
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
            style_node: node,
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
            for pseudo in [
                None,
                Some(PseudoElement::Before),
                Some(PseudoElement::After),
                Some(PseudoElement::FirstLetter),
            ] {
                let specificity = collected
                    .rule
                    .selectors
                    .iter()
                    .filter(|selector| selector.pseudo_element == pseudo)
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
                        let matched = MatchedDeclaration {
                            declaration,
                            specificity,
                            source_order,
                            source: StyleSource::Stylesheet,
                            style_node: collected.style_node,
                            value_from_var: false,
                        };
                        if let Some(pseudo) = pseudo {
                            styles
                                .pseudo_entries
                                .entry((node, pseudo))
                                .or_default()
                                .push(matched);
                        } else {
                            styles.entries.entry(node).or_default().push(matched);
                        }
                    }
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
                        style_node: node,
                        value_from_var: false,
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
        SimpleSelector::Lang(ranges) => language_matches(document, node, ranges),
        SimpleSelector::Dir(direction) => direction_matches(document, node, direction),
        SimpleSelector::Is(selectors) | SimpleSelector::Where(selectors) => selectors
            .iter()
            .any(|selector| selector_matches(document, node, selector)),
        SimpleSelector::Not(selectors) => selectors
            .iter()
            .all(|selector| !selector_matches(document, node, selector)),
        SimpleSelector::Has(selectors) => selectors
            .iter()
            .any(|relative| relative_selector_matches(document, node, relative)),
        SimpleSelector::NthChild(nth) => nth_child_matches(document, node, nth),
    })
}

fn relative_selector_matches(
    document: &Document,
    anchor: NodeId,
    relative: &RelativeSelector,
) -> bool {
    let selector = &relative.selector;
    let Some(first) = selector.compounds.first() else {
        return false;
    };

    let mut current = related_elements(document, anchor, relative.leading_combinator)
        .into_iter()
        .filter(|candidate| compound_matches(document, *candidate, first))
        .collect::<Vec<_>>();

    for (index, combinator) in selector.combinators.iter().copied().enumerate() {
        if current.is_empty() {
            return false;
        }
        let next_compound = &selector.compounds[index + 1];
        let mut next = Vec::new();
        for source in current {
            for candidate in related_elements(document, source, combinator) {
                if compound_matches(document, candidate, next_compound)
                    && !next.contains(&candidate)
                {
                    next.push(candidate);
                }
            }
        }
        current = next;
    }

    !current.is_empty()
}

fn related_elements(document: &Document, node: NodeId, combinator: Combinator) -> Vec<NodeId> {
    match combinator {
        Combinator::Child => document
            .children(node)
            .iter()
            .copied()
            .filter(|candidate| document.element(*candidate).is_some())
            .collect(),
        Combinator::Descendant => {
            let mut result = Vec::new();
            let mut stack = document.children(node).to_vec();
            while let Some(candidate) = stack.pop() {
                if document.element(candidate).is_some() {
                    result.push(candidate);
                }
                stack.extend(document.children(candidate).iter().copied());
            }
            result
        }
        Combinator::AdjacentSibling => next_element_sibling(document, node).into_iter().collect(),
        Combinator::GeneralSibling => following_element_siblings(document, node),
    }
}

fn next_element_sibling(document: &Document, node: NodeId) -> Option<NodeId> {
    following_element_siblings(document, node)
        .into_iter()
        .next()
}

fn following_element_siblings(document: &Document, node: NodeId) -> Vec<NodeId> {
    let Some(parent) = document.node(node).and_then(|current| current.parent) else {
        return Vec::new();
    };
    let siblings = document.children(parent);
    let Some(index) = siblings.iter().position(|candidate| *candidate == node) else {
        return Vec::new();
    };
    siblings[index + 1..]
        .iter()
        .copied()
        .filter(|candidate| document.element(*candidate).is_some())
        .collect()
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

fn language_matches(document: &Document, node: NodeId, ranges: &[String]) -> bool {
    let language = effective_language(document, node).unwrap_or("");
    ranges
        .iter()
        .any(|range| extended_language_range_matches(language, range))
}

fn effective_language(document: &Document, mut node: NodeId) -> Option<&str> {
    loop {
        if let Some(element) = document.element(node)
            && let Some(language) = attribute_value(element, "lang")
        {
            // An explicit empty lang means "unknown", and intentionally stops inheritance.
            return Some(language);
        }
        node = document.node(node)?.parent?;
    }
}

fn extended_language_range_matches(language: &str, range: &str) -> bool {
    if range.is_empty() {
        return language.is_empty();
    }
    if language.is_empty() {
        return false;
    }

    let language: Vec<_> = language.split('-').collect();
    let range: Vec<_> = range.split('-').collect();
    if language.iter().any(|part| part.is_empty()) || range.iter().any(|part| part.is_empty()) {
        return false;
    }

    let first_range = range[0];
    if first_range != "*" && !first_range.eq_ignore_ascii_case(language[0]) {
        return false;
    }

    let mut language_index = 1usize;
    let mut range_index = 1usize;
    while range_index < range.len() {
        let expected = range[range_index];
        if expected == "*" {
            range_index += 1;
            continue;
        }
        if language_index >= language.len() {
            return false;
        }
        let actual = language[language_index];
        if expected.eq_ignore_ascii_case(actual) {
            range_index += 1;
            language_index += 1;
            continue;
        }
        if actual.len() == 1 && actual.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return false;
        }
        language_index += 1;
    }
    true
}

fn direction_matches(document: &Document, node: NodeId, requested: &str) -> bool {
    if !requested.eq_ignore_ascii_case("ltr") && !requested.eq_ignore_ascii_case("rtl") {
        return false;
    }
    effective_direction(document, node).eq_ignore_ascii_case(requested)
}

fn effective_direction(document: &Document, mut node: NodeId) -> &'static str {
    loop {
        if let Some(element) = document.element(node)
            && let Some(direction) = attribute_value(element, "dir")
        {
            if direction.eq_ignore_ascii_case("ltr") {
                return "ltr";
            }
            if direction.eq_ignore_ascii_case("rtl") {
                return "rtl";
            }
            // Invalid values are ignored for directionality and inherit from the ancestor.
        }
        let Some(parent) = document.node(node).and_then(|current| current.parent) else {
            return "ltr";
        };
        node = parent;
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
        PseudoClass::FirstOfType => {
            type_siblings(document, node).and_then(|siblings| siblings.first().copied())
                == Some(node)
        }
        PseudoClass::LastOfType => {
            type_siblings(document, node).and_then(|siblings| siblings.last().copied())
                == Some(node)
        }
        PseudoClass::OnlyOfType => {
            type_siblings(document, node).is_some_and(|siblings| siblings.as_slice() == [node])
        }
        PseudoClass::Empty => document.children(node).iter().all(|child| {
            document.node(*child).is_none_or(|child| match &child.kind {
                NodeKind::Element(_) => false,
                NodeKind::Text(text) => text.is_empty(),
                NodeKind::Document | NodeKind::Comment(_) | NodeKind::DocumentType(_) => true,
            })
        }),
        PseudoClass::Link => document.element(node).is_some_and(|element| {
            element.tag_name.eq_ignore_ascii_case("a") && attribute_value(element, "href").is_some()
        }),
        // Link-history state is intentionally not exposed yet. Treating all links as unvisited
        // preserves privacy while still giving :visited valid selector semantics.
        PseudoClass::Visited => false,
        PseudoClass::Required => document.element(node).is_some_and(|element| {
            supports_required_state(element) && attribute_value(element, "required").is_some()
        }),
        PseudoClass::Optional => document.element(node).is_some_and(|element| {
            supports_required_state(element) && attribute_value(element, "required").is_none()
        }),
        PseudoClass::Open => document.element(node).is_some_and(|element| {
            element.tag_name.eq_ignore_ascii_case("details")
                && attribute_value(element, "open").is_some()
        }),
    }
}

fn supports_required_state(element: &op_dom::ElementData) -> bool {
    if element.tag_name.eq_ignore_ascii_case("select")
        || element.tag_name.eq_ignore_ascii_case("textarea")
    {
        return true;
    }
    if !element.tag_name.eq_ignore_ascii_case("input") {
        return false;
    }
    let input_type = attribute_value(element, "type").unwrap_or("text");
    !matches!(
        input_type.to_ascii_lowercase().as_str(),
        "hidden" | "range" | "color" | "submit" | "reset" | "button" | "image"
    )
}

fn nth_child_matches(document: &Document, node: NodeId, nth: &NthSelector) -> bool {
    let Some(siblings) = (if nth.same_type {
        type_siblings(document, node)
    } else {
        element_siblings(document, node)
    }) else {
        return false;
    };
    let siblings: Vec<_> = siblings
        .into_iter()
        .filter(|candidate| {
            nth.of.is_empty()
                || nth
                    .of
                    .iter()
                    .any(|selector| selector_matches(document, *candidate, selector))
        })
        .collect();
    let Some(position) = siblings.iter().position(|candidate| *candidate == node) else {
        return false;
    };
    let index = if nth.from_end {
        siblings.len() - position
    } else {
        position + 1
    } as i64;
    let a = i64::from(nth.expression.a);
    let b = i64::from(nth.expression.b);
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

fn type_siblings(document: &Document, node: NodeId) -> Option<Vec<NodeId>> {
    let tag = &document.element(node)?.tag_name;
    Some(
        element_siblings(document, node)?
            .into_iter()
            .filter(|candidate| {
                document
                    .element(*candidate)
                    .is_some_and(|element| element.tag_name.eq_ignore_ascii_case(tag))
            })
            .collect(),
    )
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
                NodeKind::Document | NodeKind::Element(_) => {
                    append_descendant_text(document, *child, output)
                }
                NodeKind::Comment(_) | NodeKind::DocumentType(_) => {}
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
    fn matched_declarations_retain_style_link_and_inline_source_nodes() {
        let document = parse_document(
            "<style id='embedded'>p { color:red } p::before { content:'A' } p::first-letter { font-size:2em }</style><link id='linked' rel='stylesheet'><p id='target' style='font-size:20px'>Body</p>",
        );
        let target = element_by_id(&document, "target");
        let embedded = element_by_id(&document, "embedded");
        let linked = element_by_id(&document, "linked");
        let sheets = HashMap::from([(linked, "p { color:blue } p::after { content:'B' }".into())]);
        let collection = collect_author_styles_with_linked(&document, &sheets);
        assert!(collection.errors.is_empty());
        let declarations = collection.styles.declarations_for(target);
        assert_eq!(
            declarations
                .iter()
                .map(|declaration| declaration.style_node)
                .collect::<Vec<_>>(),
            [embedded, linked, target]
        );
        assert_eq!(
            collection
                .styles
                .declarations_for_pseudo(target, PseudoElement::Before)[0]
                .style_node,
            embedded
        );
        assert_eq!(
            collection
                .styles
                .declarations_for_pseudo(target, PseudoElement::After)[0]
                .style_node,
            linked
        );
        assert_eq!(
            collection
                .styles
                .declarations_for_pseudo(target, PseudoElement::FirstLetter)[0]
                .style_node,
            embedded
        );
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
    fn matches_has_from_anchor_across_descendants_children_and_following_siblings() {
        let document = parse_document(
            "<main id='anchor'><section><span class='deep'></span></section><b class='direct'></b></main>
             <aside id='adjacent' class='next'><em class='inside'></em></aside>
             <div id='later' class='later'></div>",
        );
        let anchor = element_by_id(&document, "anchor");
        let adjacent = element_by_id(&document, "adjacent");
        let matches = |node, source: &str| {
            let parsed = parse_stylesheet(&format!("{source} {{ color:red }}"));
            assert!(parsed.errors.is_empty(), "{source}: {:?}", parsed.errors);
            selector_matches(&document, node, &parsed.value.rules[0].selectors[0])
        };

        assert!(matches(anchor, "main:has(.deep)"));
        assert!(matches(anchor, "main:has(> .direct)"));
        assert!(matches(anchor, "main:has(+ aside.next)"));
        assert!(matches(anchor, "main:has(~ .later)"));
        assert!(matches(anchor, "main:has(+ aside .inside)"));
        assert!(matches(anchor, "main:not(:has(.missing))"));
        assert!(!matches(anchor, "main:has(> .deep)"));
        assert!(!matches(adjacent, "aside:has(.deep)"));
    }

    #[test]
    fn matches_language_direction_and_basic_state_pseudos() {
        let document = parse_document(
            "<html lang='en-US' dir='rtl'><body>
               <div id='fr' lang='fr-Latn-FR'><span id='fr-child'></span></div>
               <div id='ltr' dir='ltr'><span id='ltr-child' dir='foopy'></span></div>
               <input id='required' required><input id='optional'>
               <details id='details' open><summary>x</summary></details>
               <a id='link' href='#'>x</a>
             </body></html>",
        );
        let matches = |node, source: &str| {
            let parsed = parse_stylesheet(&format!("{source} {{ color:red }}"));
            assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
            selector_matches(&document, node, &parsed.value.rules[0].selectors[0])
        };

        let fr_child = element_by_id(&document, "fr-child");
        assert!(matches(fr_child, r#":lang("FR")"#));
        assert!(matches(fr_child, r#":lang("*-Latn")"#));
        assert!(matches(fr_child, r#":lang(de, "*-FR")"#));
        assert!(!matches(fr_child, r#":lang("fr-CH")"#));

        let ltr_child = element_by_id(&document, "ltr-child");
        assert!(matches(ltr_child, ":dir(ltr)"));
        assert!(!matches(ltr_child, ":dir(rtl)"));
        assert!(!matches(ltr_child, ":dir(foopy)"));
        assert!(matches(
            first_element_by_tag(&document, "body"),
            ":dir(rtl)"
        ));

        assert!(matches(element_by_id(&document, "required"), ":required"));
        assert!(matches(element_by_id(&document, "optional"), ":optional"));
        assert!(matches(element_by_id(&document, "details"), ":open"));
        assert!(matches(element_by_id(&document, "link"), ":link"));
        assert!(!matches(element_by_id(&document, "link"), ":visited"));
    }

    #[test]
    fn keeps_host_before_and_after_declarations_in_separate_style_buckets() {
        let document = parse_document(
            "<style>
               .note { color:red }
               .note::before { content:'['; color:blue }
               .note::after { content:']'; font-weight:bold }
               .note, .note::before { padding:2px }
             </style><p id='note' class='note'>text</p>",
        );
        let note = element_by_id(&document, "note");
        let collected = collect_author_styles(&document);
        assert!(collected.errors.is_empty(), "{:?}", collected.errors);

        let host = collected.styles.declarations_for(note);
        let before = collected
            .styles
            .declarations_for_pseudo(note, PseudoElement::Before);
        let after = collected
            .styles
            .declarations_for_pseudo(note, PseudoElement::After);
        assert!(
            host.iter()
                .any(|matched| matched.declaration.name == "color")
        );
        assert!(
            host.iter()
                .any(|matched| matched.declaration.name == "padding")
        );
        assert!(
            !host
                .iter()
                .any(|matched| matched.declaration.name == "content")
        );
        assert!(
            before
                .iter()
                .any(|matched| matched.declaration.name == "content")
        );
        assert!(
            before
                .iter()
                .any(|matched| matched.declaration.name == "padding")
        );
        assert!(
            after
                .iter()
                .any(|matched| matched.declaration.name == "content")
        );
        assert!(
            !after
                .iter()
                .any(|matched| matched.declaration.name == "padding")
        );
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
