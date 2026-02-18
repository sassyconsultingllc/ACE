//! Sandbox Architecture
//!
//! FOUR LAYERS OF PROTECTION:
//!
//! 1. NETWORK SANDBOX (network.rs - not yet implemented)
//!    - All content quarantined in memory
//!    - DNS validated before resolution
//!
//! 2. PAGE SANDBOX (page.rs) [NEW]
//!    - Every page starts untrusted
//!    - 3 meaningful interactions to earn trust
//!    - Blocks: clipboard, downloads, popups, notifications
//!
//! 3. POPUP SANDBOX (popup.rs) [NEW]
//!    - Smart popup handling
//!    - Allows: OAuth, CAPTCHA, payments
//!    - Blocks: spam, drive-bys, ad popups
//!
//! 4. DOWNLOAD QUARANTINE (quarantine.rs)
//!    - Files held in memory vault
//!    - 3 interactions + 5 sec wait to release
//!    - Heuristic scanning
//!
//! WHY THIS MATTERS:
//! ==============================================================================
//! Chrome: "This page wants to send you notifications"
//!         -> User clicks Allow (muscle memory)
//!         -> Spam forever
//!
//! Sassy: Page must EARN the right to even ASK.
//!        3 meaningful interactions first.
//!        Most spam sites never get there.

pub mod quarantine;
pub mod page;
pub mod popup;
pub mod network;

pub use quarantine::{Quarantine, QuarantinedFile, ReleaseStatus, WarningLevel};
pub use page::{PageTrust, SandboxManager, Interaction};
pub use popup::{PopupHandler, PopupRequest, PopupDecision};

pub type Warning = quarantine::Warning;
pub type PageSandbox = page::PageSandbox;
pub type BlockedPopup = popup::BlockedPopup;
pub type NetworkSandbox = network::NetworkSandbox;

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Trust level earned through deliberate user interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    Untrusted = 0,
    Acknowledged = 1,
    Reviewed = 2,
    Approved = 3,
    Established = 4,
}

impl TrustLevel {
    pub fn can_execute(&self) -> bool {
        *self >= TrustLevel::Approved
    }
    
    pub fn can_write_filesystem(&self) -> bool {
        *self >= TrustLevel::Approved
    }
    
    pub fn can_access_network(&self) -> bool {
        *self >= TrustLevel::Established
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            TrustLevel::Untrusted => "Untrusted - Just loaded",
            TrustLevel::Acknowledged => "Acknowledged - 1 interaction",
            TrustLevel::Reviewed => "Reviewed - 2 interactions",
            TrustLevel::Approved => "Approved - 3 interactions",
            TrustLevel::Established => "Established - Trusted",
        }
    }
}

/// Security context for downloads
#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub id: String,
    pub origin: String,
    pub content_type: ContentType,
    pub trust_level: TrustLevel,
    pub interactions: Vec<InteractionRecord>,
    pub violations: Vec<Violation>,
    pub created_at: Instant,
    pub last_interaction: Option<Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    WebPage,
    Script,
    Image,
    Font,
    Download,
    Extension,
}

#[derive(Debug, Clone)]
pub struct InteractionRecord {
    pub action: InteractionType,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy)]
pub enum InteractionType {
    Acknowledge,
    Review,
    Approve,
    Execute,
    Deny,
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub description: String,
    pub severity: ViolationSeverity,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl ContentType {
    /// Classify content by file extension or MIME prefix
    pub fn from_extension(ext: &str) -> Self {
        match crate::fontcase::ascii_lower(ext).as_str() {
            "js" | "mjs" | "ts" => ContentType::Script,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" => ContentType::Image,
            "woff" | "woff2" | "ttf" | "otf" | "eot" => ContentType::Font,
            "crx" | "xpi" => ContentType::Extension,
            _ => ContentType::Download,
        }
    }
}

impl SecurityContext {
    pub fn new(origin: String, content_type: ContentType) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        origin.hash(&mut hasher);
        std::time::SystemTime::now().hash(&mut hasher);
        
        Self {
            id: format!("ctx_{:016x}", hasher.finish()),
            origin,
            content_type,
            trust_level: TrustLevel::Untrusted,
            interactions: Vec::new(),
            violations: Vec::new(),
            created_at: Instant::now(),
            last_interaction: None,
        }
    }
    
    pub fn record_interaction(&mut self, action: InteractionType) {
        let now = Instant::now();
        
        if let Some(last) = self.last_interaction {
            if now.duration_since(last) < Duration::from_millis(500) {
                self.record_violation(
                    "Interactions too rapid",
                    ViolationSeverity::Medium,
                );
                return;
            }
        }
        
        self.interactions.push(InteractionRecord {
            action,
            timestamp: now,
        });
        self.last_interaction = Some(now);
        
        let meaningful = self.interactions.iter()
            .filter(|i| matches!(i.action, 
                InteractionType::Acknowledge | 
                InteractionType::Review | 
                InteractionType::Approve
            ))
            .count();
        
        self.trust_level = match meaningful {
            0 => TrustLevel::Untrusted,
            1 => TrustLevel::Acknowledged,
            2 => TrustLevel::Reviewed,
            3..=9 => TrustLevel::Approved,
            _ => TrustLevel::Established,
        };
    }
    
    pub fn record_violation(&mut self, description: &str, severity: ViolationSeverity) {
        self.violations.push(Violation {
            description: description.to_string(),
            severity,
            timestamp: Instant::now(),
        });
        
        match severity {
            ViolationSeverity::Critical | ViolationSeverity::High => {
                self.trust_level = TrustLevel::Untrusted;
                self.interactions.clear();
            }
            ViolationSeverity::Medium if self.violations.len() >= 3 => {
                self.trust_level = TrustLevel::Untrusted;
                self.interactions.clear();
            }
            _ => {}
        }
    }
    
    pub fn meets_time_requirement(&self) -> bool {
        self.created_at.elapsed() >= Duration::from_secs(5)
    }

    /// Map a user action string to the appropriate interaction type
    pub fn interaction_for(action: &str) -> InteractionType {
        match action {
            "approve" => InteractionType::Approve,
            "review"  => InteractionType::Review,
            "ack"     => InteractionType::Acknowledge,
            "execute" => InteractionType::Execute,
            "deny"    => InteractionType::Deny,
            _         => InteractionType::Acknowledge,
        }
    }

    /// Human-readable summary of this context's current state
    pub fn describe(&self) -> String {
        let time_ok = if self.meets_time_requirement() { "met" } else { "pending" };
        let last_action = self.interactions.last().map(|i| {
            let action_name = match i.action {
                InteractionType::Acknowledge => "ack",
                InteractionType::Review => "review",
                InteractionType::Approve => "approve",
                InteractionType::Execute => "exec",
                InteractionType::Deny => "deny",
            };
            format!("{}@{:?}", action_name, i.timestamp.elapsed())
        }).unwrap_or_default();
        let last_violation = self.violations.last().map(|v| {
            let sev = match v.severity {
                ViolationSeverity::Low => "low",
                ViolationSeverity::Medium => "med",
                ViolationSeverity::High => "high",
                ViolationSeverity::Critical => "crit",
            };
            format!("{}({})@{:?}", v.description, sev, v.timestamp.elapsed())
        }).unwrap_or_default();
        let last_int_ago = self.last_interaction.map(|t| format!("{:?}", t.elapsed())).unwrap_or_default();
        let content = format!("{:?}", self.content_type);
        let age = format!("{:?}", self.created_at.elapsed());
        format!(
            "[{}] origin={} type={} trust={} interactions={} violations={} time_req={} age={} last_int={} last_action={} last_violation={}",
            self.id,
            self.origin,
            content,
            self.trust_level.description(),
            self.interactions.len(),
            self.violations.len(),
            time_ok,
            age,
            last_int_ago,
            last_action,
            last_violation,
        )
    }
}
