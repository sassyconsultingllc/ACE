//! Smart Popup Handling
//!
//! NOT a dumb popup blocker. Smart detection of legitimate vs spam popups.
//!
//! ALLOW (even without full trust):
//! - OAuth flows (accounts.google.com, login.microsoftonline.com, etc.)
//! - CAPTCHA verification (recaptcha, hcaptcha, cloudflare)
//! - Payment processors (stripe, paypal, etc.)
//! - Print dialogs
//! - User-gesture initiated (actual click, not synthetic)
//!
//! BLOCK:
//! - Script-initiated with no user gesture
//! - Popups during page load
//! - Multiple popups in quick succession
//! - Popups to suspicious domains
//! - Popups with deceptive sizes (tiny or huge)
//!
//! SHOW INDICATOR:
//! - Blocked popup count in UI
//! - Click to review and allow specific ones

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Known legitimate popup domains
const OAUTH_DOMAINS: &[&str] = &[
    "accounts.google.com",
    "login.microsoftonline.com",
    "login.live.com",
    "github.com/login/oauth",
    "api.twitter.com/oauth",
    "facebook.com/dialog/oauth",
    "discord.com/oauth2",
    "id.twitch.tv",
    "appleid.apple.com",
    "login.yahoo.com",
    "auth.atlassian.com",
    "slack.com/oauth",
];

const CAPTCHA_DOMAINS: &[&str] = &[
    "google.com/recaptcha",
    "recaptcha.net",
    "hcaptcha.com",
    "challenges.cloudflare.com",
    "captcha", // Generic pattern
    "verify",  // Generic pattern
];

const PAYMENT_DOMAINS: &[&str] = &[
    "checkout.stripe.com",
    "paypal.com",
    "pay.google.com",
    "apple.com/payment",
    "checkout.shopify.com",
    "secure.authorize.net",
];

/// Popup request with context
#[derive(Debug, Clone)]
pub struct PopupRequest {
    pub source_url: String,
    pub target_url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub user_gesture: bool,
    pub timestamp: Instant,
}

/// Result of popup evaluation
#[derive(Debug, Clone)]
pub enum PopupDecision {
    Allow { reason: &'static str },
    Block { reason: &'static str },
    Prompt { reason: &'static str }, // Ask user
}

/// Blocked popup record
#[derive(Debug, Clone)]
pub struct BlockedPopup {
    pub request: PopupRequest,
    pub reason: String,
    pub timestamp: Instant,
}

/// Smart popup handler
#[derive(Debug)]
pub struct PopupHandler {
    /// Recent popup attempts (for rate limiting)
    recent_attempts: VecDeque<Instant>,
    /// Blocked popups (user can review)
    blocked: Vec<BlockedPopup>,
    /// User-allowed domains for this session
    session_allowed: Vec<String>,
    /// Page load timestamp (block popups during load)
    page_loaded_at: Option<Instant>,
}

impl PopupHandler {
    pub fn new() -> Self {
        Self {
            recent_attempts: VecDeque::new(),
            blocked: Vec::new(),
            session_allowed: Vec::new(),
            page_loaded_at: None,
        }
    }

    /// Called when page starts loading
    pub fn page_loading(&mut self) {
        self.page_loaded_at = None;
    }

    /// Called when page finishes loading
    pub fn page_loaded(&mut self) {
        self.page_loaded_at = Some(Instant::now());
    }

    /// Evaluate a popup request
    pub fn evaluate(&mut self, request: &PopupRequest) -> PopupDecision {
        self.recent_attempts.push_back(request.timestamp);

        // Clean old attempts (keep last 30 seconds)
        let cutoff = Instant::now() - Duration::from_secs(30);
        while self
            .recent_attempts
            .front()
            .map(|t| *t < cutoff)
            .unwrap_or(false)
        {
            self.recent_attempts.pop_front();
        }

        // Check: Too many popups?
        if self.recent_attempts.len() > 5 {
            return PopupDecision::Block {
                reason: "Too many popup attempts",
            };
        }

        // Check: During page load?
        if let Some(loaded_at) = self.page_loaded_at {
            if loaded_at.elapsed() < Duration::from_millis(500) {
                return PopupDecision::Block {
                    reason: "Popup during page load",
                };
            }
        } else {
            return PopupDecision::Block {
                reason: "Page still loading",
            };
        }

        // Check: User gesture?
        if request.user_gesture {
            return PopupDecision::Allow {
                reason: "User initiated",
            };
        }

        // Check: Session allowed?
        let target_domain = extract_domain(&request.target_url);
        if self
            .session_allowed
            .iter()
            .any(|d| target_domain.contains(d))
        {
            return PopupDecision::Allow {
                reason: "Previously allowed",
            };
        }

        // Check: OAuth domain?
        for domain in OAUTH_DOMAINS {
            if request.target_url.contains(domain) {
                return PopupDecision::Allow {
                    reason: "OAuth authentication",
                };
            }
        }

        // Check: CAPTCHA domain?
        for domain in CAPTCHA_DOMAINS {
            if request.target_url.contains(domain) {
                return PopupDecision::Allow {
                    reason: "CAPTCHA verification",
                };
            }
        }

        // Check: Payment processor?
        for domain in PAYMENT_DOMAINS {
            if request.target_url.contains(domain) {
                return PopupDecision::Allow {
                    reason: "Payment processor",
                };
            }
        }

        // Check: Suspicious size?
        if let (Some(w), Some(h)) = (request.width, request.height) {
            // Tiny popup (probably trying to hide)
            if w < 100 || h < 100 {
                return PopupDecision::Block {
                    reason: "Suspiciously small popup",
                };
            }
            // Huge popup (probably trying to cover screen)
            if w > 2000 || h > 2000 {
                return PopupDecision::Block {
                    reason: "Suspiciously large popup",
                };
            }
        }

        // Check: Same domain as source?
        let source_domain = extract_domain(&request.source_url);
        if source_domain == target_domain && !source_domain.is_empty() {
            return PopupDecision::Prompt {
                reason: "Same-site popup without gesture",
            };
        }

        // Default: Block
        PopupDecision::Block {
            reason: "No user gesture, unknown domain",
        }
    }

    /// Handle the decision
    pub fn handle(&mut self, request: PopupRequest, decision: &PopupDecision) -> bool {
        match decision {
            PopupDecision::Allow { .. } => true,
            PopupDecision::Block { reason } => {
                self.blocked.push(BlockedPopup {
                    request,
                    reason: reason.to_string(),
                    timestamp: Instant::now(),
                });
                false
            }
            PopupDecision::Prompt { .. } => {
                // For now, treat prompt as block
                // In full implementation, show UI dialog
                self.blocked.push(BlockedPopup {
                    request,
                    reason: "Awaiting user decision".to_string(),
                    timestamp: Instant::now(),
                });
                false
            }
        }
    }

    /// Allow a domain for this session
    pub fn allow_domain(&mut self, domain: &str) {
        if !self.session_allowed.contains(&domain.to_string()) {
            self.session_allowed.push(domain.to_string());
        }
    }

    /// Get blocked popup count
    pub fn blocked_count(&self) -> usize {
        self.blocked.len()
    }

    /// Get recent blocked popups
    pub fn recent_blocked(&self) -> &[BlockedPopup] {
        &self.blocked
    }

    /// Clear blocked list
    pub fn clear_blocked(&mut self) {
        self.blocked.clear();
    }

    /// Describe current popup handler state for diagnostics
    pub fn describe(&self) -> String {
        let recent = self.recent_attempts.len();
        let blocked_total = self.blocked.len();
        let allowed_domains = self.session_allowed.len();
        let last_blocked = self
            .blocked
            .last()
            .map(|b| {
                format!(
                    "{} -> {} ({:?} ago)",
                    b.reason,
                    b.request.target_url,
                    b.timestamp.elapsed()
                )
            })
            .unwrap_or_default();
        // Read blocked popup details for diagnostics
        let _blocked_details: Vec<String> = self
            .blocked
            .iter()
            .map(|b| {
                format!(
                    "src={} tgt={} w={:?} h={:?} gesture={} age={:?} reason={} ts={:?}",
                    b.request.source_url,
                    b.request.target_url,
                    b.request.width,
                    b.request.height,
                    b.request.user_gesture,
                    b.request.timestamp.elapsed(),
                    b.reason,
                    b.timestamp.elapsed()
                )
            })
            .collect();
        format!(
            "PopupHandler[recent_attempts={}, blocked={}, session_allowed={}, loaded={:?}, last={}]",
            recent, blocked_total, allowed_domains,
            self.page_loaded_at.map(|t| t.elapsed()),
            last_blocked,
        )
    }

    /// Retry a blocked popup (user clicked "allow")
    pub fn allow_blocked(&mut self, index: usize) -> Option<PopupRequest> {
        if index < self.blocked.len() {
            let blocked = self.blocked.remove(index);
            let domain = extract_domain(&blocked.request.target_url);
            self.allow_domain(&domain);
            Some(blocked.request)
        } else {
            None
        }
    }
}

impl Default for PopupHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract domain from URL
fn extract_domain(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_allowed() {
        let mut handler = PopupHandler::new();
        handler.page_loaded();

        // Wait a bit for page load check
        std::thread::sleep(Duration::from_millis(600));

        let request = PopupRequest {
            source_url: "https://example.com".into(),
            target_url: "https://accounts.google.com/oauth".into(),
            width: Some(500),
            height: Some(600),
            user_gesture: false,
            timestamp: Instant::now(),
        };

        let decision = handler.evaluate(&request);
        assert!(matches!(decision, PopupDecision::Allow { .. }));
    }

    #[test]
    fn test_captcha_allowed() {
        let mut handler = PopupHandler::new();
        handler.page_loaded();
        std::thread::sleep(Duration::from_millis(600));

        let request = PopupRequest {
            source_url: "https://example.com".into(),
            target_url: "https://challenges.cloudflare.com/verify".into(),
            width: Some(400),
            height: Some(500),
            user_gesture: false,
            timestamp: Instant::now(),
        };

        let decision = handler.evaluate(&request);
        assert!(matches!(decision, PopupDecision::Allow { .. }));
    }

    #[test]
    fn test_spam_blocked() {
        let mut handler = PopupHandler::new();
        handler.page_loaded();
        std::thread::sleep(Duration::from_millis(600));

        let request = PopupRequest {
            source_url: "https://example.com".into(),
            target_url: "https://totally-not-spam.xyz/winner".into(),
            width: Some(800),
            height: Some(600),
            user_gesture: false,
            timestamp: Instant::now(),
        };

        let decision = handler.evaluate(&request);
        assert!(matches!(decision, PopupDecision::Block { .. }));
    }
}
