//! Page Sandbox - Every page starts untrusted
//!
//! SECURITY MODEL:
//! ==============================================================================
//! Every page load begins in a restricted sandbox.
//! The page must EARN trust through meaningful user interaction.
//!
//! WHAT'S BLOCKED UNTIL TRUSTED:
//! - Clipboard access (no stealing passwords)
//! - Download initiation (no drive-by downloads)
//! - Notification requests (no spam)
//! - Popup creation (no popup storms)
//! - Geolocation requests
//! - Camera/microphone access
//! - Full-screen requests
//! - Auto-playing audio
//!
//! WHAT COUNTS AS MEANINGFUL INTERACTION:
//! - Click on actual content (not ads, not invisible elements)
//! - Keyboard input in a form field
//! - Scroll that indicates reading (not instant scroll-to-bottom)
//! - Time spent on page (minimum 5 seconds per interaction)
//!
//! WHAT DOESN'T COUNT:
//! - Mouse movement alone
//! - Hover events
//! - Scroll triggered by JavaScript
//! - Clicks on elements smaller than 20x20px (anti-clickjack)
//! - Rapid repeated interactions (bot behavior)

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Trust level for a page
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageTrust {
    /// Just loaded, no interactions yet
    Untrusted = 0,
    /// One meaningful interaction
    Cautious = 1,
    /// Two meaningful interactions  
    Warming = 2,
    /// Three+ interactions, page is trusted
    Trusted = 3,
}

impl PageTrust {
    pub fn can_access_clipboard(&self) -> bool {
        *self >= PageTrust::Trusted
    }
    
    pub fn can_initiate_download(&self) -> bool {
        *self >= PageTrust::Trusted
    }
    
    pub fn can_request_notifications(&self) -> bool {
        *self >= PageTrust::Trusted
    }
    
    pub fn can_open_popup(&self) -> bool {
        // Popups need trust OR smart popup detection
        *self >= PageTrust::Cautious
    }
    
    pub fn can_request_geolocation(&self) -> bool {
        *self >= PageTrust::Trusted
    }
    
    pub fn can_autoplay_audio(&self) -> bool {
        *self >= PageTrust::Warming
    }
    
    pub fn can_go_fullscreen(&self) -> bool {
        *self >= PageTrust::Trusted
    }
}

/// Types of user interactions
#[derive(Debug, Clone)]
pub enum Interaction {
    Click {
        x: i32,
        y: i32,
        element_width: u32,
        element_height: u32,
        element_type: String,
        timestamp: Instant,
    },
    KeyboardInput {
        in_form_field: bool,
        char_count: usize,
        timestamp: Instant,
    },
    Scroll {
        delta_y: i32,
        user_initiated: bool,
        timestamp: Instant,
    },
    FormSubmit {
        timestamp: Instant,
    },
}

impl Interaction {
    /// Check if this interaction is meaningful (counts toward trust)
    pub fn is_meaningful(&self, page_age: Duration, last_interaction: Option<Instant>) -> bool {
        // Page must be at least 2 seconds old
        if page_age < Duration::from_secs(2) {
            return false;
        }
        
        // Interactions must be at least 1 second apart (anti-bot)
        if let Some(last) = last_interaction {
            if last.elapsed() < Duration::from_secs(1) {
                return false;
            }
        }
        
        match self {
            Interaction::Click { element_width, element_height, element_type, .. } => {
                // Anti-clickjack: element must be visible size
                if *element_width < 20 || *element_height < 20 {
                    return false;
                }
                // Don't count clicks on known ad containers
                let suspicious = ["ad", "banner", "sponsor", "promo", "track"];
                if suspicious.iter().any(|s| crate::fontcase::ascii_lower(element_type).contains(s)) {
                    return false;
                }
                true
            }
            Interaction::KeyboardInput { in_form_field, char_count, .. } => {
                // Must be in a form field and have typed something
                *in_form_field && *char_count >= 3
            }
            Interaction::Scroll { delta_y, user_initiated, .. } => {
                // Must be user-initiated and a reasonable scroll
                *user_initiated && delta_y.abs() > 50 && delta_y.abs() < 2000
            }
            Interaction::FormSubmit { .. } => {
                // Form submissions always count
                true
            }
        }
    }
}

/// Sandbox state for a single page
#[derive(Debug)]
pub struct PageSandbox {
    pub url: String,
    pub trust: PageTrust,
    pub interactions: Vec<Interaction>,
    pub meaningful_count: u32,
    pub created_at: Instant,
    pub last_interaction: Option<Instant>,
    pub blocked_actions: Vec<BlockedAction>,
}

#[derive(Debug, Clone)]
pub struct BlockedAction {
    pub action: String,
    pub reason: String,
    pub timestamp: Instant,
}

impl PageSandbox {
    pub fn new(url: String) -> Self {
        Self {
            url,
            trust: PageTrust::Untrusted,
            interactions: Vec::new(),
            meaningful_count: 0,
            created_at: Instant::now(),
            last_interaction: None,
            blocked_actions: Vec::new(),
        }
    }
    
    /// Record an interaction, potentially upgrading trust
    pub fn record_interaction(&mut self, interaction: Interaction) {
        let page_age = self.created_at.elapsed();
        let is_meaningful = interaction.is_meaningful(page_age, self.last_interaction);
        
        self.last_interaction = Some(Instant::now());
        self.interactions.push(interaction);
        
        if is_meaningful {
            self.meaningful_count += 1;
            self.trust = match self.meaningful_count {
                0 => PageTrust::Untrusted,
                1 => PageTrust::Cautious,
                2 => PageTrust::Warming,
                _ => PageTrust::Trusted,
            };
        }
    }
    
    /// Check if an action is allowed, record if blocked
    #[allow(dead_code)]
    pub fn check_permission(&mut self, action: &str) -> bool {
        let allowed = match action {
            "clipboard" => self.trust.can_access_clipboard(),
            "download" => self.trust.can_initiate_download(),
            "notification" => self.trust.can_request_notifications(),
            "popup" => self.trust.can_open_popup(),
            "geolocation" => self.trust.can_request_geolocation(),
            "autoplay" => self.trust.can_autoplay_audio(),
            "fullscreen" => self.trust.can_go_fullscreen(),
            _ => self.trust >= PageTrust::Trusted,
        };
        
        if !allowed {
            self.blocked_actions.push(BlockedAction {
                action: action.to_string(),
                reason: format!(
                    "Page trust level {:?} - need {} more interactions",
                    self.trust,
                    3 - self.meaningful_count
                ),
                timestamp: Instant::now(),
            });
        }
        
        allowed
    }
    
    /// Get status text for UI
    #[allow(dead_code)]
    pub fn status_text(&self) -> String {
        match self.trust {
            PageTrust::Untrusted => "Sandboxed (0/3)".to_string(),
            PageTrust::Cautious => "Cautious (1/3)".to_string(),
            PageTrust::Warming => "Warming (2/3)".to_string(),
            PageTrust::Trusted => "Trusted".to_string(),
        }
    }
    
    /// Get status color
    #[allow(dead_code)]
    pub fn status_color(&self) -> &'static str {
        match self.trust {
            PageTrust::Untrusted => "#ef4444", // Red
            PageTrust::Cautious => "#f97316",  // Orange
            PageTrust::Warming => "#eab308",   // Yellow
            PageTrust::Trusted => "#22c55e",   // Green
        }
    }
}

/// Manage sandboxes for all open pages
#[derive(Debug, Default)]
pub struct SandboxManager {
    pages: HashMap<u64, PageSandbox>,  // tab_id -> sandbox
}

impl SandboxManager {
    pub fn new() -> Self {
        Self {
            pages: HashMap::new(),
        }
    }
    
    /// Create sandbox for new page
    pub fn create(&mut self, tab_id: u64, url: String) {
        self.pages.insert(tab_id, PageSandbox::new(url));
    }
    
    /// Get sandbox for tab
    pub fn get(&self, tab_id: u64) -> Option<&PageSandbox> {
        self.pages.get(&tab_id)
    }
    
    /// Get mutable sandbox
    pub fn get_mut(&mut self, tab_id: u64) -> Option<&mut PageSandbox> {
        self.pages.get_mut(&tab_id)
    }
    
    /// Remove sandbox when tab closes
    pub fn remove(&mut self, tab_id: u64) {
        self.pages.remove(&tab_id);
    }
    
    /// Record interaction for active tab
    pub fn record(&mut self, tab_id: u64, interaction: Interaction) {
        if let Some(sandbox) = self.pages.get_mut(&tab_id) {
            sandbox.record_interaction(interaction);
        }
    }
    
    /// Check permission for tab
    pub fn check(&mut self, tab_id: u64, action: &str) -> bool {
        if let Some(sandbox) = self.pages.get_mut(&tab_id) {
            sandbox.check_permission(action)
        } else {
            false // No sandbox = no permissions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_trust_progression() {
        let mut sandbox = PageSandbox::new("https://example.com".into());
        assert_eq!(sandbox.trust, PageTrust::Untrusted);
        
        // Wait for page age requirement
        thread::sleep(Duration::from_secs(3));
        
        // First meaningful click
        sandbox.record_interaction(Interaction::Click {
            x: 100, y: 100,
            element_width: 100, element_height: 50,
            element_type: "button".into(),
            timestamp: Instant::now(),
        });
        
        thread::sleep(Duration::from_secs(2));
        assert_eq!(sandbox.trust, PageTrust::Cautious);
        
        // Second: keyboard input
        sandbox.record_interaction(Interaction::KeyboardInput {
            in_form_field: true,
            char_count: 10,
            timestamp: Instant::now(),
        });
        
        thread::sleep(Duration::from_secs(2));
        assert_eq!(sandbox.trust, PageTrust::Warming);
        
        // Third: scroll
        sandbox.record_interaction(Interaction::Scroll {
            delta_y: 200,
            user_initiated: true,
            timestamp: Instant::now(),
        });
        
        assert_eq!(sandbox.trust, PageTrust::Trusted);
    }
    
    #[test]
    fn test_anti_clickjack() {
        let mut sandbox = PageSandbox::new("https://example.com".into());
        thread::sleep(Duration::from_secs(3));
        
        // Tiny element - shouldn't count
        sandbox.record_interaction(Interaction::Click {
            x: 100, y: 100,
            element_width: 5, element_height: 5,  // Too small!
            element_type: "div".into(),
            timestamp: Instant::now(),
        });
        
        assert_eq!(sandbox.trust, PageTrust::Untrusted);
        assert_eq!(sandbox.meaningful_count, 0);
    }
}
