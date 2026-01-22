#![allow(dead_code, unused_variables, unused_imports)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use super::Stmt;

pub type NativeFunction = fn(Vec<Value>) -> Value;

#[derive(Clone, Debug)]
pub enum PromiseState {
    Pending,
    Fulfilled(Box<Value>),
    Rejected(Box<Value>),
}

#[derive(Clone)]
pub struct PromiseHandle {
    pub state: Rc<RefCell<PromiseState>>,
    pub on_fulfill: Rc<RefCell<Vec<Value>>>,
    pub on_reject: Rc<RefCell<Vec<Value>>>,
}

impl PromiseHandle {
    pub fn new() -> Self {
        PromiseHandle {
            state: Rc::new(RefCell::new(PromiseState::Pending)),
            on_fulfill: Rc::new(RefCell::new(Vec::new())),
            on_reject: Rc::new(RefCell::new(Vec::new())),
        }
    }
    
    pub fn resolve(&self, value: Value) {
        *self.state.borrow_mut() = PromiseState::Fulfilled(Box::new(value));
    }
    
    pub fn reject(&self, reason: Value) {
        *self.state.borrow_mut() = PromiseState::Rejected(Box::new(reason));
    }
    
    pub fn is_pending(&self) -> bool {
        matches!(*self.state.borrow(), PromiseState::Pending)
    }
}

impl Default for PromiseHandle {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Debug for PromiseHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Promise({:?})", self.state.borrow())
    }
}

/// DOM Element reference for browser integration
#[derive(Clone, Debug)]
pub struct DomElement {
    pub id: u64,
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<DomElement>,
    pub text_content: String,
}

impl DomElement {
    pub fn new(id: u64, tag: &str) -> Self {
        DomElement {
            id,
            tag: tag.to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            text_content: String::new(),
        }
    }
}

#[derive(Clone)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<HashMap<String, Value>>>),
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        closure: Rc<RefCell<HashMap<String, Value>>>,
    },
    NativeFunction(NativeFunction),
    BoundMethod {
        this: Box<Value>,
        method_name: String,
    },
    Promise(PromiseHandle),
    DomElement(Rc<RefCell<DomElement>>),
    Event(Rc<RefCell<HashMap<String, Value>>>),
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Undefined => write!(f, "undefined"),
            Value::Null => write!(f, "null"),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::String(s) => write!(f, "{}", s),
            Value::Array(arr) => {
                let items: Vec<String> = arr.borrow().iter()
                    .map(|v| format!("{:?}", v))
                    .collect();
                write!(f, "[{}]", items.join(","))
            }
            Value::Object(map) => {
                let items: Vec<String> = map.borrow().iter()
                    .map(|(k, v)| format!("\"{}\":{:?}", k, v))
                    .collect();
                write!(f, "{{{}}}", items.join(","))
            }
            Value::Function { name, .. } => write!(f, "[Function: {}]", name),
            Value::NativeFunction(_) => write!(f, "[Native]"),
            Value::BoundMethod { method_name, .. } => write!(f, "[Method: {}]", method_name),
            Value::Promise(p) => write!(f, "Promise({:?})", p.state.borrow()),
            Value::DomElement(el) => write!(f, "[Element: <{}>]", el.borrow().tag),
            Value::Event(_) => write!(f, "[Event]"),
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }
    
    pub fn type_of(&self) -> &'static str {
        match self {
            Value::Undefined => "undefined",
            Value::Null => "object",
            Value::Boolean(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) | Value::Object(_) | Value::Promise(_) 
            | Value::DomElement(_) | Value::Event(_) => "object",
            Value::Function { .. } | Value::NativeFunction(_) 
            | Value::BoundMethod { .. } => "function",
        }
    }
    
    pub fn to_number(&self) -> f64 {
        match self {
            Value::Undefined => f64::NAN,
            Value::Null => 0.0,
            Value::Boolean(b) => if *b { 1.0 } else { 0.0 },
            Value::Number(n) => *n,
            Value::String(s) => s.trim().parse().unwrap_or(f64::NAN),
            _ => f64::NAN,
        }
    }
    
    pub fn to_string_value(&self) -> String {
        match self {
            Value::Undefined => "undefined".to_string(),
            Value::Null => "null".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::String(s) => s.clone(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.borrow().iter()
                    .map(|v| v.to_string_value())
                    .collect();
                items.join(",")
            }
            Value::Object(_) => "[object Object]".to_string(),
            Value::Function { name, .. } => format!("function {}() {{ }}", name),
            Value::NativeFunction(_) => "function native() { [native code] }".to_string(),
            Value::BoundMethod { method_name, .. } => format!("function {}() {{ }}", method_name),
            Value::Promise(_) => "[object Promise]".to_string(),
            Value::DomElement(el) => format!("[object HTMLElement: {}]", el.borrow().tag),
            Value::Event(_) => "[object Event]".to_string(),
        }
    }
    
    pub fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Undefined, Value::Undefined) => true,
            (Value::Null, Value::Null) => true,
            (Value::Undefined, Value::Null) | (Value::Null, Value::Undefined) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Number(n), Value::String(s)) | (Value::String(s), Value::Number(n)) => {
                *n == s.parse::<f64>().unwrap_or(f64::NAN)
            }
            _ => false,
        }
    }
    
    pub fn strict_equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Undefined, Value::Undefined) => true,
            (Value::Null, Value::Null) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            _ => false,
        }
    }
}
