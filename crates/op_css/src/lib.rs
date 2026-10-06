//! CSS syntax, author matching and initial computed-style primitives owned by OPBrowser.
//!
//! Parsing, selector matching, the first author cascade and inheritance live here.
//! Layout integration and broader CSS coverage remain separate milestones.

mod computed;
mod parser;
mod style;
mod tokenizer;

pub use computed::{
    BorderEdges, BorderStyle, BoxSizing, ComputedBorder, ComputedFontWeight, ComputedLineHeight,
    ComputedStyle, ComputedStyleMap, CssColor, Display, FontStyle, LengthPercentage, MarginEdges,
    MarginValue, PaddingEdges, TextAlign, TextDecorationLine, TextTransform, WhiteSpace,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoClass {
    Root,
    FirstChild,
    LastChild,
    OnlyChild,
    Empty,
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleSelector {
    Type(String),
    Universal,
    Class(String),
    Id(String),
    Attribute(AttributeSelector),
    PseudoClass(PseudoClass),
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
            SimpleSelector::Type(_) => self.types = self.types.saturating_add(1),
            SimpleSelector::Universal => {}
        }
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
