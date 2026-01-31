//! Network Activity Bar - Always visible indicator
//!
//! Shows when the browser is doing network activity, so users know
//! something is happening even when the page looks static.

 
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Network activity state
#[derive(Debug)]
pub struct NetworkBar {
    /// Active requests (URL -> state)
    pub requests: Vec<NetworkRequest>,
    
    /// Recent activity samples for animation
    pub activity_history: VecDeque<ActivitySample>,
    
    /// Current bytes/sec (smoothed)
    pub bytes_per_sec: f64,
    
    /// Peak bytes/sec (for scaling)
    pub peak_bytes_per_sec: f64,
    
    /// Total bytes transferred this session
    pub total_bytes: u64,
    
    /// Last update time
    pub last_update: Instant,
    
    /// Animation phase (0.0 - 1.0)
    pub animation_phase: f32,
    
    /// Is any request active?
    pub is_active: bool,
    
    /// Show detailed view
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkRequest {
    pub id: u64,
    pub url: String,
    pub host: String,
    pub method: String,
    pub started: Instant,
    pub state: RequestState,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub content_type: Option<String>,
    // Waterfall timing data
    pub timing: RequestTiming,
}

/// Detailed timing for waterfall chart
#[derive(Debug, Clone, Default)]
pub struct RequestTiming {
    /// When DNS resolution started
    pub dns_start: Option<Instant>,
    /// When DNS resolution completed
    pub dns_end: Option<Instant>,
    /// When TCP connection started
    pub connect_start: Option<Instant>,
    /// When TCP connection completed (or TLS handshake completed)
    pub connect_end: Option<Instant>,
    /// When request was sent
    pub send_start: Option<Instant>,
    /// When request finished sending
    pub send_end: Option<Instant>,
    /// When first byte of response received (TTFB)
    pub receive_start: Option<Instant>,
    /// When response fully received
    pub receive_end: Option<Instant>,
}

impl RequestTiming {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get duration for a timing phase (in milliseconds)
    pub fn dns_duration_ms(&self) -> Option<f64> {
        match (self.dns_start, self.dns_end) {
            (Some(start), Some(end)) => Some(end.duration_since(start).as_secs_f64() * 1000.0),
            _ => None,
        }
    }
    
    pub fn connect_duration_ms(&self) -> Option<f64> {
        match (self.connect_start, self.connect_end) {
            (Some(start), Some(end)) => Some(end.duration_since(start).as_secs_f64() * 1000.0),
            _ => None,
        }
    }
    
    pub fn send_duration_ms(&self) -> Option<f64> {
        match (self.send_start, self.send_end) {
            (Some(start), Some(end)) => Some(end.duration_since(start).as_secs_f64() * 1000.0),
            _ => None,
        }
    }
    
    pub fn wait_duration_ms(&self) -> Option<f64> {
        match (self.send_end, self.receive_start) {
            (Some(start), Some(end)) => Some(end.duration_since(start).as_secs_f64() * 1000.0),
            _ => None,
        }
    }
    
    pub fn receive_duration_ms(&self) -> Option<f64> {
        match (self.receive_start, self.receive_end) {
            (Some(start), Some(end)) => Some(end.duration_since(start).as_secs_f64() * 1000.0),
            _ => None,
        }
    }
    
    pub fn total_duration_ms(&self, started: Instant) -> f64 {
        let end = self.receive_end.unwrap_or_else(Instant::now);
        end.duration_since(started).as_secs_f64() * 1000.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequestState {
    Connecting,
    Sending,
    Waiting,      // Waiting for response
    Receiving,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy)]
pub struct ActivitySample {
    pub timestamp: Instant,
    pub bytes: u64,
    pub request_count: usize,
}

impl NetworkBar {
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
            activity_history: VecDeque::with_capacity(60), // 1 second at 60fps
            bytes_per_sec: 0.0,
            peak_bytes_per_sec: 1024.0 * 100.0, // Start with 100KB/s as baseline
            total_bytes: 0,
            last_update: Instant::now(),
            animation_phase: 0.0,
            is_active: false,
            expanded: false,
        }
    }
    
    /// Start tracking a new request
    pub fn start_request(&mut self, id: u64, url: &str, method: &str) {
        let host = extract_host(url).unwrap_or("unknown").to_string();
        let now = Instant::now();
        
        self.requests.push(NetworkRequest {
            id,
            url: url.to_string(),
            host,
            method: method.to_string(),
            started: now,
            state: RequestState::Connecting,
            bytes_downloaded: 0,
            bytes_total: None,
            content_type: None,
            timing: RequestTiming {
                dns_start: Some(now),
                ..RequestTiming::default()
            },
        });
        
        self.is_active = true;
    }
    
    /// Update request state with automatic timing tracking
    pub fn update_request(&mut self, id: u64, state: RequestState) {
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            let now = Instant::now();
            
            // Update timing based on state transition
            match state {
                RequestState::Connecting => {
                    if req.timing.dns_start.is_none() {
                        req.timing.dns_start = Some(now);
                    }
                }
                RequestState::Sending => {
                    if req.timing.dns_end.is_none() {
                        req.timing.dns_end = Some(now);
                    }
                    if req.timing.connect_start.is_none() {
                        req.timing.connect_start = Some(now);
                    }
                    if req.timing.connect_end.is_none() {
                        req.timing.connect_end = Some(now);
                    }
                    if req.timing.send_start.is_none() {
                        req.timing.send_start = Some(now);
                    }
                }
                RequestState::Waiting => {
                    if req.timing.send_end.is_none() {
                        req.timing.send_end = Some(now);
                    }
                }
                RequestState::Receiving => {
                    if req.timing.receive_start.is_none() {
                        req.timing.receive_start = Some(now);
                    }
                }
                RequestState::Complete => {
                    if req.timing.receive_end.is_none() {
                        req.timing.receive_end = Some(now);
                    }
                }
                RequestState::Failed | RequestState::Cancelled => {
                    if req.timing.receive_end.is_none() {
                        req.timing.receive_end = Some(now);
                    }
                }
            }
            
            req.state = state;
        }
    }
    
    /// Set content length for a request
    pub fn set_content_length(&mut self, id: u64, length: u64) {
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            req.bytes_total = Some(length);
        }
    }
    
    /// Set content type
    pub fn set_content_type(&mut self, id: u64, content_type: &str) {
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            req.content_type = Some(content_type.to_string());
        }
    }
    
    /// Record bytes received
    pub fn add_bytes(&mut self, id: u64, bytes: u64) {
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            req.bytes_downloaded += bytes;
            req.state = RequestState::Receiving;
        }
        self.total_bytes += bytes;
        
        // Add to activity history
        self.activity_history.push_back(ActivitySample {
            timestamp: Instant::now(),
            bytes,
            request_count: self.active_count(),
        });
        
        // Trim old samples (keep last 2 seconds)
        let cutoff = Instant::now() - Duration::from_secs(2);
        while let Some(sample) = self.activity_history.front() {
            if sample.timestamp < cutoff {
                self.activity_history.pop_front();
            } else {
                break;
            }
        }
    }
    
    /// Complete a request
    pub fn complete_request(&mut self, id: u64) {
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            req.state = RequestState::Complete;
        }
        self.cleanup_old_requests();
    }
    
    /// Fail a request
    pub fn fail_request(&mut self, id: u64) {
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            req.state = RequestState::Failed;
        }
        self.cleanup_old_requests();
    }
    
    /// Cancel a request
    pub fn cancel_request(&mut self, id: u64) {
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            req.state = RequestState::Cancelled;
        }
        self.cleanup_old_requests();
    }
    
    /// Remove completed/failed requests older than 2 seconds
    fn cleanup_old_requests(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(2);
        self.requests.retain(|r| {
            match r.state {
                RequestState::Complete | RequestState::Failed | RequestState::Cancelled => {
                    r.started > cutoff
                }
                _ => true,
            }
        });
        
        self.is_active = self.active_count() > 0;
    }
    
    /// Get count of active (not complete/failed) requests
    pub fn active_count(&self) -> usize {
        self.requests.iter().filter(|r| {
            matches!(r.state, 
                RequestState::Connecting | 
                RequestState::Sending | 
                RequestState::Waiting | 
                RequestState::Receiving
            )
        }).count()
    }
    
    /// Update animation and stats (call each frame)
    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;
        
        // Update animation
        if self.is_active {
            self.animation_phase = (self.animation_phase + dt * 2.0) % 1.0;
        }
        
        // Calculate bytes/sec over last second
        let one_sec_ago = now - Duration::from_secs(1);
        let recent_bytes: u64 = self.activity_history
            .iter()
            .filter(|s| s.timestamp > one_sec_ago)
            .map(|s| s.bytes)
            .sum();
        
        // Smooth the value
        let target = recent_bytes as f64;
        self.bytes_per_sec = self.bytes_per_sec * 0.9 + target * 0.1;
        
        // Update peak
        if self.bytes_per_sec > self.peak_bytes_per_sec {
            self.peak_bytes_per_sec = self.bytes_per_sec;
        } else {
            // Slowly decay peak
            self.peak_bytes_per_sec *= 0.999;
            if self.peak_bytes_per_sec < 1024.0 * 10.0 {
                self.peak_bytes_per_sec = 1024.0 * 10.0; // Min 10KB/s baseline
            }
        }
        
        // Cleanup
        self.cleanup_old_requests();
    }
    
    /// Get activity level (0.0 - 1.0) for visualization
    pub fn activity_level(&self) -> f32 {
        if !self.is_active {
            return 0.0;
        }
        
        let ratio = self.bytes_per_sec / self.peak_bytes_per_sec;
        (ratio as f32).clamp(0.1, 1.0) // At least 10% when active
    }
    
    /// Get status text for display
    pub fn status_text(&self) -> String {
        let active = self.active_count();
        
        if active == 0 {
            return "Idle".to_string();
        }
        
        let speed = format_bytes_per_sec(self.bytes_per_sec);
        
        if active == 1 {
            if let Some(req) = self.requests.iter().find(|r| {
                matches!(r.state, RequestState::Connecting | RequestState::Sending | 
                         RequestState::Waiting | RequestState::Receiving)
            }) {
                let progress = req.bytes_total.map(|total| {
                    if total > 0 {
                        format!(" ({:.0}%)", (req.bytes_downloaded as f64 / total as f64) * 100.0)
                    } else {
                        String::new()
                    }
                }).unwrap_or_default();
                
                return format!("{}: {}{} - {}", 
                    req.host,
                    state_text(req.state),
                    progress,
                    speed
                );
            }
        }
        
        format!("{} requests - {}", active, speed)
    }
    
    /// Get requests for expanded view
    pub fn visible_requests(&self) -> Vec<&NetworkRequest> {
        self.requests.iter()
            .filter(|r| !matches!(r.state, RequestState::Complete | RequestState::Failed | RequestState::Cancelled))
            .take(5)
            .collect()
    }
    
    /// Toggle expanded view
    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }
    
    /// Get hover tooltip text showing all active connections
    pub fn hover_tooltip(&self) -> Vec<String> {
        let mut lines = Vec::new();
        
        if self.requests.is_empty() {
            lines.push("No active connections".to_string());
            lines.push(format!("Session total: {}", format_bytes(self.total_bytes)));
            return lines;
        }
        
        lines.push(format!("Active Connections ({}):", self.active_count()));
        lines.push(String::new());
        
        for req in &self.requests {
            let state_icon = match req.state {
                RequestState::Connecting => "",
                RequestState::Sending => "",
                RequestState::Waiting => "...",
                RequestState::Receiving => "",
                RequestState::Complete => "[OK]",
                RequestState::Failed => "[X]",
                RequestState::Cancelled => "",
            };
            
            let progress = req.bytes_total.map(|total| {
                if total > 0 {
                    format!(" {}/{}", 
                        format_bytes(req.bytes_downloaded),
                        format_bytes(total))
                } else {
                    format!(" {}", format_bytes(req.bytes_downloaded))
                }
            }).unwrap_or_else(|| format!(" {}", format_bytes(req.bytes_downloaded)));
            
            let duration = req.started.elapsed().as_millis();
            
            lines.push(format!("{} {} {}{}", 
                state_icon, 
                truncate_host(&req.host, 30),
                progress,
                if duration > 1000 { format!(" ({:.1}s)", duration as f64 / 1000.0) } else { String::new() }
            ));
            
            // Show full URL on second line if different from host
            if req.url.len() > req.host.len() + 10 {
                lines.push(format!("   +- {}", truncate_url(&req.url, 50)));
            }
        }
        
        lines.push(String::new());
        lines.push(format!("Speed: {}", format_bytes_per_sec(self.bytes_per_sec)));
        lines.push(format!("Session: {}", format_bytes(self.total_bytes)));
        
        lines
    }
    
    /// Check if mouse is hovering over network bar area
    pub fn is_hovered(&self, mouse_x: i32, mouse_y: i32, bar_x: i32, bar_y: i32, bar_w: i32, bar_h: i32) -> bool {
        mouse_x >= bar_x && mouse_x <= bar_x + bar_w &&
        mouse_y >= bar_y && mouse_y <= bar_y + bar_h
    }
}

impl Default for NetworkBar {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn truncate_host(host: &str, max: usize) -> String {
    if host.len() <= max {
        host.to_string()
    } else {
        format!("{}...", &host[..max-3])
    }
}

pub fn truncate_url(url: &str, max: usize) -> String {
    if url.len() <= max {
        url.to_string()
    } else {
        // Try to keep the path visible
        let trimmed = url.strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        if trimmed.len() <= max {
            trimmed.to_string()
        } else {
            format!("{}...", &trimmed[..max-3])
        }
    }
}

pub fn state_text(state: RequestState) -> &'static str {
    match state {
        RequestState::Connecting => "Connecting",
        RequestState::Sending => "Sending",
        RequestState::Waiting => "Waiting",
        RequestState::Receiving => "Receiving",
        RequestState::Complete => "Done",
        RequestState::Failed => "Failed",
        RequestState::Cancelled => "Cancelled",
    }
}

pub fn format_bytes_per_sec(bps: f64) -> String {
    if bps < 1024.0 {
        format!("{:.0} B/s", bps)
    } else if bps < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.2} MB/s", bps / (1024.0 * 1024.0))
    }
}

fn extract_host(url: &str) -> Option<&str> {
    let url = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    url.split('/').next()
}

/// Colors for the network bar
pub struct NetworkBarColors {
    pub background: u32,
    pub idle: u32,
    pub active: u32,
    pub receiving: u32,
    pub text: u32,
    pub text_dim: u32,
}

impl NetworkBarColors {
    pub fn dark() -> Self {
        Self {
            background: 0xff1a1a1a,
            idle: 0xff3a3a3a,
            active: 0xff4a9eff,
            receiving: 0xff4aff9e,
            text: 0xffe0e0e0,
            text_dim: 0xff808080,
        }
    }
    
    pub fn light() -> Self {
        Self {
            background: 0xfff0f0f0,
            idle: 0xffd0d0d0,
            active: 0xff2080ff,
            receiving: 0xff20c060,
            text: 0xff202020,
            text_dim: 0xff808080,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_and_truncate_helpers() {
        assert_eq!(format_bytes(512), "512 B");
        assert!(format_bytes(1024 * 1024).contains("MB"));

        let long = "https://sub.example.com/path/to/resource?query=1";
        let host = truncate_host(long, 10);
        assert!(host.len() <= 10 + 3);

        let short_url = truncate_url(long, 20);
        assert!(short_url.len() <= 23);

        assert_eq!(state_text(RequestState::Connecting), "Connecting");
    }
}
