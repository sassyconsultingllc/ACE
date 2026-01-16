//! Network Activity Indicator
//!
//! Always-visible network status bar showing:
//! - Current state (idle, loading, uploading)
//! - Active connections
//! - Transfer speed
//! - Request queue depth
//!
//! Like XFCE network applet - you always know when
//! something is happening, even if it's not visible in the page.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Network state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkState {
    Idle,
    Connecting,
    Downloading,
    Uploading,
    Error,
}

/// A single network request being tracked
#[derive(Debug, Clone)]
pub struct TrackedRequest {
    pub id: u64,
    pub url: String,
    pub method: String,
    pub started_at: Instant,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub state: RequestState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestState {
    Pending,
    Connecting,
    Sending,
    Receiving,
    Complete,
    Failed,
}

/// Network activity monitor
#[derive(Debug)]
pub struct NetworkMonitor {
    state: NetworkState,
    requests: Vec<TrackedRequest>,
    next_id: u64,
    
    // Stats
    total_bytes_down: u64,
    total_bytes_up: u64,
    requests_completed: u64,
    requests_failed: u64,
    
    // Speed calculation (rolling window)
    speed_samples: VecDeque<SpeedSample>,
    last_sample_time: Instant,
    
    // Error tracking
    last_error: Option<String>,
    error_time: Option<Instant>,
}

#[derive(Debug, Clone)]
struct SpeedSample {
    timestamp: Instant,
    bytes_down: u64,
    bytes_up: u64,
}

impl NetworkMonitor {
    pub fn new() -> Self {
        Self {
            state: NetworkState::Idle,
            requests: Vec::new(),
            next_id: 1,
            total_bytes_down: 0,
            total_bytes_up: 0,
            requests_completed: 0,
            requests_failed: 0,
            speed_samples: VecDeque::new(),
            last_sample_time: Instant::now(),
            last_error: None,
            error_time: None,
        }
    }
    
    /// Start tracking a new request
    pub fn start_request(&mut self, url: &str, method: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        self.requests.push(TrackedRequest {
            id,
            url: url.to_string(),
            method: method.to_string(),
            started_at: Instant::now(),
            bytes_received: 0,
            bytes_sent: 0,
            state: RequestState::Pending,
        });
        
        self.update_state();
        id
    }
    
    /// Update request state
    pub fn update_request(&mut self, id: u64, state: RequestState) {
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            req.state = state;
        }
        self.update_state();
    }
    
    /// Update bytes received
    pub fn bytes_received(&mut self, id: u64, bytes: u64) {
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            req.bytes_received += bytes;
            self.total_bytes_down += bytes;
        }
        self.record_sample();
        self.update_state();
    }
    
    /// Update bytes sent
    pub fn bytes_sent(&mut self, id: u64, bytes: u64) {
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            req.bytes_sent += bytes;
            self.total_bytes_up += bytes;
        }
        self.record_sample();
        self.update_state();
    }
    
    /// Complete a request
    pub fn complete_request(&mut self, id: u64, success: bool) {
        if let Some(pos) = self.requests.iter().position(|r| r.id == id) {
            let req = self.requests.remove(pos);
            if success {
                self.requests_completed += 1;
            } else {
                self.requests_failed += 1;
                self.last_error = Some(format!("Failed: {}", req.url));
                self.error_time = Some(Instant::now());
            }
        }
        self.update_state();
    }
    
    /// Record an error
    pub fn record_error(&mut self, error: &str) {
        self.last_error = Some(error.to_string());
        self.error_time = Some(Instant::now());
        self.state = NetworkState::Error;
    }
    
    /// Get current state
    pub fn state(&self) -> NetworkState {
        self.state
    }
    
    /// Get active request count
    pub fn active_count(&self) -> usize {
        self.requests.iter()
            .filter(|r| !matches!(r.state, RequestState::Complete | RequestState::Failed))
            .count()
    }
    
    /// Get download speed (bytes/sec, averaged over last 3 seconds)
    pub fn download_speed(&self) -> u64 {
        self.calculate_speed(|s| s.bytes_down)
    }
    
    /// Get upload speed (bytes/sec, averaged over last 3 seconds)
    pub fn upload_speed(&self) -> u64 {
        self.calculate_speed(|s| s.bytes_up)
    }
    
    fn calculate_speed<F: Fn(&SpeedSample) -> u64>(&self, get_bytes: F) -> u64 {
        if self.speed_samples.len() < 2 {
            return 0;
        }
        
        let first = self.speed_samples.front().unwrap();
        let last = self.speed_samples.back().unwrap();
        
        let duration = last.timestamp.duration_since(first.timestamp);
        if duration.as_secs_f64() < 0.1 {
            return 0;
        }
        
        let bytes = get_bytes(last).saturating_sub(get_bytes(first));
        (bytes as f64 / duration.as_secs_f64()) as u64
    }
    
    fn record_sample(&mut self) {
        let now = Instant::now();
        
        // Sample at most once per 100ms
        if now.duration_since(self.last_sample_time) < Duration::from_millis(100) {
            return;
        }
        
        self.speed_samples.push_back(SpeedSample {
            timestamp: now,
            bytes_down: self.total_bytes_down,
            bytes_up: self.total_bytes_up,
        });
        
        // Keep last 3 seconds of samples
        let cutoff = now - Duration::from_secs(3);
        while self.speed_samples.front()
            .map(|s| s.timestamp < cutoff)
            .unwrap_or(false)
        {
            self.speed_samples.pop_front();
        }
        
        self.last_sample_time = now;
    }
    
    fn update_state(&mut self) {
        // Clear error state after 5 seconds
        if let Some(error_time) = self.error_time {
            if error_time.elapsed() > Duration::from_secs(5) {
                self.last_error = None;
                self.error_time = None;
            }
        }
        
        // Determine state from active requests
        if self.last_error.is_some() {
            self.state = NetworkState::Error;
            return;
        }
        
        let has_sending = self.requests.iter()
            .any(|r| r.state == RequestState::Sending);
        let has_receiving = self.requests.iter()
            .any(|r| r.state == RequestState::Receiving);
        let has_connecting = self.requests.iter()
            .any(|r| r.state == RequestState::Connecting || r.state == RequestState::Pending);
        
        self.state = if has_sending {
            NetworkState::Uploading
        } else if has_receiving {
            NetworkState::Downloading
        } else if has_connecting {
            NetworkState::Connecting
        } else {
            NetworkState::Idle
        };
    }
    
    /// Get status for display
    pub fn status_text(&self) -> String {
        match self.state {
            NetworkState::Idle => "Idle".to_string(),
            NetworkState::Connecting => {
                let count = self.active_count();
                if count > 1 {
                    format!("Connecting ({})", count)
                } else {
                    "Connecting...".to_string()
                }
            }
            NetworkState::Downloading => {
                let speed = self.download_speed();
                format_speed(speed, "Downloading")
            }
            NetworkState::Uploading => {
                let speed = self.upload_speed();
                format_speed(speed, "Uploading")
            }
            NetworkState::Error => {
                self.last_error.clone().unwrap_or_else(|| "Error".to_string())
            }
        }
    }
    
    /// Get status color
    pub fn status_color(&self) -> &'static str {
        match self.state {
            NetworkState::Idle => "#64748b",       // Gray
            NetworkState::Connecting => "#f97316", // Orange
            NetworkState::Downloading => "#22c55e",// Green
            NetworkState::Uploading => "#3b82f6",  // Blue
            NetworkState::Error => "#ef4444",      // Red
        }
    }
    
    /// Get icon state (for animation)
    pub fn icon_state(&self) -> IconState {
        match self.state {
            NetworkState::Idle => IconState::Static,
            NetworkState::Connecting => IconState::Pulse,
            NetworkState::Downloading => IconState::ArrowDown,
            NetworkState::Uploading => IconState::ArrowUp,
            NetworkState::Error => IconState::Warning,
        }
    }
    
    /// Get active requests for detail view
    pub fn active_requests(&self) -> &[TrackedRequest] {
        &self.requests
    }
    
    /// Get statistics
    pub fn stats(&self) -> NetworkStats {
        NetworkStats {
            total_bytes_down: self.total_bytes_down,
            total_bytes_up: self.total_bytes_up,
            requests_completed: self.requests_completed,
            requests_failed: self.requests_failed,
            active_requests: self.active_count(),
        }
    }
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Static,
    Pulse,
    ArrowDown,
    ArrowUp,
    Warning,
}

#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub total_bytes_down: u64,
    pub total_bytes_up: u64,
    pub requests_completed: u64,
    pub requests_failed: u64,
    pub active_requests: usize,
}

/// Format speed for display
fn format_speed(bytes_per_sec: u64, prefix: &str) -> String {
    if bytes_per_sec < 1024 {
        format!("{} {} B/s", prefix, bytes_per_sec)
    } else if bytes_per_sec < 1024 * 1024 {
        format!("{} {:.1} KB/s", prefix, bytes_per_sec as f64 / 1024.0)
    } else {
        format!("{} {:.1} MB/s", prefix, bytes_per_sec as f64 / (1024.0 * 1024.0))
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
