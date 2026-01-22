#![allow(unexpected_cfgs)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use super::parser::{Parser, Expr, Stmt, ArrowBody};
use super::value::{Value, PromiseHandle, PromiseState};
use super::dom::DomBridge;

pub struct JsInterpreter {
    globals: Rc<RefCell<HashMap<String, Value>>>,
    scope_stack: Vec<Rc<RefCell<HashMap<String, Value>>>>,
    console_output: Vec<String>,
    loop_depth: usize,
    pub dom: Option<DomBridge>,
}

enum ControlFlow {
    Return(Value),
    Break,
    Continue,
}

impl JsInterpreter {
    pub fn new() -> Self {
        let globals = Rc::new(RefCell::new(HashMap::new()));
        let mut interp = JsInterpreter {
            globals: globals.clone(),
            scope_stack: vec![globals],
            console_output: Vec::new(),
            loop_depth: 0,
            dom: None,
        };
        interp.init_globals();
        interp
    }
    
    pub fn with_dom(mut self, dom: DomBridge) -> Self {
        // Inject DOM APIs
        self.globals.borrow_mut().insert("document".to_string(), dom.create_document_object());
        self.globals.borrow_mut().insert("window".to_string(), dom.create_window_object());
        self.dom = Some(dom);
        self
    }
    
    fn init_globals(&mut self) {
        // console
        let console = Rc::new(RefCell::new(HashMap::new()));
        self.globals.borrow_mut().insert("console".to_string(), Value::Object(console));
        
        // Math
        let math = Rc::new(RefCell::new(HashMap::new()));
        math.borrow_mut().insert("PI".to_string(), Value::Number(std::f64::consts::PI));
        math.borrow_mut().insert("E".to_string(), Value::Number(std::f64::consts::E));
        self.globals.borrow_mut().insert("Math".to_string(), Value::Object(math));
        
        // JSON, Object, Array
        self.globals.borrow_mut().insert("Object".to_string(), Value::Object(Rc::new(RefCell::new(HashMap::new()))));
        self.globals.borrow_mut().insert("Array".to_string(), Value::Object(Rc::new(RefCell::new(HashMap::new()))));
        self.globals.borrow_mut().insert("JSON".to_string(), Value::Object(Rc::new(RefCell::new(HashMap::new()))));
        
        // Global functions
        self.globals.borrow_mut().insert("parseInt".to_string(), Value::NativeFunction(native_parse_int));
        self.globals.borrow_mut().insert("parseFloat".to_string(), Value::NativeFunction(native_parse_float));
        self.globals.borrow_mut().insert("isNaN".to_string(), Value::NativeFunction(native_is_nan));
        self.globals.borrow_mut().insert("isFinite".to_string(), Value::NativeFunction(native_is_finite));
        self.globals.borrow_mut().insert("Boolean".to_string(), Value::NativeFunction(native_boolean));
        self.globals.borrow_mut().insert("Number".to_string(), Value::NativeFunction(native_number));
        self.globals.borrow_mut().insert("String".to_string(), Value::NativeFunction(native_string_fn));
        self.globals.borrow_mut().insert("setTimeout".to_string(), Value::NativeFunction(native_set_timeout));
        self.globals.borrow_mut().insert("clearTimeout".to_string(), Value::NativeFunction(native_clear_timeout));
        self.globals.borrow_mut().insert("setInterval".to_string(), Value::NativeFunction(native_set_interval));
        self.globals.borrow_mut().insert("Promise".to_string(), Value::NativeFunction(native_promise_constructor));
        self.globals.borrow_mut().insert("fetch".to_string(), Value::NativeFunction(native_fetch));
        self.globals.borrow_mut().insert("alert".to_string(), Value::NativeFunction(native_alert));
        self.globals.borrow_mut().insert("confirm".to_string(), Value::NativeFunction(native_confirm));
        self.globals.borrow_mut().insert("prompt".to_string(), Value::NativeFunction(native_prompt));
    }
    
    pub fn get_console_output(&self) -> &Vec<String> {
        &self.console_output
    }
    
    pub fn execute(&mut self, source: &str) -> Result<Value, String> {
        let mut parser = Parser::new(source);
        let statements = parser.parse()?;
        
        let mut result = Value::Undefined;
        for stmt in statements {
            match self.execute_stmt(&stmt)? {
                None => {}
                Some(ControlFlow::Return(v)) => { result = v; break; }
                Some(ControlFlow::Break) | Some(ControlFlow::Continue) => {}
            }
        }
        Ok(result)
    }
    
    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<Option<ControlFlow>, String> {
        match stmt {
            Stmt::Expr(expr) => { self.evaluate(expr)?; Ok(None) }
            Stmt::Var { name, init } => {
                let value = match init {
                    Some(expr) => self.evaluate(expr)?,
                    None => Value::Undefined,
                };
                self.define(name.clone(), value);
                Ok(None)
            }
            Stmt::Block(stmts) => {
                self.push_scope();
                let mut result = None;
                for s in stmts {
                    if let Some(cf) = self.execute_stmt(s)? {
                        result = Some(cf);
                        break;
                    }
                }
                self.pop_scope();
                Ok(result)
            }
            Stmt::If { condition, consequent, alternate } => {
                if self.evaluate(condition)?.is_truthy() {
                    self.execute_stmt(consequent)
                } else if let Some(alt) = alternate {
                    self.execute_stmt(alt)
                } else {
                    Ok(None)
                }
            }
            Stmt::While { condition, body } => {
                self.loop_depth += 1;
                while self.evaluate(condition)?.is_truthy() {
                    if let Some(cf) = self.execute_stmt(body)? {
                        match cf {
                            ControlFlow::Break => break,
                            ControlFlow::Continue => continue,
                            ControlFlow::Return(v) => {
                                self.loop_depth -= 1;
                                return Ok(Some(ControlFlow::Return(v)));
                            }
                        }
                    }
                }
                self.loop_depth -= 1;
                Ok(None)
            }
            Stmt::For { init, condition, update, body } => {
                self.push_scope();
                if let Some(init_stmt) = init { self.execute_stmt(init_stmt)?; }
                
                self.loop_depth += 1;
                loop {
                    if let Some(cond) = condition {
                        if !self.evaluate(cond)?.is_truthy() { break; }
                    }
                    if let Some(cf) = self.execute_stmt(body)? {
                        match cf {
                            ControlFlow::Break => break,
                            ControlFlow::Continue => {}
                            ControlFlow::Return(v) => {
                                self.loop_depth -= 1;
                                self.pop_scope();
                                return Ok(Some(ControlFlow::Return(v)));
                            }
                        }
                    }
                    if let Some(upd) = update { self.evaluate(upd)?; }
                }
                self.loop_depth -= 1;
                self.pop_scope();
                Ok(None)
            }
            Stmt::Function { name, params, body } => {
                let closure = self.current_scope();
                let func = Value::Function {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure,
                };
                self.define(name.clone(), func);
                Ok(None)
            }
            Stmt::Return(expr) => {
                let value = match expr {
                    Some(e) => self.evaluate(e)?,
                    None => Value::Undefined,
                };
                Ok(Some(ControlFlow::Return(value)))
            }
            Stmt::Break => Ok(Some(ControlFlow::Break)),
            Stmt::Continue => Ok(Some(ControlFlow::Continue)),
            Stmt::Empty => Ok(None),
            Stmt::Try { body, catch_param, catch_body, finally_body } => {
                let result = self.execute_block(body);
                
                match result {
                    Err(e) if catch_body.is_some() => {
                        self.push_scope();
                        if let Some(param) = catch_param {
                            self.define(param.clone(), Value::String(e));
                        }
                        let r = self.execute_block(catch_body.as_ref().unwrap());
                        self.pop_scope();
                        if let Some(fin) = finally_body {
                            self.execute_block(fin)?;
                        }
                        r?;
                    }
                    _ => {
                        if let Some(fin) = finally_body {
                            self.execute_block(fin)?;
                        }
                    }
                }
                Ok(None)
            }
            Stmt::Throw(expr) => {
                let val = self.evaluate(expr)?;
                Err(val.to_string_value())
            }
        }
    }
    
    fn execute_block(&mut self, stmts: &[Stmt]) -> Result<Option<ControlFlow>, String> {
        for stmt in stmts {
            if let Some(cf) = self.execute_stmt(stmt)? {
                return Ok(Some(cf));
            }
        }
        Ok(None)
    }
    
    fn evaluate(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Boolean(b) => Ok(Value::Boolean(*b)),
            Expr::Null => Ok(Value::Null),
            Expr::Undefined => Ok(Value::Undefined),
            Expr::This => Ok(Value::Undefined),
            Expr::Identifier(name) => self.lookup(name),
            
            Expr::Array(elements) => {
                let values: Result<Vec<Value>, String> = elements.iter().map(|e| self.evaluate(e)).collect();
                Ok(Value::Array(Rc::new(RefCell::new(values?))))
            }
            
            Expr::Object(props) => {
                let map = Rc::new(RefCell::new(HashMap::new()));
                for (key, val_expr) in props {
                    map.borrow_mut().insert(key.clone(), self.evaluate(val_expr)?);
                }
                Ok(Value::Object(map))
            }
            
            Expr::Binary { left, op, right } => {
                let l = self.evaluate(left)?;
                let r = self.evaluate(right)?;
                self.binary_op(&l, op, &r)
            }
            
            Expr::Unary { op, expr } => {
                let val = self.evaluate(expr)?;
                self.unary_op(op, &val)
            }
            
            Expr::Logical { left, op, right } => {
                let l = self.evaluate(left)?;
                match op.as_str() {
                    "&&" => if !l.is_truthy() { Ok(l) } else { self.evaluate(right) },
                    "||" => if l.is_truthy() { Ok(l) } else { self.evaluate(right) },
                    "??" => if !matches!(l, Value::Null | Value::Undefined) { Ok(l) } else { self.evaluate(right) },
                    _ => Err(format!("Unknown logical operator: {}", op))
                }
            }
            
            Expr::Ternary { condition, consequent, alternate } => {
                if self.evaluate(condition)?.is_truthy() {
                    self.evaluate(consequent)
                } else {
                    self.evaluate(alternate)
                }
            }
            
            Expr::Assignment { target, value } => {
                let val = self.evaluate(value)?;
                self.assign(target, val.clone())?;
                Ok(val)
            }
            
            Expr::Compound { target, op, value } => {
                let current = self.evaluate(target)?;
                let rhs = self.evaluate(value)?;
                let result = self.binary_op(&current, op, &rhs)?;
                self.assign(target, result.clone())?;
                Ok(result)
            }
            
            Expr::Member { object, property } => {
                let obj = self.evaluate(object)?;
                self.get_property(&obj, property)
            }
            
            Expr::Index { object, index } => {
                let obj = self.evaluate(object)?;
                let idx = self.evaluate(index)?;
                match &obj {
                    Value::Array(arr) => {
                        if let Value::Number(n) = idx {
                            Ok(arr.borrow().get(n as usize).cloned().unwrap_or(Value::Undefined))
                        } else { Ok(Value::Undefined) }
                    }
                    Value::Object(map) => Ok(map.borrow().get(&idx.to_string_value()).cloned().unwrap_or(Value::Undefined)),
                    Value::String(s) => {
                        if let Value::Number(n) = idx {
                            Ok(s.chars().nth(n as usize).map(|c| Value::String(c.to_string())).unwrap_or(Value::Undefined))
                        } else { Ok(Value::Undefined) }
                    }
                    _ => Ok(Value::Undefined)
                }
            }
            
            Expr::Call { callee, args } => self.call_function(callee, args),
            
            Expr::New { callee, args: _ } => {
                match callee.as_ref() {
                    Expr::Identifier(name) if name == "Object" => Ok(Value::Object(Rc::new(RefCell::new(HashMap::new())))),
                    Expr::Identifier(name) if name == "Array" => Ok(Value::Array(Rc::new(RefCell::new(Vec::new())))),
                    _ => Ok(Value::Object(Rc::new(RefCell::new(HashMap::new()))))
                }
            }
            
            Expr::Function { name, params, body } => {
                Ok(Value::Function {
                    name: name.clone().unwrap_or_default(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: self.current_scope(),
                })
            }
            
            Expr::Arrow { params, body } => {
                let body_stmts = match body.as_ref() {
                    ArrowBody::Block(stmts) => stmts.clone(),
                    ArrowBody::Expr(expr) => vec![Stmt::Return(Some(expr.clone()))],
                };
                Ok(Value::Function {
                    name: String::new(),
                    params: params.clone(),
                    body: body_stmts,
                    closure: self.current_scope(),
                })
            }
            
            Expr::Typeof(expr) => Ok(Value::String(self.evaluate(expr)?.type_of().to_string())),
            
            Expr::PreIncrement(expr) => {
                let new_val = Value::Number(self.evaluate(expr)?.to_number() + 1.0);
                self.assign(expr, new_val.clone())?;
                Ok(new_val)
            }
            Expr::PreDecrement(expr) => {
                let new_val = Value::Number(self.evaluate(expr)?.to_number() - 1.0);
                self.assign(expr, new_val.clone())?;
                Ok(new_val)
            }
            Expr::PostIncrement(expr) => {
                let old = self.evaluate(expr)?.to_number();
                self.assign(expr, Value::Number(old + 1.0))?;
                Ok(Value::Number(old))
            }
            Expr::PostDecrement(expr) => {
                let old = self.evaluate(expr)?.to_number();
                self.assign(expr, Value::Number(old - 1.0))?;
                Ok(Value::Number(old))
            }
        }
    }
    
    fn binary_op(&self, left: &Value, op: &str, right: &Value) -> Result<Value, String> {
        match op {
            "+" => match (left, right) {
                (Value::String(a), b) => Ok(Value::String(format!("{}{}", a, b.to_string_value()))),
                (a, Value::String(b)) => Ok(Value::String(format!("{}{}", a.to_string_value(), b))),
                _ => Ok(Value::Number(left.to_number() + right.to_number()))
            },
            "-" => Ok(Value::Number(left.to_number() - right.to_number())),
            "*" => Ok(Value::Number(left.to_number() * right.to_number())),
            "/" => Ok(Value::Number(left.to_number() / right.to_number())),
            "%" => Ok(Value::Number(left.to_number() % right.to_number())),
            "**" => Ok(Value::Number(left.to_number().powf(right.to_number()))),
            "<" => Ok(Value::Boolean(left.to_number() < right.to_number())),
            "<=" => Ok(Value::Boolean(left.to_number() <= right.to_number())),
            ">" => Ok(Value::Boolean(left.to_number() > right.to_number())),
            ">=" => Ok(Value::Boolean(left.to_number() >= right.to_number())),
            "==" => Ok(Value::Boolean(left.equals(right))),
            "!=" => Ok(Value::Boolean(!left.equals(right))),
            "===" => Ok(Value::Boolean(left.strict_equals(right))),
            "!==" => Ok(Value::Boolean(!left.strict_equals(right))),
            _ => Err(format!("Unknown operator: {}", op))
        }
    }
    
    fn unary_op(&self, op: &str, val: &Value) -> Result<Value, String> {
        match op {
            "!" => Ok(Value::Boolean(!val.is_truthy())),
            "-" => Ok(Value::Number(-val.to_number())),
            "+" => Ok(Value::Number(val.to_number())),
            _ => Err(format!("Unknown unary operator: {}", op))
        }
    }
    
    fn push_scope(&mut self) { self.scope_stack.push(Rc::new(RefCell::new(HashMap::new()))); }
    fn pop_scope(&mut self) { if self.scope_stack.len() > 1 { self.scope_stack.pop(); } }
    fn current_scope(&self) -> Rc<RefCell<HashMap<String, Value>>> { self.scope_stack.last().unwrap().clone() }
    
    pub fn define<S: Into<String>>(&mut self, name: S, value: Value) {
        self.scope_stack.last().unwrap().borrow_mut().insert(name.into(), value);
    }
    
    fn lookup(&self, name: &str) -> Result<Value, String> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(val) = scope.borrow().get(name) { return Ok(val.clone()); }
        }
        Ok(Value::Undefined)
    }
    
    fn assign(&mut self, target: &Expr, value: Value) -> Result<(), String> {
        match target {
            Expr::Identifier(name) => {
                for scope in self.scope_stack.iter().rev() {
                    if scope.borrow().contains_key(name) {
                        scope.borrow_mut().insert(name.clone(), value);
                        return Ok(());
                    }
                }
                self.globals.borrow_mut().insert(name.clone(), value);
                Ok(())
            }
            Expr::Member { object, property } => {
                if let Value::Object(map) = self.evaluate(object)? {
                    map.borrow_mut().insert(property.clone(), value);
                }
                Ok(())
            }
            Expr::Index { object, index } => {
                let obj = self.evaluate(object)?;
                let idx = self.evaluate(index)?;
                match obj {
                    Value::Array(arr) => {
                        if let Value::Number(n) = idx {
                            let mut arr_ref = arr.borrow_mut();
                            let i = n as usize;
                            while arr_ref.len() <= i { arr_ref.push(Value::Undefined); }
                            arr_ref[i] = value;
                        }
                    }
                    Value::Object(map) => { map.borrow_mut().insert(idx.to_string_value(), value); }
                    _ => {}
                }
                Ok(())
            }
            _ => Err("Invalid assignment target".to_string())
        }
    }
    
    fn get_property(&mut self, obj: &Value, prop: &str) -> Result<Value, String> {
        match obj {
            Value::String(s) => self.string_property(s, prop),
            Value::Array(arr) => self.array_property(arr, prop),
            Value::Object(map) => {
                if let Some(val) = map.borrow().get(prop) { return Ok(val.clone()); }
                Ok(Value::Undefined)
            }
            Value::Promise(_) => Ok(Value::BoundMethod { this: Box::new(obj.clone()), method_name: prop.to_string() }),
            _ => Ok(Value::Undefined)
        }
    }
    
    fn string_property(&self, s: &str, prop: &str) -> Result<Value, String> {
        match prop {
            "length" => Ok(Value::Number(s.len() as f64)),
            _ => Ok(Value::BoundMethod { this: Box::new(Value::String(s.to_string())), method_name: prop.to_string() })
        }
    }
    
    fn array_property(&self, arr: &Rc<RefCell<Vec<Value>>>, prop: &str) -> Result<Value, String> {
        match prop {
            "length" => Ok(Value::Number(arr.borrow().len() as f64)),
            _ => Ok(Value::BoundMethod { this: Box::new(Value::Array(arr.clone())), method_name: prop.to_string() })
        }
    }
    
    fn call_function(&mut self, callee: &Expr, args: &[Expr]) -> Result<Value, String> {
        // console.log
        if let Expr::Member { object, property } = callee {
            if let Expr::Identifier(name) = object.as_ref() {
                if name == "console" && property == "log" {
                    let output: Vec<String> = args.iter()
                        .filter_map(|a| self.evaluate(a).ok())
                        .map(|v| format!("{:?}", v))
                        .collect();
                    self.console_output.push(output.join(" "));
                    return Ok(Value::Undefined);
                }
                if name == "Math" { return self.call_math_method(property, args); }
                if name == "JSON" { return self.call_json_method(property, args); }
                if name == "Object" { return self.call_object_method(property, args); }
                if name == "Array" { return self.call_array_static_method(property, args); }
            }
        }
        
        let func = self.evaluate(callee)?;
        let arg_vals: Vec<Value> = args.iter().filter_map(|a| self.evaluate(a).ok()).collect();
        
        match func {
            Value::Function { params, body, closure, .. } => {
                self.scope_stack.push(closure);
                self.push_scope();
                for (i, param) in params.iter().enumerate() {
                    self.define(param.clone(), arg_vals.get(i).cloned().unwrap_or(Value::Undefined));
                }
                let mut result = Value::Undefined;
                for stmt in &body {
                    if let Some(ControlFlow::Return(v)) = self.execute_stmt(stmt)? {
                        result = v;
                        break;
                    }
                }
                self.pop_scope();
                self.pop_scope();
                Ok(result)
            }
            Value::NativeFunction(func) => Ok(func(arg_vals)),
            Value::BoundMethod { this, method_name } => self.call_bound_method(&this, &method_name, &arg_vals),
            _ => Err("Not a function".to_string())
        }
    }
    
    fn call_bound_method(&mut self, this: &Value, method: &str, args: &[Value]) -> Result<Value, String> {
        match this {
            Value::String(s) => self.call_string_method(s, method, args),
            Value::Array(arr) => self.call_array_method(arr, method, args),
            Value::Promise(handle) => self.call_promise_method(handle, method, args),
            _ => Ok(Value::Undefined)
        }
    }
    
    fn call_promise_method(&mut self, handle: &PromiseHandle, method: &str, args: &[Value]) -> Result<Value, String> {
        let new_handle = PromiseHandle::new();
        let state = handle.state.borrow().clone();
        
        match method {
            "then" => {
                let on_fulfilled = args.first().cloned();
                if let PromiseState::Fulfilled(value) = state {
                    if let Some(cb) = on_fulfilled {
                        let result = self.call_value(&cb, &[*value])?;
                        new_handle.resolve(result);
                    }
                }
                Ok(Value::Promise(new_handle))
            }
            "catch" => Ok(Value::Promise(new_handle)),
            _ => Ok(Value::Undefined)
        }
    }
    
    pub fn call_value(&mut self, func: &Value, args: &[Value]) -> Result<Value, String> {
        match func {
            Value::Function { params, body, closure, .. } => {
                self.scope_stack.push(closure.clone());
                for (i, param) in params.iter().enumerate() {
                    self.scope_stack.last().unwrap().borrow_mut().insert(param.clone(), args.get(i).cloned().unwrap_or(Value::Undefined));
                }
                let mut result = Value::Undefined;
                for stmt in body {
                    if let Ok(Some(ControlFlow::Return(v))) = self.execute_stmt(stmt) {
                        result = v;
                        break;
                    }
                }
                self.scope_stack.pop();
                Ok(result)
            }
            Value::NativeFunction(f) => Ok(f(args.to_vec())),
            _ => Err("Not callable".to_string())
        }
    }
    
    fn call_string_method(&self, s: &str, method: &str, args: &[Value]) -> Result<Value, String> {
        match method {
            "toUpperCase" => Ok(Value::String(s.to_uppercase())),
            "toLowerCase" => Ok(Value::String(s.to_lowercase())),
            "trim" => Ok(Value::String(s.trim().to_string())),
            "split" => {
                let sep = args.first().map(|v| v.to_string_value()).unwrap_or_default();
                let parts: Vec<Value> = if sep.is_empty() {
                    s.chars().map(|c| Value::String(c.to_string())).collect()
                } else {
                    s.split(&sep).map(|p| Value::String(p.to_string())).collect()
                };
                Ok(Value::Array(Rc::new(RefCell::new(parts))))
            }
            "includes" => Ok(Value::Boolean(s.contains(&args.first().map(|v| v.to_string_value()).unwrap_or_default()))),
            "indexOf" => {
                let search = args.first().map(|v| v.to_string_value()).unwrap_or_default();
                Ok(Value::Number(s.find(&search).map(|i| i as f64).unwrap_or(-1.0)))
            }
            "substring" | "slice" => {
                let start = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
                let end = args.get(1).map(|v| v.to_number() as usize).unwrap_or(s.len());
                Ok(Value::String(s.chars().skip(start).take(end.saturating_sub(start)).collect()))
            }
            "replace" => {
                let search = args.first().map(|v| v.to_string_value()).unwrap_or_default();
                let replace = args.get(1).map(|v| v.to_string_value()).unwrap_or_default();
                Ok(Value::String(s.replacen(&search, &replace, 1)))
            }
            _ => Ok(Value::Undefined)
        }
    }
    
    fn call_array_method(&mut self, arr: &Rc<RefCell<Vec<Value>>>, method: &str, args: &[Value]) -> Result<Value, String> {
        match method {
            "push" => { for arg in args { arr.borrow_mut().push(arg.clone()); } Ok(Value::Number(arr.borrow().len() as f64)) }
            "pop" => Ok(arr.borrow_mut().pop().unwrap_or(Value::Undefined)),
            "shift" => Ok(if arr.borrow().is_empty() { Value::Undefined } else { arr.borrow_mut().remove(0) }),
            "join" => {
                let sep = args.first().map(|v| v.to_string_value()).unwrap_or_else(|| ",".to_string());
                Ok(Value::String(arr.borrow().iter().map(|v| v.to_string_value()).collect::<Vec<_>>().join(&sep)))
            }
            "map" => {
                let cb = args.first().cloned().unwrap_or(Value::Undefined);
                let mut result = Vec::new();
                for (i, v) in arr.borrow().iter().enumerate() {
                    result.push(self.call_value(&cb, &[v.clone(), Value::Number(i as f64)])?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(result))))
            }
            "filter" => {
                let cb = args.first().cloned().unwrap_or(Value::Undefined);
                let mut result = Vec::new();
                for (i, v) in arr.borrow().iter().enumerate() {
                    if self.call_value(&cb, &[v.clone(), Value::Number(i as f64)])?.is_truthy() {
                        result.push(v.clone());
                    }
                }
                Ok(Value::Array(Rc::new(RefCell::new(result))))
            }
            "forEach" => {
                let cb = args.first().cloned().unwrap_or(Value::Undefined);
                for (i, v) in arr.borrow().clone().iter().enumerate() {
                    self.call_value(&cb, &[v.clone(), Value::Number(i as f64)])?;
                }
                Ok(Value::Undefined)
            }
            "includes" => {
                let search = args.first().cloned().unwrap_or(Value::Undefined);
                Ok(Value::Boolean(arr.borrow().iter().any(|v| v.strict_equals(&search))))
            }
            "indexOf" => {
                let search = args.first().cloned().unwrap_or(Value::Undefined);
                Ok(Value::Number(arr.borrow().iter().position(|v| v.strict_equals(&search)).map(|i| i as f64).unwrap_or(-1.0)))
            }
            "reverse" => { arr.borrow_mut().reverse(); Ok(Value::Array(arr.clone())) }
            "slice" => {
                let len = arr.borrow().len() as i64;
                let start = args.first().map(|v| { let n = v.to_number() as i64; if n < 0 { (len + n).max(0) as usize } else { n as usize } }).unwrap_or(0);
                let end = args.get(1).map(|v| { let n = v.to_number() as i64; if n < 0 { (len + n).max(0) as usize } else { n as usize } }).unwrap_or(len as usize);
                Ok(Value::Array(Rc::new(RefCell::new(arr.borrow().iter().skip(start).take(end.saturating_sub(start)).cloned().collect()))))
            }
            _ => Ok(Value::Undefined)
        }
    }
    
    fn call_math_method(&mut self, method: &str, args: &[Expr]) -> Result<Value, String> {
        let vals: Vec<f64> = args.iter().filter_map(|a| self.evaluate(a).ok()).map(|v| v.to_number()).collect();
        let a = vals.first().copied().unwrap_or(0.0);
        let b = vals.get(1).copied();
        Ok(Value::Number(match method {
            "abs" => a.abs(), "floor" => a.floor(), "ceil" => a.ceil(), "round" => a.round(),
            "sqrt" => a.sqrt(), "pow" => a.powf(b.unwrap_or(1.0)), "sin" => a.sin(), "cos" => a.cos(),
            "tan" => a.tan(), "log" => a.ln(), "exp" => a.exp(), "random" => rand::random(),
            "max" => vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            "min" => vals.iter().cloned().fold(f64::INFINITY, f64::min),
            _ => f64::NAN
        }))
    }
    
    fn call_json_method(&mut self, method: &str, args: &[Expr]) -> Result<Value, String> {
        match method {
            "stringify" => Ok(Value::String(format!("{:?}", self.evaluate(args.first().ok_or("No arg")?)?).replace("\"", "\\\""))),
            "parse" => {
                let s = self.evaluate(args.first().ok_or("No arg")?)?.to_string_value();
                // Basic JSON parse
                if let Ok(n) = s.parse::<f64>() { return Ok(Value::Number(n)); }
                if s == "null" { return Ok(Value::Null); }
                if s == "true" { return Ok(Value::Boolean(true)); }
                if s == "false" { return Ok(Value::Boolean(false)); }
                Ok(Value::String(s))
            }
            _ => Ok(Value::Undefined)
        }
    }
    
    fn call_object_method(&mut self, method: &str, args: &[Expr]) -> Result<Value, String> {
        match method {
            "keys" => {
                if let Ok(Value::Object(map)) = self.evaluate(args.first().ok_or("No arg")?) {
                    Ok(Value::Array(Rc::new(RefCell::new(map.borrow().keys().map(|k| Value::String(k.clone())).collect()))))
                } else { Ok(Value::Array(Rc::new(RefCell::new(Vec::new())))) }
            }
            "values" => {
                if let Ok(Value::Object(map)) = self.evaluate(args.first().ok_or("No arg")?) {
                    Ok(Value::Array(Rc::new(RefCell::new(map.borrow().values().cloned().collect()))))
                } else { Ok(Value::Array(Rc::new(RefCell::new(Vec::new())))) }
            }
            _ => Ok(Value::Undefined)
        }
    }
    
    fn call_array_static_method(&mut self, method: &str, args: &[Expr]) -> Result<Value, String> {
        match method {
            "isArray" => Ok(Value::Boolean(matches!(self.evaluate(args.first().ok_or("No arg")?)?, Value::Array(_)))),
            _ => Ok(Value::Undefined)
        }
    }
}

impl Default for JsInterpreter {
    fn default() -> Self { Self::new() }
}

// Native functions
fn native_parse_int(args: Vec<Value>) -> Value {
    Value::Number(args.first().map(|v| v.to_string_value()).unwrap_or_default().trim().parse::<i64>().map(|n| n as f64).unwrap_or(f64::NAN))
}
fn native_parse_float(args: Vec<Value>) -> Value {
    Value::Number(args.first().map(|v| v.to_string_value()).unwrap_or_default().trim().parse().unwrap_or(f64::NAN))
}
fn native_is_nan(args: Vec<Value>) -> Value { Value::Boolean(args.first().map(|v| v.to_number()).unwrap_or(f64::NAN).is_nan()) }
fn native_is_finite(args: Vec<Value>) -> Value { Value::Boolean(args.first().map(|v| v.to_number()).unwrap_or(f64::NAN).is_finite()) }
fn native_boolean(args: Vec<Value>) -> Value { Value::Boolean(args.first().cloned().unwrap_or(Value::Undefined).is_truthy()) }
fn native_number(args: Vec<Value>) -> Value { Value::Number(args.first().cloned().unwrap_or(Value::Undefined).to_number()) }
fn native_string_fn(args: Vec<Value>) -> Value { Value::String(args.first().cloned().unwrap_or(Value::Undefined).to_string_value()) }
fn native_set_timeout(_args: Vec<Value>) -> Value { Value::Number(0.0) }
fn native_clear_timeout(_args: Vec<Value>) -> Value { Value::Undefined }
fn native_set_interval(_args: Vec<Value>) -> Value { Value::Number(0.0) }
fn native_alert(args: Vec<Value>) -> Value { println!("[ALERT] {}", args.first().map(|v| v.to_string_value()).unwrap_or_default()); Value::Undefined }
fn native_confirm(_args: Vec<Value>) -> Value { Value::Boolean(true) }
fn native_prompt(_args: Vec<Value>) -> Value { Value::Null }

fn native_promise_constructor(args: Vec<Value>) -> Value {
    let handle = PromiseHandle::new();
    if let Some(Value::Function { params, body, .. }) = args.first() {
        for stmt in body {
            if let Stmt::Expr(Expr::Call { callee, args: call_args }) = stmt {
                if let Expr::Identifier(name) = callee.as_ref() {
                    if name == params.first().map(|s| s.as_str()).unwrap_or("") {
                        if let Some(Expr::Number(n)) = call_args.first() {
                            handle.resolve(Value::Number(*n));
                        }
                    }
                }
            }
        }
    }
    Value::Promise(handle)
}

fn native_fetch(args: Vec<Value>) -> Value {
    let url = args.first().map(|v| v.to_string_value()).unwrap_or_default();
    let handle = PromiseHandle::new();
    
    // Sync fetch using ureq if available, otherwise mock
    #[cfg(feature = "fetch")]
    {
        match ureq::get(&url).call() {
            Ok(resp) => {
                let body = resp.into_string().unwrap_or_default();
                let obj = Rc::new(RefCell::new(HashMap::new()));
                obj.borrow_mut().insert("ok".to_string(), Value::Boolean(true));
                obj.borrow_mut().insert("status".to_string(), Value::Number(200.0));
                obj.borrow_mut().insert("text".to_string(), Value::String(body));
                handle.resolve(Value::Object(obj));
            }
            Err(_) => handle.reject(Value::String("Fetch failed".to_string())),
        }
    }
    #[cfg(not(feature = "fetch"))]
    {
        let _ = url;
        handle.reject(Value::String("fetch not enabled".to_string()));
    }
    
    Value::Promise(handle)
}
