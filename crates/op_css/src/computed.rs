use crate::custom::{contains_var, resolve_custom_values, substitute_vars};
use crate::{MatchedDeclaration, PseudoElement, Specificity, StyleMap, StyleSource, TokenKind};
use op_dom::{Document, ElementData, NodeId};
use std::collections::HashMap;

pub type CustomPropertyMap = HashMap<String, Vec<TokenKind>>;

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
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Start,
    End,
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhiteSpace {
    Normal,
    NoWrap,
    Pre,
    PreWrap,
    PreLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextTransform {
    None,
    Uppercase,
    Lowercase,
    Capitalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDecorationLine {
    pub underline: bool,
    pub line_through: bool,
}

impl TextDecorationLine {
    pub const NONE: Self = Self {
        underline: false,
        line_through: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComputedLineHeight {
    Normal,
    Number(f32),
    Px(f32),
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
pub enum LengthPercentage {
    Px(f32),
    Percent(f32),
}

impl LengthPercentage {
    pub const ZERO: Self = Self::Px(0.0);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarginValue {
    Auto,
    Length(LengthPercentage),
}

impl MarginValue {
    pub const ZERO: Self = Self::Length(LengthPercentage::ZERO);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarginEdges {
    pub top: MarginValue,
    pub right: MarginValue,
    pub bottom: MarginValue,
    pub left: MarginValue,
}

impl MarginEdges {
    pub const ZERO: Self = Self {
        top: MarginValue::ZERO,
        right: MarginValue::ZERO,
        bottom: MarginValue::ZERO,
        left: MarginValue::ZERO,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaddingEdges {
    pub top: LengthPercentage,
    pub right: LengthPercentage,
    pub bottom: LengthPercentage,
    pub left: LengthPercentage,
}

impl PaddingEdges {
    pub const ZERO: Self = Self {
        top: LengthPercentage::ZERO,
        right: LengthPercentage::ZERO,
        bottom: LengthPercentage::ZERO,
        left: LengthPercentage::ZERO,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    None,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComputedBorder {
    pub width_px: f32,
    pub color: CssColor,
    pub style: BorderStyle,
}

impl ComputedBorder {
    pub const NONE: Self = Self {
        // CSS border-width starts at `medium`; border-style:none keeps it invisible.
        width_px: 3.0,
        color: CssColor::BLACK,
        style: BorderStyle::None,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderEdges {
    pub top: ComputedBorder,
    pub right: ComputedBorder,
    pub bottom: ComputedBorder,
    pub left: ComputedBorder,
}

impl BorderEdges {
    pub const NONE: Self = Self {
        top: ComputedBorder::NONE,
        right: ComputedBorder::NONE,
        bottom: ComputedBorder::NONE,
        left: ComputedBorder::NONE,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxSizing {
    ContentBox,
    BorderBox,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComputedStyle {
    pub display: Display,
    pub color: CssColor,
    pub font_size_px: f32,
    pub font_weight: ComputedFontWeight,
    pub font_style: FontStyle,
    pub line_height: ComputedLineHeight,
    pub text_align: TextAlign,
    pub white_space: WhiteSpace,
    pub text_decoration_line: TextDecorationLine,
    pub letter_spacing_px: f32,
    pub word_spacing_px: f32,
    pub text_transform: TextTransform,
    pub background_color: CssColor,
    pub margin: MarginEdges,
    pub padding: PaddingEdges,
    pub border: BorderEdges,
    pub width: Option<LengthPercentage>,
    pub min_width: LengthPercentage,
    pub max_width: Option<LengthPercentage>,
    pub height: Option<LengthPercentage>,
    pub min_height: LengthPercentage,
    pub max_height: Option<LengthPercentage>,
    pub box_sizing: BoxSizing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedPseudoStyle {
    pub style: ComputedStyle,
    pub content: String,
    pub quotes: ComputedQuotes,
}

/// Inherited quotation pairs. `Auto` currently uses deterministic English pairs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ComputedQuotes {
    #[default]
    Auto,
    None,
    Pairs(Vec<(String, String)>),
}

impl ComputedQuotes {
    fn mark(&self, depth: usize, opening: bool) -> &str {
        let pair = match self {
            Self::Auto => {
                return match (depth == 0, opening) {
                    (true, true) => "“",
                    (true, false) => "”",
                    (false, true) => "‘",
                    (false, false) => "’",
                };
            }
            Self::None => return "",
            Self::Pairs(pairs) => pairs.get(depth).or_else(|| pairs.last()),
        };
        pair.map(|(open, close)| {
            if opening {
                open.as_str()
            } else {
                close.as_str()
            }
        })
        .unwrap_or_default()
    }
}

impl ComputedStyle {
    pub fn initial() -> Self {
        Self {
            display: Display::Inline,
            color: CssColor::BLACK,
            font_size_px: 18.0,
            font_weight: ComputedFontWeight::Normal,
            font_style: FontStyle::Normal,
            line_height: ComputedLineHeight::Normal,
            text_align: TextAlign::Start,
            white_space: WhiteSpace::Normal,
            text_decoration_line: TextDecorationLine::NONE,
            letter_spacing_px: 0.0,
            word_spacing_px: 0.0,
            text_transform: TextTransform::None,
            background_color: CssColor::TRANSPARENT,
            margin: MarginEdges::ZERO,
            padding: PaddingEdges::ZERO,
            border: BorderEdges::NONE,
            width: None,
            min_width: LengthPercentage::ZERO,
            max_width: None,
            height: None,
            min_height: LengthPercentage::ZERO,
            max_height: None,
            box_sizing: BoxSizing::ContentBox,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComputedStyleMap {
    entries: HashMap<NodeId, ComputedStyle>,
    pseudo_entries: HashMap<(NodeId, PseudoElement), ComputedPseudoStyle>,
    custom_properties: HashMap<NodeId, CustomPropertyMap>,
    pseudo_custom_properties: HashMap<(NodeId, PseudoElement), CustomPropertyMap>,
    quotes: HashMap<NodeId, ComputedQuotes>,
}

impl ComputedStyleMap {
    pub fn quotes_for(&self, node: NodeId) -> Option<&ComputedQuotes> {
        self.quotes.get(&node)
    }

    pub fn style_for(&self, node: NodeId) -> Option<&ComputedStyle> {
        self.entries.get(&node)
    }

    pub fn pseudo_style_for(
        &self,
        node: NodeId,
        pseudo: PseudoElement,
    ) -> Option<&ComputedPseudoStyle> {
        self.pseudo_entries.get(&(node, pseudo))
    }

    pub fn custom_properties_for(&self, node: NodeId) -> Option<&CustomPropertyMap> {
        self.custom_properties.get(&node)
    }

    pub fn pseudo_custom_properties_for(
        &self,
        node: NodeId,
        pseudo: PseudoElement,
    ) -> Option<&CustomPropertyMap> {
        self.pseudo_custom_properties.get(&(node, pseudo))
    }

    pub fn len(&self) -> usize {
        self.entries.len().saturating_add(self.pseudo_entries.len())
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.pseudo_entries.is_empty()
    }
}

#[derive(Debug, Default)]
struct CounterContext {
    stacks: HashMap<String, Vec<i32>>,
}

impl CounterContext {
    fn snapshot_lengths(&self) -> HashMap<String, usize> {
        self.stacks
            .iter()
            .map(|(name, stack)| (name.clone(), stack.len()))
            .collect()
    }

    fn restore_lengths(&mut self, snapshot: &HashMap<String, usize>) {
        self.stacks.retain(|name, stack| {
            let Some(length) = snapshot.get(name).copied() else {
                return false;
            };
            stack.truncate(length);
            true
        });
    }

    fn reset(&mut self, name: &str, value: i32) {
        self.stacks.entry(name.to_owned()).or_default().push(value);
    }

    fn set(&mut self, name: &str, value: i32) {
        let stack = self.stacks.entry(name.to_owned()).or_default();
        if let Some(current) = stack.last_mut() {
            *current = value;
        } else {
            stack.push(value);
        }
    }

    fn increment(&mut self, name: &str, amount: i32) {
        let stack = self.stacks.entry(name.to_owned()).or_default();
        if stack.is_empty() {
            stack.push(0);
        }
        if let Some(current) = stack.last_mut() {
            *current = current.saturating_add(amount);
        }
    }

    fn current(&self, name: &str) -> i32 {
        self.stacks
            .get(name)
            .and_then(|stack| stack.last())
            .copied()
            .unwrap_or(0)
    }

    fn values(&self, name: &str) -> Vec<i32> {
        self.stacks.get(name).cloned().unwrap_or_else(|| vec![0])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CounterOperation {
    name: String,
    value: i32,
}

#[derive(Debug, Default)]
struct GeneratedContext {
    counters: CounterContext,
    quote_depth: usize,
    suppressed: bool,
}

pub fn compute_styles(document: &Document, author_styles: &StyleMap) -> ComputedStyleMap {
    let mut computed = ComputedStyleMap::default();
    let mut generated = GeneratedContext::default();
    for child in document.children(document.root()) {
        compute_subtree(
            document,
            *child,
            None,
            None,
            author_styles,
            &mut generated,
            &mut computed,
        );
    }
    computed
}

fn compute_subtree(
    document: &Document,
    node: NodeId,
    parent_style: Option<ComputedStyle>,
    parent_custom: Option<CustomPropertyMap>,
    author_styles: &StyleMap,
    generated: &mut GeneratedContext,
    computed: &mut ComputedStyleMap,
) {
    let was_suppressed = generated.suppressed;
    let current = document.element(node).map(|element| {
        let declarations = author_styles.declarations_for(node);
        let custom = compute_custom_properties(parent_custom.as_ref(), declarations);
        let resolved = substitute_declarations(declarations, &custom);
        let mut style = inherited_base(parent_style);
        apply_ua_defaults(&mut style, element.tag_name.as_str());
        apply_author_declarations(&mut style, parent_style, &resolved);
        generated.suppressed |= style.display == Display::None;
        if !generated.suppressed {
            apply_counter_declarations(&mut generated.counters, &resolved);
        }
        let parent_quotes = document
            .node(node)
            .and_then(|node| node.parent)
            .and_then(|parent| computed.quotes_for(parent));
        let quotes = compute_quotes(parent_quotes, &resolved);
        computed.entries.insert(node, style);
        computed.custom_properties.insert(node, custom.clone());
        computed.quotes.insert(node, quotes.clone());
        compute_pseudo_style(
            node,
            PseudoElement::Before,
            PseudoHost {
                element,
                style,
                custom: &custom,
                quotes: &quotes,
            },
            author_styles,
            generated,
            computed,
        );
        (style, custom)
    });

    let inherited_style = current.as_ref().map(|(style, _)| *style).or(parent_style);
    let inherited_custom = current
        .as_ref()
        .map(|(_, custom)| custom.clone())
        .or(parent_custom);

    let child_scope = generated.counters.snapshot_lengths();
    for child in document.children(node) {
        compute_subtree(
            document,
            *child,
            inherited_style,
            inherited_custom.clone(),
            author_styles,
            generated,
            computed,
        );
    }
    generated.counters.restore_lengths(&child_scope);

    if let Some((style, custom)) = current.as_ref()
        && let Some(element) = document.element(node)
    {
        compute_pseudo_style(
            node,
            PseudoElement::After,
            PseudoHost {
                element,
                style: *style,
                custom,
                quotes: computed.quotes_for(node).cloned().as_ref().unwrap(),
            },
            author_styles,
            generated,
            computed,
        );
    }
    generated.suppressed = was_suppressed;
}

struct PseudoHost<'a> {
    element: &'a ElementData,
    style: ComputedStyle,
    custom: &'a CustomPropertyMap,
    quotes: &'a ComputedQuotes,
}

fn compute_pseudo_style(
    node: NodeId,
    pseudo: PseudoElement,
    host: PseudoHost<'_>,
    author_styles: &StyleMap,
    generated: &mut GeneratedContext,
    computed: &mut ComputedStyleMap,
) {
    if generated.suppressed || matches!(host.element.tag_name.as_str(), "img" | "br") {
        return;
    }
    let declarations = author_styles.declarations_for_pseudo(node, pseudo);
    let custom = compute_custom_properties(Some(host.custom), declarations);
    let resolved = substitute_declarations(declarations, &custom);

    let mut style = inherited_base(Some(host.style));
    apply_author_declarations(&mut style, Some(host.style), &resolved);
    if style.display == Display::None {
        return;
    }
    let quotes = compute_quotes(Some(host.quotes), &resolved);
    // Validate candidates without mutating document-order quote or counter state.
    let tokens = winning_generated_content(&resolved, host.element, &generated.counters);
    let ua_content = [TokenKind::Ident(
        match pseudo {
            PseudoElement::Before => "open-quote",
            PseudoElement::After => "close-quote",
        }
        .to_owned(),
    )];
    let tokens = match tokens {
        Some(tokens) => tokens,
        None if host.element.tag_name == "q" => &ua_content,
        None => return,
    };
    if !matches!(
        parse_generated_content(tokens, host.element, &generated.counters),
        Some(Some(_))
    ) {
        return;
    }
    let pseudo_scope = generated.counters.snapshot_lengths();
    apply_counter_declarations(&mut generated.counters, &resolved);
    let pieces = parse_generated_content(tokens, host.element, &generated.counters)
        .flatten()
        .expect("validated generated content");
    generated.counters.restore_lengths(&pseudo_scope);
    let content = render_generated_content(pieces, &quotes, &mut generated.quote_depth);
    computed.pseudo_entries.insert(
        (node, pseudo),
        ComputedPseudoStyle {
            style,
            content,
            quotes,
        },
    );
    computed
        .pseudo_custom_properties
        .insert((node, pseudo), custom);
}

fn compute_quotes(
    parent: Option<&ComputedQuotes>,
    declarations: &[MatchedDeclaration],
) -> ComputedQuotes {
    let inherited = parent.cloned().unwrap_or_default();
    declarations
        .iter()
        .filter(|matched| matched.declaration.name == "quotes")
        .filter_map(|matched| {
            parse_quotes(&matched.declaration.value, &inherited)
                .or_else(|| matched.value_from_var.then(|| inherited.clone()))
                .map(|value| (matched, value))
        })
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
        .map(|(_, value)| value)
        .unwrap_or(inherited)
}

fn parse_quotes(tokens: &[TokenKind], inherited: &ComputedQuotes) -> Option<ComputedQuotes> {
    if let Some(keyword) = single_ident(tokens) {
        return match keyword.to_ascii_lowercase().as_str() {
            "auto" | "initial" => Some(ComputedQuotes::Auto),
            "none" => Some(ComputedQuotes::None),
            "inherit" | "unset" => Some(inherited.clone()),
            _ => None,
        };
    }
    let strings = significant_tokens(tokens)
        .map(|token| match token {
            TokenKind::String(value) => Some(value.clone()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    if strings.is_empty() || strings.len() % 2 != 0 {
        return None;
    }
    Some(ComputedQuotes::Pairs(
        strings
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| (pair[0].clone(), pair[1].clone()))
            .collect(),
    ))
}

fn apply_counter_declarations(counters: &mut CounterContext, declarations: &[MatchedDeclaration]) {
    if let Some(operations) = winning_counter_operations(declarations, "counter-reset", 0) {
        for operation in operations {
            counters.reset(&operation.name, operation.value);
        }
    }
    if let Some(operations) = winning_counter_operations(declarations, "counter-set", 0) {
        for operation in operations {
            counters.set(&operation.name, operation.value);
        }
    }
    if let Some(operations) = winning_counter_operations(declarations, "counter-increment", 1) {
        for operation in operations {
            counters.increment(&operation.name, operation.value);
        }
    }
}

fn winning_counter_operations(
    declarations: &[MatchedDeclaration],
    property: &str,
    default_value: i32,
) -> Option<Vec<CounterOperation>> {
    declarations
        .iter()
        .filter(|matched| matched.declaration.name == property)
        .filter_map(|matched| {
            parse_counter_operations(&matched.declaration.value, default_value)
                .or_else(|| matched.value_from_var.then(Vec::new))
                .map(|value| (matched, value))
        })
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
        .map(|(_, value)| value)
}

fn parse_counter_operations(
    tokens: &[TokenKind],
    default_value: i32,
) -> Option<Vec<CounterOperation>> {
    if let Some(keyword) = single_ident(tokens) {
        if matches!(
            keyword.to_ascii_lowercase().as_str(),
            "none" | "initial" | "unset"
        ) {
            return Some(Vec::new());
        }
        if keyword.eq_ignore_ascii_case("inherit") {
            return None;
        }
    }

    let tokens: Vec<&TokenKind> = significant_tokens(tokens).collect();
    let mut operations = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let TokenKind::Ident(name) = tokens[index] else {
            return None;
        };
        if is_reserved_counter_name(name) {
            return None;
        }
        index += 1;

        let value = if let Some(TokenKind::Number(number)) = tokens.get(index).copied() {
            index += 1;
            parse_counter_integer(number)?
        } else {
            default_value
        };
        operations.push(CounterOperation {
            name: name.clone(),
            value,
        });
    }
    (!operations.is_empty()).then_some(operations)
}

fn parse_counter_integer(number: &str) -> Option<i32> {
    let value = number.parse::<i64>().ok()?;
    i32::try_from(value).ok()
}

fn is_reserved_counter_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "none" | "initial" | "inherit" | "unset" | "revert" | "revert-layer"
    )
}

fn compute_custom_properties(
    parent: Option<&CustomPropertyMap>,
    declarations: &[MatchedDeclaration],
) -> CustomPropertyMap {
    let mut raw = parent.cloned().unwrap_or_default();
    let mut winners: HashMap<&str, &MatchedDeclaration> = HashMap::new();
    for matched in declarations
        .iter()
        .filter(|matched| matched.declaration.name.starts_with("--"))
    {
        winners
            .entry(matched.declaration.name.as_str())
            .and_modify(|winner| {
                if cascade_key(matched) > cascade_key(winner) {
                    *winner = matched;
                }
            })
            .or_insert(matched);
    }

    for (name, winner) in winners {
        match custom_property_keyword(&winner.declaration.value) {
            Some(CustomPropertyKeyword::Initial) => {
                raw.remove(name);
            }
            Some(CustomPropertyKeyword::Inherit) => {
                if let Some(value) = parent.and_then(|parent| parent.get(name)) {
                    raw.insert(name.to_owned(), value.clone());
                } else {
                    raw.remove(name);
                }
            }
            None => {
                raw.insert(name.to_owned(), winner.declaration.value.clone());
            }
        }
    }

    resolve_custom_values(&raw)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CustomPropertyKeyword {
    Initial,
    Inherit,
}

fn custom_property_keyword(tokens: &[TokenKind]) -> Option<CustomPropertyKeyword> {
    match single_ident(tokens)?.to_ascii_lowercase().as_str() {
        "initial" => Some(CustomPropertyKeyword::Initial),
        "inherit" | "unset" => Some(CustomPropertyKeyword::Inherit),
        _ => None,
    }
}

fn substitute_declarations(
    declarations: &[MatchedDeclaration],
    custom: &CustomPropertyMap,
) -> Vec<MatchedDeclaration> {
    declarations
        .iter()
        .filter(|matched| !matched.declaration.name.starts_with("--"))
        .filter_map(|matched| {
            let from_var = contains_var(&matched.declaration.value);
            let value = substitute_vars(&matched.declaration.value, |name| {
                custom.get(name).map(Vec::as_slice)
            });
            if value.is_none() && !from_var {
                return None;
            }
            let mut resolved = matched.clone();
            // Empty normal-property tokens represent failed substitution. The property
            // parser converts their retained candidate to unset, preserving cascade priority.
            resolved.declaration.value = value.unwrap_or_default();
            resolved.value_from_var = from_var;
            Some(resolved)
        })
        .collect()
}

fn trim_token_whitespace(mut tokens: &[TokenKind]) -> &[TokenKind] {
    while matches!(tokens.first(), Some(TokenKind::Whitespace)) {
        tokens = &tokens[1..];
    }
    while matches!(tokens.last(), Some(TokenKind::Whitespace)) {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

fn winning_generated_content<'a>(
    declarations: &'a [MatchedDeclaration],
    element: &ElementData,
    counters: &CounterContext,
) -> Option<&'a [TokenKind]> {
    declarations
        .iter()
        .filter(|matched| matched.declaration.name == "content")
        .filter_map(|matched| {
            parse_generated_content(&matched.declaration.value, element, counters)
                .or_else(|| matched.value_from_var.then_some(None))
                .map(|value| (matched, value))
        })
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
        .map(|(matched, _)| matched.declaration.value.as_slice())
}

#[derive(Debug)]
enum ContentPiece {
    Text(String),
    Quote(QuoteAction),
}

#[derive(Debug, Clone, Copy)]
enum QuoteAction {
    Open,
    Close,
    NoOpen,
    NoClose,
}

fn render_generated_content(
    pieces: Vec<ContentPiece>,
    quotes: &ComputedQuotes,
    depth: &mut usize,
) -> String {
    let mut text = String::new();
    for piece in pieces {
        match piece {
            ContentPiece::Text(value) => text.push_str(&value),
            ContentPiece::Quote(QuoteAction::Open) => {
                text.push_str(quotes.mark(*depth, true));
                *depth = depth.saturating_add(1);
            }
            ContentPiece::Quote(QuoteAction::NoOpen) => *depth = depth.saturating_add(1),
            ContentPiece::Quote(QuoteAction::Close) if *depth > 0 => {
                *depth -= 1;
                text.push_str(quotes.mark(*depth, false));
            }
            ContentPiece::Quote(QuoteAction::NoClose) => *depth = depth.saturating_sub(1),
            ContentPiece::Quote(QuoteAction::Close) => {}
        }
    }
    text
}

fn parse_generated_content(
    tokens: &[TokenKind],
    element: &ElementData,
    counters: &CounterContext,
) -> Option<Option<Vec<ContentPiece>>> {
    if let Some(ident) = single_ident(tokens)
        && matches!(
            ident.to_ascii_lowercase().as_str(),
            "none" | "normal" | "inherit" | "initial" | "unset"
        )
    {
        return Some(None);
    }

    let mut content = Vec::new();
    let mut saw_piece = false;
    let mut index = 0;
    while index < tokens.len() {
        if matches!(tokens[index], TokenKind::Whitespace) {
            index += 1;
            continue;
        }

        match &tokens[index] {
            TokenKind::String(value) => {
                content.push(ContentPiece::Text(value.clone()));
                saw_piece = true;
                index += 1;
            }
            TokenKind::Function(name) => {
                let end = find_function_end(tokens, index)?;
                let arguments = &tokens[index + 1..end];
                let piece = match name.to_ascii_lowercase().as_str() {
                    "attr" => generated_attr(arguments, element)?,
                    "counter" => generated_counter(arguments, counters)?,
                    "counters" => generated_counters(arguments, counters)?,
                    _ => return None,
                };
                content.push(ContentPiece::Text(piece));
                saw_piece = true;
                index = end + 1;
            }
            TokenKind::Ident(name) => {
                let action = match name.to_ascii_lowercase().as_str() {
                    "open-quote" => QuoteAction::Open,
                    "close-quote" => QuoteAction::Close,
                    "no-open-quote" => QuoteAction::NoOpen,
                    "no-close-quote" => QuoteAction::NoClose,
                    _ => return None,
                };
                content.push(ContentPiece::Quote(action));
                saw_piece = true;
                index += 1;
            }
            _ => return None,
        }
    }

    saw_piece.then_some(Some(content))
}

fn find_function_end(tokens: &[TokenKind], start: usize) -> Option<usize> {
    let mut depth = 1_u32;
    for (index, token) in tokens.iter().enumerate().skip(start + 1) {
        match token {
            TokenKind::Function(_) | TokenKind::OpenParen => depth = depth.saturating_add(1),
            TokenKind::CloseParen => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn generated_attr(tokens: &[TokenKind], element: &ElementData) -> Option<String> {
    let tokens = trim_token_whitespace(tokens);
    let [TokenKind::Ident(name)] = tokens else {
        return None;
    };
    Some(
        element
            .attributes
            .iter()
            .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
            .map(|attribute| attribute.value.clone())
            .unwrap_or_default(),
    )
}

fn generated_counter(tokens: &[TokenKind], counters: &CounterContext) -> Option<String> {
    let arguments = split_function_arguments(tokens)?;
    if !(1..=2).contains(&arguments.len()) {
        return None;
    }
    let name = single_argument_ident(arguments[0])?;
    if is_reserved_counter_name(name) {
        return None;
    }
    let style = if arguments.len() == 2 {
        parse_counter_style(single_argument_ident(arguments[1])?)?
    } else {
        CounterStyle::Decimal
    };
    Some(format_counter(counters.current(name), style))
}

fn generated_counters(tokens: &[TokenKind], counters: &CounterContext) -> Option<String> {
    let arguments = split_function_arguments(tokens)?;
    if !(2..=3).contains(&arguments.len()) {
        return None;
    }
    let name = single_argument_ident(arguments[0])?;
    if is_reserved_counter_name(name) {
        return None;
    }
    let separator_tokens = trim_token_whitespace(arguments[1]);
    let [TokenKind::String(separator)] = separator_tokens else {
        return None;
    };
    let style = if arguments.len() == 3 {
        parse_counter_style(single_argument_ident(arguments[2])?)?
    } else {
        CounterStyle::Decimal
    };
    Some(
        counters
            .values(name)
            .into_iter()
            .map(|value| format_counter(value, style))
            .collect::<Vec<_>>()
            .join(separator),
    )
}

fn split_function_arguments(tokens: &[TokenKind]) -> Option<Vec<&[TokenKind]>> {
    let mut arguments = Vec::new();
    let mut start = 0;
    let mut depth = 0_u32;
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenKind::Function(_) | TokenKind::OpenParen => depth = depth.saturating_add(1),
            TokenKind::CloseParen => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
            }
            TokenKind::Comma if depth == 0 => {
                let argument = trim_token_whitespace(&tokens[start..index]);
                if argument.is_empty() {
                    return None;
                }
                arguments.push(argument);
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    let final_argument = trim_token_whitespace(&tokens[start..]);
    if final_argument.is_empty() {
        return None;
    }
    arguments.push(final_argument);
    Some(arguments)
}

fn single_argument_ident(tokens: &[TokenKind]) -> Option<&str> {
    let [TokenKind::Ident(value)] = trim_token_whitespace(tokens) else {
        return None;
    };
    Some(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CounterStyle {
    Decimal,
    DecimalLeadingZero,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

fn parse_counter_style(name: &str) -> Option<CounterStyle> {
    match name.to_ascii_lowercase().as_str() {
        "decimal" => Some(CounterStyle::Decimal),
        "decimal-leading-zero" => Some(CounterStyle::DecimalLeadingZero),
        "lower-alpha" | "lower-latin" => Some(CounterStyle::LowerAlpha),
        "upper-alpha" | "upper-latin" => Some(CounterStyle::UpperAlpha),
        "lower-roman" => Some(CounterStyle::LowerRoman),
        "upper-roman" => Some(CounterStyle::UpperRoman),
        _ => None,
    }
}

fn format_counter(value: i32, style: CounterStyle) -> String {
    match style {
        CounterStyle::Decimal => value.to_string(),
        CounterStyle::DecimalLeadingZero => {
            if (0..=9).contains(&value) {
                format!("0{value}")
            } else if (-9..=-1).contains(&value) {
                format!("-0{}", value.unsigned_abs())
            } else {
                value.to_string()
            }
        }
        CounterStyle::LowerAlpha => format_alpha_counter(value, false),
        CounterStyle::UpperAlpha => format_alpha_counter(value, true),
        CounterStyle::LowerRoman => format_roman_counter(value, false),
        CounterStyle::UpperRoman => format_roman_counter(value, true),
    }
}

fn format_alpha_counter(value: i32, uppercase: bool) -> String {
    if value <= 0 {
        return value.to_string();
    }

    let mut value = i64::from(value);
    let mut chars = Vec::new();
    while value > 0 {
        value -= 1;
        let ch = (b'a' + (value % 26) as u8) as char;
        chars.push(if uppercase {
            ch.to_ascii_uppercase()
        } else {
            ch
        });
        value /= 26;
    }
    chars.into_iter().rev().collect()
}

fn format_roman_counter(value: i32, uppercase: bool) -> String {
    if !(1..=3999).contains(&value) {
        return value.to_string();
    }

    const ROMAN: &[(i32, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut remaining = value;
    let mut output = String::new();
    for &(amount, numeral) in ROMAN {
        while remaining >= amount {
            remaining -= amount;
            output.push_str(numeral);
        }
    }
    if uppercase {
        output
    } else {
        output.to_ascii_lowercase()
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
            font_style: parent.font_style,
            line_height: parent.line_height,
            text_align: parent.text_align,
            white_space: parent.white_space,
            text_decoration_line: parent.text_decoration_line,
            letter_spacing_px: parent.letter_spacing_px,
            word_spacing_px: parent.word_spacing_px,
            text_transform: parent.text_transform,
            background_color: initial.background_color,
            margin: initial.margin,
            padding: initial.padding,
            border: initial.border,
            width: initial.width,
            min_width: initial.min_width,
            max_width: initial.max_width,
            height: initial.height,
            min_height: initial.min_height,
            max_height: initial.max_height,
            box_sizing: initial.box_sizing,
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
            style.margin.top = MarginValue::Length(LengthPercentage::Px(8.0));
            style.margin.bottom = MarginValue::Length(LengthPercentage::Px(16.0));
        }
        "h2" => {
            style.font_size_px = 28.0;
            style.font_weight = ComputedFontWeight::Bold;
            style.margin.top = MarginValue::Length(LengthPercentage::Px(8.0));
            style.margin.bottom = MarginValue::Length(LengthPercentage::Px(14.0));
        }
        "h3" => {
            style.font_size_px = 23.0;
            style.font_weight = ComputedFontWeight::Bold;
            style.margin.top = MarginValue::Length(LengthPercentage::Px(6.0));
            style.margin.bottom = MarginValue::Length(LengthPercentage::Px(12.0));
        }
        "h4" | "h5" | "h6" => {
            style.font_size_px = 18.0;
            style.font_weight = ComputedFontWeight::Bold;
            style.margin.top = MarginValue::Length(LengthPercentage::Px(6.0));
            style.margin.bottom = MarginValue::Length(LengthPercentage::Px(12.0));
        }
        "p" | "li" => {
            style.margin.bottom = MarginValue::Length(LengthPercentage::Px(12.0));
        }
        "b" | "strong" => style.font_weight = ComputedFontWeight::Bold,
        "i" | "em" => style.font_style = FontStyle::Italic,
        "u" => style.text_decoration_line.underline = true,
        "s" | "strike" | "del" => style.text_decoration_line.line_through = true,
        "pre" => style.white_space = WhiteSpace::Pre,
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

    let parent_font_size = parent_style
        .map(|parent| parent.font_size_px)
        .unwrap_or(ComputedStyle::initial().font_size_px);
    if let Some((_, value)) = winning_value(declarations, "font-size", |tokens| {
        parse_font_size(tokens, parent_font_size)
    }) {
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
    if let Some((_, value)) = winning_value(declarations, "font-style", parse_font_style) {
        style.font_style = resolve_inherited(
            value,
            parent_style.map(|parent| parent.font_style),
            FontStyle::Normal,
        );
    }
    if let Some((_, value)) = winning_value(declarations, "line-height", |tokens| {
        parse_line_height(tokens, style.font_size_px)
    }) {
        style.line_height = resolve_inherited(
            value,
            parent_style.map(|parent| parent.line_height),
            ComputedLineHeight::Normal,
        );
    }
    if let Some((_, value)) = winning_value(declarations, "text-align", parse_text_align) {
        style.text_align = resolve_inherited(
            value,
            parent_style.map(|parent| parent.text_align),
            TextAlign::Start,
        );
    }
    if let Some((_, value)) = winning_value(declarations, "white-space", parse_white_space) {
        style.white_space = resolve_inherited(
            value,
            parent_style.map(|parent| parent.white_space),
            WhiteSpace::Normal,
        );
    }
    if let Some((_, value)) = winning_text_decoration(declarations) {
        style.text_decoration_line = resolve_inherited(
            value,
            parent_style.map(|parent| parent.text_decoration_line),
            TextDecorationLine::NONE,
        );
    }
    if let Some((_, value)) = winning_value(declarations, "letter-spacing", |tokens| {
        parse_spacing(tokens, style.font_size_px, false)
    }) {
        style.letter_spacing_px = resolve_inherited(
            value,
            parent_style.map(|parent| parent.letter_spacing_px),
            0.0,
        );
    }
    if let Some((_, value)) = winning_value(declarations, "word-spacing", |tokens| {
        parse_spacing(tokens, style.font_size_px, true)
    }) {
        style.word_spacing_px = resolve_inherited(
            value,
            parent_style.map(|parent| parent.word_spacing_px),
            0.0,
        );
    }
    if let Some((_, value)) = winning_value(declarations, "text-transform", parse_text_transform) {
        style.text_transform = resolve_inherited(
            value,
            parent_style.map(|parent| parent.text_transform),
            TextTransform::None,
        );
    }
    if let Some((_, value)) = winning_background_color(declarations) {
        style.background_color = resolve_non_inherited(
            value,
            parent_style.map(|parent| parent.background_color),
            CssColor::TRANSPARENT,
        );
    }

    if let Some((_, value)) = winning_value(declarations, "box-sizing", parse_box_sizing) {
        style.box_sizing = resolve_non_inherited(
            value,
            parent_style.map(|parent| parent.box_sizing),
            BoxSizing::ContentBox,
        );
    }

    apply_margin_declarations(style, parent_style, declarations);
    apply_padding_declarations(style, parent_style, declarations);
    apply_border_declarations(style, parent_style, declarations);

    style.width = resolve_optional_size_property(
        declarations,
        "width",
        style.font_size_px,
        parent_style.map(|parent| parent.width),
        None,
        true,
    );
    style.min_width = resolve_size_property(
        declarations,
        "min-width",
        style.font_size_px,
        parent_style.map(|parent| parent.min_width),
        LengthPercentage::ZERO,
    );
    style.max_width = resolve_optional_size_property(
        declarations,
        "max-width",
        style.font_size_px,
        parent_style.map(|parent| parent.max_width),
        None,
        false,
    );
    style.height = resolve_optional_size_property(
        declarations,
        "height",
        style.font_size_px,
        parent_style.map(|parent| parent.height),
        None,
        true,
    );
    style.min_height = resolve_size_property(
        declarations,
        "min-height",
        style.font_size_px,
        parent_style.map(|parent| parent.min_height),
        LengthPercentage::ZERO,
    );
    style.max_height = resolve_optional_size_property(
        declarations,
        "max-height",
        style.font_size_px,
        parent_style.map(|parent| parent.max_height),
        None,
        false,
    );
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Specified<T> {
    Value(T),
    Inherit,
    Initial,
    Unset,
}

fn parsed_or_unset<T>(
    matched: &MatchedDeclaration,
    parsed: Option<Specified<T>>,
) -> Option<Specified<T>> {
    parsed.or_else(|| matched.value_from_var.then_some(Specified::Unset))
}

fn winning_value<'a, T: Copy, F>(
    declarations: &'a [MatchedDeclaration],
    property: &str,
    parse: F,
) -> Option<(&'a MatchedDeclaration, Specified<T>)>
where
    F: Fn(&[TokenKind]) -> Option<Specified<T>>,
{
    declarations
        .iter()
        .filter(|matched| matched.declaration.name == property)
        .filter_map(|matched| {
            parsed_or_unset(matched, parse(&matched.declaration.value))
                .map(|value| (matched, value))
        })
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

fn resolve_non_inherited<T: Copy>(value: Specified<T>, parent: Option<T>, initial: T) -> T {
    match value {
        Specified::Value(value) => value,
        Specified::Inherit => parent.unwrap_or(initial),
        Specified::Initial | Specified::Unset => initial,
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

fn parse_font_style(tokens: &[TokenKind]) -> Option<Specified<FontStyle>> {
    match single_ident(tokens)?.to_ascii_lowercase().as_str() {
        "normal" => Some(Specified::Value(FontStyle::Normal)),
        "italic" => Some(Specified::Value(FontStyle::Italic)),
        "oblique" => Some(Specified::Value(FontStyle::Oblique)),
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
            "bolder" => Some(Specified::Value(ComputedFontWeight::Bold)),
            "lighter" => Some(Specified::Value(ComputedFontWeight::Normal)),
            "inherit" => Some(Specified::Inherit),
            "initial" => Some(Specified::Initial),
            "unset" => Some(Specified::Unset),
            _ => None,
        };
    }

    let TokenKind::Number(number) = single_significant_token(tokens)? else {
        return None;
    };
    let value = parse_number(number)?;
    if value.fract() != 0.0 || !(1.0..=1000.0).contains(&value) {
        return None;
    }
    let weight = if value < 550.0 {
        ComputedFontWeight::Normal
    } else {
        ComputedFontWeight::Bold
    };
    Some(Specified::Value(weight))
}

fn parse_white_space(tokens: &[TokenKind]) -> Option<Specified<WhiteSpace>> {
    match single_ident(tokens)?.to_ascii_lowercase().as_str() {
        "normal" => Some(Specified::Value(WhiteSpace::Normal)),
        "nowrap" => Some(Specified::Value(WhiteSpace::NoWrap)),
        "pre" => Some(Specified::Value(WhiteSpace::Pre)),
        "pre-wrap" => Some(Specified::Value(WhiteSpace::PreWrap)),
        "pre-line" => Some(Specified::Value(WhiteSpace::PreLine)),
        "inherit" => Some(Specified::Inherit),
        "initial" => Some(Specified::Initial),
        "unset" => Some(Specified::Unset),
        _ => None,
    }
}

fn winning_text_decoration(
    declarations: &[MatchedDeclaration],
) -> Option<(&MatchedDeclaration, Specified<TextDecorationLine>)> {
    declarations
        .iter()
        .filter_map(|matched| {
            if !matches!(
                matched.declaration.name.as_str(),
                "text-decoration" | "text-decoration-line"
            ) {
                return None;
            }
            parsed_or_unset(
                matched,
                parse_text_decoration_line(&matched.declaration.value),
            )
            .map(|value| (matched, value))
        })
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
}

fn parse_text_decoration_line(tokens: &[TokenKind]) -> Option<Specified<TextDecorationLine>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| TextDecorationLine::NONE));
    }
    if single_ident(tokens).is_some_and(|value| value.eq_ignore_ascii_case("none")) {
        return Some(Specified::Value(TextDecorationLine::NONE));
    }

    let mut decoration = TextDecorationLine::NONE;
    let mut saw_value = false;
    for token in significant_tokens(tokens) {
        let TokenKind::Ident(value) = token else {
            return None;
        };
        match value.to_ascii_lowercase().as_str() {
            "underline" if !decoration.underline => decoration.underline = true,
            "line-through" if !decoration.line_through => decoration.line_through = true,
            _ => return None,
        }
        saw_value = true;
    }
    saw_value.then_some(Specified::Value(decoration))
}

fn parse_text_transform(tokens: &[TokenKind]) -> Option<Specified<TextTransform>> {
    match single_ident(tokens)?.to_ascii_lowercase().as_str() {
        "none" => Some(Specified::Value(TextTransform::None)),
        "uppercase" => Some(Specified::Value(TextTransform::Uppercase)),
        "lowercase" => Some(Specified::Value(TextTransform::Lowercase)),
        "capitalize" => Some(Specified::Value(TextTransform::Capitalize)),
        "inherit" => Some(Specified::Inherit),
        "initial" => Some(Specified::Initial),
        "unset" => Some(Specified::Unset),
        _ => None,
    }
}

fn parse_spacing(
    tokens: &[TokenKind],
    font_px: f32,
    allow_percent: bool,
) -> Option<Specified<f32>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| 0.0));
    }
    if single_ident(tokens).is_some_and(|value| value.eq_ignore_ascii_case("normal")) {
        return Some(Specified::Value(0.0));
    }
    let value = match single_significant_token(tokens)? {
        TokenKind::Number(number) if parse_number(number).is_some_and(|value| value == 0.0) => 0.0,
        TokenKind::Percentage(number) if allow_percent => font_px * parse_number(number)? / 100.0,
        TokenKind::Dimension { number, unit } => {
            absolute_or_font_relative_px(parse_number(number)?, unit, font_px)?
        }
        _ => return None,
    };
    (value.is_finite() && (-4096.0..=4096.0).contains(&value)).then_some(Specified::Value(value))
}

fn parse_text_align(tokens: &[TokenKind]) -> Option<Specified<TextAlign>> {
    match single_ident(tokens)?.to_ascii_lowercase().as_str() {
        "start" => Some(Specified::Value(TextAlign::Start)),
        "end" => Some(Specified::Value(TextAlign::End)),
        "left" => Some(Specified::Value(TextAlign::Left)),
        "right" => Some(Specified::Value(TextAlign::Right)),
        "center" => Some(Specified::Value(TextAlign::Center)),
        "inherit" => Some(Specified::Inherit),
        "initial" => Some(Specified::Initial),
        "unset" => Some(Specified::Unset),
        _ => None,
    }
}

fn parse_line_height(tokens: &[TokenKind], font_px: f32) -> Option<Specified<ComputedLineHeight>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| ComputedLineHeight::Normal));
    }
    if single_ident(tokens).is_some_and(|ident| ident.eq_ignore_ascii_case("normal")) {
        return Some(Specified::Value(ComputedLineHeight::Normal));
    }

    let value = match single_significant_token(tokens)? {
        TokenKind::Number(number) => {
            let value = parse_number(number)?;
            if !(0.0..=100.0).contains(&value) {
                return None;
            }
            ComputedLineHeight::Number(value)
        }
        TokenKind::Percentage(number) => {
            let value = font_px * parse_number(number)? / 100.0;
            if !value.is_finite() || !(0.0..=100_000.0).contains(&value) {
                return None;
            }
            ComputedLineHeight::Px(value)
        }
        TokenKind::Dimension { number, unit } => {
            let value = absolute_or_font_relative_px(parse_number(number)?, unit, font_px)?;
            if !value.is_finite() || !(0.0..=100_000.0).contains(&value) {
                return None;
            }
            ComputedLineHeight::Px(value)
        }
        _ => return None,
    };
    Some(Specified::Value(value))
}

fn parse_font_size(tokens: &[TokenKind], parent_px: f32) -> Option<Specified<f32>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| parent_px));
    }
    if let Some(ident) = single_ident(tokens) {
        let value = match ident.to_ascii_lowercase().as_str() {
            "xx-small" => 10.0,
            "x-small" => 12.0,
            "small" => 14.0,
            "medium" => 18.0,
            "large" => 21.0,
            "x-large" => 24.0,
            "xx-large" => 32.0,
            "smaller" => parent_px * 0.8,
            "larger" => parent_px * 1.2,
            _ => return None,
        };
        return bounded_font_size(value).map(Specified::Value);
    }

    let value = match single_significant_token(tokens)? {
        TokenKind::Percentage(number) => parent_px * parse_number(number)? / 100.0,
        TokenKind::Dimension { number, unit } => {
            absolute_or_font_relative_px(parse_number(number)?, unit, parent_px)?
        }
        _ => return None,
    };
    bounded_font_size(value).map(Specified::Value)
}

fn bounded_font_size(value: f32) -> Option<f32> {
    (value.is_finite() && (1.0..=4096.0).contains(&value)).then_some(value)
}

fn parse_box_sizing(tokens: &[TokenKind]) -> Option<Specified<BoxSizing>> {
    match single_ident(tokens)?.to_ascii_lowercase().as_str() {
        "content-box" => Some(Specified::Value(BoxSizing::ContentBox)),
        "border-box" => Some(Specified::Value(BoxSizing::BorderBox)),
        "inherit" => Some(Specified::Inherit),
        "initial" => Some(Specified::Initial),
        "unset" => Some(Specified::Unset),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoxSide {
    Top,
    Right,
    Bottom,
    Left,
}

const BOX_SIDES: [BoxSide; 4] = [BoxSide::Top, BoxSide::Right, BoxSide::Bottom, BoxSide::Left];

fn apply_margin_declarations(
    style: &mut ComputedStyle,
    parent: Option<ComputedStyle>,
    declarations: &[MatchedDeclaration],
) {
    for side in BOX_SIDES {
        let Some((_, value)) = winning_margin_side(declarations, side, style.font_size_px) else {
            continue;
        };
        let parent_value = parent.map(|parent| margin_side(parent.margin, side));
        let resolved = resolve_non_inherited(value, parent_value, MarginValue::ZERO);
        set_margin_side(&mut style.margin, side, resolved);
    }
}

fn apply_padding_declarations(
    style: &mut ComputedStyle,
    parent: Option<ComputedStyle>,
    declarations: &[MatchedDeclaration],
) {
    for side in BOX_SIDES {
        let Some((_, value)) = winning_padding_side(declarations, side, style.font_size_px) else {
            continue;
        };
        let parent_value = parent.map(|parent| padding_side(parent.padding, side));
        let resolved = resolve_non_inherited(value, parent_value, LengthPercentage::ZERO);
        set_padding_side(&mut style.padding, side, resolved);
    }
}

fn apply_border_declarations(
    style: &mut ComputedStyle,
    parent: Option<ComputedStyle>,
    declarations: &[MatchedDeclaration],
) {
    for side in BOX_SIDES {
        let mut current = border_side(style.border, side);
        let parent_border = parent.map(|parent| border_side(parent.border, side));

        if let Some((_, value)) =
            winning_border_width(declarations, side, style.font_size_px, style.color)
        {
            current.width_px =
                resolve_non_inherited(value, parent_border.map(|border| border.width_px), 3.0);
        }
        if let Some((_, value)) =
            winning_border_style(declarations, side, style.font_size_px, style.color)
        {
            current.style = resolve_non_inherited(
                value,
                parent_border.map(|border| border.style),
                BorderStyle::None,
            );
        }
        if let Some((_, value)) =
            winning_border_color(declarations, side, style.font_size_px, style.color)
        {
            current.color =
                resolve_non_inherited(value, parent_border.map(|border| border.color), style.color);
        }

        set_border_side(&mut style.border, side, current);
    }
}

fn winning_margin_side(
    declarations: &[MatchedDeclaration],
    side: BoxSide,
    font_px: f32,
) -> Option<(&MatchedDeclaration, Specified<MarginValue>)> {
    declarations
        .iter()
        .filter_map(|matched| {
            let value = if matched.declaration.name == "margin" {
                parse_margin_shorthand(&matched.declaration.value, font_px)
                    .map(|value| value.map(|edges| edges[side_index(side)]))
            } else if matched.declaration.name == margin_longhand(side) {
                parse_margin_value(&matched.declaration.value, font_px)
            } else {
                return None;
            };
            let value = parsed_or_unset(matched, value)?;
            Some((matched, value))
        })
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
}

fn winning_padding_side(
    declarations: &[MatchedDeclaration],
    side: BoxSide,
    font_px: f32,
) -> Option<(&MatchedDeclaration, Specified<LengthPercentage>)> {
    declarations
        .iter()
        .filter_map(|matched| {
            let value = if matched.declaration.name == "padding" {
                parse_padding_shorthand(&matched.declaration.value, font_px)
                    .map(|value| value.map(|edges| edges[side_index(side)]))
            } else if matched.declaration.name == padding_longhand(side) {
                parse_padding_value(&matched.declaration.value, font_px)
            } else {
                return None;
            };
            let value = parsed_or_unset(matched, value)?;
            Some((matched, value))
        })
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
}

fn parse_margin_shorthand(
    tokens: &[TokenKind],
    font_px: f32,
) -> Option<Specified<[MarginValue; 4]>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| [MarginValue::ZERO; 4]));
    }
    let values = significant_tokens(tokens)
        .map(|token| parse_margin_token(token, font_px))
        .collect::<Option<Vec<_>>>()?;
    expand_four(&values).map(Specified::Value)
}

fn parse_padding_shorthand(
    tokens: &[TokenKind],
    font_px: f32,
) -> Option<Specified<[LengthPercentage; 4]>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| [LengthPercentage::ZERO; 4]));
    }
    let values = significant_tokens(tokens)
        .map(|token| parse_length_percentage_token(token, font_px, false, true))
        .collect::<Option<Vec<_>>>()?;
    expand_four(&values).map(Specified::Value)
}

fn parse_margin_value(tokens: &[TokenKind], font_px: f32) -> Option<Specified<MarginValue>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| MarginValue::ZERO));
    }
    parse_margin_token(single_significant_token(tokens)?, font_px).map(Specified::Value)
}

fn parse_padding_value(tokens: &[TokenKind], font_px: f32) -> Option<Specified<LengthPercentage>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| LengthPercentage::ZERO));
    }
    parse_length_percentage_token(single_significant_token(tokens)?, font_px, false, true)
        .map(Specified::Value)
}

fn parse_margin_token(token: &TokenKind, font_px: f32) -> Option<MarginValue> {
    if matches!(token, TokenKind::Ident(value) if value.eq_ignore_ascii_case("auto")) {
        return Some(MarginValue::Auto);
    }
    parse_length_percentage_token(token, font_px, true, true).map(MarginValue::Length)
}

fn expand_four<T: Copy>(values: &[T]) -> Option<[T; 4]> {
    match values {
        [all] => Some([*all; 4]),
        [vertical, horizontal] => Some([*vertical, *horizontal, *vertical, *horizontal]),
        [top, horizontal, bottom] => Some([*top, *horizontal, *bottom, *horizontal]),
        [top, right, bottom, left] => Some([*top, *right, *bottom, *left]),
        _ => None,
    }
}

fn resolve_optional_size_property(
    declarations: &[MatchedDeclaration],
    property: &str,
    font_px: f32,
    parent: Option<Option<LengthPercentage>>,
    initial: Option<LengthPercentage>,
    auto_keyword: bool,
) -> Option<LengthPercentage> {
    let Some((_, value)) = winning_value(declarations, property, |tokens| {
        parse_optional_size(tokens, font_px, auto_keyword)
    }) else {
        return initial;
    };
    resolve_non_inherited(value, parent, initial)
}

fn resolve_size_property(
    declarations: &[MatchedDeclaration],
    property: &str,
    font_px: f32,
    parent: Option<LengthPercentage>,
    initial: LengthPercentage,
) -> LengthPercentage {
    let Some((_, value)) = winning_value(declarations, property, |tokens| {
        parse_required_size(tokens, font_px)
    }) else {
        return initial;
    };
    resolve_non_inherited(value, parent, initial)
}

fn parse_optional_size(
    tokens: &[TokenKind],
    font_px: f32,
    auto_keyword: bool,
) -> Option<Specified<Option<LengthPercentage>>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| None));
    }
    if let Some(ident) = single_ident(tokens) {
        let expected = if auto_keyword { "auto" } else { "none" };
        if ident.eq_ignore_ascii_case(expected) {
            return Some(Specified::Value(None));
        }
        return None;
    }
    parse_length_percentage_token(single_significant_token(tokens)?, font_px, false, true)
        .map(|value| Specified::Value(Some(value)))
}

fn parse_required_size(tokens: &[TokenKind], font_px: f32) -> Option<Specified<LengthPercentage>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| LengthPercentage::ZERO));
    }
    parse_length_percentage_token(single_significant_token(tokens)?, font_px, false, true)
        .map(Specified::Value)
}

fn winning_border_width(
    declarations: &[MatchedDeclaration],
    side: BoxSide,
    font_px: f32,
    current_color: CssColor,
) -> Option<(&MatchedDeclaration, Specified<f32>)> {
    declarations
        .iter()
        .filter_map(|matched| {
            let value = border_component_value(
                matched,
                side,
                font_px,
                current_color,
                BorderComponent::Width,
            )?;
            Some((matched, value.width?))
        })
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
}

fn winning_border_style(
    declarations: &[MatchedDeclaration],
    side: BoxSide,
    font_px: f32,
    current_color: CssColor,
) -> Option<(&MatchedDeclaration, Specified<BorderStyle>)> {
    declarations
        .iter()
        .filter_map(|matched| {
            let value = border_component_value(
                matched,
                side,
                font_px,
                current_color,
                BorderComponent::Style,
            )?;
            Some((matched, value.style?))
        })
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
}

fn winning_border_color(
    declarations: &[MatchedDeclaration],
    side: BoxSide,
    font_px: f32,
    current_color: CssColor,
) -> Option<(&MatchedDeclaration, Specified<CssColor>)> {
    declarations
        .iter()
        .filter_map(|matched| {
            let value = border_component_value(
                matched,
                side,
                font_px,
                current_color,
                BorderComponent::Color,
            )?;
            Some((matched, value.color?))
        })
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
}

#[derive(Clone, Copy)]
enum BorderComponent {
    Width,
    Style,
    Color,
}

struct BorderCandidate {
    width: Option<Specified<f32>>,
    style: Option<Specified<BorderStyle>>,
    color: Option<Specified<CssColor>>,
}

fn border_component_value(
    matched: &MatchedDeclaration,
    side: BoxSide,
    font_px: f32,
    current_color: CssColor,
    component: BorderComponent,
) -> Option<BorderCandidate> {
    let name = matched.declaration.name.as_str();
    let tokens = &matched.declaration.value;

    if name == "border" || name == border_side_shorthand(side) {
        let border = parsed_or_unset(
            matched,
            parse_border_shorthand(tokens, font_px, current_color),
        )?;
        return Some(BorderCandidate {
            width: matches!(component, BorderComponent::Width)
                .then(|| border.map(|border| border.width_px)),
            style: matches!(component, BorderComponent::Style)
                .then(|| border.map(|border| border.style)),
            color: matches!(component, BorderComponent::Color)
                .then(|| border.map(|border| border.color)),
        });
    }

    match component {
        BorderComponent::Width if name == "border-width" || name == border_width_longhand(side) => {
            let value = if name == "border-width" {
                parse_border_width_list(tokens, font_px)
                    .map(|value| value.map(|values| values[side_index(side)]))
            } else {
                parse_border_width_value(tokens, font_px)
            };
            let value = parsed_or_unset(matched, value)?;
            Some(BorderCandidate {
                width: Some(value),
                style: None,
                color: None,
            })
        }
        BorderComponent::Style if name == "border-style" || name == border_style_longhand(side) => {
            let value = if name == "border-style" {
                parse_border_style_list(tokens)
                    .map(|value| value.map(|values| values[side_index(side)]))
            } else {
                parse_border_style_value(tokens)
            };
            let value = parsed_or_unset(matched, value)?;
            Some(BorderCandidate {
                width: None,
                style: Some(value),
                color: None,
            })
        }
        BorderComponent::Color if name == "border-color" || name == border_color_longhand(side) => {
            let value = if name == "border-color" {
                parse_border_color_list(tokens, current_color)
                    .map(|value| value.map(|values| values[side_index(side)]))
            } else {
                parse_border_color_value(tokens, current_color)
            };
            let value = parsed_or_unset(matched, value)?;
            Some(BorderCandidate {
                width: None,
                style: None,
                color: Some(value),
            })
        }
        _ => None,
    }
}

fn parse_border_shorthand(
    tokens: &[TokenKind],
    font_px: f32,
    current_color: CssColor,
) -> Option<Specified<ComputedBorder>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| ComputedBorder::NONE));
    }

    let mut width = None;
    let mut style = None;
    let mut color = None;
    let components = top_level_components(tokens)?;
    if components.is_empty() {
        return None;
    }
    for component in components {
        if width.is_none()
            && component.len() == 1
            && let Some(value) = parse_border_width_token(&component[0], font_px)
        {
            width = Some(value);
            continue;
        }
        if style.is_none()
            && component.len() == 1
            && let Some(value) = parse_border_style_token(&component[0])
        {
            style = Some(value);
            continue;
        }
        if color.is_none()
            && let Some(value) = parse_border_color_component(component, current_color)
        {
            color = Some(value);
            continue;
        }
        return None;
    }

    Some(Specified::Value(ComputedBorder {
        width_px: width.unwrap_or(3.0),
        color: color.unwrap_or(current_color),
        style: style.unwrap_or(BorderStyle::None),
    }))
}

fn parse_border_width_list(tokens: &[TokenKind], font_px: f32) -> Option<Specified<[f32; 4]>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| [3.0; 4]));
    }
    let values = significant_tokens(tokens)
        .map(|token| parse_border_width_token(token, font_px))
        .collect::<Option<Vec<_>>>()?;
    expand_four(&values).map(Specified::Value)
}

fn parse_border_style_list(tokens: &[TokenKind]) -> Option<Specified<[BorderStyle; 4]>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| [BorderStyle::None; 4]));
    }
    let values = significant_tokens(tokens)
        .map(parse_border_style_token)
        .collect::<Option<Vec<_>>>()?;
    expand_four(&values).map(Specified::Value)
}

fn parse_border_color_list(
    tokens: &[TokenKind],
    current_color: CssColor,
) -> Option<Specified<[CssColor; 4]>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| [current_color; 4]));
    }
    let values = top_level_components(tokens)?
        .into_iter()
        .map(|component| parse_border_color_component(component, current_color))
        .collect::<Option<Vec<_>>>()?;
    expand_four(&values).map(Specified::Value)
}

fn parse_border_width_value(tokens: &[TokenKind], font_px: f32) -> Option<Specified<f32>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| 3.0));
    }
    parse_border_width_token(single_significant_token(tokens)?, font_px).map(Specified::Value)
}

fn parse_border_style_value(tokens: &[TokenKind]) -> Option<Specified<BorderStyle>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| BorderStyle::None));
    }
    parse_border_style_token(single_significant_token(tokens)?).map(Specified::Value)
}

fn parse_border_color_value(
    tokens: &[TokenKind],
    current_color: CssColor,
) -> Option<Specified<CssColor>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| current_color));
    }
    parse_border_color_component(tokens, current_color).map(Specified::Value)
}

fn parse_border_width_token(token: &TokenKind, font_px: f32) -> Option<f32> {
    if let TokenKind::Ident(value) = token {
        return match value.to_ascii_lowercase().as_str() {
            "thin" => Some(1.0),
            "medium" => Some(3.0),
            "thick" => Some(5.0),
            _ => None,
        };
    }
    match parse_length_percentage_token(token, font_px, false, false)? {
        LengthPercentage::Px(value) => Some(value),
        LengthPercentage::Percent(_) => None,
    }
}

fn parse_border_style_token(token: &TokenKind) -> Option<BorderStyle> {
    let TokenKind::Ident(value) = token else {
        return None;
    };
    match value.to_ascii_lowercase().as_str() {
        "none" => Some(BorderStyle::None),
        "solid" => Some(BorderStyle::Solid),
        _ => None,
    }
}

fn parse_border_color_component(tokens: &[TokenKind], current_color: CssColor) -> Option<CssColor> {
    if single_ident(tokens).is_some_and(|value| value.eq_ignore_ascii_case("currentcolor")) {
        Some(current_color)
    } else {
        parse_css_color(tokens)
    }
}

fn parse_length_percentage_token(
    token: &TokenKind,
    font_px: f32,
    allow_negative: bool,
    allow_percent: bool,
) -> Option<LengthPercentage> {
    let value = match token {
        TokenKind::Number(number) if parse_number(number).is_some_and(|value| value == 0.0) => {
            LengthPercentage::ZERO
        }
        TokenKind::Percentage(number) if allow_percent => {
            LengthPercentage::Percent(parse_number(number)? / 100.0)
        }
        TokenKind::Dimension { number, unit } => LengthPercentage::Px(
            absolute_or_font_relative_px(parse_number(number)?, unit, font_px)?,
        ),
        _ => return None,
    };

    let scalar = match value {
        LengthPercentage::Px(value) | LengthPercentage::Percent(value) => value,
    };
    if !scalar.is_finite() || scalar.abs() > 1_000_000.0 {
        return None;
    }
    if !allow_negative && scalar < 0.0 {
        return None;
    }
    Some(value)
}

fn absolute_or_font_relative_px(value: f32, unit: &str, font_px: f32) -> Option<f32> {
    let factor = match unit.to_ascii_lowercase().as_str() {
        "px" => 1.0,
        "em" => font_px,
        "rem" => ComputedStyle::initial().font_size_px,
        "in" => 96.0,
        "cm" => 96.0 / 2.54,
        "mm" => 96.0 / 25.4,
        "q" => 96.0 / 101.6,
        "pt" => 96.0 / 72.0,
        "pc" => 16.0,
        _ => return None,
    };
    let result = value * factor;
    result.is_finite().then_some(result)
}

fn parse_number(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().filter(|value| value.is_finite())
}

fn significant_tokens(tokens: &[TokenKind]) -> impl Iterator<Item = &TokenKind> {
    tokens
        .iter()
        .filter(|token| !matches!(token, TokenKind::Whitespace))
}

fn margin_side(edges: MarginEdges, side: BoxSide) -> MarginValue {
    match side {
        BoxSide::Top => edges.top,
        BoxSide::Right => edges.right,
        BoxSide::Bottom => edges.bottom,
        BoxSide::Left => edges.left,
    }
}

fn set_margin_side(edges: &mut MarginEdges, side: BoxSide, value: MarginValue) {
    match side {
        BoxSide::Top => edges.top = value,
        BoxSide::Right => edges.right = value,
        BoxSide::Bottom => edges.bottom = value,
        BoxSide::Left => edges.left = value,
    }
}

fn padding_side(edges: PaddingEdges, side: BoxSide) -> LengthPercentage {
    match side {
        BoxSide::Top => edges.top,
        BoxSide::Right => edges.right,
        BoxSide::Bottom => edges.bottom,
        BoxSide::Left => edges.left,
    }
}

fn set_padding_side(edges: &mut PaddingEdges, side: BoxSide, value: LengthPercentage) {
    match side {
        BoxSide::Top => edges.top = value,
        BoxSide::Right => edges.right = value,
        BoxSide::Bottom => edges.bottom = value,
        BoxSide::Left => edges.left = value,
    }
}

fn border_side(edges: BorderEdges, side: BoxSide) -> ComputedBorder {
    match side {
        BoxSide::Top => edges.top,
        BoxSide::Right => edges.right,
        BoxSide::Bottom => edges.bottom,
        BoxSide::Left => edges.left,
    }
}

fn set_border_side(edges: &mut BorderEdges, side: BoxSide, value: ComputedBorder) {
    match side {
        BoxSide::Top => edges.top = value,
        BoxSide::Right => edges.right = value,
        BoxSide::Bottom => edges.bottom = value,
        BoxSide::Left => edges.left = value,
    }
}

fn side_index(side: BoxSide) -> usize {
    match side {
        BoxSide::Top => 0,
        BoxSide::Right => 1,
        BoxSide::Bottom => 2,
        BoxSide::Left => 3,
    }
}

fn margin_longhand(side: BoxSide) -> &'static str {
    match side {
        BoxSide::Top => "margin-top",
        BoxSide::Right => "margin-right",
        BoxSide::Bottom => "margin-bottom",
        BoxSide::Left => "margin-left",
    }
}

fn padding_longhand(side: BoxSide) -> &'static str {
    match side {
        BoxSide::Top => "padding-top",
        BoxSide::Right => "padding-right",
        BoxSide::Bottom => "padding-bottom",
        BoxSide::Left => "padding-left",
    }
}

fn border_side_shorthand(side: BoxSide) -> &'static str {
    match side {
        BoxSide::Top => "border-top",
        BoxSide::Right => "border-right",
        BoxSide::Bottom => "border-bottom",
        BoxSide::Left => "border-left",
    }
}

fn border_width_longhand(side: BoxSide) -> &'static str {
    match side {
        BoxSide::Top => "border-top-width",
        BoxSide::Right => "border-right-width",
        BoxSide::Bottom => "border-bottom-width",
        BoxSide::Left => "border-left-width",
    }
}

fn border_style_longhand(side: BoxSide) -> &'static str {
    match side {
        BoxSide::Top => "border-top-style",
        BoxSide::Right => "border-right-style",
        BoxSide::Bottom => "border-bottom-style",
        BoxSide::Left => "border-left-style",
    }
}

fn border_color_longhand(side: BoxSide) -> &'static str {
    match side {
        BoxSide::Top => "border-top-color",
        BoxSide::Right => "border-right-color",
        BoxSide::Bottom => "border-bottom-color",
        BoxSide::Left => "border-left-color",
    }
}

fn global_keyword(tokens: &[TokenKind]) -> Option<Specified<()>> {
    match single_ident(tokens)?.to_ascii_lowercase().as_str() {
        "inherit" => Some(Specified::Inherit),
        "initial" => Some(Specified::Initial),
        "unset" => Some(Specified::Unset),
        _ => None,
    }
}

impl<T> Specified<T> {
    fn map<U>(self, value: impl FnOnce(T) -> U) -> Specified<U> {
        match self {
            Specified::Value(inner) => Specified::Value(value(inner)),
            Specified::Inherit => Specified::Inherit,
            Specified::Initial => Specified::Initial,
            Specified::Unset => Specified::Unset,
        }
    }
}

fn top_level_components(tokens: &[TokenKind]) -> Option<Vec<&[TokenKind]>> {
    let mut components = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        while index < tokens.len() && matches!(tokens[index], TokenKind::Whitespace) {
            index += 1;
        }
        if index == tokens.len() {
            break;
        }

        let start = index;
        if matches!(tokens[index], TokenKind::Function(_)) {
            let mut depth = 1usize;
            index += 1;
            while index < tokens.len() && depth > 0 {
                match tokens[index] {
                    TokenKind::Function(_) | TokenKind::OpenParen => depth += 1,
                    TokenKind::CloseParen => depth -= 1,
                    _ => {}
                }
                index += 1;
            }
            if depth != 0 {
                return None;
            }
        } else {
            index += 1;
        }
        components.push(&tokens[start..index]);
    }
    Some(components)
}

fn winning_background_color(
    declarations: &[MatchedDeclaration],
) -> Option<(&MatchedDeclaration, Specified<CssColor>)> {
    declarations
        .iter()
        .filter_map(|matched| {
            let value = match matched.declaration.name.as_str() {
                "background-color" => parse_color(&matched.declaration.value),
                "background" => parse_background_color(&matched.declaration.value),
                _ => return None,
            };
            let value = parsed_or_unset(matched, value)?;
            Some((matched, value))
        })
        .max_by(|(left, _), (right, _)| cascade_key(left).cmp(&cascade_key(right)))
}

fn parse_background_color(tokens: &[TokenKind]) -> Option<Specified<CssColor>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| CssColor::TRANSPARENT));
    }
    if single_ident(tokens).is_some_and(|value| value.eq_ignore_ascii_case("none")) {
        return Some(Specified::Value(CssColor::TRANSPARENT));
    }
    parse_css_color(tokens).map(Specified::Value)
}

fn parse_color(tokens: &[TokenKind]) -> Option<Specified<CssColor>> {
    if let Some(keyword) = global_keyword(tokens) {
        return Some(keyword.map(|()| CssColor::BLACK));
    }
    parse_css_color(tokens).map(Specified::Value)
}

fn parse_css_color(tokens: &[TokenKind]) -> Option<CssColor> {
    if let Some(token) = single_significant_token(tokens) {
        return parse_color_token(token);
    }

    let significant: Vec<&TokenKind> = significant_tokens(tokens).collect();
    let TokenKind::Function(name) = significant.first()? else {
        return None;
    };
    if !matches!(significant.last(), Some(TokenKind::CloseParen)) {
        return None;
    }
    let arguments = &significant[1..significant.len() - 1];
    match name.to_ascii_lowercase().as_str() {
        "rgb" | "rgba" => parse_rgb_function(arguments),
        "hsl" | "hsla" => parse_hsl_function(arguments),
        _ => None,
    }
}

fn parse_color_token(token: &TokenKind) -> Option<CssColor> {
    match token {
        TokenKind::Ident(value) => named_color(value),
        TokenKind::Hash { value, .. } => parse_hex_color(value),
        _ => None,
    }
}

fn named_color(value: &str) -> Option<CssColor> {
    let (red, green, blue, alpha) = match value.to_ascii_lowercase().as_str() {
        "black" => (0, 0, 0, 255),
        "silver" => (192, 192, 192, 255),
        "gray" | "grey" => (128, 128, 128, 255),
        "white" => (255, 255, 255, 255),
        "maroon" => (128, 0, 0, 255),
        "red" => (255, 0, 0, 255),
        "purple" => (128, 0, 128, 255),
        "fuchsia" | "magenta" => (255, 0, 255, 255),
        "green" => (0, 128, 0, 255),
        "lime" => (0, 255, 0, 255),
        "olive" => (128, 128, 0, 255),
        "yellow" => (255, 255, 0, 255),
        "navy" => (0, 0, 128, 255),
        "blue" => (0, 0, 255, 255),
        "teal" => (0, 128, 128, 255),
        "aqua" | "cyan" => (0, 255, 255, 255),
        "orange" => (255, 165, 0, 255),
        "rebeccapurple" => (102, 51, 153, 255),
        "transparent" => (0, 0, 0, 0),
        _ => return None,
    };
    Some(CssColor {
        red,
        green,
        blue,
        alpha,
    })
}

fn parse_rgb_function(tokens: &[&TokenKind]) -> Option<CssColor> {
    let (channels, alpha) = if tokens.iter().any(|token| matches!(token, TokenKind::Comma)) {
        let groups = split_comma_groups(tokens)?;
        if !(3..=4).contains(&groups.len()) || groups.iter().any(|group| group.len() != 1) {
            return None;
        }
        let alpha = if groups.len() == 4 {
            Some(parse_alpha(groups[3][0])?)
        } else {
            None
        };
        (
            [
                parse_rgb_channel(groups[0][0])?,
                parse_rgb_channel(groups[1][0])?,
                parse_rgb_channel(groups[2][0])?,
            ],
            alpha,
        )
    } else {
        let slash = tokens
            .iter()
            .position(|token| matches!(token, TokenKind::Delim('/')));
        let color_end = slash.unwrap_or(tokens.len());
        if color_end != 3 {
            return None;
        }
        let alpha = match slash {
            Some(index) if index + 2 == tokens.len() => Some(parse_alpha(tokens[index + 1])?),
            Some(_) => return None,
            None => None,
        };
        (
            [
                parse_rgb_channel(tokens[0])?,
                parse_rgb_channel(tokens[1])?,
                parse_rgb_channel(tokens[2])?,
            ],
            alpha,
        )
    };

    Some(CssColor {
        red: channels[0],
        green: channels[1],
        blue: channels[2],
        alpha: alpha.unwrap_or(255),
    })
}

fn parse_hsl_function(tokens: &[&TokenKind]) -> Option<CssColor> {
    let (hue, saturation, lightness, alpha) =
        if tokens.iter().any(|token| matches!(token, TokenKind::Comma)) {
            let groups = split_comma_groups(tokens)?;
            if !(3..=4).contains(&groups.len()) || groups.iter().any(|group| group.len() != 1) {
                return None;
            }
            (
                parse_hue(groups[0][0])?,
                parse_percentage_fraction(groups[1][0])?,
                parse_percentage_fraction(groups[2][0])?,
                if groups.len() == 4 {
                    Some(parse_alpha(groups[3][0])?)
                } else {
                    None
                },
            )
        } else {
            let slash = tokens
                .iter()
                .position(|token| matches!(token, TokenKind::Delim('/')));
            let color_end = slash.unwrap_or(tokens.len());
            if color_end != 3 {
                return None;
            }
            let alpha = match slash {
                Some(index) if index + 2 == tokens.len() => Some(parse_alpha(tokens[index + 1])?),
                Some(_) => return None,
                None => None,
            };
            (
                parse_hue(tokens[0])?,
                parse_percentage_fraction(tokens[1])?,
                parse_percentage_fraction(tokens[2])?,
                alpha,
            )
        };

    let hue = hue.rem_euclid(360.0) / 60.0;
    let saturation = saturation.clamp(0.0, 1.0);
    let lightness = lightness.clamp(0.0, 1.0);
    let chroma = (1.0_f32 - (2.0_f32 * lightness - 1.0_f32).abs()) * saturation;
    let x = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hue {
        value if value < 1.0 => (chroma, x, 0.0),
        value if value < 2.0 => (x, chroma, 0.0),
        value if value < 3.0 => (0.0, chroma, x),
        value if value < 4.0 => (0.0, x, chroma),
        value if value < 5.0 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;

    Some(CssColor {
        red: fraction_byte(r1 + m),
        green: fraction_byte(g1 + m),
        blue: fraction_byte(b1 + m),
        alpha: alpha.unwrap_or(255),
    })
}

fn split_comma_groups<'a>(tokens: &'a [&'a TokenKind]) -> Option<Vec<Vec<&'a TokenKind>>> {
    let mut groups = vec![Vec::new()];
    for token in tokens {
        if matches!(token, TokenKind::Comma) {
            if groups.last().is_none_or(Vec::is_empty) {
                return None;
            }
            groups.push(Vec::new());
        } else {
            groups.last_mut()?.push(*token);
        }
    }
    (!groups.last()?.is_empty()).then_some(groups)
}

fn parse_rgb_channel(token: &TokenKind) -> Option<u8> {
    let value = match token {
        TokenKind::Number(number) => parse_number(number)?.clamp(0.0, 255.0),
        TokenKind::Percentage(number) => parse_number(number)?.clamp(0.0, 100.0) * 2.55,
        _ => return None,
    };
    Some(value.round().clamp(0.0, 255.0) as u8)
}

fn parse_alpha(token: &TokenKind) -> Option<u8> {
    let fraction = match token {
        TokenKind::Number(number) => parse_number(number)?.clamp(0.0, 1.0),
        TokenKind::Percentage(number) => parse_number(number)?.clamp(0.0, 100.0) / 100.0,
        _ => return None,
    };
    Some(fraction_byte(fraction))
}

fn parse_percentage_fraction(token: &TokenKind) -> Option<f32> {
    let TokenKind::Percentage(number) = token else {
        return None;
    };
    Some((parse_number(number)? / 100.0).clamp(0.0, 1.0))
}

fn parse_hue(token: &TokenKind) -> Option<f32> {
    match token {
        TokenKind::Number(number) => parse_number(number),
        TokenKind::Dimension { number, unit } => {
            let value = parse_number(number)?;
            match unit.to_ascii_lowercase().as_str() {
                "deg" => Some(value),
                "grad" => Some(value * 0.9),
                "rad" => Some(value * 180.0 / std::f32::consts::PI),
                "turn" => Some(value * 360.0),
                _ => None,
            }
        }
        _ => None,
    }
}

fn fraction_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
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
    fn custom_properties_cascade_inherit_and_substitute_before_value_parsing() {
        let document = parse_document(
            "<style>
               #scope { --accent:#123456; --gap:3px 7px; --frozen:var(--accent); --weight:400 }
               .card { --priority:red !important }
               #parent { --accent:blue; --weight:700; color:var(--frozen); padding:var(--gap); font-weight:var(--weight) }
               #child { --accent:green; --priority:blue; color:var(--frozen); background:var(--accent); margin-left:var(--missing, 12px); width:var(--w, 50%) }
             </style>
             <div id='scope'><div id='parent'><span id='child' class='card'>x</span></div></div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let parent_id = find_by_id(&document, "parent");
        let child_id = find_by_id(&document, "child");
        let parent = computed.style_for(parent_id).unwrap();
        let child = computed.style_for(child_id).unwrap();

        let frozen = CssColor {
            red: 0x12,
            green: 0x34,
            blue: 0x56,
            alpha: 255,
        };
        assert_eq!(parent.color, frozen);
        assert_eq!(child.color, frozen);
        assert_eq!(parent.font_weight, ComputedFontWeight::Bold);
        assert_eq!(parent.padding.top, LengthPercentage::Px(3.0));
        assert_eq!(parent.padding.right, LengthPercentage::Px(7.0));
        assert_eq!(child.background_color, CssColor::GREEN);
        assert_eq!(
            child.margin.left,
            MarginValue::Length(LengthPercentage::Px(12.0))
        );
        assert_eq!(child.width, Some(LengthPercentage::Percent(0.5)));

        let custom = computed.custom_properties_for(child_id).unwrap();
        assert!(custom.contains_key("--frozen"));
        assert_eq!(
            custom.get("--priority"),
            Some(&vec![TokenKind::Ident("red".into())])
        );
    }

    #[test]
    fn custom_property_cycles_become_invalid_and_nested_var_fallbacks_apply() {
        let document = parse_document(
            "<style>
               #cycle {
                 --a:var(--b);
                 --b:var(--a);
                 --fallback:#123456;
                 color:var(--a, var(--fallback, blue));
                 background:var(--missing-bg, rgb(238 242 255));
                 border:var(--missing-border, 2px solid #4338ca);
               }
             </style><div id='cycle'>x</div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let id = find_by_id(&document, "cycle");
        let style = computed.style_for(id).unwrap();
        let custom = computed.custom_properties_for(id).unwrap();

        assert!(!custom.contains_key("--a"));
        assert!(!custom.contains_key("--b"));
        assert_eq!(
            style.color,
            CssColor {
                red: 0x12,
                green: 0x34,
                blue: 0x56,
                alpha: 255,
            }
        );
        assert_eq!(
            style.background_color,
            CssColor {
                red: 238,
                green: 242,
                blue: 255,
                alpha: 255,
            }
        );
        assert_eq!(style.border.left.width_px, 2.0);
        assert_eq!(
            style.border.left.color,
            CssColor {
                red: 0x43,
                green: 0x38,
                blue: 0xca,
                alpha: 255,
            }
        );
    }

    #[test]
    fn pseudo_elements_inherit_custom_properties_and_can_override_them() {
        let document = parse_document(
            "<style>
               #note { --label:'[VAR] '; --tone:#b42318; }
               #note::before {
                 --pad:2px 4px;
                 content:var(--label);
                 color:var(--tone);
                 padding:var(--pad);
               }
             </style><p id='note'>Body</p>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let note = find_by_id(&document, "note");
        let before = computed
            .pseudo_style_for(note, PseudoElement::Before)
            .expect("before should be generated");
        let custom = computed
            .pseudo_custom_properties_for(note, PseudoElement::Before)
            .expect("pseudo custom properties should be retained");

        assert_eq!(before.content, "[VAR] ");
        assert_eq!(
            before.style.color,
            CssColor {
                red: 180,
                green: 35,
                blue: 24,
                alpha: 255,
            }
        );
        assert_eq!(before.style.padding.top, LengthPercentage::Px(2.0));
        assert_eq!(before.style.padding.right, LengthPercentage::Px(4.0));
        assert!(custom.contains_key("--label"));
        assert!(custom.contains_key("--tone"));
        assert!(custom.contains_key("--pad"));
    }

    #[test]
    fn cyclic_overrides_are_invalid_while_inherited_values_stay_frozen_and_empty_is_valid() {
        let document = parse_document(
            "<style>
             #parent { --a:red; --b:var(--a); --empty:; --priority: !important; }
             #child { --a:var(--b); color:var(--a); --self:var(--self,blue);
                 background:var(--self,green); --bad:initial; }
             #child::before { --self:var(--self, 'BAD'); content:var(--self,'GOOD') var(--empty,'BAD'); }
             </style><div id='parent'><span id='child'>x</span></div>",
        );
        let collected = collect_author_styles(&document);
        assert!(collected.errors.is_empty());
        let computed = compute_styles(&document, &collected.styles);
        let child = find_by_id(&document, "child");
        let custom = computed.custom_properties_for(child).unwrap();
        assert!(!custom.contains_key("--self"));
        assert!(!custom.contains_key("--bad"));
        assert_eq!(custom["--empty"], Vec::<TokenKind>::new());
        assert_eq!(custom["--priority"], Vec::<TokenKind>::new());
        assert_eq!(computed.style_for(child).unwrap().color, CssColor::RED);
        assert_eq!(
            computed.style_for(child).unwrap().background_color,
            CssColor::GREEN
        );
        assert_eq!(
            computed
                .pseudo_style_for(child, PseudoElement::Before)
                .unwrap()
                .content,
            "GOOD"
        );
    }

    #[test]
    fn invalid_computed_var_winners_unset_instead_of_revealing_older_values() {
        let document = parse_document(
            "<style>#parent { color:#123456; font-size:24px; font-weight:bold; quotes:'A' 'Z' }
             #child { --bad:banana; --empty:; color:red; color:var(--missing);
                 font-size:40px; font-size:var(--bad); font-weight:normal; font-weight:var(--empty);
                 background:red; background:var(--bad); display:block; display:var(--missing);
                 width:80px; width:var(--bad); letter-spacing:3px; letter-spacing:var(--missing);
                 quotes:none; quotes:var(--bad); }
             #child::before { content:'BAD'; content:var(--missing); counter-increment:n 99 }
             #next::before { content:counter(n) }
             </style><div id='parent'><span id='child'>x</span><span id='next'>y</span></div>",
        );
        let computed = compute_styles(&document, &collect_author_styles(&document).styles);
        let child = find_by_id(&document, "child");
        let parent = find_by_id(&document, "parent");
        let style = computed.style_for(child).unwrap();
        assert_eq!(style.color, computed.style_for(parent).unwrap().color);
        assert_eq!(style.font_size_px, 24.0);
        assert_eq!(style.font_weight, ComputedFontWeight::Bold);
        assert_eq!(style.background_color, CssColor::TRANSPARENT);
        assert_eq!(style.display, Display::Inline);
        assert_eq!(style.width, None);
        assert_eq!(style.letter_spacing_px, 0.0);
        assert_eq!(computed.quotes_for(child), computed.quotes_for(parent));
        assert!(
            computed
                .pseudo_style_for(child, PseudoElement::Before)
                .is_none()
        );
        assert_eq!(
            computed
                .pseudo_style_for(find_by_id(&document, "next"), PseudoElement::Before)
                .unwrap()
                .content,
            "0"
        );
    }

    #[test]
    fn invalid_var_shorthands_preserve_component_cascade_and_importance() {
        let document = parse_document(
            "<style>#box { --bad:nope; margin:9px; margin:var(--missing); margin-left:7px;
                 padding:9px; padding:var(--bad) !important; padding-left:7px;
                 border:4px solid red; border:var(--missing); border-left:2px solid blue;
                 color:red; color:var(--bad) !important; counter-increment:n 10;
                 counter-increment:var(--bad); }
             #box::before { content:counter(n); color:blue; color:var(--bad) }
             </style><div id='box' style='color:green'>x</div>",
        );
        let computed = compute_styles(&document, &collect_author_styles(&document).styles);
        let box_id = find_by_id(&document, "box");
        let style = computed.style_for(box_id).unwrap();
        assert_eq!(
            style.margin.left,
            MarginValue::Length(LengthPercentage::Px(7.0))
        );
        assert_eq!(style.margin.top, MarginValue::ZERO);
        assert_eq!(style.padding, PaddingEdges::ZERO);
        assert_eq!(style.border.top.style, BorderStyle::None);
        assert_eq!(style.border.left.style, BorderStyle::Solid);
        assert_eq!(style.border.left.width_px, 2.0);
        assert_eq!(style.border.left.color, CssColor::BLUE);
        assert_eq!(style.color, CssColor::BLACK);
        let before = computed
            .pseudo_style_for(box_id, PseudoElement::Before)
            .unwrap();
        assert_eq!(before.content, "0");
        assert_eq!(before.style.color, CssColor::BLACK);
    }

    #[test]
    fn invalid_var_values_match_explicit_unset_for_every_supported_style_property() {
        let properties = [
            ("display", "block"),
            ("color", "red"),
            ("font-size", "40px"),
            ("font-weight", "normal"),
            ("font-style", "italic"),
            ("line-height", "2"),
            ("text-align", "right"),
            ("white-space", "pre"),
            ("text-decoration", "underline"),
            ("text-decoration-line", "line-through"),
            ("text-transform", "uppercase"),
            ("letter-spacing", "4px"),
            ("word-spacing", "6px"),
            ("background", "red"),
            ("background-color", "blue"),
            ("box-sizing", "border-box"),
            ("width", "100px"),
            ("min-width", "40px"),
            ("max-width", "90px"),
            ("height", "70px"),
            ("min-height", "20px"),
            ("max-height", "50px"),
            ("margin", "20px"),
            ("margin-top", "20px"),
            ("margin-right", "20px"),
            ("margin-bottom", "20px"),
            ("margin-left", "20px"),
            ("padding", "20px"),
            ("padding-top", "20px"),
            ("padding-right", "20px"),
            ("padding-bottom", "20px"),
            ("padding-left", "20px"),
            ("border", "4px solid red"),
            ("border-top", "4px solid red"),
            ("border-right", "4px solid red"),
            ("border-bottom", "4px solid red"),
            ("border-left", "4px solid red"),
            ("border-width", "4px"),
            ("border-style", "solid"),
            ("border-color", "red"),
            ("border-top-width", "4px"),
            ("border-right-width", "4px"),
            ("border-bottom-width", "4px"),
            ("border-left-width", "4px"),
            ("border-top-style", "solid"),
            ("border-right-style", "solid"),
            ("border-bottom-style", "solid"),
            ("border-left-style", "solid"),
            ("border-top-color", "red"),
            ("border-right-color", "red"),
            ("border-bottom-color", "red"),
            ("border-left-color", "red"),
        ];
        for (property, older) in properties {
            for invalid in ["var(--missing)", "var(--bad)", "var(--empty)"] {
                let document = parse_document(&format!(
                    "<div style='color:green;font-size:24px;font-weight:bold;font-style:italic;line-height:1.5;text-align:center'>
                     <span id='actual' style='--bad:!; --empty:; {property}:{older}; {property}:{invalid}'>x</span>
                     <span id='reference' style='{property}:unset'>y</span></div>"
                ));
                let computed = compute_styles(&document, &collect_author_styles(&document).styles);
                assert_eq!(
                    computed.style_for(find_by_id(&document, "actual")),
                    computed.style_for(find_by_id(&document, "reference")),
                    "{property}:{invalid}"
                );
            }
        }
    }

    #[test]
    fn filtered_nth_matching_counts_union_siblings_in_both_directions() {
        let document = parse_document(
            "<style>
             #two { color:red }
             :nth-child(2 of #unmatched, .pick, .pick) { color:blue }
             :nth-child(-n+2 of #scope > .pick) { padding-left:7px }
             :nth-last-child(1 of .pick) { font-weight:bold }
             :nth-last-child(2) { background:green }
             :is(:unsupported, .pick) { font-style:italic }
             :where() { color:red }
             </style><div id='scope'><span id='one' class='pick'>A</span>text
             <em id='skip'>S</em><span id='two' class='pick'>B</span>
             <span id='three' class='pick'>C</span><span id='four'>D</span></div>",
        );
        let collection = collect_author_styles(&document);
        assert!(collection.errors.is_empty());
        let computed = compute_styles(&document, &collection.styles);
        let style = |id| computed.style_for(find_by_id(&document, id)).unwrap();
        assert_eq!(
            style("two").color,
            CssColor::BLUE,
            "maximum filter specificity applies even if that branch did not match"
        );
        assert_eq!(style("one").padding.left, LengthPercentage::Px(7.0));
        assert_eq!(style("two").padding.left, LengthPercentage::Px(7.0));
        assert_eq!(style("three").padding.left, LengthPercentage::ZERO);
        assert_eq!(style("three").font_weight, ComputedFontWeight::Bold);
        assert_eq!(style("three").background_color, CssColor::GREEN);
        assert_eq!(style("one").font_style, FontStyle::Italic);
        assert_eq!(style("four").font_style, FontStyle::Normal);
        assert_eq!(style("four").color, CssColor::BLACK);
    }

    #[test]
    fn typed_structural_matching_ignores_other_element_types_and_text() {
        let document = parse_document(
            "<style>span:first-of-type { color:red } span:last-of-type { background:blue }
             span:nth-of-type(2) { color:blue } span:nth-last-of-type(2) { font-weight:bold }
             em:only-of-type { font-weight:bold } strong:only-of-type { font-style:italic }
             span:only-of-type { padding:99px }
             </style><div><em id='em'>E</em><span id='one'>A</span>text
             <strong id='strong'>S</strong><span id='two'>B</span><span id='three'>C</span><b>Tail</b></div>",
        );
        let collection = collect_author_styles(&document);
        assert!(collection.errors.is_empty());
        let computed = compute_styles(&document, &collection.styles);
        let style = |id| computed.style_for(find_by_id(&document, id)).unwrap();
        assert_eq!(style("one").color, CssColor::RED);
        assert_eq!(style("two").color, CssColor::BLUE);
        assert_eq!(style("two").font_weight, ComputedFontWeight::Bold);
        assert_eq!(style("three").background_color, CssColor::BLUE);
        assert_eq!(style("em").font_weight, ComputedFontWeight::Bold);
        assert_eq!(style("strong").font_style, FontStyle::Italic);
        for id in ["one", "two", "three"] {
            assert_eq!(style(id).padding, PaddingEdges::ZERO);
        }
    }

    #[test]
    fn valid_var_fallbacks_and_literal_invalid_values_keep_their_distinct_cascade_rules() {
        let document = parse_document(
            "<style>#one { --empty:; color:red; color:invalid; padding:5px; padding:var(--missing,2px 4px) }
             #two { color:green; color:var(--empty,red) }
             #three { color:var(--missing); color:blue }
             </style><div id='one'><span id='two'>x</span><span id='three'>y</span></div>",
        );
        let computed = compute_styles(&document, &collect_author_styles(&document).styles);
        let one = computed.style_for(find_by_id(&document, "one")).unwrap();
        assert_eq!(one.color, CssColor::RED);
        assert_eq!(one.padding.top, LengthPercentage::Px(2.0));
        assert_eq!(one.padding.left, LengthPercentage::Px(4.0));
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "two"))
                .unwrap()
                .color,
            CssColor::RED
        );
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "three"))
                .unwrap()
                .color,
            CssColor::BLUE
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
    fn computes_initial_block_box_model_values_and_global_keywords() {
        let document = parse_document(
            "<div id='parent' style='background-color:#abc; margin:1px 2px 3px 4px; padding:5px 6px; border:2px solid #123456'>
                <div id='child' style='background-color:inherit; margin:inherit; padding:unset; border:inherit'>x</div>
             </div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let parent = computed.style_for(find_by_id(&document, "parent")).unwrap();
        let child = computed.style_for(find_by_id(&document, "child")).unwrap();

        assert_eq!(
            parent.background_color,
            CssColor {
                red: 0xaa,
                green: 0xbb,
                blue: 0xcc,
                alpha: 255,
            }
        );
        assert_eq!(
            parent.margin,
            MarginEdges {
                top: MarginValue::Length(LengthPercentage::Px(1.0)),
                right: MarginValue::Length(LengthPercentage::Px(2.0)),
                bottom: MarginValue::Length(LengthPercentage::Px(3.0)),
                left: MarginValue::Length(LengthPercentage::Px(4.0)),
            }
        );
        assert_eq!(
            parent.padding,
            PaddingEdges {
                top: LengthPercentage::Px(5.0),
                right: LengthPercentage::Px(6.0),
                bottom: LengthPercentage::Px(5.0),
                left: LengthPercentage::Px(6.0),
            }
        );
        let border = ComputedBorder {
            width_px: 2.0,
            color: CssColor {
                red: 0x12,
                green: 0x34,
                blue: 0x56,
                alpha: 255,
            },
            style: BorderStyle::Solid,
        };
        assert_eq!(
            parent.border,
            BorderEdges {
                top: border,
                right: border,
                bottom: border,
                left: border,
            }
        );
        assert_eq!(child.background_color, parent.background_color);
        assert_eq!(child.margin, parent.margin);
        assert_eq!(child.padding, PaddingEdges::ZERO);
        assert_eq!(child.border, parent.border);
    }

    #[test]
    fn ua_spacing_is_represented_as_computed_margins_and_can_be_overridden() {
        let document =
            parse_document("<p id='default'>a</p><p id='custom' style='margin:4px 5px'>b</p>");
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let default = computed
            .style_for(find_by_id(&document, "default"))
            .unwrap();
        let custom = computed.style_for(find_by_id(&document, "custom")).unwrap();

        assert_eq!(
            default.margin.bottom,
            MarginValue::Length(LengthPercentage::Px(12.0))
        );
        assert_eq!(default.margin.top, MarginValue::ZERO);
        assert_eq!(
            custom.margin,
            MarginEdges {
                top: MarginValue::Length(LengthPercentage::Px(4.0)),
                right: MarginValue::Length(LengthPercentage::Px(5.0)),
                bottom: MarginValue::Length(LengthPercentage::Px(4.0)),
                left: MarginValue::Length(LengthPercentage::Px(5.0)),
            }
        );
    }

    #[test]
    fn longhands_and_shorthands_compete_by_real_cascade_priority() {
        let document = parse_document(
            "<style>
                .card { margin: 1px 2px 3px 4px !important; padding: 2px; border: 1px solid red }
                #hero { margin-left: 99px; padding-right: 3em; border-left: 5px solid blue }
             </style>
             <div id='hero' class='card'>x</div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let hero = computed.style_for(find_by_id(&document, "hero")).unwrap();

        assert_eq!(
            hero.margin.left,
            MarginValue::Length(LengthPercentage::Px(4.0))
        );
        assert_eq!(hero.padding.right, LengthPercentage::Px(54.0));
        assert_eq!(hero.border.left.width_px, 5.0);
        assert_eq!(hero.border.left.style, BorderStyle::Solid);
        assert_eq!(hero.border.left.color, CssColor::BLUE);
        assert_eq!(hero.border.top.width_px, 1.0);
        assert_eq!(hero.border.top.color, CssColor::RED);
    }

    #[test]
    fn computes_percent_auto_sizes_box_sizing_and_relative_units() {
        let document = parse_document(
            "<div id='box' style='font-size:20px; width:50%; min-width:12em; max-width:40rem; height:120px; min-height:4em; margin:10% auto -1em; padding:5% 1em; box-sizing:border-box; border-width:thin medium thick 4px; border-style:solid; border-color:red green blue #123456'>x</div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let style = computed.style_for(find_by_id(&document, "box")).unwrap();

        assert_eq!(style.width, Some(LengthPercentage::Percent(0.5)));
        assert_eq!(style.min_width, LengthPercentage::Px(240.0));
        assert_eq!(style.max_width, Some(LengthPercentage::Px(720.0)));
        assert_eq!(style.height, Some(LengthPercentage::Px(120.0)));
        assert_eq!(style.min_height, LengthPercentage::Px(80.0));
        assert_eq!(style.box_sizing, BoxSizing::BorderBox);
        assert_eq!(
            style.margin.top,
            MarginValue::Length(LengthPercentage::Percent(0.1))
        );
        assert_eq!(style.margin.right, MarginValue::Auto);
        assert_eq!(
            style.margin.bottom,
            MarginValue::Length(LengthPercentage::Px(-20.0))
        );
        assert_eq!(style.margin.left, MarginValue::Auto);
        assert_eq!(style.padding.top, LengthPercentage::Percent(0.05));
        assert_eq!(style.padding.right, LengthPercentage::Px(20.0));
        assert_eq!(style.border.top.width_px, 1.0);
        assert_eq!(style.border.right.width_px, 3.0);
        assert_eq!(style.border.bottom.width_px, 5.0);
        assert_eq!(style.border.left.width_px, 4.0);
        assert_eq!(style.border.top.color, CssColor::RED);
        assert_eq!(style.border.right.color, CssColor::GREEN);
        assert_eq!(style.border.bottom.color, CssColor::BLUE);
        assert_eq!(
            style.border.left.color,
            CssColor {
                red: 0x12,
                green: 0x34,
                blue: 0x56,
                alpha: 255,
            }
        );
    }

    #[test]
    fn parses_legacy_and_modern_rgb_hsl_colors_across_box_properties() {
        let document = parse_document(
            "<div id='modern' style='color:rgb(300 -10 50 / 50%); background-color:hsl(240 100% 50%); border:2px solid rgb(10 20 30); border-right-color:hsla(120,100%,25%,.5)'>x</div>
             <div id='legacy' style='color:rgba(255, 0, 128, .25); background-color:hsl(.5turn, 100%, 50%); border-color:rgb(100%,0%,0%) rebeccapurple hsl(60 100% 50%) cyan'>y</div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let modern = computed.style_for(find_by_id(&document, "modern")).unwrap();
        let legacy = computed.style_for(find_by_id(&document, "legacy")).unwrap();

        assert_eq!(
            modern.color,
            CssColor {
                red: 255,
                green: 0,
                blue: 50,
                alpha: 128,
            }
        );
        assert_eq!(modern.background_color, CssColor::BLUE);
        assert_eq!(
            modern.border.top.color,
            CssColor {
                red: 10,
                green: 20,
                blue: 30,
                alpha: 255,
            }
        );
        assert_eq!(
            modern.border.right.color,
            CssColor {
                red: 0,
                green: 128,
                blue: 0,
                alpha: 128,
            }
        );

        assert_eq!(
            legacy.color,
            CssColor {
                red: 255,
                green: 0,
                blue: 128,
                alpha: 64,
            }
        );
        assert_eq!(
            legacy.background_color,
            CssColor {
                red: 0,
                green: 255,
                blue: 255,
                alpha: 255,
            }
        );
        assert_eq!(legacy.border.top.color, CssColor::RED);
        assert_eq!(
            legacy.border.right.color,
            CssColor {
                red: 102,
                green: 51,
                blue: 153,
                alpha: 255,
            }
        );
        assert_eq!(
            legacy.border.bottom.color,
            CssColor {
                red: 255,
                green: 255,
                blue: 0,
                alpha: 255,
            }
        );
        assert_eq!(
            legacy.border.left.color,
            CssColor {
                red: 0,
                green: 255,
                blue: 255,
                alpha: 255,
            }
        );
    }

    #[test]
    fn background_color_shorthand_competes_with_longhand_in_the_cascade() {
        let document = parse_document(
            "<style>
               #a { background-color:red; background:#00ff00 }
               #b { background:blue; background-color:rgb(180 35 24) }
               #c { background:red !important; background-color:blue }
               #d { background:none }
             </style>
             <div id='a'></div><div id='b'></div><div id='c'></div><div id='d'></div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "a"))
                .unwrap()
                .background_color,
            CssColor {
                red: 0,
                green: 255,
                blue: 0,
                alpha: 255
            }
        );
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "b"))
                .unwrap()
                .background_color,
            CssColor {
                red: 180,
                green: 35,
                blue: 24,
                alpha: 255
            }
        );
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "c"))
                .unwrap()
                .background_color,
            CssColor::RED
        );
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "d"))
                .unwrap()
                .background_color,
            CssColor::TRANSPARENT
        );
    }

    #[test]
    fn computes_generated_pseudo_content_with_host_inheritance_and_own_box_style() {
        let document = parse_document(
            "<style>
               #note { color:#123456; font-size:22px }
               #note::before { content:'[' 'NEW' '] '; color:#b42318; background:#eef2ff; padding:2px 4px; border:1px solid #4338ca }
               #note::after { content:' hidden'; content:none }
             </style><p id='note'>Body</p>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let note = find_by_id(&document, "note");
        let before = computed
            .pseudo_style_for(note, PseudoElement::Before)
            .expect("before should be generated");
        assert_eq!(before.content, "[NEW] ");
        assert_eq!(before.style.font_size_px, 22.0);
        assert_eq!(
            before.style.color,
            CssColor {
                red: 180,
                green: 35,
                blue: 24,
                alpha: 255,
            }
        );
        assert_eq!(
            before.style.background_color,
            CssColor {
                red: 238,
                green: 242,
                blue: 255,
                alpha: 255,
            }
        );
        assert_eq!(before.style.padding.left, LengthPercentage::Px(4.0));
        assert_eq!(before.style.border.left.width_px, 1.0);
        assert!(
            computed
                .pseudo_style_for(note, PseudoElement::After)
                .is_none()
        );
    }

    #[test]
    fn generated_content_reads_originating_element_attributes() {
        let document = parse_document(
            "<style>
               #badge::before { content:'[' attr(data-kind) '] ' attr(title) ' ' attr(missing) }
             </style>
             <p id='badge' data-kind='ready' title='Launch'>Body</p>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let badge = find_by_id(&document, "badge");
        let before = computed
            .pseudo_style_for(badge, PseudoElement::Before)
            .expect("attr() content should generate before text");

        assert_eq!(before.content, "[ready] Launch ");
    }

    #[test]
    fn quotes_inherit_and_nested_q_uses_last_pair_at_deeper_levels() {
        let document = parse_document(
            "<style>div { --marks:'«' '»' '‹' '›'; quotes:var(--marks) }
             #inner::before { quotes:'(' ')' '[' ']'; color:red }
             </style><div><q id='outer'>A<q id='inner'>B<q id='deep'>C</q></q></q>
             <q id='next'>D</q></div>",
        );
        let computed = compute_styles(&document, &collect_author_styles(&document).styles);
        let content = |id, pseudo| {
            computed
                .pseudo_style_for(find_by_id(&document, id), pseudo)
                .unwrap()
                .content
                .as_str()
        };
        assert_eq!(content("outer", PseudoElement::Before), "«");
        assert_eq!(content("inner", PseudoElement::Before), "[");
        assert_eq!(content("deep", PseudoElement::Before), "‹");
        assert_eq!(content("deep", PseudoElement::After), "›");
        assert_eq!(content("inner", PseudoElement::After), "›");
        assert_eq!(content("outer", PseudoElement::After), "»");
        assert_eq!(content("next", PseudoElement::Before), "«");
        assert_eq!(
            computed.quotes_for(find_by_id(&document, "inner")),
            computed.quotes_for(find_by_id(&document, "outer"))
        );
    }

    #[test]
    fn quote_commands_follow_document_order_with_none_and_underflow() {
        let document = parse_document(
            "<style>
             div { quotes:'A' 'Z' 'B' 'Y' }
             #underflow::before { content:close-quote no-close-quote open-quote }
             #silent::before { quotes:none; content:open-quote }
             #controls::before { content:no-open-quote close-quote no-close-quote }
             #end::before { content:close-quote close-quote open-quote close-quote }
             </style><div><span id='underflow'></span><span id='silent'></span>
             <span id='controls'></span><span id='end'></span></div>",
        );
        let computed = compute_styles(&document, &collect_author_styles(&document).styles);
        let before = |id| {
            computed
                .pseudo_style_for(find_by_id(&document, id), PseudoElement::Before)
                .unwrap()
                .content
                .as_str()
        };
        assert_eq!(before("underflow"), "A");
        assert_eq!(before("silent"), "");
        assert_eq!(before("controls"), "Y");
        assert_eq!(before("end"), "ZAZ");
    }

    #[test]
    fn hidden_and_absent_pseudos_do_not_mutate_quotes_or_counters() {
        let document = parse_document(
            "<style>
             div { quotes:'A' 'Z' 'B' 'Y'; counter-reset:n }
             .hidden { display:none; counter-increment:n 10 }
             .hidden span { display:block; counter-increment:n 10 }
             .hidden span::before { content:open-quote; counter-increment:n 10 }
             #disabled::before { display:none; content:open-quote; counter-increment:n 10 }
             #absent::before { content:none; counter-increment:n 10 }
             img::before, br::before { content:open-quote; counter-increment:n 10 }
             #visible::before { content:open-quote ' ' counter(n) }
             </style><div><span class='hidden'><span id='hidden'>x</span></span>
             <span id='disabled'></span><span id='absent'></span><img><br><span id='visible'>x</span></div>",
        );
        let computed = compute_styles(&document, &collect_author_styles(&document).styles);
        for id in ["hidden", "disabled", "absent"] {
            assert!(
                computed
                    .pseudo_style_for(find_by_id(&document, id), PseudoElement::Before)
                    .is_none()
            );
        }
        assert_eq!(
            computed
                .pseudo_style_for(find_by_id(&document, "visible"), PseudoElement::Before)
                .unwrap()
                .content,
            "A 0"
        );
    }

    #[test]
    fn quote_cascade_ignores_invalid_values_and_losing_content_has_no_effect() {
        let document = parse_document(
            "<style>
             div { quotes:'A' 'Z' 'B' 'Y' !important; quotes:'bad'; }
             #one::before { content:open-quote; content:'winner' !important; content:open-quote bad; }
             #two::before { content:open-quote; quotes:unset }
             #two::after { content:close-quote; quotes:initial }
             #auto::before { content:open-quote; quotes:auto }
             #auto::after { content:close-quote }
             </style><div><span id='one'></span><span id='two'></span></div><q id='auto'></q>",
        );
        let computed = compute_styles(&document, &collect_author_styles(&document).styles);
        let content = |id, pseudo| {
            computed
                .pseudo_style_for(find_by_id(&document, id), pseudo)
                .unwrap()
                .content
                .as_str()
        };
        assert_eq!(content("one", PseudoElement::Before), "winner");
        assert_eq!(content("two", PseudoElement::Before), "A");
        assert_eq!(content("two", PseudoElement::After), "”");
        assert_eq!(content("auto", PseudoElement::Before), "“");
        assert_eq!(content("auto", PseudoElement::After), "”");
    }

    #[test]
    fn generated_counters_follow_sibling_and_nested_reset_scopes() {
        let document = parse_document(
            "<style>
               #outline { counter-reset: chapter }
               .item { counter-increment: chapter }
               .item::before { content:attr(data-label) ' ' counter(chapter, upper-roman) ': ' }
               .set { counter-set:chapter 9 }
               .set::before { content:'set=' counter(chapter, decimal-leading-zero) ' ' }
               .nested { counter-reset:chapter }
               .nested .sub { counter-increment:chapter }
               .nested .sub::before { content:counters(chapter, '.') ' ' }
               #outline::after { content:' total=' counter(chapter) }
             </style>
             <div id='outline'>
               <p id='one' class='item' data-label='Chapter'>One</p>
               <p id='two' class='item' data-label='Chapter'>Two</p>
               <p id='set' class='set'>Nine</p>
               <div class='nested'>
                 <p id='sub-one' class='sub'>Nested one</p>
                 <p id='sub-two' class='sub'>Nested two</p>
               </div>
             </div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);

        let before = |id: &str| {
            computed
                .pseudo_style_for(find_by_id(&document, id), PseudoElement::Before)
                .expect("expected generated before content")
                .content
                .clone()
        };

        assert_eq!(before("one"), "Chapter I: ");
        assert_eq!(before("two"), "Chapter II: ");
        assert_eq!(before("set"), "set=09 ");
        assert_eq!(before("sub-one"), "9.1 ");
        assert_eq!(before("sub-two"), "9.2 ");
        assert_eq!(
            computed
                .pseudo_style_for(find_by_id(&document, "outline"), PseudoElement::After)
                .expect("after content should observe completed child counter work")
                .content,
            " total=9"
        );
    }

    #[test]
    fn counter_formatters_cover_alpha_roman_and_leading_zero() {
        assert_eq!(format_counter(0, CounterStyle::DecimalLeadingZero), "00");
        assert_eq!(format_counter(7, CounterStyle::DecimalLeadingZero), "07");
        assert_eq!(format_counter(-3, CounterStyle::DecimalLeadingZero), "-03");
        assert_eq!(format_counter(27, CounterStyle::LowerAlpha), "aa");
        assert_eq!(format_counter(28, CounterStyle::UpperAlpha), "AB");
        assert_eq!(format_counter(9, CounterStyle::UpperRoman), "IX");
        assert_eq!(format_counter(14, CounterStyle::LowerRoman), "xiv");
        assert_eq!(format_counter(4000, CounterStyle::UpperRoman), "4000");
    }

    #[test]
    fn computes_text_alignment_line_height_and_extended_font_weight() {
        let document = parse_document(
            "<div id='parent' style='font-size:20px; line-height:1.5; text-align:center; font-weight:650'>
                <span id='inherited'>x</span>
                <span id='percent' style='font-size:10px; line-height:180%; text-align:end; font-weight:lighter'>y</span>
                <span id='length' style='line-height:2em; font-weight:500'>z</span>
             </div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let parent = computed.style_for(find_by_id(&document, "parent")).unwrap();
        let inherited = computed
            .style_for(find_by_id(&document, "inherited"))
            .unwrap();
        let percent = computed
            .style_for(find_by_id(&document, "percent"))
            .unwrap();
        let length = computed.style_for(find_by_id(&document, "length")).unwrap();

        assert_eq!(parent.text_align, TextAlign::Center);
        assert_eq!(parent.line_height, ComputedLineHeight::Number(1.5));
        assert_eq!(parent.font_weight, ComputedFontWeight::Bold);
        assert_eq!(inherited.text_align, TextAlign::Center);
        assert_eq!(inherited.line_height, ComputedLineHeight::Number(1.5));
        assert_eq!(percent.text_align, TextAlign::End);
        assert_eq!(percent.line_height, ComputedLineHeight::Px(18.0));
        assert_eq!(percent.font_weight, ComputedFontWeight::Normal);
        assert_eq!(length.line_height, ComputedLineHeight::Px(40.0));
        assert_eq!(length.font_weight, ComputedFontWeight::Normal);
    }

    #[test]
    fn computes_font_style_white_space_and_text_decoration_with_ua_defaults() {
        let document = parse_document(
            "<div id='parent' style='font-style:italic; white-space:pre-wrap; text-decoration:underline line-through'>
                <span id='child'>x</span>
                <span id='clear' style='font-style:normal; white-space:nowrap; text-decoration:none'>y</span>
             </div>
             <strong id='strong'>b</strong><em id='em'>i</em><u id='u'>u</u><del id='del'>d</del><pre id='pre'>p</pre>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let parent = computed.style_for(find_by_id(&document, "parent")).unwrap();
        let child = computed.style_for(find_by_id(&document, "child")).unwrap();
        let clear = computed.style_for(find_by_id(&document, "clear")).unwrap();

        assert_eq!(parent.font_style, FontStyle::Italic);
        assert_eq!(parent.white_space, WhiteSpace::PreWrap);
        assert_eq!(
            parent.text_decoration_line,
            TextDecorationLine {
                underline: true,
                line_through: true,
            }
        );
        assert_eq!(child.font_style, FontStyle::Italic);
        assert_eq!(child.white_space, WhiteSpace::PreWrap);
        assert_eq!(child.text_decoration_line, parent.text_decoration_line);
        assert_eq!(clear.font_style, FontStyle::Normal);
        assert_eq!(clear.white_space, WhiteSpace::NoWrap);
        assert_eq!(clear.text_decoration_line, TextDecorationLine::NONE);
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "strong"))
                .unwrap()
                .font_weight,
            ComputedFontWeight::Bold
        );
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "em"))
                .unwrap()
                .font_style,
            FontStyle::Italic
        );
        assert!(
            computed
                .style_for(find_by_id(&document, "u"))
                .unwrap()
                .text_decoration_line
                .underline
        );
        assert!(
            computed
                .style_for(find_by_id(&document, "del"))
                .unwrap()
                .text_decoration_line
                .line_through
        );
        assert_eq!(
            computed
                .style_for(find_by_id(&document, "pre"))
                .unwrap()
                .white_space,
            WhiteSpace::Pre
        );
    }

    #[test]
    fn computes_spacing_and_text_transform_with_inheritance_and_relative_units() {
        let document = parse_document(
            "<div id='parent' style='font-size:20px; letter-spacing:0.1em; word-spacing:25%; text-transform:uppercase'>
                <span id='child'>straße test</span>
                <span id='override' style='font-size:10px; letter-spacing:-1px; word-spacing:2em; text-transform:lowercase'>HELLO WORLD</span>
                <span id='caps' style='text-transform:capitalize'>hello world</span>
             </div>",
        );
        let author = collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let parent = computed.style_for(find_by_id(&document, "parent")).unwrap();
        let child = computed.style_for(find_by_id(&document, "child")).unwrap();
        let override_style = computed
            .style_for(find_by_id(&document, "override"))
            .unwrap();
        let caps = computed.style_for(find_by_id(&document, "caps")).unwrap();

        assert_eq!(parent.letter_spacing_px, 2.0);
        assert_eq!(parent.word_spacing_px, 5.0);
        assert_eq!(parent.text_transform, TextTransform::Uppercase);
        assert_eq!(child.letter_spacing_px, 2.0);
        assert_eq!(child.word_spacing_px, 5.0);
        assert_eq!(child.text_transform, TextTransform::Uppercase);
        assert_eq!(override_style.letter_spacing_px, -1.0);
        assert_eq!(override_style.word_spacing_px, 20.0);
        assert_eq!(override_style.text_transform, TextTransform::Lowercase);
        assert_eq!(caps.text_transform, TextTransform::Capitalize);
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
