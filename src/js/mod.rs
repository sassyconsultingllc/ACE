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

pub mod lexer;
pub mod value;
pub mod parser;
pub mod interpreter;
pub mod dom;
pub mod css;

#[cfg(test)]
mod tests;

pub use lexer::Lexer;
pub use value::Value;
pub use parser::{Parser, Expr, Stmt};
pub use interpreter::JsInterpreter;
pub use dom::DomBridge;
pub use css::CssEngine;
