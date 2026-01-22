// DOM - Document Object Model representation
#![allow(dead_code)]

use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::cell::RefCell;

pub type NodeRef = Rc<RefCell<Node>>;
pub type WeakNodeRef = Weak<RefCell<Node>>;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Document,
    Element,
    Text,
    Comment,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub node_type: NodeType,
    pub tag_name: Option<String>,
    pub text_content: Option<String>,
    pub attributes: HashMap<String, String>,
    pub children: Vec<NodeRef>,
    pub parent: Option<WeakNodeRef>,
    pub styles: HashMap<String, String>,
    pub event_listeners: HashMap<String, Vec<String>>,
}

impl Node {
    pub fn new_document() -> NodeRef {
        Rc::new(RefCell::new(Node {
            node_type: NodeType::Document,
            tag_name: None,
            text_content: None,
            attributes: HashMap::new(),
            children: Vec::new(),
            parent: None,
            styles: HashMap::new(),
            event_listeners: HashMap::new(),
        }))
    }

    pub fn new_element(tag: &str) -> NodeRef {
        Rc::new(RefCell::new(Node {
            node_type: NodeType::Element,
            tag_name: Some(crate::fontcase::ascii_lower(tag)),
            text_content: None,
            attributes: HashMap::new(),
            children: Vec::new(),
            parent: None,
            styles: HashMap::new(),
            event_listeners: HashMap::new(),
        }))
    }

    pub fn new_text(text: &str) -> NodeRef {
        Rc::new(RefCell::new(Node {
            node_type: NodeType::Text,
            tag_name: None,
            text_content: Some(text.to_string()),
            attributes: HashMap::new(),
            children: Vec::new(),
            parent: None,
            styles: HashMap::new(),
            event_listeners: HashMap::new(),
        }))
    }

    pub fn new_comment(text: &str) -> NodeRef {
        Rc::new(RefCell::new(Node {
            node_type: NodeType::Comment,
            tag_name: None,
            text_content: Some(text.to_string()),
            attributes: HashMap::new(),
            children: Vec::new(),
            parent: None,
            styles: HashMap::new(),
            event_listeners: HashMap::new(),
        }))
    }

    pub fn append_child(parent: &NodeRef, child: &NodeRef) {
        child.borrow_mut().parent = Some(Rc::downgrade(parent));
        parent.borrow_mut().children.push(Rc::clone(child));
    }
    
    /// Find the enclosing form element of a given node
    pub fn find_parent_form(node: &NodeRef) -> Option<NodeRef> {
        let mut current = node.clone();
        loop {
            // Check if current is a form
            let is_form = current.borrow().tag_name.as_deref() == Some("form");
            if is_form {
                return Some(current);
            }
            // Get parent
            let parent = current.borrow().parent.as_ref().and_then(|weak| weak.upgrade());
            match parent {
                Some(p) => current = p,
                None => return None,
            }
        }
    }

    pub fn get_attribute(&self, name: &str) -> Option<String> {
        self.attributes.get(name).cloned()
    }

    pub fn set_attribute(&mut self, name: &str, value: &str) {
        self.attributes.insert(name.to_string(), value.to_string());
    }

    pub fn remove_attribute(&mut self, name: &str) {
        self.attributes.remove(name);
    }

    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    pub fn get_id(&self) -> Option<String> {
        self.attributes.get("id").cloned()
    }

    pub fn get_classes(&self) -> Vec<String> {
        self.attributes
            .get("class")
            .map(|c| c.split_whitespace().map(String::from).collect())
            .unwrap_or_default()
    }

    pub fn has_class(&self, class: &str) -> bool {
        self.get_classes().contains(&class.to_string())
    }

    pub fn add_event_listener(&mut self, event: &str, callback_id: String) {
        self.event_listeners
            .entry(event.to_string())
            .or_default()
            .push(callback_id);
    }

    pub fn get_inner_text(&self) -> String {
        let mut text = String::new();
        self.collect_text(&mut text);
        text
    }

    fn collect_text(&self, out: &mut String) {
        if let Some(ref t) = self.text_content {
            out.push_str(t);
        }
        for child in &self.children {
            child.borrow().collect_text(out);
        }
    }

    pub fn get_inner_html(&self) -> String {
        let mut html = String::new();
        for child in &self.children {
            html.push_str(&Self::serialize_node(&child.borrow()));
        }
        html
    }

    fn serialize_node(node: &Node) -> String {
        match node.node_type {
            NodeType::Text => node.text_content.clone().unwrap_or_default(),
            NodeType::Comment => format!("<!--{}-->", node.text_content.clone().unwrap_or_default()),
            NodeType::Element => {
                let tag = node.tag_name.as_ref().unwrap();
                let mut attrs = String::new();
                for (k, v) in &node.attributes {
                    attrs.push_str(&format!(" {}=\"{}\"", k, v));
                }
                let mut inner = String::new();
                for child in &node.children {
                    inner.push_str(&Self::serialize_node(&child.borrow()));
                }
                if Self::is_void_element(tag) {
                    format!("<{}{} />", tag, attrs)
                } else {
                    format!("<{}{}>{}</{}>", tag, attrs, inner, tag)
                }
            }
            NodeType::Document => {
                let mut html = String::new();
                for child in &node.children {
                    html.push_str(&Self::serialize_node(&child.borrow()));
                }
                html
            }
        }
    }

    fn is_void_element(tag: &str) -> bool {
        matches!(tag, "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | 
                 "input" | "link" | "meta" | "param" | "source" | "track" | "wbr")
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub root: NodeRef,
    pub title: String,
    pub base_url: Option<String>,
}

impl Document {
    pub fn new() -> Self {
        Document {
            root: Node::new_document(),
            title: String::new(),
            base_url: None,
        }
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<NodeRef> {
        Self::find_by_id(&self.root, id)
    }

    fn find_by_id(node: &NodeRef, id: &str) -> Option<NodeRef> {
        let n = node.borrow();
        if n.get_id().as_deref() == Some(id) {
            return Some(Rc::clone(node));
        }
        for child in &n.children {
            if let Some(found) = Self::find_by_id(child, id) {
                return Some(found);
            }
        }
        None
    }

    pub fn get_elements_by_tag(&self, tag: &str) -> Vec<NodeRef> {
        let mut results = Vec::new();
        let tag_lower = crate::fontcase::ascii_lower(tag);
        Self::find_by_tag(&self.root, &tag_lower, &mut results);
        results
    }

    fn find_by_tag(node: &NodeRef, tag: &str, results: &mut Vec<NodeRef>) {
        let n = node.borrow();
        if n.tag_name.as_deref() == Some(tag) {
            results.push(Rc::clone(node));
        }
        for child in &n.children {
            Self::find_by_tag(child, tag, results);
        }
    }

    pub fn get_elements_by_class(&self, class: &str) -> Vec<NodeRef> {
        let mut results = Vec::new();
        Self::find_by_class(&self.root, class, &mut results);
        results
    }

    fn find_by_class(node: &NodeRef, class: &str, results: &mut Vec<NodeRef>) {
        let n = node.borrow();
        if n.has_class(class) {
            results.push(Rc::clone(node));
        }
        for child in &n.children {
            Self::find_by_class(child, class, results);
        }
    }

    pub fn query_selector(&self, selector: &str) -> Option<NodeRef> {
        self.query_selector_all(selector).into_iter().next()
    }

    pub fn query_selector_all(&self, selector: &str) -> Vec<NodeRef> {
        let mut results = Vec::new();
        let selectors: Vec<&str> = selector.split(',').map(|s| s.trim()).collect();
        for sel in selectors {
            Self::match_selector(&self.root, sel, &mut results);
        }
        results
    }

    fn match_selector(node: &NodeRef, selector: &str, results: &mut Vec<NodeRef>) {
        let n = node.borrow();
        if Self::node_matches_selector(&n, selector)
            && !results.iter().any(|r| Rc::ptr_eq(r, node)) {
                results.push(Rc::clone(node));
            }
        for child in &n.children {
            Self::match_selector(child, selector, results);
        }
    }

    fn node_matches_selector(node: &Node, selector: &str) -> bool {
        let selector = selector.trim();
        if let Some(stripped) = selector.strip_prefix('#') {
            node.get_id().as_deref() == Some(stripped)
        } else if let Some(stripped) = selector.strip_prefix('.') {
            node.has_class(stripped)
        } else if selector.contains('.') {
            let parts: Vec<&str> = selector.splitn(2, '.').collect();
            node.tag_name.as_deref() == Some(parts[0]) && node.has_class(parts[1])
        } else if selector.contains('#') {
            let parts: Vec<&str> = selector.splitn(2, '#').collect();
            node.tag_name.as_deref() == Some(parts[0]) && node.get_id().as_deref() == Some(parts[1])
        } else {
            node.tag_name.as_deref() == Some(selector)
        }
    }

    pub fn create_element(&self, tag: &str) -> NodeRef {
        Node::new_element(tag)
    }

    pub fn create_text_node(&self, text: &str) -> NodeRef {
        Node::new_text(text)
    }

    pub fn get_body(&self) -> Option<NodeRef> {
        self.get_elements_by_tag("body").into_iter().next()
    }

    pub fn get_head(&self) -> Option<NodeRef> {
        self.get_elements_by_tag("head").into_iter().next()
    }

    pub fn get_images(&self) -> Vec<NodeRef> {
        self.get_elements_by_tag("img")
    }

    pub fn get_links(&self) -> Vec<NodeRef> {
        self.get_elements_by_tag("a")
    }

    pub fn get_forms(&self) -> Vec<NodeRef> {
        self.get_elements_by_tag("form")
    }

    pub fn get_scripts(&self) -> Vec<NodeRef> {
        self.get_elements_by_tag("script")
    }

    pub fn get_stylesheets(&self) -> Vec<NodeRef> {
        let mut results = self.get_elements_by_tag("style");
        for link in self.get_elements_by_tag("link") {
            let n = link.borrow();
            if n.get_attribute("rel").as_deref() == Some("stylesheet") {
                results.push(Rc::clone(&link));
            }
        }
        results
    }
}

/// Form data for submission
#[derive(Debug, Clone, Default)]
pub struct FormData {
    pub action: String,
    pub method: String, // GET or POST
    pub enctype: String,
    pub fields: Vec<(String, String)>,
}

impl FormData {
    /// Collect form data from a form element
    pub fn from_form(form_node: &NodeRef) -> Self {
        let form = form_node.borrow();
        let action = form.get_attribute("action").unwrap_or_default();
        let method = form.get_attribute("method")
            .map(|m| m.to_uppercase())
            .unwrap_or_else(|| "GET".to_string());
        let enctype = form.get_attribute("enctype")
            .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string());
        
        let mut fields = Vec::new();
        Self::collect_inputs(form_node, &mut fields);
        
        FormData { action, method, enctype, fields }
    }
    
    fn collect_inputs(node: &NodeRef, fields: &mut Vec<(String, String)>) {
        let n = node.borrow();
        
        // Check if this is an input, select, or textarea
        if let Some(ref tag) = n.tag_name {
            match tag.as_str() {
                "input" => {
                    let name = n.get_attribute("name").unwrap_or_default();
                    if name.is_empty() { return; }
                    
                    let input_type = n.get_attribute("type")
                        .map(|t| crate::fontcase::ascii_lower(&t))
                        .unwrap_or_else(|| "text".to_string());
                    
                    match input_type.as_str() {
                        "checkbox" | "radio" => {
                            // Only include if checked
                            if n.get_attribute("checked").is_some() {
                                let value = n.get_attribute("value").unwrap_or_else(|| "on".to_string());
                                fields.push((name, value));
                            }
                        }
                        "submit" | "button" | "image" | "reset" => {
                            // Don't include these
                        }
                        "file" => {
                            // Would need file handling - skip for now
                        }
                        _ => {
                            // text, password, hidden, email, number, etc.
                            let value = n.get_attribute("value").unwrap_or_default();
                            fields.push((name, value));
                        }
                    }
                }
                "textarea" => {
                    let name = n.get_attribute("name").unwrap_or_default();
                    if !name.is_empty() {
                        let value = n.text_content.clone().unwrap_or_default();
                        fields.push((name, value));
                    }
                }
                "select" => {
                    let name = n.get_attribute("name").unwrap_or_default();
                    if !name.is_empty() {
                        // Find selected option
                        for child in &n.children {
                            let c = child.borrow();
                            if c.tag_name.as_deref() == Some("option")
                                && c.get_attribute("selected").is_some() {
                                    let value = c.get_attribute("value")
                                        .or_else(|| c.text_content.clone())
                                        .unwrap_or_default();
                                    fields.push((name.clone(), value));
                                    break;
                                }
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Recurse into children
        drop(n);
        for child in &node.borrow().children {
            Self::collect_inputs(child, fields);
        }
    }
    
    /// Encode as application/x-www-form-urlencoded
    pub fn to_urlencoded(&self) -> String {
        self.fields.iter()
            .map(|(k, v)| format!("{}={}", Self::url_encode(k), Self::url_encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }
    
    fn url_encode(s: &str) -> String {
        let mut result = String::new();
        for c in s.chars() {
            match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                    result.push(c);
                }
                ' ' => result.push('+'),
                _ => {
                    for b in c.to_string().as_bytes() {
                        result.push_str(&format!("%{:02X}", b));
                    }
                }
            }
        }
        result
    }
    
    /// Get the full URL for GET submission (resolves relative to base_url)
    pub fn get_url(&self, base_url: &str) -> String {
        let encoded = self.to_urlencoded();
        
        // Resolve action URL relative to base
        let action = if self.action.is_empty() {
            base_url.to_string()
        } else if self.action.starts_with("http://") || self.action.starts_with("https://") {
            self.action.clone()
        } else if self.action.starts_with('/') {
            // Absolute path - find origin
            if let Some(idx) = base_url.find("://") {
                if let Some(slash_idx) = base_url[idx + 3..].find('/') {
                    format!("{}{}", &base_url[..idx + 3 + slash_idx], &self.action)
                } else {
                    format!("{}{}", base_url, &self.action)
                }
            } else {
                self.action.clone()
            }
        } else {
            // Relative path
            if let Some(last_slash) = base_url.rfind('/') {
                format!("{}/{}", &base_url[..last_slash], &self.action)
            } else {
                format!("{}/{}", base_url, &self.action)
            }
        };
        
        if encoded.is_empty() {
            action
        } else if action.contains('?') {
            format!("{}&{}", action, encoded)
        } else {
            format!("{}?{}", action, encoded)
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}
