//! JavaScript Interpreter Module
//! A pure-Rust JS interpreter for Sassy Browser
//!
//! Features:
//! - Full lexer/parser/interpreter
//! - String, Array, Object methods
//! - Math, JSON objects
//! - Promise with then/catch/finally
//! - fetch() API with HTTP support
//! - DOM bridge for browser integration

pub mod css;
pub mod dom;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod value;

#[cfg(test)]
mod tests;

pub use css::CssEngine;
pub use dom::DomBridge;
pub use interpreter::JsInterpreter;
pub use lexer::Lexer;
pub use parser::{Expr, Parser, Stmt};
pub use value::Value;
