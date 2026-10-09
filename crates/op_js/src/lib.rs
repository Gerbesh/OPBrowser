//! Original ECMAScript lexer, parser, bytecode compiler and interpreter.
//!
//! This crate intentionally owns the JavaScript implementation. It does not embed a
//! third-party ECMAScript engine.

mod bytecode;
mod error;
mod json;
mod lexer;
mod parser;
mod runtime;
mod value;

pub use error::{JsError, JsErrorKind};
pub use lexer::{Token, TokenKind, tokenize};
pub use parser::{
    AssignmentTarget, BinaryOp, CatchClause, Expression, ForInitializer, LogicalOp, ObjectProperty,
    Program, Statement, SwitchCase, UnaryOp, UpdateOp, VariableDeclarator, VariableKind,
    parse_script,
};
pub use runtime::{DomElementSnapshot, DomOperation, DomTextMutation, JsRuntime, TextResponse};
pub use value::{JsValue, ObjectId};

pub use bytecode::{CompiledScript, compile_script};
