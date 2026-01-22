//! JSON Viewer
//!
//! Pretty-print and navigate JSON responses.
//! Collapsible tree view with syntax highlighting.

#![allow(dead_code)]

use crate::style::Color;
#[allow(unused_imports)]
use std::collections::HashMap;

/// JSON value type (simplified for our use)
#[derive(Debug, Clone)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),  // Preserves order
}

impl JsonValue {
    /// Parse JSON from string
    pub fn parse(s: &str) -> Result<JsonValue, String> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err("Empty input".to_string());
        }
        
        let (value, _) = Self::parse_value(trimmed)?;
        Ok(value)
    }
    
    fn parse_value(s: &str) -> Result<(JsonValue, &str), String> {
        let s = s.trim_start();
        
        if s.is_empty() {
            return Err("Unexpected end of input".to_string());
        }
        
        let first = s.chars().next().unwrap();
        
        match first {
            'n' => Self::parse_null(s),
            't' | 'f' => Self::parse_bool(s),
            '"' => Self::parse_string(s),
            '[' => Self::parse_array(s),
            '{' => Self::parse_object(s),
            c if c.is_ascii_digit() || c == '-' => Self::parse_number(s),
            _ => Err(format!("Unexpected character: {}", first)),
        }
    }
    
    fn parse_null(s: &str) -> Result<(JsonValue, &str), String> {
        if let Some(stripped) = s.strip_prefix("null") {
            Ok((JsonValue::Null, stripped))
        } else {
            Err("Expected 'null'".to_string())
        }
    }
    
    fn parse_bool(s: &str) -> Result<(JsonValue, &str), String> {
        if let Some(stripped) = s.strip_prefix("true") {
            Ok((JsonValue::Bool(true), stripped))
        } else if let Some(stripped) = s.strip_prefix("false") {
            Ok((JsonValue::Bool(false), stripped))
        } else {
            Err("Expected 'true' or 'false'".to_string())
        }
    }
    
    fn parse_number(s: &str) -> Result<(JsonValue, &str), String> {
        let mut end = 0;
        let chars: Vec<char> = s.chars().collect();
        
        // Optional minus
        if end < chars.len() && chars[end] == '-' {
            end += 1;
        }
        
        // Digits
        while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.' || chars[end] == 'e' || chars[end] == 'E' || chars[end] == '+' || chars[end] == '-') {
            end += 1;
        }
        
        let num_str = &s[..end];
        let num = num_str.parse::<f64>()
            .map_err(|e| format!("Invalid number: {}", e))?;
        
        Ok((JsonValue::Number(num), &s[end..]))
    }
    
    fn parse_string(s: &str) -> Result<(JsonValue, &str), String> {
        if !s.starts_with('"') {
            return Err("Expected string".to_string());
        }
        
        let chars: Vec<char> = s.chars().collect();
        let mut result = String::new();
        let mut i = 1;
        
        while i < chars.len() {
            match chars[i] {
                '"' => {
                    return Ok((JsonValue::String(result), &s[i + 1..]));
                }
                '\\' if i + 1 < chars.len() => {
                    i += 1;
                    match chars[i] {
                        '"' => result.push('"'),
                        '\\' => result.push('\\'),
                        '/' => result.push('/'),
                        'b' => result.push('\x08'),
                        'f' => result.push('\x0C'),
                        'n' => result.push('\n'),
                        'r' => result.push('\r'),
                        't' => result.push('\t'),
                        'u' => {
                            // Unicode escape
                            if i + 4 < chars.len() {
                                let hex: String = chars[i+1..i+5].iter().collect();
                                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                    if let Some(c) = char::from_u32(code) {
                                        result.push(c);
                                    }
                                }
                                i += 4;
                            }
                        }
                        _ => {
                            result.push('\\');
                            result.push(chars[i]);
                        }
                    }
                }
                c => result.push(c),
            }
            i += 1;
        }
        
        Err("Unterminated string".to_string())
    }
    
    fn parse_array(s: &str) -> Result<(JsonValue, &str), String> {
        if !s.starts_with('[') {
            return Err("Expected array".to_string());
        }
        
        let mut rest = s[1..].trim_start();
        let mut items = Vec::new();
        
        if let Some(stripped) = rest.strip_prefix(']') {
            return Ok((JsonValue::Array(items), stripped));
        }
        
        loop {
            let (value, r) = Self::parse_value(rest)?;
            items.push(value);
            rest = r.trim_start();
            
            if let Some(stripped) = rest.strip_prefix(']') {
                return Ok((JsonValue::Array(items), stripped));
            } else if let Some(stripped) = rest.strip_prefix(',') {
                rest = stripped.trim_start();
            } else {
                return Err("Expected ',' or ']'".to_string());
            }
        }
    }
    
    fn parse_object(s: &str) -> Result<(JsonValue, &str), String> {
        if !s.starts_with('{') {
            return Err("Expected object".to_string());
        }
        
        let mut rest = s[1..].trim_start();
        let mut pairs = Vec::new();
        
        if let Some(stripped) = rest.strip_prefix('}') {
            return Ok((JsonValue::Object(pairs), stripped));
        }
        
        loop {
            // Parse key
            let (key, r) = Self::parse_string(rest)?;
            let key_str = match key {
                JsonValue::String(s) => s,
                _ => return Err("Expected string key".to_string()),
            };
            
            rest = r.trim_start();
            
            if !rest.starts_with(':') {
                return Err("Expected ':'".to_string());
            }
            rest = rest[1..].trim_start();
            
            // Parse value
            let (value, r) = Self::parse_value(rest)?;
            pairs.push((key_str, value));
            rest = r.trim_start();
            
            if let Some(stripped) = rest.strip_prefix('}') {
                return Ok((JsonValue::Object(pairs), stripped));
            } else if let Some(stripped) = rest.strip_prefix(',') {
                rest = stripped.trim_start();
            } else {
                return Err("Expected ',' or '}'".to_string());
            }
        }
    }
    
    /// Pretty print with indentation
    pub fn pretty_print(&self, indent: usize) -> String {
        let spaces = "  ".repeat(indent);
        
        match self {
            JsonValue::Null => "null".to_string(),
            JsonValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            JsonValue::String(s) => format!("\"{}\"", Self::escape_string(s)),
            JsonValue::Array(items) => {
                if items.is_empty() {
                    "[]".to_string()
                } else if items.len() <= 3 && self.is_simple_array() {
                    // Single line for simple short arrays
                    let inner: Vec<String> = items.iter()
                        .map(|v| v.pretty_print(0))
                        .collect();
                    format!("[{}]", inner.join(", "))
                } else {
                    let inner_spaces = "  ".repeat(indent + 1);
                    let mut result = "[\n".to_string();
                    for (i, item) in items.iter().enumerate() {
                        result.push_str(&inner_spaces);
                        result.push_str(&item.pretty_print(indent + 1));
                        if i < items.len() - 1 {
                            result.push(',');
                        }
                        result.push('\n');
                    }
                    result.push_str(&spaces);
                    result.push(']');
                    result
                }
            }
            JsonValue::Object(pairs) => {
                if pairs.is_empty() {
                    "{}".to_string()
                } else {
                    let inner_spaces = "  ".repeat(indent + 1);
                    let mut result = "{\n".to_string();
                    for (i, (key, value)) in pairs.iter().enumerate() {
                        result.push_str(&inner_spaces);
                        result.push_str(&format!("\"{}\": ", Self::escape_string(key)));
                        result.push_str(&value.pretty_print(indent + 1));
                        if i < pairs.len() - 1 {
                            result.push(',');
                        }
                        result.push('\n');
                    }
                    result.push_str(&spaces);
                    result.push('}');
                    result
                }
            }
        }
    }
    
    fn is_simple_array(&self) -> bool {
        match self {
            JsonValue::Array(items) => items.iter().all(|v| matches!(v, 
                JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_)
            )),
            _ => false,
        }
    }
    
    fn escape_string(s: &str) -> String {
        let mut result = String::new();
        for c in s.chars() {
            match c {
                '"' => result.push_str("\\\""),
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                c if c.is_control() => result.push_str(&format!("\\u{:04x}", c as u32)),
                c => result.push(c),
            }
        }
        result
    }
    
    /// Get type name
    pub fn type_name(&self) -> &'static str {
        match self {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => "array",
            JsonValue::Object(_) => "object",
        }
    }
    
    /// Get child count for arrays/objects
    pub fn child_count(&self) -> usize {
        match self {
            JsonValue::Array(items) => items.len(),
            JsonValue::Object(pairs) => pairs.len(),
            _ => 0,
        }
    }
}

/// Tree node for JSON viewer
#[derive(Debug, Clone)]
pub struct JsonTreeNode {
    pub key: Option<String>,
    pub value: JsonValue,
    pub expanded: bool,
    pub depth: usize,
    pub path: Vec<String>,
}

impl JsonTreeNode {
    pub fn new(value: JsonValue) -> Self {
        JsonTreeNode {
            key: None,
            value,
            expanded: true,
            depth: 0,
            path: Vec::new(),
        }
    }
    
    pub fn with_key(mut self, key: String) -> Self {
        self.key = Some(key);
        self
    }
}

/// JSON Viewer state
pub struct JsonViewer {
    pub root: Option<JsonValue>,
    pub expanded_paths: std::collections::HashSet<String>,
    pub selected_path: Option<String>,
    pub search_query: String,
    pub search_results: Vec<String>,
    pub current_search_index: usize,
}

impl JsonViewer {
    pub fn new() -> Self {
        JsonViewer {
            root: None,
            expanded_paths: std::collections::HashSet::new(),
            selected_path: None,
            search_query: String::new(),
            search_results: Vec::new(),
            current_search_index: 0,
        }
    }
    
    /// Load JSON from string
    pub fn load(&mut self, json: &str) -> Result<(), String> {
        let value = JsonValue::parse(json)?;
        self.root = Some(value);
        self.expanded_paths.clear();
        self.expanded_paths.insert("$".to_string());
        self.selected_path = None;
        Ok(())
    }
    
    /// Toggle expand/collapse at path
    pub fn toggle_expand(&mut self, path: &str) {
        if self.expanded_paths.contains(path) {
            self.expanded_paths.remove(path);
        } else {
            self.expanded_paths.insert(path.to_string());
        }
    }
    
    /// Expand all
    pub fn expand_all(&mut self) {
        if let Some(root) = self.root.clone() {
            self.expand_recursive(&root, "$".to_string());
        }
    }
    
    fn expand_recursive(&mut self, value: &JsonValue, path: String) {
        self.expanded_paths.insert(path.clone());
        match value {
            JsonValue::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    self.expand_recursive(item, format!("{}[{}]", path, i));
                }
            }
            JsonValue::Object(pairs) => {
                for (key, val) in pairs {
                    self.expand_recursive(val, format!("{}.{}", path, key));
                }
            }
            _ => {}
        }
    }
    
    /// Collapse all
    pub fn collapse_all(&mut self) {
        self.expanded_paths.clear();
        self.expanded_paths.insert("$".to_string());
    }
    
    /// Search for text in JSON
    pub fn search(&mut self, query: &str) {
        self.search_query = query.to_string();
        self.search_results.clear();
        self.current_search_index = 0;
        
        if query.is_empty() {
            return;
        }
        
        if let Some(root) = self.root.clone() {
            let query_lower = crate::fontcase::ascii_lower(query);
            self.search_recursive(&root, "$".to_string(), &query_lower);
        }
    }
    
    fn search_recursive(&mut self, value: &JsonValue, path: String, query: &str) {
        match value {
            JsonValue::String(s) if crate::fontcase::ascii_lower(s).contains(query) => {
                self.search_results.push(path.clone());
            }
            JsonValue::Number(n) if n.to_string().contains(query) => {
                self.search_results.push(path.clone());
            }
            JsonValue::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    self.search_recursive(item, format!("{}[{}]", path, i), query);
                }
            }
            JsonValue::Object(pairs) => {
                for (key, val) in pairs {
                    if crate::fontcase::ascii_lower(key).contains(query) {
                        self.search_results.push(format!("{}.{}", path, key));
                    }
                    self.search_recursive(val, format!("{}.{}", path, key), query);
                }
            }
            _ => {}
        }
    }
    
    /// Go to next search result
    pub fn next_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_index = (self.current_search_index + 1) % self.search_results.len();
            let path = self.search_results[self.current_search_index].clone();
            self.selected_path = Some(path.clone());
            
            // Expand path to selected
            self.expand_path_to(&path);
        }
    }
    
    /// Go to previous search result
    pub fn prev_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_index = if self.current_search_index == 0 {
                self.search_results.len() - 1
            } else {
                self.current_search_index - 1
            };
            let path = self.search_results[self.current_search_index].clone();
            self.selected_path = Some(path.clone());
            
            self.expand_path_to(&path);
        }
    }
    
    fn expand_path_to(&mut self, path: &str) {
        let mut current = String::new();
        for part in path.split(['.', '[']) {
            if current.is_empty() {
                current = part.trim_end_matches(']').to_string();
            } else if part.ends_with(']') {
                current = format!("{}[{}", current, part);
            } else {
                current = format!("{}.{}", current, part);
            }
            self.expanded_paths.insert(current.clone());
        }
    }
    
    /// Copy value at path to clipboard format
    pub fn copy_value(&self, path: &str) -> Option<String> {
        self.get_value_at_path(path).map(|v| v.pretty_print(0))
    }
    
    /// Copy path
    pub fn copy_path(&self, path: &str) -> String {
        path.to_string()
    }
    
    fn get_value_at_path(&self, path: &str) -> Option<&JsonValue> {
        let root = self.root.as_ref()?;
        
        if path == "$" {
            return Some(root);
        }
        
        let mut current = root;
        let parts: Vec<&str> = path.trim_start_matches('$').trim_start_matches('.').split(['.', '[']).collect();
        
        for part in parts {
            if part.is_empty() {
                continue;
            }
            
            let part = part.trim_end_matches(']');
            
            match current {
                JsonValue::Array(items) => {
                    let idx: usize = part.parse().ok()?;
                    current = items.get(idx)?;
                }
                JsonValue::Object(pairs) => {
                    current = pairs.iter().find(|(k, _)| k == part).map(|(_, v)| v)?;
                }
                _ => return None,
            }
        }
        
        Some(current)
    }
}

impl Default for JsonViewer {
    fn default() -> Self {
        Self::new()
    }
}

/// Color scheme for JSON syntax highlighting
pub struct JsonColors {
    pub key: Color,
    pub string: Color,
    pub number: Color,
    pub bool_true: Color,
    pub bool_false: Color,
    pub null: Color,
    pub bracket: Color,
    pub colon: Color,
}

impl Default for JsonColors {
    fn default() -> Self {
        JsonColors {
            key: Color::new(224, 108, 117, 255),    // Red
            string: Color::new(152, 195, 121, 255), // Green
            number: Color::new(209, 154, 102, 255), // Orange
            bool_true: Color::new(86, 182, 194, 255), // Cyan
            bool_false: Color::new(198, 120, 221, 255), // Purple
            null: Color::new(92, 99, 112, 255),     // Gray
            bracket: Color::new(171, 178, 191, 255), // Light gray
            colon: Color::new(171, 178, 191, 255),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple() {
        let json = r#"{"name": "test", "value": 42}"#;
        let value = JsonValue::parse(json).unwrap();
        
        match value {
            JsonValue::Object(pairs) => {
                assert_eq!(pairs.len(), 2);
            }
            _ => panic!("Expected object"),
        }
    }
    
    #[test]
    fn test_parse_array() {
        let json = "[1, 2, 3]";
        let value = JsonValue::parse(json).unwrap();
        
        match value {
            JsonValue::Array(items) => {
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected array"),
        }
    }
    
    #[test]
    fn test_pretty_print() {
        let json = r#"{"name":"test"}"#;
        let value = JsonValue::parse(json).unwrap();
        let pretty = value.pretty_print(0);
        
        assert!(pretty.contains("\"name\""));
        assert!(pretty.contains("\"test\""));
    }
}
