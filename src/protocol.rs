// Protocol - HTTP/HTTPS handling
#![allow(dead_code)]

use url::Url;
use std::collections::HashMap;
use std::time::Duration;

pub struct HttpClient {
    user_agent: String,
    timeout: Duration,
    max_redirects: u32,
    cookies: HashMap<String, HashMap<String, String>>,
}

impl HttpClient {
    pub fn new() -> Self {
        HttpClient {
            user_agent: "SassyBrowser/0.3.0 (Rust)".to_string(),
            timeout: Duration::from_secs(30),
            max_redirects: 10,
            cookies: HashMap::new(),
        }
    }

    pub fn fetch(&mut self, url: &str) -> Result<HttpResponse, String> {
        let parsed = Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
        self.fetch_with_options(&parsed, &FetchOptions::default())
    }

    pub fn fetch_with_options(&mut self, url: &Url, options: &FetchOptions) -> Result<HttpResponse, String> {
        let mut current_url = url.clone();
        let mut redirect_count = 0;
        
        loop {
            let response = self.make_request(&current_url, options)?;
            
            // Store cookies
            for cookie in &response.cookies {
                self.store_cookie(&current_url, cookie);
            }
            
            // Handle redirects
            if response.status >= 300 && response.status < 400 {
                if redirect_count >= self.max_redirects {
                    return Err("Too many redirects".to_string());
                }
                
                if let Some(location) = response.headers.get("location") {
                    current_url = current_url.join(location)
                        .map_err(|e| format!("Invalid redirect URL: {}", e))?;
                    redirect_count += 1;
                    continue;
                }
            }
            
            if response.status >= 400 {
                return Err(format!("HTTP error: {}", response.status));
            }
            
            return Ok(response);
        }
    }

    fn make_request(&self, url: &Url, options: &FetchOptions) -> Result<HttpResponse, String> {
        let mut request = ureq::request(&options.method, url.as_str())
            .set("User-Agent", &self.user_agent)
            .timeout(self.timeout);
        
        // Add cookies
        if let Some(cookies) = self.cookies.get(url.host_str().unwrap_or("")) {
            let cookie_header: String = cookies.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            if !cookie_header.is_empty() {
                request = request.set("Cookie", &cookie_header);
            }
        }
        
        // Add custom headers
        for (key, value) in &options.headers {
            request = request.set(key, value);
        }
        
        // Send request
        let response = if let Some(ref body) = options.body {
            request.send_string(body)
        } else {
            request.call()
        };
        
        match response {
            Ok(resp) => {
                let status = resp.status();
                let mut headers = HashMap::new();
                let mut cookies = Vec::new();
                
                for name in resp.headers_names() {
                    if let Some(value) = resp.header(&name) {
                        if crate::fontcase::ascii_lower(&name) == "set-cookie" {
                            cookies.push(value.to_string());
                        }
                        headers.insert(crate::fontcase::ascii_lower(&name), value.to_string());
                    }
                }
                
                let body = resp.into_string()
                    .map_err(|e| format!("Failed to read response: {}", e))?;
                
                Ok(HttpResponse { status, headers, body, cookies })
            }
            Err(ureq::Error::Status(code, resp)) => {
                let mut headers = HashMap::new();
                for name in resp.headers_names() {
                    if let Some(value) = resp.header(&name) {
                        headers.insert(crate::fontcase::ascii_lower(&name), value.to_string());
                    }
                }
                let body = resp.into_string().unwrap_or_default();
                Ok(HttpResponse { status: code, headers, body, cookies: Vec::new() })
            }
            Err(e) => Err(format!("Request failed: {}", e)),
        }
    }

    fn store_cookie(&mut self, url: &Url, cookie_str: &str) {
        let host = url.host_str().unwrap_or("").to_string();
        let cookies = self.cookies.entry(host).or_default();
        
        // Parse cookie (simplified)
        if let Some((name_value, _)) = cookie_str.split_once(';') {
            if let Some((name, value)) = name_value.split_once('=') {
                cookies.insert(name.trim().to_string(), value.trim().to_string());
            }
        } else if let Some((name, value)) = cookie_str.split_once('=') {
            cookies.insert(name.trim().to_string(), value.trim().to_string());
        }
    }

    pub fn set_user_agent(&mut self, ua: &str) {
        self.user_agent = ua.to_string();
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn get_cookie(&self, host: &str, name: &str) -> Option<&String> {
        self.cookies.get(host).and_then(|c| c.get(name))
    }

    pub fn set_cookie(&mut self, host: &str, name: &str, value: &str) {
        let cookies = self.cookies.entry(host.to_string()).or_default();
        cookies.insert(name.to_string(), value.to_string());
    }

    pub fn clear_cookies(&mut self) {
        self.cookies.clear();
    }

    pub fn clear_cookies_for_host(&mut self, host: &str) {
        self.cookies.remove(host);
    }
}

impl Default for HttpClient {
    fn default() -> Self { Self::new() }
}

pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub cookies: Vec<String>,
}

#[derive(Clone)]
pub struct FetchOptions {
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub credentials: CredentialsMode,
    pub cache: CacheMode,
    pub redirect: RedirectMode,
}

impl Default for FetchOptions {
    fn default() -> Self {
        FetchOptions {
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
            credentials: CredentialsMode::SameOrigin,
            cache: CacheMode::Default,
            redirect: RedirectMode::Follow,
        }
    }
}

impl FetchOptions {
    pub fn get() -> Self { FetchOptions::default() }
    
    pub fn post(body: &str) -> Self {
        FetchOptions { method: "POST".to_string(), body: Some(body.to_string()), ..Default::default() }
    }
    
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }
    
    pub fn with_json_body(mut self, body: &str) -> Self {
        self.body = Some(body.to_string());
        self.headers.insert("Content-Type".to_string(), "application/json".to_string());
        self
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum CredentialsMode {
    Omit,
    SameOrigin,
    Include,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CacheMode {
    Default,
    NoStore,
    Reload,
    NoCache,
    ForceCache,
    OnlyIfCached,
}

#[derive(Clone, Copy, PartialEq)]
pub enum RedirectMode {
    Follow,
    Error,
    Manual,
}

// Data URLs
pub fn parse_data_url(url: &str) -> Option<(String, Vec<u8>)> {
    if !url.starts_with("data:") { return None; }
    
    let content = &url[5..];
    let (media_type, data) = if let Some(comma_pos) = content.find(',') {
        (&content[..comma_pos], &content[comma_pos + 1..])
    } else {
        return None;
    };
    
    let is_base64 = media_type.ends_with(";base64");
    let mime = if is_base64 {
        &media_type[..media_type.len() - 7]
    } else {
        media_type
    };
    
    let decoded = if is_base64 {
        base64_decode(data)?
    } else {
        url_decode(data).into_bytes()
    };
    
    Some((mime.to_string(), decoded))
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let input: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    
    for c in input.chars() {
        if c == '=' { break; }
        let val = ALPHABET.iter().position(|&x| x == c as u8)? as u32;
        buffer = (buffer << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    
    Some(output)
}

fn url_decode(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    
    result
}

// URL encoding
pub fn url_encode(input: &str) -> String {
    let mut result = String::new();
    for c in input.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                for byte in c.to_string().bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

// Form data encoding
pub fn encode_form_data(data: &HashMap<String, String>) -> String {
    data.iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

// Multipart form data
pub struct MultipartFormData {
    boundary: String,
    parts: Vec<MultipartPart>,
}

struct MultipartPart {
    name: String,
    filename: Option<String>,
    content_type: String,
    data: Vec<u8>,
}

impl MultipartFormData {
    pub fn new() -> Self {
        let boundary = format!("----SassyBrowser{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos());
        MultipartFormData { boundary, parts: Vec::new() }
    }

    pub fn add_field(&mut self, name: &str, value: &str) {
        self.parts.push(MultipartPart {
            name: name.to_string(),
            filename: None,
            content_type: "text/plain".to_string(),
            data: value.as_bytes().to_vec(),
        });
    }

    pub fn add_file(&mut self, name: &str, filename: &str, content_type: &str, data: Vec<u8>) {
        self.parts.push(MultipartPart {
            name: name.to_string(),
            filename: Some(filename.to_string()),
            content_type: content_type.to_string(),
            data,
        });
    }

    pub fn get_content_type(&self) -> String {
        format!("multipart/form-data; boundary={}", self.boundary)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut result = Vec::new();
        
        for part in &self.parts {
            result.extend_from_slice(format!("--{}`r`n", self.boundary).as_bytes());
            
            if let Some(ref filename) = part.filename {
                result.extend_from_slice(format!(
                    "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"`r`n",
                    part.name, filename
                ).as_bytes());
            } else {
                result.extend_from_slice(format!(
                    "Content-Disposition: form-data; name=\"{}\"`r`n",
                    part.name
                ).as_bytes());
            }
            
            result.extend_from_slice(format!("Content-Type: {}`r`n`r`n", part.content_type).as_bytes());
            result.extend_from_slice(&part.data);
            result.extend_from_slice(b"`r`n");
        }
        
        result.extend_from_slice(format!("--{}--`r`n", self.boundary).as_bytes());
        result
    }
}

impl Default for MultipartFormData {
    fn default() -> Self { Self::new() }
}
