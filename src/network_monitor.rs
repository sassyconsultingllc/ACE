// ==============================================================================
// SASSY BROWSER - NETWORK ACTIVITY MONITOR
// ==============================================================================
// Always-visible activity indicator showing all connections, downloads, traffic
// NO MYSTERY TRAFFIC. NO HIDDEN REQUESTS. FULL TRANSPARENCY.
// ==============================================================================

#![allow(dead_code, unused_variables, unused_imports)]

use std::collections::HashMap;
use std::time::{Duration, Instant};

// ==============================================================================
// CONNECTION TRACKING
// ==============================================================================

#[derive(Debug, Clone)]
pub struct ActiveConnection {
    pub id: u64,
    pub url: String,
    pub domain: String,
    pub connection_type: ConnectionType,
    pub state: ConnectionState,
    pub started_at: Instant,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub request_method: String,
    pub status_code: Option<u16>,
    pub content_type: Option<String>,
    pub is_secure: bool,
    pub ip_address: Option<String>,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    Document,
    Download,
    Script,
    Stylesheet,
    Image,
    Font,
    Xhr,
    WebSocket,
    Media,
    Other,
}

impl ConnectionType {
    pub fn icon(&self) -> &'static str {
        match self {
            ConnectionType::Document => "",
            ConnectionType::Download => "v",
            ConnectionType::Script => "*",
            ConnectionType::Stylesheet => "",
            ConnectionType::Image => "",
            ConnectionType::Font => "",
            ConnectionType::Xhr => "",
            ConnectionType::WebSocket => "",
            ConnectionType::Media => "",
            ConnectionType::Other => "[X]",
        }
    }
    
    pub fn from_content_type(ct: &str) -> Self {
        let ct_lower = crate::fontcase::ascii_lower(ct);
        if ct_lower.contains("octet-stream") || ct_lower.contains("zip") || ct_lower.contains("application/x-msdownload") {
            ConnectionType::Download
        } else if ct_lower.contains("html") {
            ConnectionType::Document
        } else if ct_lower.contains("javascript") || ct_lower.contains("ecmascript") {
            ConnectionType::Script
        } else if ct_lower.contains("css") {
            ConnectionType::Stylesheet
        } else if ct_lower.contains("image") {
            ConnectionType::Image
        } else if ct_lower.contains("font") || ct_lower.contains("woff") {
            ConnectionType::Font
        } else if ct_lower.contains("json") || ct_lower.contains("xml") {
            ConnectionType::Xhr
        } else if ct_lower.contains("video") || ct_lower.contains("audio") {
            ConnectionType::Media
        } else {
            ConnectionType::Other
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Uploading,
    Downloading,
    Complete,
    Failed(String),
    Cancelled,
}

impl ConnectionState {
    pub fn color(&self) -> [u8; 3] {
        match self {
            ConnectionState::Connecting => [255, 200, 0],
            ConnectionState::Uploading => [100, 150, 255],
            ConnectionState::Downloading => [0, 200, 100],
            ConnectionState::Complete => [150, 150, 150],
            ConnectionState::Failed(_) => [255, 80, 80],
            ConnectionState::Cancelled => [200, 200, 200],
        }
    }
}

// ==============================================================================
// BANDWIDTH TRACKING
// ==============================================================================

#[derive(Debug, Clone)]
pub struct BandwidthSample {
    pub timestamp: Instant,
    pub bytes_down: u64,
    pub bytes_up: u64,
}

pub struct BandwidthTracker {
    samples: Vec<BandwidthSample>,
    total_down: u64,
    total_up: u64,
    session_start: Instant,
}

impl BandwidthTracker {
    pub fn new() -> Self {
        Self {
            samples: Vec::with_capacity(120),
            total_down: 0,
            total_up: 0,
            session_start: Instant::now(),
        }
    }
    
    pub fn record(&mut self, bytes_down: u64, bytes_up: u64) {
        self.total_down += bytes_down;
        self.total_up += bytes_up;
        
        self.samples.push(BandwidthSample {
            timestamp: Instant::now(),
            bytes_down,
            bytes_up,
        });
        
        let cutoff = Instant::now() - Duration::from_secs(120);
        self.samples.retain(|s| s.timestamp > cutoff);
    }
    
    pub fn current_speed(&self) -> (f64, f64) {
        let now = Instant::now();
        let window = Duration::from_secs(1);
        let cutoff = now - window;
        
        let (down, up) = self.samples.iter()
            .filter(|s| s.timestamp > cutoff)
            .fold((0u64, 0u64), |(d, u), s| (d + s.bytes_down, u + s.bytes_up));
        
        (down as f64, up as f64)
    }
    
    pub fn total(&self) -> (u64, u64) {
        (self.total_down, self.total_up)
    }
}

// ==============================================================================
// NETWORK MONITOR
// ==============================================================================

pub struct NetworkMonitor {
    connections: HashMap<u64, ActiveConnection>,
    next_id: u64,
    bandwidth: BandwidthTracker,
    blocked_domains: Vec<String>,
    blocked_count: u64,
    domain_stats: HashMap<String, DomainStats>,
}

#[derive(Debug, Clone, Default)]
pub struct DomainStats {
    pub request_count: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub blocked_count: u64,
}

impl NetworkMonitor {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            next_id: 1,
            bandwidth: BandwidthTracker::new(),
            blocked_domains: vec![
                "google-analytics.com".to_string(),
                "googletagmanager.com".to_string(),
                "doubleclick.net".to_string(),
                "facebook.com/tr".to_string(),
                "connect.facebook.net".to_string(),
                "analytics.twitter.com".to_string(),
                "bat.bing.com".to_string(),
                "ads.linkedin.com".to_string(),
                "pixel.quantserve.com".to_string(),
                "scorecardresearch.com".to_string(),
                "hotjar.com".to_string(),
                "fullstory.com".to_string(),
                "segment.io".to_string(),
                "mixpanel.com".to_string(),
                "amplitude.com".to_string(),
            ],
            blocked_count: 0,
            domain_stats: HashMap::new(),
        }
    }
    
    pub fn start_connection(&mut self, url: &str, conn_type: ConnectionType) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let domain = extract_domain(url);
        let is_secure = url.starts_with("https://");
        let port = if is_secure { 443 } else { 80 };
        
        let connection = ActiveConnection {
            id,
            url: url.to_string(),
            domain: domain.clone(),
            connection_type: conn_type,
            state: ConnectionState::Connecting,
            started_at: Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
            request_method: "GET".to_string(),
            status_code: None,
            content_type: None,
            is_secure,
            ip_address: None,
            port,
        };
        
        self.connections.insert(id, connection);
        self.domain_stats.entry(domain).or_default().request_count += 1;
        
        id
    }
    
    pub fn update_connection(&mut self, id: u64, bytes_down: u64, bytes_up: u64) {
        if let Some(conn) = self.connections.get_mut(&id) {
            conn.bytes_received += bytes_down;
            conn.bytes_sent += bytes_up;
            
            if bytes_down > 0 {
                conn.state = ConnectionState::Downloading;
            } else if bytes_up > 0 {
                conn.state = ConnectionState::Uploading;
            }
            
            self.bandwidth.record(bytes_down, bytes_up);
            
            if let Some(stats) = self.domain_stats.get_mut(&conn.domain) {
                stats.bytes_received += bytes_down;
                stats.bytes_sent += bytes_up;
            }
        }
    }
    
    pub fn complete_connection(&mut self, id: u64, status_code: u16, content_type: Option<String>) {
        if let Some(conn) = self.connections.get_mut(&id) {
            conn.state = ConnectionState::Complete;
            conn.status_code = Some(status_code);
            conn.content_type = content_type;
        }
    }
    
    pub fn fail_connection(&mut self, id: u64, error: &str) {
        if let Some(conn) = self.connections.get_mut(&id) {
            conn.state = ConnectionState::Failed(error.to_string());
        }
    }
    
    pub fn is_blocked(&mut self, url: &str) -> bool {
        let domain = extract_domain(url);
        
        for blocked in &self.blocked_domains {
            if domain.contains(blocked) || url.contains(blocked) {
                self.blocked_count += 1;
                if let Some(stats) = self.domain_stats.get_mut(&domain) {
                    stats.blocked_count += 1;
                }
                return true;
            }
        }
        
        false
    }
    
    pub fn active_connections(&self) -> Vec<&ActiveConnection> {
        self.connections.values()
            .filter(|c| matches!(c.state, 
                ConnectionState::Connecting | 
                ConnectionState::Uploading | 
                ConnectionState::Downloading))
            .collect()
    }
    
    pub fn all_connections(&self) -> Vec<&ActiveConnection> {
        let mut conns: Vec<_> = self.connections.values().collect();
        conns.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        conns
    }
    
    pub fn active_count(&self) -> usize {
        self.active_connections().len()
    }
    
    pub fn current_speed(&self) -> (f64, f64) {
        self.bandwidth.current_speed()
    }
    
    pub fn total_transferred(&self) -> (u64, u64) {
        self.bandwidth.total()
    }
    
    pub fn blocked_count(&self) -> u64 {
        self.blocked_count
    }
    
    pub fn cleanup_old(&mut self, max_age: Duration) {
        let cutoff = Instant::now() - max_age;
        self.connections.retain(|_, c| {
            c.started_at > cutoff || 
            matches!(c.state, ConnectionState::Connecting | ConnectionState::Uploading | ConnectionState::Downloading)
        });
    }
}

// ==============================================================================
// ACTIVITY INDICATOR UI STATE
// ==============================================================================

#[derive(Debug, Clone)]
pub struct ActivityIndicatorState {
    pub expanded: bool,
    pub selected_connection: Option<u64>,
    pub filter: ConnectionFilter,
    pub sort_by: ConnectionSort,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionFilter {
    All,
    Active,
    Documents,
    Downloads,
    Scripts,
    Images,
    Xhr,
    Blocked,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionSort {
    Newest,
    Oldest,
    Largest,
    Slowest,
    Domain,
}

impl Default for ActivityIndicatorState {
    fn default() -> Self {
        Self {
            expanded: false,
            selected_connection: None,
            filter: ConnectionFilter::All,
            sort_by: ConnectionSort::Newest,
        }
    }
}

// ==============================================================================
// HELPERS
// ==============================================================================

fn extract_domain(url: &str) -> String {
    let url = url.trim_start_matches("https://")
        .trim_start_matches("http://");
    
    url.split('/').next()
        .unwrap_or(url)
        .split(':').next()
        .unwrap_or(url)
        .to_string()
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn format_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec < 1024.0 {
        format!("{:.0} B/s", bytes_per_sec)
    } else if bytes_per_sec < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0))
    }
}

pub fn format_duration(dur: Duration) -> String {
    let secs = dur.as_secs();
    if secs < 1 {
        format!("{}ms", dur.as_millis())
    } else if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

// ==============================================================================
// TESTS
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_monitor() {
        let mut monitor = NetworkMonitor::new();
        
        let id = monitor.start_connection("https://example.com/page.html", ConnectionType::Document);
        assert_eq!(monitor.active_count(), 1);
        
        monitor.update_connection(id, 1024, 0);
        monitor.complete_connection(id, 200, Some("text/html".to_string()));
        
        assert_eq!(monitor.active_count(), 0);
    }
    
    #[test]
    fn test_blocked_domains() {
        let mut monitor = NetworkMonitor::new();
        
        assert!(monitor.is_blocked("https://google-analytics.com/collect"));
        assert!(!monitor.is_blocked("https://example.com"));
    }
    
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1500), "1.5 KB");
        assert_eq!(format_bytes(1_500_000), "1.4 MB");
    }
}
