//! Original ECMAScript lexer, parser, bytecode compiler and interpreter.
//!
//! This crate intentionally owns the JavaScript implementation. It does not embed a
//! third-party ECMAScript engine.

mod bytecode;
mod error;
mod lexer;
mod parser;
mod runtime;
mod value;

pub use error::{JsError, JsErrorKind};
pub use lexer::{Token, TokenKind, tokenize};
pub use parser::{
    BinaryOp, Expression, LogicalOp, Program, Statement, UnaryOp, VariableDeclarator, VariableKind,
    parse_script,
};
pub use runtime::JsRuntime;
pub use value::JsValue;

pub use bytecode::{CompiledScript, compile_script};
