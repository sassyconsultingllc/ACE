//! Network Activity Monitor
//!
//! Always-visible indicator showing network state.
//! Like XFCE panel - you always know when something's happening.
//!
//! States:
//! - Idle: No network activity
//! - Connecting: Establishing connection

 
//! - Downloading: Receiving data
//! - Uploading: Sending data (form submit, etc)
//! - Stalled: Connection open but no data
//! - Error: Connection failed

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Network activity state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkState {
    Idle,
    Connecting,
    Downloading,
    Uploading,
    Stalled,
    Error,
}

impl NetworkState {
    pub fn icon(&self) -> &'static str {
        match self {
            NetworkState::Idle => "o",        // Empty circle
            NetworkState::Connecting => "~",  // Wave
            NetworkState::Downloading => "v", // Down arrow
            NetworkState::Uploading => "^",   // Up arrow
            NetworkState::Stalled => "!",     // Warning
            NetworkState::Error => "x",       // Error
        }
    }
    
    pub fn color(&self) -> u32 {
        match self {
            NetworkState::Idle => 0x6b7280,       // Gray
            NetworkState::Connecting => 0x3b82f6, // Blue
            NetworkState::Downloading => 0x22c55e,// Green
            NetworkState::Uploading => 0x8b5cf6,  // Purple
            NetworkState::Stalled => 0xf59e0b,    // Orange
            NetworkState::Error => 0xef4444,      // Red
        }
    }
    
    pub fn is_active(&self) -> bool {
        !matches!(self, NetworkState::Idle)
    }
}

/// Single network request tracking
#[derive(Debug, Clone)]
pub struct NetworkRequest {
    pub id: u64,
    pub url: String,
    pub method: String,
    pub state: NetworkState,
    pub started_at: Instant,
    pub bytes_downloaded: u64,
    pub bytes_uploaded: u64,
    pub total_size: Option<u64>,
    pub last_activity: Instant,
}

impl NetworkRequest {
    pub fn new(id: u64, url: String, method: String) -> Self {
        let now = Instant::now();
        Self {
            id,
            url,
            method,
            state: NetworkState::Connecting,
            started_at: now,
            bytes_downloaded: 0,
            bytes_uploaded: 0,
            total_size: None,
            last_activity: now,
        }
    }
    
    /// Progress as percentage (0-100)
    pub fn progress(&self) -> Option<u8> {
        self.total_size.map(|total| {
            if total == 0 { 100 } 
            else { ((self.bytes_downloaded * 100) / total).min(100) as u8 }
        })
    }
    
    /// Duration since start
    pub fn duration(&self) -> Duration {
        self.started_at.elapsed()
    }
    
    /// Check if stalled (no activity for 5 seconds)
    pub fn is_stalled(&self) -> bool {
        self.state != NetworkState::Idle && 
        self.state != NetworkState::Error &&
        self.last_activity.elapsed() > Duration::from_secs(5)
    }
}

/// Network monitor - tracks all active requests
#[derive(Debug)]
pub struct NetworkMonitor {
    requests: HashMap<u64, NetworkRequest>,
    next_id: u64,
    history: Vec<NetworkRequest>,
    max_history: usize,
    total_downloaded: u64,
    total_uploaded: u64,
    session_start: Instant,
}

impl NetworkMonitor {
    pub fn new() -> Self {
        Self {
            requests: HashMap::new(),
            next_id: 1,
            history: Vec::new(),
            max_history: 100,
            total_downloaded: 0,
            total_uploaded: 0,
            session_start: Instant::now(),
        }
    }
    
    /// Start tracking a new request
    pub fn start_request(&mut self, url: String, method: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        self.requests.insert(id, NetworkRequest::new(id, url, method));
        id
    }
    
    /// Update download progress
    pub fn update_download(&mut self, id: u64, bytes: u64, total: Option<u64>) {
        if let Some(req) = self.requests.get_mut(&id) {
            req.bytes_downloaded = bytes;
            req.total_size = total;
            req.state = NetworkState::Downloading;
            req.last_activity = Instant::now();
        }
    }
    
    /// Update upload progress
    pub fn update_upload(&mut self, id: u64, bytes: u64) {
        if let Some(req) = self.requests.get_mut(&id) {
            req.bytes_uploaded = bytes;
            req.state = NetworkState::Uploading;
            req.last_activity = Instant::now();
        }
    }
    
    /// Mark request complete
    pub fn complete(&mut self, id: u64) {
        if let Some(mut req) = self.requests.remove(&id) {
            req.state = NetworkState::Idle;
            self.total_downloaded += req.bytes_downloaded;
            self.total_uploaded += req.bytes_uploaded;
            
            // Add to history
            self.history.push(req);
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }
    }
    
    /// Mark request failed
    pub fn error(&mut self, id: u64, _error: &str) {
        if let Some(mut req) = self.requests.remove(&id) {
            req.state = NetworkState::Error;
            self.history.push(req);
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }
    }
    
    /// Get overall network state
    pub fn state(&self) -> NetworkState {
        if self.requests.is_empty() {
            return NetworkState::Idle;
        }
        
        // Check for errors first
        if self.requests.values().any(|r| r.state == NetworkState::Error) {
            return NetworkState::Error;
        }
        
        // Check for stalled
        if self.requests.values().any(|r| r.is_stalled()) {
            return NetworkState::Stalled;
        }
        
        // Check for uploads
        if self.requests.values().any(|r| r.state == NetworkState::Uploading) {
            return NetworkState::Uploading;
        }
        
        // Check for downloads
        if self.requests.values().any(|r| r.state == NetworkState::Downloading) {
            return NetworkState::Downloading;
        }
        
        // Must be connecting
        NetworkState::Connecting
    }
    
    /// Get active request count
    pub fn active_count(&self) -> usize {
        self.requests.len()
    }
    
    /// Get total bytes for session
    pub fn session_stats(&self) -> (u64, u64) {
        let active_down: u64 = self.requests.values().map(|r| r.bytes_downloaded).sum();
        let active_up: u64 = self.requests.values().map(|r| r.bytes_uploaded).sum();
        
        (self.total_downloaded + active_down, self.total_uploaded + active_up)
    }
    
    /// Get active requests for display
    pub fn active_requests(&self) -> Vec<&NetworkRequest> {
        self.requests.values().collect()
    }
    
    /// Check for stalled requests and update state
    pub fn tick(&mut self) {
        for req in self.requests.values_mut() {
            if req.is_stalled() && req.state != NetworkState::Stalled {
                req.state = NetworkState::Stalled;
            }
        }
    }
    
    /// Format bytes for display
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
    
    /// Format speed (bytes per second)
    pub fn format_speed(bps: u64) -> String {
        if bps < 1024 {
            format!("{} B/s", bps)
        } else if bps < 1024 * 1024 {
            format!("{:.1} KB/s", bps as f64 / 1024.0)
        } else {
            format!("{:.1} MB/s", bps as f64 / (1024.0 * 1024.0))
        }
    }
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe wrapper for use across async boundaries
pub type SharedNetworkMonitor = Arc<Mutex<NetworkMonitor>>;

pub fn shared_monitor() -> SharedNetworkMonitor {
    Arc::new(Mutex::new(NetworkMonitor::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_request_lifecycle() {
        let mut monitor = NetworkMonitor::new();
        
        let id = monitor.start_request("https://example.com".into(), "GET".into());
        assert_eq!(monitor.state(), NetworkState::Connecting);
        assert_eq!(monitor.active_count(), 1);
        
        monitor.update_download(id, 1000, Some(5000));
        assert_eq!(monitor.state(), NetworkState::Downloading);
        
        monitor.complete(id);
        assert_eq!(monitor.state(), NetworkState::Idle);
        assert_eq!(monitor.active_count(), 0);
    }
    
    #[test]
    fn test_format_bytes() {
        assert_eq!(NetworkMonitor::format_bytes(500), "500 B");
        assert_eq!(NetworkMonitor::format_bytes(1536), "1.5 KB");
        assert_eq!(NetworkMonitor::format_bytes(1048576), "1.0 MB");
    }
}
