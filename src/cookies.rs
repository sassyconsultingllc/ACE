//! Cookie Jar
//!
//! HTTP cookie storage and management with proper security handling.


use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

/// A single HTTP cookie
#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<DateTime<Utc>>,
    pub max_age: Option<i64>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub created: DateTime<Utc>,
}

/// SameSite cookie attribute
#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Default)]
pub enum SameSite {
    Strict,
    #[default]
    Lax,
    None,
}


impl Cookie {
    pub fn new(name: &str, value: &str, domain: &str) -> Self {
        Cookie {
            name: name.to_string(),
            value: value.to_string(),
            domain: domain.to_string(),
            path: "/".to_string(),
            expires: None,
            max_age: None,
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            created: Utc::now(),
        }
    }
    
    /// Parse a Set-Cookie header value
    pub fn parse(header: &str, domain: &str) -> Option<Cookie> {
        let parts: Vec<&str> = header.split(';').collect();
        if parts.is_empty() {
            return None;
        }
        
        // First part is name=value
        let name_value: Vec<&str> = parts[0].splitn(2, '=').collect();
        if name_value.len() != 2 {
            return None;
        }
        
        let name = name_value[0].trim();
        let value = name_value[1].trim();
        
        let mut cookie = Cookie::new(name, value, domain);
        
        // Parse attributes
        for part in parts.iter().skip(1) {
            let attr: Vec<&str> = part.splitn(2, '=').collect();
            let attr_name = crate::fontcase::ascii_lower(attr[0].trim());
            let attr_value = attr.get(1).map(|v| v.trim()).unwrap_or("");
            
            match attr_name.as_str() {
                "domain" => {
                    cookie.domain = attr_value.trim_start_matches('.').to_string();
                }
                "path" => {
                    cookie.path = attr_value.to_string();
                }
                "expires" => {
                    // Parse HTTP date format
                    if let Ok(dt) = DateTime::parse_from_rfc2822(attr_value) {
                        cookie.expires = Some(dt.with_timezone(&Utc));
                    }
                }
                "max-age" => {
                    if let Ok(secs) = attr_value.parse::<i64>() {
                        cookie.max_age = Some(secs);
                        cookie.expires = Some(Utc::now() + Duration::seconds(secs));
                    }
                }
                "secure" => {
                    cookie.secure = true;
                }
                "httponly" => {
                    cookie.http_only = true;
                }
                "samesite" => {
                    cookie.same_site = match crate::fontcase::ascii_lower(attr_value).as_str() {
                        "strict" => SameSite::Strict,
                        "lax" => SameSite::Lax,
                        "none" => SameSite::None,
                        _ => SameSite::Lax,
                    };
                }
                _ => {}
            }
        }
        
        Some(cookie)
    }
    
    /// Check if cookie is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires {
            expires < Utc::now()
        } else {
            false
        }
    }
    
    /// Check if cookie matches a URL
    pub fn matches(&self, domain: &str, path: &str, is_secure: bool) -> bool {
        // Check domain
        if !self.domain_matches(domain) {
            return false;
        }
        
        // Check path
        if !path.starts_with(&self.path) {
            return false;
        }
        
        // Check secure
        if self.secure && !is_secure {
            return false;
        }
        
        // Check expiry
        if self.is_expired() {
            return false;
        }
        
        true
    }
    
    fn domain_matches(&self, domain: &str) -> bool {
        if domain == self.domain {
            return true;
        }
        
        // Check if domain is a subdomain
        if domain.ends_with(&format!(".{}", self.domain)) {
            return true;
        }
        
        false
    }
    
    /// Convert to Cookie header format (just name=value)
    pub fn to_header_value(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
    
    /// Convert to Set-Cookie header format
    pub fn to_set_cookie(&self) -> String {
        let mut parts = vec![format!("{}={}", self.name, self.value)];
        
        parts.push(format!("Domain={}", self.domain));
        parts.push(format!("Path={}", self.path));
        
        if let Some(expires) = self.expires {
            parts.push(format!("Expires={}", expires.format("%a, %d %b %Y %H:%M:%S GMT")));
        }
        
        if self.secure {
            parts.push("Secure".to_string());
        }
        
        if self.http_only {
            parts.push("HttpOnly".to_string());
        }
        
        parts.push(format!("SameSite={}", match self.same_site {
            SameSite::Strict => "Strict",
            SameSite::Lax => "Lax",
            SameSite::None => "None",
        }));
        
        parts.join("; ")
    }
}

/// Cookie storage
pub struct CookieJar {
    /// Cookies organized by domain
    cookies: HashMap<String, Vec<Cookie>>,
}

impl CookieJar {
    pub fn new() -> Self {
        CookieJar {
            cookies: HashMap::new(),
        }
    }
    
    /// Add a cookie
    pub fn set(&mut self, cookie: Cookie) {
        let domain = cookie.domain.clone();
        let name = cookie.name.clone();
        
        let cookies = self.cookies.entry(domain).or_default();
        
        // Remove existing cookie with same name and path
        cookies.retain(|c| !(c.name == name && c.path == cookie.path));
        
        // Don't add if already expired
        if !cookie.is_expired() {
            cookies.push(cookie);
        }
    }
    
    /// Get a specific cookie
    pub fn get(&self, domain: &str, name: &str) -> Option<&Cookie> {
        self.cookies.get(domain)?.iter().find(|c| c.name == name && !c.is_expired())
    }
    
    /// Get all cookies for a URL
    pub fn cookies_for_url(&self, url: &str) -> Vec<&Cookie> {
        let Ok(parsed) = url::Url::parse(url) else {
            return Vec::new();
        };
        
        let domain = parsed.host_str().unwrap_or("");
        let path = parsed.path();
        let is_secure = parsed.scheme() == "https";
        
        let mut result = Vec::new();
        
        // Check all domains that might match
        for (_cookie_domain, cookies) in &self.cookies {
            for cookie in cookies {
                if cookie.matches(domain, path, is_secure) {
                    result.push(cookie);
                }
            }
        }
        
        // Sort by path length (more specific first)
        result.sort_by(|a, b| b.path.len().cmp(&a.path.len()));
        
        result
    }
    
    /// Get Cookie header value for a URL
    pub fn cookie_header(&self, url: &str) -> Option<String> {
        let cookies = self.cookies_for_url(url);
        
        if cookies.is_empty() {
            return None;
        }
        
        let header: Vec<String> = cookies.iter()
            .filter(|c| !c.http_only)  // HttpOnly cookies shouldn't be sent via JS
            .map(|c| c.to_header_value())
            .collect();
        
        if header.is_empty() {
            None
        } else {
            Some(header.join("; "))
        }
    }
    
    /// Process Set-Cookie headers from a response
    pub fn process_set_cookie(&mut self, url: &str, headers: &[(String, String)]) {
        let Ok(parsed) = url::Url::parse(url) else {
            return;
        };
        
        let domain = parsed.host_str().unwrap_or("");
        
        for (name, value) in headers {
            if crate::fontcase::ascii_lower(name) == "set-cookie" {
                if let Some(cookie) = Cookie::parse(value, domain) {
                    self.set(cookie);
                }
            }
        }
    }
    
    /// Remove a cookie
    pub fn remove(&mut self, domain: &str, name: &str, path: &str) {
        if let Some(cookies) = self.cookies.get_mut(domain) {
            cookies.retain(|c| !(c.name == name && c.path == path));
        }
    }
    
    /// Clear all cookies for a domain
    pub fn clear_domain(&mut self, domain: &str) {
        self.cookies.remove(domain);
    }
    
    /// Clear all cookies
    pub fn clear(&mut self) {
        self.cookies.clear();
    }
    
    /// Remove expired cookies
    pub fn cleanup(&mut self) {
        for cookies in self.cookies.values_mut() {
            cookies.retain(|c| !c.is_expired());
        }
        
        // Remove empty domains
        self.cookies.retain(|_, v| !v.is_empty());
    }
    
    /// Count total cookies
    pub fn len(&self) -> usize {
        self.cookies.values().map(|v| v.len()).sum()
    }
    
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

// Global cookie jar
lazy_static::lazy_static! {
    pub static ref COOKIES: RwLock<CookieJar> = RwLock::new(CookieJar::new());
}

/// Set a cookie globally
pub fn set_cookie(cookie: Cookie) {
    if let Ok(mut jar) = COOKIES.write() {
        jar.set(cookie);
    }
}

/// Get cookies for a URL
pub fn get_cookies_for_url(url: &str) -> Option<String> {
    if let Ok(jar) = COOKIES.read() {
        jar.cookie_header(url)
    } else {
        None
    }
}

/// Process Set-Cookie headers
pub fn process_response_cookies(url: &str, headers: &[(String, String)]) {
    if let Ok(mut jar) = COOKIES.write() {
        jar.process_set_cookie(url, headers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cookie_parse() {
        let cookie = Cookie::parse(
            "session=abc123; Domain=example.com; Path=/; Secure; HttpOnly",
            "example.com"
        ).unwrap();
        
        assert_eq!(cookie.name, "session");
        assert_eq!(cookie.value, "abc123");
        assert!(cookie.secure);
        assert!(cookie.http_only);
    }
    
    #[test]
    fn test_cookie_jar() {
        let mut jar = CookieJar::new();
        
        let cookie = Cookie::new("test", "value", "example.com");
        jar.set(cookie);
        
        assert_eq!(jar.len(), 1);
        assert!(jar.get("example.com", "test").is_some());
    }
    
    #[test]
    fn test_cookies_for_url() {
        let mut jar = CookieJar::new();
        
        let cookie = Cookie::new("auth", "token123", "api.example.com");
        jar.set(cookie);
        
        let cookies = jar.cookies_for_url("https://api.example.com/v1/users");
        assert_eq!(cookies.len(), 1);
    }
}
