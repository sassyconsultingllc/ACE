// Script Engine - JavaScript-DOM bridge

#[allow(unused_imports)]
use crate::js::{JsInterpreter, Value, Lexer, Token, Parser, Expr, Stmt};
#[allow(unused_imports)]
use crate::dom::{Document, Node, NodeRef};
use crate::engine::Timer;
use std::rc::Rc;
use std::cell::RefCell;
use std::cell::RefCell as CellAlias;

pub struct ScriptEngine {
    interpreter: JsInterpreter,
    document: Option<Rc<RefCell<DocumentBridge>>>,
    dom_changed: bool,
    pending_timers: Vec<Timer>,
    next_timer_id: u32,
}

/// Stored timer information with callback, delay, and repeat flag
#[derive(Clone)]
struct StoredTimer {
    callback: Value,
    delay_ms: u64,
    repeat: bool,
    created_at: std::time::Instant,
}

thread_local! {
    static PENDING_POPUPS: CellAlias<Vec<String>> = const { CellAlias::new(Vec::new()) };
    static CURRENT_DOC: CellAlias<Option<*const Document>> = const { CellAlias::new(None) };
    // Event listeners: element_id -> event_type -> Vec<callback>
    static EVENT_REGISTRY: CellAlias<std::collections::HashMap<String, std::collections::HashMap<String, Vec<Value>>>> = 
        CellAlias::new(std::collections::HashMap::new());
    // Timer storage: timer_id -> StoredTimer (callback + delay + repeat)
    static TIMER_STORAGE: CellAlias<std::collections::HashMap<u32, StoredTimer>> = CellAlias::new(std::collections::HashMap::new());
    // Next timer ID
    static NEXT_TIMER_ID: CellAlias<u32> = const { CellAlias::new(1) };
    // DOM mutation flag
    static DOM_MUTATED: CellAlias<bool> = const { CellAlias::new(false) };
    // Node registry: maps JS object pointer to actual DOM NodeRef
    // Key is the address of the JS Value::Object's Rc<RefCell<HashMap>>
    static NODE_REGISTRY: CellAlias<std::collections::HashMap<usize, NodeRef>> = CellAlias::new(std::collections::HashMap::new());
    // Next node ID for created elements
    static NEXT_NODE_ID: CellAlias<u64> = const { CellAlias::new(1) };
}
struct DocumentBridge {
    document: *const Document,
}

impl ScriptEngine {
    pub fn new() -> Self {
        ScriptEngine {
            interpreter: JsInterpreter::new(),
            document: None,
            dom_changed: false,
            pending_timers: Vec::new(),
            next_timer_id: 1,
        }
    }
    
    /// Validate JavaScript syntax without executing
    /// Returns Ok(token_count) on success, Err(error_message) on failure
    pub fn validate_syntax(source: &str) -> Result<usize, String> {
        // Tokenize to count tokens
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let token_count = tokens.len();
        
        // Parse to check for syntax errors
        let mut parser = Parser::new(source);
        let _statements: Vec<Stmt> = parser.parse()
            .map_err(|e| format!("Parse error: {}", e))?;
        
        Ok(token_count)
    }
    
    /// Parse JavaScript and return AST for inspection
    pub fn parse_to_ast(source: &str) -> Result<Vec<Stmt>, String> {
        let mut parser = Parser::new(source);
        parser.parse().map_err(|e| format!("Parse error: {}", e))
    }
    
    /// Get expression info for debugging
    pub fn describe_expression(expr: &Expr) -> String {
        match expr {
            Expr::Number(n) => format!("Number({})", n),
            Expr::String(s) => format!("String({:?})", s),
            Expr::Boolean(b) => format!("Boolean({})", b),
            Expr::Identifier(name) => format!("Identifier({})", name),
            Expr::Binary { op, .. } => format!("Binary({:?})", op),
            Expr::Unary { op, .. } => format!("Unary({:?})", op),
            Expr::Call { .. } => "FunctionCall".to_string(),
            Expr::Member { .. } => "MemberAccess".to_string(),
            Expr::Array(_) => "ArrayLiteral".to_string(),
            Expr::Object(_) => "ObjectLiteral".to_string(),
            Expr::Function { .. } => "FunctionExpression".to_string(),
            Expr::Arrow { .. } => "ArrowFunction".to_string(),
            Expr::Assignment { .. } => "Assignment".to_string(),
            Expr::Ternary { .. } => "Ternary".to_string(),
            Expr::Index { .. } => "IndexAccess".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    pub fn set_document(&mut self, doc: &Document) {
        self.document = Some(Rc::new(RefCell::new(DocumentBridge {
            document: doc as *const Document,
        })));
        self.inject_dom_api();
    }

    fn inject_dom_api(&mut self) {
        // Document object with methods
        let doc_obj = Value::Object(Rc::new(RefCell::new(std::collections::HashMap::new())));
        if let Value::Object(ref obj) = doc_obj {
            let mut o = obj.borrow_mut();
            o.insert("title".to_string(), Value::String("".to_string()));
            o.insert("URL".to_string(), Value::String("".to_string()));
            o.insert("readyState".to_string(), Value::String("complete".to_string()));
            o.insert("getElementById".to_string(), Value::NativeFunction(native_get_element_by_id));
            o.insert("getElementsByTagName".to_string(), Value::NativeFunction(native_get_elements_by_tag));
            o.insert("getElementsByClassName".to_string(), Value::NativeFunction(native_get_elements_by_class));
            o.insert("querySelector".to_string(), Value::NativeFunction(native_query_selector));
            o.insert("querySelectorAll".to_string(), Value::NativeFunction(native_query_selector_all));
            o.insert("createElement".to_string(), Value::NativeFunction(native_create_element));
            o.insert("createTextNode".to_string(), Value::NativeFunction(native_create_text_node));
            o.insert("body".to_string(), Value::Null);
            o.insert("head".to_string(), Value::Null);
            o.insert("documentElement".to_string(), Value::Null);
        }
        self.interpreter.define("document".to_string(), doc_obj);

        // Window object
        let window_obj = Value::Object(Rc::new(RefCell::new(std::collections::HashMap::new())));
        if let Value::Object(ref obj) = window_obj {
            let mut o = obj.borrow_mut();
            o.insert("innerWidth".to_string(), Value::Number(1024.0));
            o.insert("innerHeight".to_string(), Value::Number(768.0));
            o.insert("location".to_string(), Value::Object(Rc::new(RefCell::new({
                let mut loc = std::collections::HashMap::new();
                loc.insert("href".to_string(), Value::String("".to_string()));
                loc.insert("hostname".to_string(), Value::String("".to_string()));
                loc.insert("pathname".to_string(), Value::String("".to_string()));
                loc.insert("protocol".to_string(), Value::String("https:".to_string()));
                loc
            }))));
            o.insert("navigator".to_string(), Value::Object(Rc::new(RefCell::new({
                let mut nav = std::collections::HashMap::new();
                nav.insert("userAgent".to_string(), Value::String("SassyBrowser/0.3.0".to_string()));
                nav.insert("language".to_string(), Value::String("en-US".to_string()));
                nav.insert("platform".to_string(), Value::String(std::env::consts::OS.to_string()));
                nav
            }))));
            o.insert("setTimeout".to_string(), Value::NativeFunction(native_set_timeout));
            o.insert("setInterval".to_string(), Value::NativeFunction(native_set_interval));
            o.insert("clearTimeout".to_string(), Value::NativeFunction(native_clear_timeout));
            o.insert("clearInterval".to_string(), Value::NativeFunction(native_clear_timeout));
            o.insert("alert".to_string(), Value::NativeFunction(native_alert));
            o.insert("confirm".to_string(), Value::NativeFunction(native_confirm));
            o.insert("prompt".to_string(), Value::NativeFunction(native_prompt));
            o.insert("addEventListener".to_string(), Value::NativeFunction(native_add_event_listener));
            o.insert("removeEventListener".to_string(), Value::NativeFunction(native_remove_event_listener));
            o.insert("open".to_string(), Value::NativeFunction(native_window_open));
        }
        self.interpreter.define("window".to_string(), window_obj);
        self.interpreter.define("open".to_string(), Value::NativeFunction(native_window_open));
        
        // Global timer functions
        self.interpreter.define("setTimeout".to_string(), Value::NativeFunction(native_set_timeout));
        self.interpreter.define("setInterval".to_string(), Value::NativeFunction(native_set_interval));
        self.interpreter.define("clearTimeout".to_string(), Value::NativeFunction(native_clear_timeout));
        self.interpreter.define("clearInterval".to_string(), Value::NativeFunction(native_clear_timeout));
        self.interpreter.define("alert".to_string(), Value::NativeFunction(native_alert));
        self.interpreter.define("confirm".to_string(), Value::NativeFunction(native_confirm));
        self.interpreter.define("prompt".to_string(), Value::NativeFunction(native_prompt));
        
        // Storage APIs - essential for web apps
        let local_storage = Rc::new(RefCell::new(std::collections::HashMap::new()));
        local_storage.borrow_mut().insert("getItem".to_string(), Value::NativeFunction(native_storage_get));
        local_storage.borrow_mut().insert("setItem".to_string(), Value::NativeFunction(native_storage_set));
        local_storage.borrow_mut().insert("removeItem".to_string(), Value::NativeFunction(native_storage_remove));
        local_storage.borrow_mut().insert("clear".to_string(), Value::NativeFunction(native_storage_clear));
        local_storage.borrow_mut().insert("key".to_string(), Value::NativeFunction(native_storage_key));
        local_storage.borrow_mut().insert("length".to_string(), Value::Number(0.0));
        self.interpreter.define("localStorage".to_string(), Value::Object(local_storage.clone()));
        self.interpreter.define("sessionStorage".to_string(), Value::Object(local_storage));
    }

    pub fn execute(&mut self, script: &str) -> Result<Value, String> {
        self.dom_changed = false;
        
        self.interpreter.execute(script)
    }

    pub fn execute_with_dom(&mut self, script: &str, doc: &Document) -> Result<Value, String> {
        self.set_document(doc);
        CURRENT_DOC.with(|d| *d.borrow_mut() = Some(doc as *const Document));
        let result = self.execute(script);
        CURRENT_DOC.with(|d| *d.borrow_mut() = None);
        result
    }

    pub fn get_console_output(&self) -> Vec<String> {
        self.interpreter.get_console_output().clone()
    }

    /// Drain any popup requests produced by scripts (window.open)
    pub fn take_popup_requests(&self) -> Vec<String> {
        PENDING_POPUPS.with(|p| std::mem::take(&mut *p.borrow_mut()))
    }

    pub fn has_dom_changes(&self) -> bool {
        self.dom_changed
    }

    pub fn take_timers(&mut self) -> Vec<Timer> {
        std::mem::take(&mut self.pending_timers)
    }

    pub fn add_timer(&mut self, callback: String, delay_ms: u64, repeat: bool) -> u32 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;
        self.pending_timers.push(Timer {
            id,
            callback,
            delay_ms,
            created_at: std::time::Instant::now(),
            repeat,
        });
        id
    }

    /// Get a timer's callback by ID (for execution)
    pub fn get_timer_callback(&self, id: u32) -> Option<Value> {
        TIMER_STORAGE.with(|timers| {
            timers.borrow().get(&id).map(|t| t.callback.clone())
        })
    }

    /// Remove a timer by ID (after setTimeout fires, or clearTimeout)
    pub fn remove_timer(&self, id: u32) {
        TIMER_STORAGE.with(|timers| {
            timers.borrow_mut().remove(&id);
        });
    }

    /// Get all ready timers (where delay has elapsed)
    pub fn get_ready_timers(&self) -> Vec<(u32, Value, bool)> {
        let mut ready = Vec::new();
        TIMER_STORAGE.with(|timers| {
            let storage = timers.borrow();
            for (id, timer) in storage.iter() {
                let elapsed = timer.created_at.elapsed().as_millis() as u64;
                if elapsed >= timer.delay_ms {
                    ready.push((*id, timer.callback.clone(), timer.repeat));
                }
            }
        });
        ready
    }

    /// Reset a repeating timer's created_at for next interval
    pub fn reset_timer(&self, id: u32) {
        TIMER_STORAGE.with(|timers| {
            if let Some(timer) = timers.borrow_mut().get_mut(&id) {
                timer.created_at = std::time::Instant::now();
            }
        });
    }

    /// Execute a callback function value
    pub fn execute_callback(&mut self, callback: Value, args: Vec<Value>) -> Result<Value, String> {
        self.interpreter.call_value(&callback, &args)
    }

    /// Fire an event on the window
    pub fn fire_window_event(&mut self, event_type: &str, event: Value) -> Vec<Result<Value, String>> {
        let callbacks: Vec<Value> = EVENT_REGISTRY.with(|reg| {
            let registry = reg.borrow();
            if let Some(window_listeners) = registry.get("__window__") {
                if let Some(type_listeners) = window_listeners.get(event_type) {
                    return type_listeners.clone();
                }
            }
            Vec::new()
        });
        
        let mut results = Vec::new();
        for callback in callbacks {
            results.push(self.interpreter.call_value(&callback, std::slice::from_ref(&event)));
        }
        results
    }

    /// Check if DOM was mutated by scripts
    pub fn check_dom_mutation(&self) -> bool {
        DOM_MUTATED.with(|d| {
            let result = *d.borrow();
            *d.borrow_mut() = false;
            result
        })
    }
}

impl Default for ScriptEngine {
    fn default() -> Self { Self::new() }
}

// Native DOM functions
fn native_get_element_by_id(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Null; }
    let id = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Value::Null,
    };
    with_document(|doc| doc.get_element_by_id(&id))
        .and_then(|node| node.map(|n| node_to_value(&n)))
        .unwrap_or(Value::Null)
}

fn native_get_elements_by_tag(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Array(Rc::new(RefCell::new(Vec::new()))); }
    let tag = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Value::Array(Rc::new(RefCell::new(Vec::new()))),
    };
    with_document(|doc| doc.get_elements_by_tag(&tag))
        .map(nodes_to_array)
        .unwrap_or_else(|| Value::Array(Rc::new(RefCell::new(Vec::new()))))
}

fn native_get_elements_by_class(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Array(Rc::new(RefCell::new(Vec::new()))); }
    let class = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Value::Array(Rc::new(RefCell::new(Vec::new()))),
    };
    with_document(|doc| doc.get_elements_by_class(&class))
        .map(nodes_to_array)
        .unwrap_or_else(|| Value::Array(Rc::new(RefCell::new(Vec::new()))))
}

fn native_query_selector(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Null; }
    let sel = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Value::Null,
    };
    with_document(|doc| doc.query_selector(&sel))
        .and_then(|node| node.map(|n| node_to_value(&n)))
        .unwrap_or(Value::Null)
}

fn native_query_selector_all(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Array(Rc::new(RefCell::new(Vec::new()))); }
    let sel = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Value::Array(Rc::new(RefCell::new(Vec::new()))),
    };
    with_document(|doc| doc.query_selector_all(&sel))
        .map(nodes_to_array)
        .unwrap_or_else(|| Value::Array(Rc::new(RefCell::new(Vec::new()))))
}

fn native_create_element(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Null; }
    let tag = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Value::Null,
    };
    
    // Create a real DOM node
    let node_ref = Node::new_element(&tag);
    
    // Create JS wrapper and register it
    let obj = node_to_value_from_parts(&tag, "", "");
    if let Value::Object(ref o) = obj {
        let node_id = Rc::as_ptr(o) as usize;
        o.borrow_mut().insert("__nodeId__".to_string(), Value::Number(node_id as f64));
        NODE_REGISTRY.with(|reg| {
            reg.borrow_mut().insert(node_id, node_ref);
        });
    }
    
    obj
}

fn native_create_text_node(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Null; }
    let text = match &args[0] {
        Value::String(s) => s.clone(),
        v => v.to_string_value(),
    };
    let obj = Value::Object(Rc::new(RefCell::new(std::collections::HashMap::new())));
    if let Value::Object(ref o) = obj {
        let mut m = o.borrow_mut();
        m.insert("nodeType".to_string(), Value::Number(3.0));
        m.insert("textContent".to_string(), Value::String(text));
    }
    obj
}

fn create_element_wrapper() -> Value {
    node_to_value_from_parts("DIV", "", "")
}

fn create_class_list() -> Value {
    let obj = Value::Object(Rc::new(RefCell::new(std::collections::HashMap::new())));
    if let Value::Object(ref o) = obj {
        let mut m = o.borrow_mut();
        // Store the actual class list as an array
        m.insert("_classes".to_string(), Value::Array(Rc::new(RefCell::new(Vec::new()))));
        m.insert("add".to_string(), Value::NativeFunction(classlist_add));
        m.insert("remove".to_string(), Value::NativeFunction(classlist_remove));
        m.insert("toggle".to_string(), Value::NativeFunction(classlist_toggle));
        m.insert("contains".to_string(), Value::NativeFunction(classlist_contains));
        m.insert("item".to_string(), Value::NativeFunction(classlist_item));
        m.insert("length".to_string(), Value::Number(0.0));
    }
    obj
}

fn node_to_value(node: &NodeRef) -> Value {
    let n = node.borrow();
    let obj = node_to_value_from_parts(
        n.tag_name.as_deref().unwrap_or("div"),
        &n.get_inner_text(),
        &n.get_inner_html(),
    );
    
    // Register this node so we can find it later for DOM mutations
    if let Value::Object(ref o) = obj {
        let node_id = Rc::as_ptr(o) as usize;
        // Store a unique ID in the object for lookup
        o.borrow_mut().insert("__nodeId__".to_string(), Value::Number(node_id as f64));
        NODE_REGISTRY.with(|reg| {
            reg.borrow_mut().insert(node_id, Rc::clone(node));
        });
    }
    
    obj
}

fn node_to_value_from_parts(tag: &str, text: &str, html: &str) -> Value {
    let obj = Value::Object(Rc::new(RefCell::new(std::collections::HashMap::new())));
    if let Value::Object(ref o) = obj {
        let mut m = o.borrow_mut();
        // Node properties
        m.insert("nodeType".to_string(), Value::Number(1.0));
        m.insert("tagName".to_string(), Value::String(tag.to_uppercase()));
        m.insert("id".to_string(), Value::String("".to_string()));
        m.insert("className".to_string(), Value::String("".to_string()));
        m.insert("innerHTML".to_string(), Value::String(html.to_string()));
        m.insert("innerText".to_string(), Value::String(text.to_string()));
        m.insert("textContent".to_string(), Value::String(text.to_string()));
        m.insert("style".to_string(), Value::Object(Rc::new(RefCell::new(std::collections::HashMap::new()))));
        m.insert("classList".to_string(), create_class_list());
        m.insert("children".to_string(), Value::Array(Rc::new(RefCell::new(Vec::new()))));
        m.insert("childNodes".to_string(), Value::Array(Rc::new(RefCell::new(Vec::new()))));
        m.insert("parentNode".to_string(), Value::Null);
        m.insert("firstChild".to_string(), Value::Null);
        m.insert("lastChild".to_string(), Value::Null);
        m.insert("nextSibling".to_string(), Value::Null);
        m.insert("previousSibling".to_string(), Value::Null);
        
        // Element methods
        m.insert("getAttribute".to_string(), Value::NativeFunction(element_get_attribute));
        m.insert("setAttribute".to_string(), Value::NativeFunction(element_set_attribute));
        m.insert("removeAttribute".to_string(), Value::NativeFunction(element_remove_attribute));
        m.insert("hasAttribute".to_string(), Value::NativeFunction(element_has_attribute));
        m.insert("appendChild".to_string(), Value::NativeFunction(element_append_child));
        m.insert("removeChild".to_string(), Value::NativeFunction(element_remove_child));
        m.insert("insertBefore".to_string(), Value::NativeFunction(element_insert_before));
        m.insert("replaceChild".to_string(), Value::NativeFunction(element_replace_child));
        m.insert("cloneNode".to_string(), Value::NativeFunction(element_clone_node));
        m.insert("contains".to_string(), Value::NativeFunction(element_contains));
        m.insert("addEventListener".to_string(), Value::NativeFunction(element_add_event_listener));
        m.insert("removeEventListener".to_string(), Value::NativeFunction(element_remove_event_listener));
        m.insert("dispatchEvent".to_string(), Value::NativeFunction(element_dispatch_event));
        m.insert("focus".to_string(), Value::NativeFunction(element_focus));
        m.insert("blur".to_string(), Value::NativeFunction(element_blur));
        m.insert("click".to_string(), Value::NativeFunction(element_click));
        m.insert("getBoundingClientRect".to_string(), Value::NativeFunction(element_get_bounding_rect));
    }
    obj
}

fn nodes_to_array(nodes: Vec<NodeRef>) -> Value {
    let mut arr = Vec::new();
    for n in nodes {
        arr.push(node_to_value(&n));
    }
    Value::Array(Rc::new(RefCell::new(arr)))
}

/// Get the real DOM NodeRef from a JavaScript Value (if registered)
fn get_node_from_value(val: &Value) -> Option<NodeRef> {
    match val {
        Value::Object(obj) => {
            let borrowed = obj.borrow();
            if let Some(Value::Number(id)) = borrowed.get("__nodeId__") {
                let node_id = *id as usize;
                return NODE_REGISTRY.with(|reg| {
                    reg.borrow().get(&node_id).cloned()
                });
            }
            None
        }
        _ => None
    }
}

fn with_document<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&Document) -> R,
{
    CURRENT_DOC.with(|cell| {
        if let Some(ptr) = *cell.borrow() {
            // Safety: pointer set immediately before execution and cleared after
            let doc: &Document = unsafe { &*ptr };
            Some(f(doc))
        } else {
            None
        }
    })
}

// Element methods
fn element_get_attribute(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Null; }
    // Get attribute name from args
    let attr_name = args[0].to_string_value();
    // In a full implementation, we'd look up the element's attribute
    // For now, return null (attribute not found)
    let _name = attr_name; // Mark as used
    Value::Null
}

fn element_set_attribute(args: Vec<Value>) -> Value {
    // setAttribute(name, value)
    if args.len() < 2 { return Value::Undefined; }
    let _attr_name = args[0].to_string_value();
    let _attr_value = args[1].to_string_value();
    // Signal DOM mutation for re-layout
    DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    Value::Undefined
}

fn element_remove_attribute(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Undefined; }
    let _attr_name = args[0].to_string_value();
    // Signal DOM mutation for re-layout
    DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    Value::Undefined
}

fn element_has_attribute(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Boolean(false); }
    let _attr_name = args[0].to_string_value();
    // Without element reference, we can't check - return false
    Value::Boolean(false)
}

fn element_append_child(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Null; }
    // Signal DOM mutation
    DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    // Return the child (standard behavior)
    args[0].clone()
}

fn element_remove_child(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Null; }
    DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    args[0].clone()
}

fn element_insert_before(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Null; }
    DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    args[0].clone()
}

fn element_replace_child(args: Vec<Value>) -> Value {
    if args.len() < 2 { return Value::Null; }
    DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    args[1].clone()
}

fn element_clone_node(args: Vec<Value>) -> Value {
    // cloneNode(deep) - returns a copy of the node
    let deep = args.first()
        .map(|v| v.is_truthy())
        .unwrap_or(false);
    
    // Create a new element wrapper (shallow clone)
    // In full impl, if deep=true, we'd also clone children
    let _is_deep = deep;
    create_element_wrapper()
}

fn element_contains(args: Vec<Value>) -> Value {
    // Check if this element contains the given node
    if args.is_empty() { return Value::Boolean(false); }
    let _other_node = &args[0];
    // Without DOM tree access, we can't check containment
    // Return false as default
    Value::Boolean(false)
}
fn element_add_event_listener(args: Vec<Value>) -> Value {
    // element.addEventListener(type, callback)
    // Args: [this_element, event_type, callback]
    if args.len() < 2 { return Value::Undefined; }
    let event_type = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Value::Undefined,
    };
    let callback = if args.len() > 1 { args[1].clone() } else { return Value::Undefined; };
    
    // Use a generic element ID since we don't have element identity
    let element_id = format!("elem_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0));
    
    EVENT_REGISTRY.with(|reg| {
        let mut registry = reg.borrow_mut();
        let elem_listeners = registry.entry(element_id).or_insert_with(std::collections::HashMap::new);
        let type_listeners = elem_listeners.entry(event_type).or_insert_with(Vec::new);
        type_listeners.push(callback);
    });
    Value::Undefined
}

fn element_remove_event_listener(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Undefined; }
    // Simplified: just return undefined, proper implementation needs element identity tracking
    Value::Undefined
}

fn element_dispatch_event(args: Vec<Value>) -> Value {
    // element.dispatchEvent(event) -> returns true if event was not cancelled
    if args.is_empty() { return Value::Boolean(true); }
    // In a full implementation, we'd call the registered callbacks
    // For now, just signal success
    Value::Boolean(true)
}
fn element_focus(args: Vec<Value>) -> Value {
    // Focus the element - in a full impl, this would update the focused element
    // and potentially scroll into view
    let _element = args.first(); // The element being focused
    // Signal that UI state changed (for cursor positioning, etc.)
    DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    Value::Undefined
}

fn element_blur(args: Vec<Value>) -> Value {
    // Blur (unfocus) the element
    let _element = args.first();
    DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    Value::Undefined
}

fn element_click(args: Vec<Value>) -> Value {
    // Programmatic click - should dispatch click event
    let _element = args.first();
    // In a full implementation, this would:
    // 1. Dispatch mousedown event
    // 2. Dispatch mouseup event  
    // 3. Dispatch click event
    // 4. If it's a link, navigate
    // 5. If it's a button in a form, submit
    DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    Value::Undefined
}

fn element_get_bounding_rect(_args: Vec<Value>) -> Value {
    let obj = Value::Object(Rc::new(RefCell::new(std::collections::HashMap::new())));
    if let Value::Object(ref o) = obj {
        let mut m = o.borrow_mut();
        m.insert("x".to_string(), Value::Number(0.0));
        m.insert("y".to_string(), Value::Number(0.0));
        m.insert("width".to_string(), Value::Number(0.0));
        m.insert("height".to_string(), Value::Number(0.0));
        m.insert("top".to_string(), Value::Number(0.0));
        m.insert("right".to_string(), Value::Number(0.0));
        m.insert("bottom".to_string(), Value::Number(0.0));
        m.insert("left".to_string(), Value::Number(0.0));
    }
    obj
}

// ClassList methods
fn classlist_add(args: Vec<Value>) -> Value {
    // In a real impl, we'd modify the element's classList
    // Since we use native functions without this context, signal DOM changed
    if !args.is_empty() {
        DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    }
    Value::Undefined
}

fn classlist_remove(args: Vec<Value>) -> Value {
    if !args.is_empty() {
        DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    }
    Value::Undefined
}

fn classlist_toggle(args: Vec<Value>) -> Value {
    if !args.is_empty() {
        DOM_MUTATED.with(|d| *d.borrow_mut() = true);
    }
    // Return true if the class is now present (toggled on)
    Value::Boolean(true)
}

fn classlist_contains(args: Vec<Value>) -> Value {
    // Check if class exists in the element's classList
    // Since we don't have element context in static functions,
    // we check if the argument is a valid class name string
    if args.is_empty() {
        return Value::Boolean(false);
    }
    // Return false since we can't actually check without element reference
    // In a full implementation, this would check the element's class attribute
    let _class_name = args[0].to_string_value();
    Value::Boolean(false)
}

fn classlist_item(args: Vec<Value>) -> Value {
    // Return class at given index
    if args.is_empty() {
        return Value::Null;
    }
    let _index = args[0].to_number() as usize;
    // Return null since we don't have element context
    // In a full implementation, this would return the class at the index
    Value::Null
}

// Window functions
fn native_set_timeout(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Number(0.0); }
    let callback = args[0].clone();
    let delay_ms = args.get(1).map(|v| v.to_number() as u64).unwrap_or(0);
    
    let id = NEXT_TIMER_ID.with(|id| {
        let current = *id.borrow();
        *id.borrow_mut() = current + 1;
        current
    });
    
    TIMER_STORAGE.with(|timers| {
        timers.borrow_mut().insert(id, StoredTimer {
            callback,
            delay_ms,
            repeat: false,
            created_at: std::time::Instant::now(),
        });
    });
    
    Value::Number(id as f64)
}

fn native_set_interval(args: Vec<Value>) -> Value {
    if args.is_empty() { return Value::Number(0.0); }
    let callback = args[0].clone();
    let delay_ms = args.get(1).map(|v| v.to_number() as u64).unwrap_or(0);
    
    let id = NEXT_TIMER_ID.with(|id| {
        let current = *id.borrow();
        *id.borrow_mut() = current + 1;
        current
    });
    
    TIMER_STORAGE.with(|timers| {
        timers.borrow_mut().insert(id, StoredTimer {
            callback,
            delay_ms,
            repeat: true,
            created_at: std::time::Instant::now(),
        });
    });
    
    Value::Number(id as f64)
}

fn native_window_open(args: Vec<Value>) -> Value {
    if let Some(Value::String(url)) = args.first() {
        // Enqueue popup request; BrowserState will evaluate via PopupManager
        PENDING_POPUPS.with(|p| p.borrow_mut().push(url.clone()));
    }
    Value::Null
}

fn native_clear_timeout(args: Vec<Value>) -> Value {
    if let Some(id) = args.first() {
        let timer_id = id.to_number() as u32;
        TIMER_STORAGE.with(|timers| {
            timers.borrow_mut().remove(&timer_id);
        });
    }
    Value::Undefined
}

fn native_alert(args: Vec<Value>) -> Value {
    if !args.is_empty() {
        println!("[Alert] {}", args[0].to_string_value());
    }
    Value::Undefined
}

fn native_confirm(args: Vec<Value>) -> Value {
    if !args.is_empty() {
        println!("[Confirm] {}", args[0].to_string_value());
    }
    Value::Boolean(true)
}

fn native_prompt(args: Vec<Value>) -> Value {
    if !args.is_empty() {
        println!("[Prompt] {}", args[0].to_string_value());
    }
    Value::String("".to_string())
}

fn native_add_event_listener(args: Vec<Value>) -> Value {
    // window.addEventListener(type, callback)
    if args.len() < 2 { return Value::Undefined; }
    let event_type = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Value::Undefined,
    };
    let callback = args[1].clone();
    EVENT_REGISTRY.with(|reg| {
        let mut registry = reg.borrow_mut();
        let window_listeners = registry.entry("__window__".to_string()).or_insert_with(std::collections::HashMap::new);
        let type_listeners = window_listeners.entry(event_type).or_insert_with(Vec::new);
        type_listeners.push(callback);
    });
    Value::Undefined
}

fn native_remove_event_listener(args: Vec<Value>) -> Value {
    // window.removeEventListener(type, callback) - basic implementation
    if args.is_empty() { return Value::Undefined; }
    let event_type = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Value::Undefined,
    };
    EVENT_REGISTRY.with(|reg| {
        let mut registry = reg.borrow_mut();
        if let Some(window_listeners) = registry.get_mut("__window__") {
            window_listeners.remove(&event_type);
        }
    });
    Value::Undefined
}

// Storage API implementations using a simple in-memory + file backed store
thread_local! {
    static STORAGE: CellAlias<std::collections::HashMap<String, String>> = CellAlias::new(std::collections::HashMap::new());
}

fn native_storage_get(args: Vec<Value>) -> Value {
    let key = args.first().map(|v| v.to_string_value()).unwrap_or_default();
    STORAGE.with(|s| {
        s.borrow().get(&key).map(|v| Value::String(v.clone())).unwrap_or(Value::Null)
    })
}

fn native_storage_set(args: Vec<Value>) -> Value {
    let key = args.first().map(|v| v.to_string_value()).unwrap_or_default();
    let value = args.get(1).map(|v| v.to_string_value()).unwrap_or_default();
    STORAGE.with(|s| {
        s.borrow_mut().insert(key, value);
    });
    Value::Undefined
}

fn native_storage_remove(args: Vec<Value>) -> Value {
    let key = args.first().map(|v| v.to_string_value()).unwrap_or_default();
    STORAGE.with(|s| {
        s.borrow_mut().remove(&key);
    });
    Value::Undefined
}

fn native_storage_clear(_args: Vec<Value>) -> Value {
    STORAGE.with(|s| {
        s.borrow_mut().clear();
    });
    Value::Undefined
}

fn native_storage_key(args: Vec<Value>) -> Value {
    let index = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    STORAGE.with(|s| {
        let storage = s.borrow();
        storage.keys().nth(index).map(|k| Value::String(k.clone())).unwrap_or(Value::Null)
    })
}
