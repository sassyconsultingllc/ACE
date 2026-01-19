//! DOM Bridge - Connects JS interpreter to HTML rendering
#![allow(dead_code, unused_variables, unused_imports)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use super::value::{Value, DomElement};

/// Bridge between JavaScript and the DOM
pub struct DomBridge {
    elements: Rc<RefCell<HashMap<u64, Rc<RefCell<DomElement>>>>>,
    next_id: Rc<RefCell<u64>>,
    document_title: Rc<RefCell<String>>,
    document_body: Rc<RefCell<Option<Rc<RefCell<DomElement>>>>>,
    event_listeners: Rc<RefCell<HashMap<(u64, String), Vec<Value>>>>,
}

impl DomBridge {
    pub fn new() -> Self {
        DomBridge {
            elements: Rc::new(RefCell::new(HashMap::new())),
            next_id: Rc::new(RefCell::new(1)),
            document_title: Rc::new(RefCell::new("Sassy Browser".to_string())),
            document_body: Rc::new(RefCell::new(None)),
            event_listeners: Rc::new(RefCell::new(HashMap::new())),
        }
    }
    
    pub fn create_element(&self, tag: &str) -> Value {
        let id = {
            let mut next = self.next_id.borrow_mut();
            let id = *next;
            *next += 1;
            id
        };
        
        let element = Rc::new(RefCell::new(DomElement::new(id, tag)));
        self.elements.borrow_mut().insert(id, element.clone());
        Value::DomElement(element)
    }
    
    pub fn get_element_by_id(&self, id: &str) -> Value {
        for element in self.elements.borrow().values() {
            if element.borrow().attributes.get("id") == Some(&id.to_string()) {
                return Value::DomElement(element.clone());
            }
        }
        Value::Null
    }
    
    pub fn query_selector(&self, selector: &str) -> Value {
        // Simple selector matching
        for element in self.elements.borrow().values() {
            let el = element.borrow();
            
            // ID selector
            if selector.starts_with('#') {
                if el.attributes.get("id") == Some(&selector[1..].to_string()) {
                    return Value::DomElement(element.clone());
                }
            }
            // Class selector
            else if selector.starts_with('.') {
                if let Some(classes) = el.attributes.get("class") {
                    if classes.split_whitespace().any(|c| c == &selector[1..]) {
                        return Value::DomElement(element.clone());
                    }
                }
            }
            // Tag selector
            else if el.tag.eq_ignore_ascii_case(selector) {
                return Value::DomElement(element.clone());
            }
        }
        Value::Null
    }
    
    pub fn query_selector_all(&self, selector: &str) -> Value {
        let mut results = Vec::new();
        
        for element in self.elements.borrow().values() {
            let el = element.borrow();
            let matches = if selector.starts_with('#') {
                el.attributes.get("id") == Some(&selector[1..].to_string())
            } else if selector.starts_with('.') {
                el.attributes.get("class")
                    .map(|c| c.split_whitespace().any(|cls| cls == &selector[1..]))
                    .unwrap_or(false)
            } else {
                el.tag.eq_ignore_ascii_case(selector)
            };
            
            if matches {
                results.push(Value::DomElement(element.clone()));
            }
        }
        
        Value::Array(Rc::new(RefCell::new(results)))
    }
    
    pub fn create_document_object(&self) -> Value {
        let doc = Rc::new(RefCell::new(HashMap::new()));
        
        // document.title
        doc.borrow_mut().insert("title".to_string(), 
            Value::String(self.document_title.borrow().clone()));
        
        // Store reference to bridge for native functions
        doc.borrow_mut().insert("_bridge".to_string(), Value::Null);
        
        Value::Object(doc)
    }
    
    pub fn create_window_object(&self) -> Value {
        let win = Rc::new(RefCell::new(HashMap::new()));
        
        // window.location
        let location = Rc::new(RefCell::new(HashMap::new()));
        location.borrow_mut().insert("href".to_string(), Value::String("about:blank".to_string()));
        location.borrow_mut().insert("protocol".to_string(), Value::String("about:".to_string()));
        location.borrow_mut().insert("host".to_string(), Value::String("".to_string()));
        location.borrow_mut().insert("pathname".to_string(), Value::String("blank".to_string()));
        win.borrow_mut().insert("location".to_string(), Value::Object(location));
        
        // window.innerWidth, window.innerHeight
        win.borrow_mut().insert("innerWidth".to_string(), Value::Number(1920.0));
        win.borrow_mut().insert("innerHeight".to_string(), Value::Number(1080.0));
        
        // window.navigator
        let navigator = Rc::new(RefCell::new(HashMap::new()));
        navigator.borrow_mut().insert("userAgent".to_string(), 
            Value::String("SassyBrowser/1.0 (Rust; egui)".to_string()));
        navigator.borrow_mut().insert("language".to_string(), Value::String("en-US".to_string()));
        navigator.borrow_mut().insert("platform".to_string(), Value::String(std::env::consts::OS.to_string()));
        win.borrow_mut().insert("navigator".to_string(), Value::Object(navigator));
        
        Value::Object(win)
    }
    
    pub fn set_attribute(&self, element: &Rc<RefCell<DomElement>>, name: &str, value: &str) {
        element.borrow_mut().attributes.insert(name.to_string(), value.to_string());
    }
    
    pub fn get_attribute(&self, element: &Rc<RefCell<DomElement>>, name: &str) -> Option<String> {
        element.borrow().attributes.get(name).cloned()
    }
    
    pub fn set_text_content(&self, element: &Rc<RefCell<DomElement>>, text: &str) {
        element.borrow_mut().text_content = text.to_string();
    }
    
    pub fn append_child(&self, parent: &Rc<RefCell<DomElement>>, child: DomElement) {
        parent.borrow_mut().children.push(child);
    }
    
    pub fn add_event_listener(&self, element_id: u64, event: &str, callback: Value) {
        let key = (element_id, event.to_string());
        self.event_listeners.borrow_mut()
            .entry(key)
            .or_insert_with(Vec::new)
            .push(callback);
    }
    
    pub fn dispatch_event(&self, element_id: u64, event: &str) -> Vec<Value> {
        let key = (element_id, event.to_string());
        self.event_listeners.borrow()
            .get(&key)
            .cloned()
            .unwrap_or_default()
    }
    
    pub fn set_title(&self, title: &str) {
        *self.document_title.borrow_mut() = title.to_string();
    }
    
    pub fn get_title(&self) -> String {
        self.document_title.borrow().clone()
    }
}

impl Default for DomBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DomBridge {
    fn clone(&self) -> Self {
        DomBridge {
            elements: self.elements.clone(),
            next_id: self.next_id.clone(),
            document_title: self.document_title.clone(),
            document_body: self.document_body.clone(),
            event_listeners: self.event_listeners.clone(),
        }
    }
}
