use crate::{MatchedDeclaration, Specificity, StyleMap, StyleSource, TokenKind};
use op_dom::{Document, NodeId};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Inline,
    Block,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputedFontWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl CssColor {
    pub const BLACK: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 255,
    };
    pub const WHITE: Self = Self {
        red: 255,
        green: 255,
        blue: 255,
        alpha: 255,
    };
    pub const RED: Self = Self {
        red: 255,
        green: 0,
        blue: 0,
        alpha: 255,
    };
    pub const GREEN: Self = Self {
        red: 0,
        green: 128,
        blue: 0,
        alpha: 255,
    };
    pub const BLUE: Self = Self {
        red: 0,
        green: 0,
        blue: 255,
        alpha: 255,
    };
    pub const TRANSPARENT: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComputedStyle {
    pub display: Display,
    pub color: CssColor,
    pub font_size_px: f32,
    pub font_weight: ComputedFontWeight,
}

impl ComputedStyle {
    pub fn initial() -> Self {
        Self {
            display: Display::Inline,
            color: CssColor::BLACK,
            font_size_px: 18.0,
            font_weight: ComputedFontWeight::Normal,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComputedStyleMap {
    entries: HashMap<NodeId, ComputedStyle>,
}

impl ComputedStyleMap {
    pub fn style_for(&self, node: NodeId) -> Option<&ComputedStyle> {
        self.entries.get(&node)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn compute_styles(document: &Document, author_styles: &StyleMap) -> ComputedStyleMap {
    let mut computed = ComputedStyleMap::default();
    for child in document.children(document.root()) {
        compute_subtree(document, *child, None, author_styles, &mut computed);
    }
    computed
}

fn compute_subtree(
    document: &Document,
    node: NodeId,
    parent_style: Option<ComputedStyle>,
    author_styles: &StyleMap,
    computed: &mut ComputedStyleMap,
) {
    let current = document.element(node).map(|element| {
        let mut style = inherited_base(parent_style);
        apply_ua_defaults(&mut style, element.tag_name.as_str());
        apply_author_declarations(
            &mut style,
            parent_style,
            author_styles.declarations_for(node),
        );
        computed.entries.insert(node, style);
        style
    });

    let inherited_parent = current.or(parent_style);
    for child in document.children(node) {
        compute_subtree(document, *child, inherited_parent, author_styles, computed);
    }
}

fn inherited_base(parent: Option<ComputedStyle>) -> ComputedStyle {
    let initial = ComputedStyle::initial();
    match parent {
        Some(parent) => ComputedStyle {
            display: initial.display,
            color: parent.color,
            font_size_px: parent.font_size_px,
            font_weight: parent.font_weight,
        },
        None => initial,
    }
}

fn apply_ua_defaults(style: &mut ComputedStyle, tag: &str) {
    style.display = if hidden_tag(tag) {
        Display::None
    } else if block_tag(tag) {
        Display::Block
    } else {
        Display::Inline
    };

    match tag {
        "h1" => {
            style.font_size_px = 34.0;
            style.font_weight = ComputedFontWeight::Bold;
        }
        "h2" => {
            style.font_size_px = 28.0;
            style.font_weight = ComputedFontWeight::Bold;
        }
        "h3" => {
            style.font_size_px = 23.0;
            style.font_weight = ComputedFontWeight::Bold;
        }
        "h4" | "h5" | "h6" => {
            style.font_size_px = 18.0;
            style.font_weight = ComputedFontWeight::Bold;
        }
        _ => {}
    }
}

fn hidden_tag(tag: &str) -> bool {
    matches!(
        tag,
        "head" | "title" | "style" | "script" | "meta" | "link" | "template"
    )
}

fn block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "html"
            | "body"
            | "div"
            | "main"
            | "article"
            | "section"
            | "nav"
            | "header"
            | "footer"
            | "aside"
            | "ul"
            | "ol"
            | "blockquote"
            | "p"
            | "li"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "pre"
            | "address"
            | "dl"
            | "dt"
            | "dd"
            | "figure"
            | "figcaption"
    )
}

fn apply_author_declarations(
    style: &mut ComputedStyle,
    parent_style: Option<ComputedStyle>,
    declarations: &[MatchedDeclaration],
) {
    if let Some((_, value)) = winning_value(declarations, "display", parse_display) {
        style.display = resolve_display(value, parent_style);
    }
    if let Some((_, value)) = winning_value(declarations, "color", parse_color) {
        style.color = resolve_inherited(
            value,
            parent_style.map(|parent| parent.color),
            CssColor::BLACK,
        );
    }
    if let Some((_, value)) = winning_value(declarations, "font-size", parse_font_size) {
        style.font_size_px = resolve_inherited(
            value,
            parent_style.map(|parent| parent.font_size_px),
            ComputedStyle::initial().font_size_px,
        );
    }
    if let Some((_, value)) = winning_value(declarations, "font-weight", parse_font_weight) {
        style.font_weight = resolve_inherited(
            value,
            parent_style.map(|parent| parent.font_weight),
            ComputedFontWeight::Normal,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Specified<T> {
    Value(T),
    Inherit,
    Initial,
    Unset,
}

fn winning_value<'a, T: Copy>(
    declarations: &'a [MatchedDeclaration],
    property: &str,
    parse: fn(&[TokenKind]) -> Option<Specified<T>>,
) -> Option<(&'a MatchedDeclaration, Specified<T>)> {
    declarations
        .iter()
        .filter(|matched| matched.declaration.name == property)
        .filter_map(|matched| parse(&matched.declaration.value).map(|value| (matched, value)))
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
}

fn cascade_key(matched: &MatchedDeclaration) -> (bool, bool, Specificity, usize) {
    (
        matched.declaration.important,
        matched.source == StyleSource::Inline,
        matched.specificity,
        matched.source_order,
    )
}

fn resolve_display(value: Specified<Display>, parent: Option<ComputedStyle>) -> Display {
    match value {
        Specified::Value(value) => value,
        Specified::Inherit => parent.map_or(Display::Inline, |parent| parent.display),
        Specified::Initial | Specified::Unset => Display::Inline,
    }
}

fn resolve_inherited<T: Copy>(value: Specified<T>, parent: Option<T>, initial: T) -> T {
    match value {
        Specified::Value(value) => value,
        Specified::Inherit | Specified::Unset => parent.unwrap_or(initial),
        Specified::Initial => initial,
    }
}

fn parse_display(tokens: &[TokenKind]) -> Option<Specified<Display>> {
    match single_ident(tokens)?.to_ascii_lowercase().as_str() {
        "inline" => Some(Specified::Value(Display::Inline)),
        "block" => Some(Specified::Value(Display::Block)),
        "none" => Some(Specified::Value(Display::None)),
        "inherit" => Some(Specified::Inherit),
        "initial" => Some(Specified::Initial),
        "unset" => Some(Specified::Unset),
        _ => None,
    }
}

fn parse_font_weight(tokens: &[TokenKind]) -> Option<Specified<ComputedFontWeight>> {
    if let Some(ident) = single_ident(tokens) {
        return match ident.to_ascii_lowercase().as_str() {
            "normal" => Some(Specified::Value(ComputedFontWeight::Normal)),
            "bold" => Some(Specified::Value(ComputedFontWeight::Bold)),
            "inherit" => Some(Specified::Inherit),
            "initial" => Some(Specified::Initial),
            "unset" => Some(Specified::Unset),
            _ => None,
        };
    }

    let TokenKind::Number(number) = single_significant_token(tokens)? else {
        return None;
    };
    match number.as_str() {
        "400" => Some(Specified::Value(ComputedFontWeight::Normal)),
        "700" => Some(Specified::Value(ComputedFontWeight::Bold)),
        _ => None,
    }
}

fn parse_font_size(tokens: &[TokenKind]) -> Option<Specified<f32>> {
    if let Some(ident) = single_ident(tokens) {
        return match ident.to_ascii_lowercase().as_str() {
            "inherit" => Some(Specified::Inherit),
            "initial" => Some(Specified::Initial),
            "unset" => Some(Specified::Unset),
            _ => None,
        };
    }

    let TokenKind::Dimension { number, unit } = single_significant_token(tokens)? else {
        return None;
    };
    if !unit.eq_ignore_ascii_case("px") {
        return None;
    }
    let value = number.parse::<f32>().ok()?;
    (value.is_finite() && (1.0..=4096.0).contains(&value)).then_some(Specified::Value(value))
}

fn parse_color(tokens: &[TokenKind]) -> Option<Specified<CssColor>> {
    if let Some(ident) = single_ident(tokens) {
        return match ident.to_ascii_lowercase().as_str() {
            "black" => Some(Specified::Value(CssColor::BLACK)),
            "white" => Some(Specified::Value(CssColor::WHITE)),
            "red" => Some(Specified::Value(CssColor::RED)),
            "green" => Some(Specified::Value(CssColor::GREEN)),
            "blue" => Some(Specified::Value(CssColor::BLUE)),
            "transparent" => Some(Specified::Value(CssColor::TRANSPARENT)),
            "inherit" => Some(Specified::Inherit),
            "initial" => Some(Specified::Initial),
            "unset" => Some(Specified::Unset),
            _ => None,
        };
    }

    let TokenKind::Hash { value, .. } = single_significant_token(tokens)? else {
        return None;
    };
    parse_hex_color(value).map(Specified::Value)
}

fn parse_hex_color(hex: &str) -> Option<CssColor> {
    fn nibble(ch: u8) -> Option<u8> {
        (ch as char).to_digit(16).map(|digit| digit as u8)
    }
    fn byte(pair: &[u8]) -> Option<u8> {
        Some(nibble(pair[0])? * 16 + nibble(pair[1])?)
    }

    let bytes = hex.as_bytes();
    match bytes.len() {
        3 | 4 => {
            let red = nibble(bytes[0])? * 17;
            let green = nibble(bytes[1])? * 17;
            let blue = nibble(bytes[2])? * 17;
            let alpha = if bytes.len() == 4 {
                nibble(bytes[3])? * 17
            } else {
                255
            };
            Some(CssColor {
                red,
                green,
                blue,
                alpha,
            })
        }
        6 | 8 => Some(CssColor {
            red: byte(&bytes[0..2])?,
            green: byte(&bytes[2..4])?,
            blue: byte(&bytes[4..6])?,
            alpha: if bytes.len() == 8 {
                byte(&bytes[6..8])?
            } else {
                255
            },
        }),
        _ => None,
    }
}

fn single_ident(tokens: &[TokenKind]) -> Option<&str> {
    let TokenKind::Ident(value) = single_significant_token(tokens)? else {
        return None;
    };
    Some(value)
}

fn single_significant_token(tokens: &[TokenKind]) -> Option<&TokenKind> {
    let mut tokens = tokens
        .iter()
        .filter(|token| !matches!(token, TokenKind::Whitespace));
    let first = tokens.next()?;
    tokens.next().is_none().then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collect_author_styles;
    use op_html::parse_document;

    fn find_by_id(document: &Document, id: &str) -> NodeId {
        fn find(document: &Document, node: NodeId, id: &str) -> Option<NodeId> {
            if document.element(node).is_some_and(|element| {
                element
                    .attributes
                    .iter()
                    .any(|attribute| attribute.name == "id" && attribute.value == id)
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
    fn cascade_orders_importance_inline_specificity_and_source_order() {
        let document = parse_document(
            "<style>
                .card { color: red; font-size: 19px }
                #hero { color: blue; font-size: 20px }
                #hero { font-size: 21px }
                div { color: green !important }
             </style>
             <div id='hero' class='card' style='color: white; font-weight: bold'>x</div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let hero = computed.style_for(find_by_id(&document, "hero")).unwrap();

        assert_eq!(hero.color, CssColor::GREEN);
        assert_eq!(hero.font_size_px, 21.0);
        assert_eq!(hero.font_weight, ComputedFontWeight::Bold);
    }

    #[test]
    fn inline_important_beats_stylesheet_important() {
        let document = parse_document(
            "<style>#hero { color: red !important }</style>
             <div id='hero' style='color: #123456 !important'>x</div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let hero = computed.style_for(find_by_id(&document, "hero")).unwrap();

        assert_eq!(
            hero.color,
            CssColor {
                red: 0x12,
                green: 0x34,
                blue: 0x56,
                alpha: 255,
            }
        );
    }

    #[test]
    fn inherited_properties_flow_while_ua_heading_defaults_remain_explicit() {
        let document = parse_document(
            "<div id='parent' style='color:#abc; font-size:22px; font-weight:bold'>
                <span id='child'>x</span>
                <h1 id='heading'>y</h1>
             </div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let child = computed.style_for(find_by_id(&document, "child")).unwrap();
        let heading = computed
            .style_for(find_by_id(&document, "heading"))
            .unwrap();

        assert_eq!(
            child.color,
            CssColor {
                red: 0xaa,
                green: 0xbb,
                blue: 0xcc,
                alpha: 255,
            }
        );
        assert_eq!(child.font_size_px, 22.0);
        assert_eq!(child.font_weight, ComputedFontWeight::Bold);
        assert_eq!(heading.color, child.color);
        assert_eq!(heading.font_size_px, 34.0);
        assert_eq!(heading.font_weight, ComputedFontWeight::Bold);
        assert_eq!(heading.display, Display::Block);
    }

    #[test]
    fn global_keywords_follow_inherited_and_non_inherited_rules() {
        let document = parse_document(
            "<div id='parent' style='color:red; font-size:24px; display:block'>
                <span id='child' style='color:initial; font-size:inherit; font-weight:unset; display:unset'>x</span>
             </div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let child = computed.style_for(find_by_id(&document, "child")).unwrap();

        assert_eq!(child.color, CssColor::BLACK);
        assert_eq!(child.font_size_px, 24.0);
        assert_eq!(child.font_weight, ComputedFontWeight::Normal);
        assert_eq!(child.display, Display::Inline);
    }

    #[test]
    fn invalid_higher_priority_value_does_not_hide_lower_valid_declaration() {
        let document = parse_document(
            "<style>.card { font-size:20px; color:#0f08 } #hero { font-size:huge; color:not-a-color }</style>
             <div id='hero' class='card'>x</div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let hero = computed.style_for(find_by_id(&document, "hero")).unwrap();

        assert_eq!(hero.font_size_px, 20.0);
        assert_eq!(
            hero.color,
            CssColor {
                red: 0,
                green: 255,
                blue: 0,
                alpha: 136,
            }
        );
    }

    #[test]
    fn ua_display_defaults_cover_hidden_block_and_inline_elements() {
        let document = parse_document(
            "<html id='html'><head id='head'><style>x{color:red}</style></head><body><p id='p'><span id='span'>x</span></p></body></html>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);

        assert_eq!(
            computed
                .style_for(find_by_id(&document, "html"))
                .unwrap()
                .display,
            Display::Block
        );
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "head"))
                .unwrap()
                .display,
            Display::None
        );
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "p"))
                .unwrap()
                .display,
            Display::Block
        );
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "span"))
                .unwrap()
                .display,
            Display::Inline
        );
    }
}
