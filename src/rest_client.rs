//! Built-in REST Client
//!
//! Test APIs directly in the browser - like Postman built-in.
//! The killer feature for developers.

 
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)] // These are HTTP method names
pub enum Method {
    #[serde(rename = "GET")]
    Get,
    #[serde(rename = "POST")]
    Post,
    #[serde(rename = "PUT")]
    Put,
    #[serde(rename = "PATCH")]
    Patch,
    #[serde(rename = "DELETE")]
    Delete,
    #[serde(rename = "HEAD")]
    Head,
    #[serde(rename = "OPTIONS")]
    Options,
}

impl Method {
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Method::Get),
            "POST" => Some(Method::Post),
            "PUT" => Some(Method::Put),
            "PATCH" => Some(Method::Patch),
            "DELETE" => Some(Method::Delete),
            "HEAD" => Some(Method::Head),
            "OPTIONS" => Some(Method::Options),
            _ => None,
        }
    }
    
    pub fn has_body(&self) -> bool {
        matches!(self, Method::Post | Method::Put | Method::Patch)
    }
}

/// Content type for request body
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ContentType {
    Json,
    FormUrlEncoded,
    FormData,
    Text,
    Binary,
}

impl ContentType {
    pub fn mime_type(&self) -> &'static str {
        match self {
            ContentType::Json => "application/json",
            ContentType::FormUrlEncoded => "application/x-www-form-urlencoded",
            ContentType::FormData => "multipart/form-data",
            ContentType::Text => "text/plain",
            ContentType::Binary => "application/octet-stream",
        }
    }
}

/// A saved request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRequest {
    pub id: String,
    pub name: String,
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub content_type: ContentType,
    pub created: chrono::DateTime<chrono::Utc>,
    pub updated: chrono::DateTime<chrono::Utc>,
}

impl SavedRequest {
    pub fn new(name: &str, method: Method, url: &str) -> Self {
        let now = chrono::Utc::now();
        SavedRequest {
            id: uuid_v4(),
            name: name.to_string(),
            method,
            url: url.to_string(),
            headers: Vec::new(),
            body: None,
            content_type: ContentType::Json,
            created: now,
            updated: now,
        }
    }
}

/// Collection of saved requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestCollection {
    pub id: String,
    pub name: String,
    pub requests: Vec<SavedRequest>,
    pub variables: HashMap<String, String>,
}

impl RequestCollection {
    pub fn new(name: &str) -> Self {
        RequestCollection {
            id: uuid_v4(),
            name: name.to_string(),
            requests: Vec::new(),
            variables: HashMap::new(),
        }
    }
    
    pub fn add_request(&mut self, request: SavedRequest) {
        self.requests.push(request);
    }
}

/// Response from a request
#[derive(Debug, Clone)]
pub struct RestResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub body_text: Option<String>,
    pub duration_ms: u64,
    pub size_bytes: usize,
}

impl RestResponse {
    /// Parse body as JSON (pretty-printed)
    pub fn json_pretty(&self) -> Option<String> {
        let text = self.body_text.as_ref()?;
        let value: serde_json::Value = serde_json::from_str(text).ok()?;
        serde_json::to_string_pretty(&value).ok()
    }
    
    /// Get content type from headers
    pub fn content_type(&self) -> Option<&str> {
        self.headers.iter()
            .find(|(k, _)| crate::fontcase::ascii_lower(k) == "content-type")
            .map(|(_, v)| v.as_str())
    }
    
    /// Is this a JSON response?
    pub fn is_json(&self) -> bool {
        self.content_type()
            .map(|ct| ct.contains("json"))
            .unwrap_or(false)
    }
    
    /// Is this HTML?
    pub fn is_html(&self) -> bool {
        self.content_type()
            .map(|ct| ct.contains("html"))
            .unwrap_or(false)
    }
}

/// REST Client state
pub struct RestClient {
    /// Panel visibility
    pub visible: bool,

    /// Current request being built
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String, bool)>,  // name, value, enabled
    pub body: String,
    pub content_type: ContentType,

    /// Response
    pub response: Option<RestResponse>,
    pub error: Option<String>,
    pub is_loading: bool,

    /// Saved collections
    pub collections: Vec<RequestCollection>,

    /// Environment variables
    pub environment: HashMap<String, String>,

    /// History
    pub history: Vec<SavedRequest>,
    pub max_history: usize,
}

impl RestClient {
    pub fn new() -> Self {
        RestClient {
            visible: false,
            method: Method::Get,
            url: String::new(),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string(), true),
                ("Accept".to_string(), "application/json".to_string(), true),
            ],
            body: String::new(),
            content_type: ContentType::Json,
            response: None,
            error: None,
            is_loading: false,
            collections: Vec::new(),
            environment: HashMap::new(),
            history: Vec::new(),
            max_history: 100,
        }
    }

    /// Toggle panel visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Substitute environment variables in a string
    pub fn substitute_vars(&self, text: &str) -> String {
        let mut result = text.to_string();
        
        for (key, value) in &self.environment {
            let pattern = format!("{{{{{}}}}}", key);  // {{key}}
            result = result.replace(&pattern, value);
        }
        
        result
    }
    
    /// Execute the current request
    pub fn execute(&mut self) {
        self.error = None;
        self.response = None;
        self.is_loading = true;
        
        let url = self.substitute_vars(&self.url);
        
        // Validate URL
        if url.is_empty() {
            self.error = Some("URL is required".to_string());
            self.is_loading = false;
            return;
        }
        
        if !url.starts_with("http://") && !url.starts_with("https://") {
            self.error = Some("URL must start with http:// or https://".to_string());
            self.is_loading = false;
            return;
        }
        
        let start = std::time::Instant::now();
        
        // Build request
        let mut request = match self.method {
            Method::Get => ureq::get(&url),
            Method::Post => ureq::post(&url),
            Method::Put => ureq::put(&url),
            Method::Patch => ureq::patch(&url),
            Method::Delete => ureq::delete(&url),
            Method::Head => ureq::head(&url),
            Method::Options => ureq::request("OPTIONS", &url),
        };
        
        // Add headers
        for (name, value, enabled) in &self.headers {
            if *enabled {
                let name_sub = self.substitute_vars(name);
                let value_sub = self.substitute_vars(value);
                request = request.set(&name_sub, &value_sub);
            }
        }
        
        // Send request
        let result = if self.method.has_body() && !self.body.is_empty() {
            let body = self.substitute_vars(&self.body);
            request.send_string(&body)
        } else {
            request.call()
        };
        
        let duration = start.elapsed().as_millis() as u64;
        
        match result {
            Ok(response) => {
                let status = response.status();
                let status_text = response.status_text().to_string();
                
                // Collect headers
                let mut headers = Vec::new();
                for name in response.headers_names() {
                    if let Some(value) = response.header(&name) {
                        headers.push((name, value.to_string()));
                    }
                }
                
                // Read body
                let mut body = Vec::new();
                if response.into_reader().read_to_end(&mut body).is_err() {
                    self.error = Some("Failed to read response body".to_string());
                    self.is_loading = false;
                    return;
                }
                
                let body_text = String::from_utf8(body.clone()).ok();
                let size_bytes = body.len();
                
                self.response = Some(RestResponse {
                    status,
                    status_text,
                    headers,
                    body,
                    body_text,
                    duration_ms: duration,
                    size_bytes,
                });
                
                // Add to history
                let saved = SavedRequest {
                    id: uuid_v4(),
                    name: format!("{} {}", self.method.as_str(), self.url),
                    method: self.method,
                    url: self.url.clone(),
                    headers: self.headers.iter()
                        .filter(|(_, _, e)| *e)
                        .map(|(n, v, _)| (n.clone(), v.clone()))
                        .collect(),
                    body: if self.body.is_empty() { None } else { Some(self.body.clone()) },
                    content_type: self.content_type,
                    created: chrono::Utc::now(),
                    updated: chrono::Utc::now(),
                };
                
                self.history.insert(0, saved);
                if self.history.len() > self.max_history {
                    self.history.pop();
                }
            }
            Err(e) => {
                match e {
                    ureq::Error::Status(code, response) => {
                        let status_text = response.status_text().to_string();
                        
                        let mut headers = Vec::new();
                        for name in response.headers_names() {
                            if let Some(value) = response.header(&name) {
                                headers.push((name, value.to_string()));
                            }
                        }
                        
                        let mut body = Vec::new();
                        let _ = response.into_reader().read_to_end(&mut body);
                        let body_text = String::from_utf8(body.clone()).ok();
                        let size_bytes = body.len();
                        
                        self.response = Some(RestResponse {
                            status: code,
                            status_text,
                            headers,
                            body,
                            body_text,
                            duration_ms: duration,
                            size_bytes,
                        });
                    }
                    ureq::Error::Transport(t) => {
                        self.error = Some(format!("Transport error: {}", t));
                    }
                }
            }
        }
        
        self.is_loading = false;
    }
    
    /// Load a saved request
    pub fn load_request(&mut self, request: &SavedRequest) {
        self.method = request.method;
        self.url = request.url.clone();
        self.headers = request.headers.iter()
            .map(|(n, v)| (n.clone(), v.clone(), true))
            .collect();
        self.body = request.body.clone().unwrap_or_default();
        self.content_type = request.content_type;
        self.response = None;
        self.error = None;
    }
    
    /// Clear the current request
    pub fn clear(&mut self) {
        self.method = Method::Get;
        self.url.clear();
        self.headers = vec![
            ("Content-Type".to_string(), "application/json".to_string(), true),
            ("Accept".to_string(), "application/json".to_string(), true),
        ];
        self.body.clear();
        self.response = None;
        self.error = None;
    }
    
    /// Add a header
    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.push((name.to_string(), value.to_string(), true));
    }
    
    /// Remove a header
    pub fn remove_header(&mut self, index: usize) {
        if index < self.headers.len() {
            self.headers.remove(index);
        }
    }
    
    /// Toggle header enabled state
    pub fn toggle_header(&mut self, index: usize) {
        if let Some(header) = self.headers.get_mut(index) {
            header.2 = !header.2;
        }
    }
    
    /// Generate cURL command for current request
    pub fn to_curl(&self) -> String {
        let mut parts = vec![format!("curl -X {}", self.method.as_str())];
        
        for (name, value, enabled) in &self.headers {
            if *enabled {
                let name_sub = self.substitute_vars(name);
                let value_sub = self.substitute_vars(value);
                parts.push(format!("-H '{}: {}'", name_sub, value_sub));
            }
        }
        
        if self.method.has_body() && !self.body.is_empty() {
            let body = self.substitute_vars(&self.body);
            parts.push(format!("-d '{}'", body.replace("'", "\\'")));
        }
        
        let url = self.substitute_vars(&self.url);
        parts.push(format!("'{}'", url));
        
        parts.join(" \\\n  ")
    }
    
    /// Generate JavaScript fetch code
    pub fn to_fetch(&self) -> String {
        let url = self.substitute_vars(&self.url);
        let mut lines = vec![format!("fetch('{}', {{", url)];
        
        lines.push(format!("  method: '{}',", self.method.as_str()));
        
        // Headers
        lines.push("  headers: {".to_string());
        for (i, (name, value, enabled)) in self.headers.iter().enumerate() {
            if *enabled {
                let name_sub = self.substitute_vars(name);
                let value_sub = self.substitute_vars(value);
                let comma = if i < self.headers.iter().filter(|(_, _, e)| *e).count() - 1 { "," } else { "" };
                lines.push(format!("    '{}': '{}'{}", name_sub, value_sub, comma));
            }
        }
        lines.push("  },".to_string());
        
        // Body
        if self.method.has_body() && !self.body.is_empty() {
            let body = self.substitute_vars(&self.body);
            let escaped = body.replace("'", "\\'").replace("\n", "\\n");
            lines.push(format!("  body: '{}'", escaped));
        }
        
        lines.push("})".to_string());
        lines.push(".then(response => response.json())".to_string());
        lines.push(".then(data => console.log(data))".to_string());
        lines.push(".catch(error => console.error(error));".to_string());
        
        lines.join("\n")
    }
}

impl Default for RestClient {
    fn default() -> Self {
        Self::new()
    }
}

use std::io::Read;

/// Generate a simple UUID v4
fn uuid_v4() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.r#gen();
    
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        (bytes[6] & 0x0f) | 0x40, bytes[7],
        (bytes[8] & 0x3f) | 0x80, bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_substitute_vars() {
        let mut client = RestClient::new();
        client.environment.insert("host".to_string(), "api.example.com".to_string());
        client.environment.insert("version".to_string(), "v1".to_string());
        
        let result = client.substitute_vars("https://{{host}}/{{version}}/users");
        assert_eq!(result, "https://api.example.com/v1/users");
    }
    
    #[test]
    fn test_to_curl() {
        let mut client = RestClient::new();
        client.method = Method::Post;
        client.url = "https://api.example.com/users".to_string();
        client.body = r#"{"name": "test"}"#.to_string();
        
        let curl = client.to_curl();
        assert!(curl.contains("curl"));
        assert!(curl.contains("POST"));
    }
}
