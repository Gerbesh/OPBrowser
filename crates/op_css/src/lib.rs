//! CSS syntax, author matching and initial computed-style primitives owned by OPBrowser.
//!
//! Parsing, selector matching, the first author cascade and inheritance live here.
//! Layout integration and broader CSS coverage remain separate milestones.

mod color;
mod computed;
mod custom;
mod named;
mod parser;
mod style;
mod tokenizer;

pub use computed::{
    BorderCollapse, BorderEdges, BorderSpacing, BorderStyle, BoxSizing, CaptionSide,
    ComputedBorder, ComputedFontWeight, ComputedLineHeight, ComputedPseudoStyle, ComputedQuotes,
    ComputedStyle, ComputedStyleMap, CssColor, CustomPropertyMap, Display, FontStyle,
    GeneratedContentItem, InsetEdges, LengthPercentage, MarginEdges, MarginValue, PaddingEdges,
    Position, TableLayout, TextAlign, TextDecorationLine, TextTransform, VerticalAlign, WhiteSpace,
    compute_styles,
};
pub use parser::{parse_declaration_list, parse_stylesheet};
pub use style::{
    MatchedDeclaration, StyleCollection, StyleError, StyleMap, StyleSource, collect_author_styles,
    collect_author_styles_with_linked, selector_matches,
};
pub use tokenizer::tokenize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssError {
    pub offset: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult<T> {
    pub value: T,
    pub errors: Vec<CssError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stylesheet {
    pub rules: Vec<StyleRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleRule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub name: String,
    pub value: Vec<TokenKind>,
    pub important: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub compounds: Vec<CompoundSelector>,
    pub combinators: Vec<Combinator>,
    pub pseudo_element: Option<PseudoElement>,
    pub specificity: Specificity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundSelector {
    pub simple: Vec<SimpleSelector>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeSelector {
    pub name: String,
    pub matcher: AttributeMatcher,
    pub value: Option<String>,
    pub case_insensitive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeMatcher {
    Exists,
    Exact,
    Includes,
    DashMatch,
    Prefix,
    Suffix,
    Substring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoElement {
    Before,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoClass {
    Root,
    FirstChild,
    LastChild,
    OnlyChild,
    FirstOfType,
    LastOfType,
    OnlyOfType,
    Empty,
    Link,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NthExpression {
    pub a: i32,
    pub b: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NthSelector {
    pub expression: NthExpression,
    pub of: Vec<Selector>,
    pub from_end: bool,
    pub same_type: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleSelector {
    Type(String),
    Universal,
    Class(String),
    Id(String),
    Attribute(AttributeSelector),
    PseudoClass(PseudoClass),
    Is(Vec<Selector>),
    Where(Vec<Selector>),
    Not(Vec<Selector>),
    NthChild(NthSelector),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
    AdjacentSibling,
    GeneralSibling,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity {
    pub ids: u16,
    pub classes: u16,
    pub types: u16,
}

impl Specificity {
    fn add_simple(&mut self, selector: &SimpleSelector) {
        match selector {
            SimpleSelector::Id(_) => self.ids = self.ids.saturating_add(1),
            SimpleSelector::Class(_)
            | SimpleSelector::Attribute(_)
            | SimpleSelector::PseudoClass(_) => {
                self.classes = self.classes.saturating_add(1);
            }
            SimpleSelector::NthChild(nth) => {
                self.classes = self.classes.saturating_add(1);
                self.add_specificity(
                    nth.of
                        .iter()
                        .map(|selector| selector.specificity)
                        .max()
                        .unwrap_or_default(),
                );
            }
            SimpleSelector::Is(selectors) | SimpleSelector::Not(selectors) => {
                self.add_specificity(
                    selectors
                        .iter()
                        .map(|selector| selector.specificity)
                        .max()
                        .unwrap_or_default(),
                );
            }
            SimpleSelector::Where(_) => {}
            SimpleSelector::Type(_) => self.types = self.types.saturating_add(1),
            SimpleSelector::Universal => {}
        }
    }

    fn add_specificity(&mut self, other: Specificity) {
        self.ids = self.ids.saturating_add(other.ids);
        self.classes = self.classes.saturating_add(other.classes);
        self.types = self.types.saturating_add(other.types);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Whitespace,
    Ident(String),
    AtKeyword(String),
    Hash { value: String, id: bool },
    String(String),
    BadString,
    Url(String),
    BadUrl,
    Number(String),
    Percentage(String),
    Dimension { number: String, unit: String },
    Function(String),
    Colon,
    Semicolon,
    Comma,
    OpenCurly,
    CloseCurly,
    OpenParen,
    CloseParen,
    OpenSquare,
    CloseSquare,
    Delim(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizeResult {
    pub tokens: Vec<Token>,
    pub errors: Vec<CssError>,
}
