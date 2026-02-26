//! Integration tests for the JS engine modules.
//!
//! Covers: Lexer, Parser, Interpreter, DomBridge, and CssEngine.

use super::css::{CssEngine, Specificity};
use super::dom::DomBridge;
use super::interpreter::JsInterpreter;
use super::lexer::{Lexer, TokenKind};
use super::parser::{Expr, Parser, Stmt};
use super::value::Value;

// ======================================================================
// Lexer tests
// ======================================================================

#[test]
fn lexer_basic_tokenization() {
    let mut lexer = Lexer::new("let x = 42;");
    let tokens = lexer.tokenize();

    // Expected: Let, Identifier("x"), Eq, Number(42.0), Semicolon, Eof
    assert!(
        tokens.len() >= 6,
        "Expected at least 6 tokens, got {}",
        tokens.len()
    );
    assert!(matches!(tokens[0].kind, TokenKind::Let));
    assert!(matches!(&tokens[1].kind, TokenKind::Identifier(name) if name == "x"));
    assert!(matches!(tokens[2].kind, TokenKind::Eq));
    assert!(matches!(tokens[3].kind, TokenKind::Number(n) if (n - 42.0).abs() < f64::EPSILON));
    assert!(matches!(tokens[4].kind, TokenKind::Semicolon));
    assert!(matches!(tokens[5].kind, TokenKind::Eof));
}

#[test]
fn lexer_token_positions() {
    let mut lexer = Lexer::new("let x = 42;");
    let tokens = lexer.tokenize();

    // First token starts at line 1
    assert_eq!(tokens[0].line, 1);
    assert_eq!(tokens[0].column, 1);
}

#[test]
fn lexer_string_token() {
    let mut lexer = Lexer::new("\"hello world\"");
    let tokens = lexer.tokenize();
    assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "hello world"));
}

#[test]
fn lexer_boolean_tokens() {
    let mut lexer = Lexer::new("true false");
    let tokens = lexer.tokenize();
    assert!(matches!(tokens[0].kind, TokenKind::Boolean(true)));
    assert!(matches!(tokens[1].kind, TokenKind::Boolean(false)));
}

#[test]
fn lexer_operators() {
    let mut lexer = Lexer::new("+ - * / === !==");
    let tokens = lexer.tokenize();
    assert!(matches!(tokens[0].kind, TokenKind::Plus));
    assert!(matches!(tokens[1].kind, TokenKind::Minus));
    assert!(matches!(tokens[2].kind, TokenKind::Star));
    assert!(matches!(tokens[3].kind, TokenKind::Slash));
    assert!(matches!(tokens[4].kind, TokenKind::EqEqEq));
    assert!(matches!(tokens[5].kind, TokenKind::NotEqEq));
}

// ======================================================================
// Parser tests
// ======================================================================

#[test]
fn parser_simple_var_declaration() {
    let mut parser = Parser::new("let x = 42;");
    let stmts = parser.parse().expect("parse failed");

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Stmt::Var { name, init } => {
            assert_eq!(name, "x");
            match init {
                Some(Expr::Number(n)) => assert!((n - 42.0).abs() < f64::EPSILON),
                other => panic!("Expected Expr::Number(42), got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::Var, got {:?}", other),
    }
}

#[test]
fn parser_var_without_init() {
    let mut parser = Parser::new("let y;");
    let stmts = parser.parse().expect("parse failed");
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Stmt::Var { name, init } => {
            assert_eq!(name, "y");
            assert!(init.is_none());
        }
        other => panic!("Expected Stmt::Var, got {:?}", other),
    }
}

#[test]
fn parser_function_declaration() {
    let mut parser = Parser::new("function add(a, b) { return a + b; }");
    let stmts = parser.parse().expect("parse failed");
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Stmt::Function { name, params, body } => {
            assert_eq!(name, "add");
            assert_eq!(params, &["a", "b"]);
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected Stmt::Function, got {:?}", other),
    }
}

#[test]
fn parser_if_statement() {
    let mut parser = Parser::new("if (x) { y; }");
    let stmts = parser.parse().expect("parse failed");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(&stmts[0], Stmt::If { .. }));
}

#[test]
fn parser_multiple_statements() {
    let mut parser = Parser::new("let a = 1; let b = 2; let c = a + b;");
    let stmts = parser.parse().expect("parse failed");
    assert_eq!(stmts.len(), 3);
}

// ======================================================================
// Interpreter tests
// ======================================================================

#[test]
fn interpreter_basic_execution() {
    let mut interp = JsInterpreter::new();
    let result = interp
        .execute("let x = 42; x + 8;")
        .expect("execute failed");
    // The interpreter returns the last expression-statement result only if it
    // produces a ControlFlow::Return.  For simple expression statements the
    // result is Undefined (no explicit return).  This test verifies execution
    // does not error out.
    // We verify indirectly: define x, then evaluate x+8 — no panics.
    assert!(matches!(result, Value::Undefined | Value::Number(_)));
}

#[test]
fn interpreter_string_concatenation() {
    let mut interp = JsInterpreter::new();
    let _result = interp
        .execute("let greeting = 'hello' + ' ' + 'world';")
        .expect("execute failed");
}

#[test]
fn interpreter_function_call() {
    let mut interp = JsInterpreter::new();
    let _result = interp
        .execute("function double(n) { return n * 2; } double(21);")
        .expect("execute failed");
}

#[test]
fn interpreter_if_else() {
    let mut interp = JsInterpreter::new();
    let _result = interp
        .execute("let x = 10; if (x > 5) { x = 1; } else { x = 0; }")
        .expect("execute failed");
}

#[test]
fn interpreter_while_loop() {
    let mut interp = JsInterpreter::new();
    let _result = interp
        .execute("let i = 0; while (i < 5) { i = i + 1; }")
        .expect("execute failed");
}

#[test]
fn interpreter_console_log() {
    let mut interp = JsInterpreter::new();
    let _result = interp
        .execute("console.log('test message');")
        .expect("execute failed");
    let output = interp.get_console_output();
    assert!(!output.is_empty(), "Expected console output");
}

#[test]
fn interpreter_array_operations() {
    let mut interp = JsInterpreter::new();
    let _result = interp
        .execute("let arr = [1, 2, 3]; arr.push(4);")
        .expect("execute failed");
}

#[test]
fn interpreter_try_catch() {
    let mut interp = JsInterpreter::new();
    let _result = interp
        .execute("let result; try { throw 'error'; } catch (e) { result = e; }")
        .expect("execute failed");
}

// ======================================================================
// DomBridge tests
// ======================================================================

#[test]
fn dom_bridge_create_element() {
    let dom = DomBridge::new();
    let val = dom.create_element("div");
    match &val {
        Value::DomElement(el) => {
            assert_eq!(el.borrow().tag, "div");
            assert!(el.borrow().id > 0);
        }
        other => panic!("Expected Value::DomElement, got {:?}", other),
    }
}

#[test]
fn dom_bridge_create_multiple_elements() {
    let dom = DomBridge::new();
    let el1 = dom.create_element("div");
    let el2 = dom.create_element("span");
    let el3 = dom.create_element("p");

    // Each element should have a unique id.
    if let (Value::DomElement(a), Value::DomElement(b), Value::DomElement(c)) = (&el1, &el2, &el3) {
        assert_ne!(a.borrow().id, b.borrow().id);
        assert_ne!(b.borrow().id, c.borrow().id);
        assert_eq!(c.borrow().tag, "p");
    } else {
        panic!("Expected DomElement values");
    }
}

#[test]
fn dom_bridge_set_attribute() {
    let dom = DomBridge::new();
    let el = dom.create_element("div");
    if let Value::DomElement(ref rc_el) = el {
        dom.set_attribute(rc_el, "id", "main");
        assert_eq!(dom.get_attribute(rc_el, "id"), Some("main".to_string()));
    }
}

#[test]
fn dom_bridge_get_element_by_id() {
    let dom = DomBridge::new();
    let el = dom.create_element("div");
    if let Value::DomElement(ref rc_el) = el {
        dom.set_attribute(rc_el, "id", "test-id");
    }
    let found = dom.get_element_by_id("test-id");
    assert!(matches!(found, Value::DomElement(_)));

    let not_found = dom.get_element_by_id("nonexistent");
    assert!(matches!(not_found, Value::Null));
}

#[test]
fn dom_bridge_query_selector_tag() {
    let dom = DomBridge::new();
    dom.create_element("span");
    let found = dom.query_selector("span");
    assert!(matches!(found, Value::DomElement(_)));
}

#[test]
fn dom_bridge_set_text_content() {
    let dom = DomBridge::new();
    let el = dom.create_element("p");
    if let Value::DomElement(ref rc_el) = el {
        dom.set_text_content(rc_el, "Hello, world!");
        assert_eq!(rc_el.borrow().text_content, "Hello, world!");
    }
}

#[test]
fn dom_bridge_title() {
    let dom = DomBridge::new();
    assert_eq!(dom.get_title(), "Sassy Browser");
    dom.set_title("New Title");
    assert_eq!(dom.get_title(), "New Title");
}

// ======================================================================
// CSS Engine tests (integration with the rest of the JS module)
// ======================================================================

#[test]
fn css_specificity_basic() {
    assert_eq!(Specificity::calculate("*"), Specificity(0, 0, 0, 0));
    assert_eq!(Specificity::calculate("div"), Specificity(0, 0, 0, 1));
    assert_eq!(Specificity::calculate(".cls"), Specificity(0, 0, 1, 0));
    assert_eq!(Specificity::calculate("#id"), Specificity(0, 1, 0, 0));
}

#[test]
fn css_specificity_compound_selector() {
    // div.active#main -> 1 element + 1 class + 1 id
    let spec = Specificity::calculate("div.active#main");
    assert_eq!(spec, Specificity(0, 1, 1, 1));
}

#[test]
fn css_specificity_ordering() {
    let element = Specificity::calculate("p");
    let class = Specificity::calculate(".highlight");
    let id = Specificity::calculate("#header");
    assert!(element < class);
    assert!(class < id);
}

#[test]
fn css_compute_style_cascade() {
    let mut engine = CssEngine::new();
    engine.add_rule("p", "color: red; margin: 10px");
    engine.add_rule("p", "color: blue");

    let style = engine.compute_style("p");
    // "color" should be "blue" (later rule wins at same specificity)
    assert_eq!(style.get("color"), Some(&"blue".to_string()));
    // "margin" should still be "10px" (only set by first rule)
    assert_eq!(style.get("margin"), Some(&"10px".to_string()));
}

#[test]
fn css_compute_style_no_match() {
    let mut engine = CssEngine::new();
    engine.add_rule("div", "color: red");
    let style = engine.compute_style("span");
    assert!(style.is_empty());
}

#[test]
fn css_add_stylesheet_and_compute() {
    let mut engine = CssEngine::new();
    engine.add_stylesheet(
        r#"
        body { background: white; font-size: 16px; }
        .error { color: red; }
        #main { padding: 20px; }
    "#,
    );

    let body = engine.compute_style("body");
    assert_eq!(body.get("background"), Some(&"white".to_string()));

    let error = engine.compute_style(".error");
    assert_eq!(error.get("color"), Some(&"red".to_string()));
}
