//! CSS syntax primitives owned by OPBrowser.
//!
//! This crate deliberately starts with syntax and data modelling only. Matching,
//! cascade, computed values and layout integration are separate milestones.

mod parser;
mod tokenizer;

pub use parser::{parse_declaration_list, parse_stylesheet};
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
pub enum SimpleSelector {
    Type(String),
    Universal,
    Class(String),
    Id(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
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
            SimpleSelector::Class(_) => self.classes = self.classes.saturating_add(1),
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
    Hash(String),
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
