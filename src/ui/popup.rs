//! Smart Popup Handling
//!
//! Blocks spam popups while allowing legitimate ones like captchas.
//! Uses heuristics to distinguish between user-initiated and spam popups.


use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Popup manager with smart blocking
#[derive(Debug)]
pub struct PopupManager {
    /// Blocked popup count per domain
    pub blocked_count: HashMap<String, u32>,
    
    /// Allowed popups waiting to open
    pub pending_popups: Vec<PendingPopup>,
    
    /// Recently opened popups (for rate limiting)
    pub recent_popups: Vec<(Instant, String)>,
    
    /// Domains with popup permission
    pub allowed_domains: Vec<String>,
    
    /// Domains always blocked
    pub blocked_domains: Vec<String>,
    
    /// User interaction tracking for popup decisions
    pub last_user_interaction: Option<Instant>,
    pub interaction_type: Option<InteractionType>,
    
    /// Stats
    pub total_blocked: u64,
    pub total_allowed: u64,
}

#[derive(Debug, Clone)]
pub struct PendingPopup {
    pub url: String,
    pub opener_url: String,
    pub opener_domain: String,
    pub reason: PopupReason,
    pub timestamp: Instant,
    pub classification: PopupClassification,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupReason {
    WindowOpen,      // window.open()
    TargetBlank,     // target="_blank"
    FormSubmit,      // Form submission to new window
    UserClick,       // User clicked a link
    Script,          // Script-initiated
    Unknown,
}

impl PopupReason {
    /// Human-readable label for this popup reason
    pub fn label(&self) -> &'static str {
        match self {
            PopupReason::WindowOpen => "window.open",
            PopupReason::TargetBlank => "target=_blank",
            PopupReason::FormSubmit => "form-submit",
            PopupReason::UserClick => "user-click",
            PopupReason::Script => "script",
            PopupReason::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupClassification {
    Allowed,         // Definitely allow
    ProbablyCaptcha, // Looks like captcha, allow
    ProbablyAuth,    // OAuth/login flow, allow
    UserInitiated,   // User click triggered it
    Suspicious,      // Needs user approval
    Blocked,         // Definitely block
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionType {
    Click,
    KeyPress,
    FormSubmit,
    Scroll,
}

impl InteractionType {
    /// Human-readable label for this interaction type
    pub fn label(&self) -> &'static str {
        match self {
            InteractionType::Click => "click",
            InteractionType::KeyPress => "keypress",
            InteractionType::FormSubmit => "form-submit",
            InteractionType::Scroll => "scroll",
        }
    }
}

/// Result of popup evaluation
#[derive(Debug, Clone)]
pub struct PopupDecision {
    pub allow: bool,
    pub classification: PopupClassification,
    pub reason: String,
    pub show_notification: bool,
}

impl PopupDecision {
    /// Describe the decision for logging / UI display
    pub fn describe(&self) -> String {
        format!(
            "allow={} classification={:?} notify={} reason={}",
            self.allow, self.classification, self.show_notification, self.reason
        )
    }
}

impl PendingPopup {
    /// Describe the pending popup for UI / logging
    pub fn describe(&self) -> String {
        format!(
            "{} from {} ({}) reason={} class={:?} age={:.1}s",
            self.url,
            self.opener_url,
            self.opener_domain,
            self.reason.label(),
            self.classification,
            self.timestamp.elapsed().as_secs_f64(),
        )
    }
}

impl PopupManager {
    pub fn new() -> Self {
        Self {
            blocked_count: HashMap::new(),
            pending_popups: Vec::new(),
            recent_popups: Vec::new(),
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
            last_user_interaction: None,
            interaction_type: None,
            total_blocked: 0,
            total_allowed: 0,
        }
    }
    
    /// Record user interaction (for popup timing heuristics)
    pub fn record_interaction(&mut self, interaction: InteractionType) {
        self.last_user_interaction = Some(Instant::now());
        self.interaction_type = Some(interaction);
    }
    
    /// Evaluate a popup request
    pub fn evaluate(&mut self, url: &str, opener_url: &str, reason: PopupReason, sandbox_allowed: bool) -> PopupDecision {
        let domain = extract_domain(url).unwrap_or("unknown").to_string();
        let opener_domain = extract_domain(opener_url).unwrap_or("unknown").to_string();
        
        // Check explicit allow/block lists
        if self.blocked_domains.iter().any(|d| domain.contains(d)) {
            self.total_blocked += 1;
            return PopupDecision {
                allow: false,
                classification: PopupClassification::Blocked,
                reason: "Domain is blocked".to_string(),
                show_notification: true,
            };
        }
        
        if self.allowed_domains.iter().any(|d| domain.contains(d) || opener_domain.contains(d)) {
            self.total_allowed += 1;
            return PopupDecision {
                allow: true,
                classification: PopupClassification::Allowed,
                reason: "Domain is allowed".to_string(),
                show_notification: false,
            };
        }
        
        // Check for captcha patterns
        if is_probable_captcha(url, &opener_domain) {
            self.total_allowed += 1;
            return PopupDecision {
                allow: true,
                classification: PopupClassification::ProbablyCaptcha,
                reason: "Detected captcha verification popup".to_string(),
                show_notification: false,
            };
        }
        
        // Check for OAuth/auth patterns
        if is_probable_auth(url) {
            self.total_allowed += 1;
            return PopupDecision {
                allow: true,
                classification: PopupClassification::ProbablyAuth,
                reason: "Detected authentication popup".to_string(),
                show_notification: false,
            };
        }

        // Sandbox gate: if the page sandbox forbids popups, block unless clearly user-initiated or whitelisted above
        if !sandbox_allowed {
            // Queue so user could allow later if we expose UI; keeps functionality discoverable
            self.pending_popups.push(PendingPopup {
                url: url.to_string(),
                opener_url: opener_url.to_string(),
                opener_domain: opener_domain.clone(),
                reason,
                timestamp: Instant::now(),
                classification: PopupClassification::Blocked,
            });
            return PopupDecision {
                allow: false,
                classification: PopupClassification::Blocked,
                reason: "Blocked by sandbox trust level".to_string(),
                show_notification: true,
            };
        }
        
        // Check user interaction timing
        if let Some(last_interaction) = self.last_user_interaction {
            let since_interaction = Instant::now().duration_since(last_interaction);
            
            // Popup within 1 second of click is likely user-initiated
            if since_interaction < Duration::from_secs(1)
                && matches!(self.interaction_type, Some(InteractionType::Click | InteractionType::FormSubmit)) {
                    self.total_allowed += 1;
                    return PopupDecision {
                        allow: true,
                        classification: PopupClassification::UserInitiated,
                        reason: "Popup followed user interaction".to_string(),
                        show_notification: false,
                    };
                }
        }
        
        // Rate limit check - too many popups is suspicious
        self.cleanup_recent();
        let recent_from_domain = self.recent_popups.iter()
            .filter(|(_, d)| d == &opener_domain)
            .count();
        
        if recent_from_domain >= 3 {
            self.total_blocked += 1;
            *self.blocked_count.entry(opener_domain.clone()).or_insert(0) += 1;
            return PopupDecision {
                allow: false,
                classification: PopupClassification::Blocked,
                reason: format!("Too many popups from {}", opener_domain),
                show_notification: true,
            };
        }
        
        // Check popup characteristics
        let classification = classify_popup(url, &opener_domain, reason);
        
        match classification {
            PopupClassification::Allowed |
            PopupClassification::ProbablyCaptcha |
            PopupClassification::ProbablyAuth |
            PopupClassification::UserInitiated => {
                self.total_allowed += 1;
                self.recent_popups.push((Instant::now(), opener_domain));
                PopupDecision {
                    allow: true,
                    classification,
                    reason: "Popup appears legitimate".to_string(),
                    show_notification: false,
                }
            }
            PopupClassification::Suspicious => {
                // Queue for user decision
                self.pending_popups.push(PendingPopup {
                    url: url.to_string(),
                    opener_url: opener_url.to_string(),
                    opener_domain: opener_domain.clone(),
                    reason,
                    timestamp: Instant::now(),
                    classification,
                });
                PopupDecision {
                    allow: false,
                    classification,
                    reason: "Popup blocked pending user approval".to_string(),
                    show_notification: true,
                }
            }
            PopupClassification::Blocked => {
                self.total_blocked += 1;
                *self.blocked_count.entry(opener_domain).or_insert(0) += 1;
                PopupDecision {
                    allow: false,
                    classification,
                    reason: "Popup blocked".to_string(),
                    show_notification: true,
                }
            }
        }
    }
    
    /// Allow a pending popup
    pub fn allow_pending(&mut self, index: usize) -> Option<String> {
        if index < self.pending_popups.len() {
            let popup = self.pending_popups.remove(index);
            self.total_allowed += 1;
            self.recent_popups.push((Instant::now(), popup.opener_domain));
            Some(popup.url)
        } else {
            None
        }
    }

    /// Allow all pending popups (used when sandbox trust level improves)
    pub fn allow_all_pending(&mut self) -> Vec<String> {
        let mut urls = Vec::new();
        while let Some(popup) = self.pending_popups.pop() {
            self.total_allowed += 1;
            self.recent_popups.push((Instant::now(), popup.opener_domain));
            urls.push(popup.url);
        }
        urls
    }
    
    /// Block a pending popup
    pub fn block_pending(&mut self, index: usize) {
        if index < self.pending_popups.len() {
            let popup = self.pending_popups.remove(index);
            self.total_blocked += 1;
            *self.blocked_count.entry(popup.opener_domain).or_insert(0) += 1;
        }
    }
    
    /// Allow all popups from domain
    pub fn allow_domain(&mut self, domain: &str) {
        if !self.allowed_domains.contains(&domain.to_string()) {
            self.allowed_domains.push(domain.to_string());
        }
        self.blocked_domains.retain(|d| d != domain);
    }
    
    /// Block all popups from domain
    pub fn block_domain(&mut self, domain: &str) {
        if !self.blocked_domains.contains(&domain.to_string()) {
            self.blocked_domains.push(domain.to_string());
        }
        self.allowed_domains.retain(|d| d != domain);
    }
    
    /// Cleanup old records
    fn cleanup_recent(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(30);
        self.recent_popups.retain(|(t, _)| *t > cutoff);
        
        // Also cleanup old pending popups
        let popup_cutoff = Instant::now() - Duration::from_secs(60);
        self.pending_popups.retain(|p| p.timestamp > popup_cutoff);
    }
    
    pub fn blocked_for(&self, domain: &str) -> u32 {
        *self.blocked_count.get(domain).unwrap_or(&0)
    }
    
    pub fn pending_count(&self) -> usize {
        self.pending_popups.len()
    }
}

impl Default for PopupManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if URL looks like a captcha service
fn is_probable_captcha(url: &str, opener_domain: &str) -> bool {
    let url_lower = crate::fontcase::ascii_lower(url);
    
    // Known captcha providers
    let captcha_patterns = [
        "recaptcha",
        "hcaptcha",
        "funcaptcha",
        "arkose",
        "captcha",
        "challenge",
        "geetest",
        "turnstile",
        "cloudflare.com/cdn-cgi/challenge",
        "google.com/recaptcha",
        "hcaptcha.com",
        "arkoselabs.com",
    ];
    
    for pattern in captcha_patterns {
        if url_lower.contains(pattern) {
            return true;
        }
    }
    
    // Frame-based captcha detection
    // Many captchas open in same-origin or related iframes
    if (url_lower.contains("iframe") || url_lower.contains("frame"))
        && (url_lower.contains("verify") || url_lower.contains("check")) {
            return true;
        }
    
    // Some sites use their own captcha endpoints
    if url_lower.contains(&format!("{}/captcha", opener_domain)) ||
       url_lower.contains(&format!("{}/verify", opener_domain)) ||
       url_lower.contains(&format!("{}/challenge", opener_domain)) {
        return true;
    }
    
    false
}

/// Check if URL looks like OAuth/authentication
fn is_probable_auth(url: &str) -> bool {
    let url_lower = crate::fontcase::ascii_lower(url);
    
    // OAuth providers
    let auth_patterns = [
        "accounts.google.com",
        "login.microsoftonline.com",
        "github.com/login/oauth",
        "facebook.com/dialog/oauth",
        "twitter.com/oauth",
        "login.yahoo.com",
        "appleid.apple.com",
        "auth0.com",
        "okta.com",
        "oauth",
        "/authorize",
        "/login",
        "/signin",
        "/sso",
        "openid",
    ];
    
    for pattern in auth_patterns {
        if url_lower.contains(pattern) {
            return true;
        }
    }
    
    false
}

/// Classify popup based on URL and context
fn classify_popup(url: &str, opener_domain: &str, reason: PopupReason) -> PopupClassification {
    let url_lower = crate::fontcase::ascii_lower(url);
    
    // User-initiated reasons get more trust
    if matches!(reason, PopupReason::UserClick | PopupReason::TargetBlank) {
        return PopupClassification::UserInitiated;
    }
    
    // Same-origin popups are usually legitimate
    if let Some(popup_domain) = extract_domain(url) {
        if popup_domain == opener_domain {
            return PopupClassification::UserInitiated;
        }
        
        // Subdomain of opener
        if popup_domain.ends_with(&format!(".{}", opener_domain)) ||
           opener_domain.ends_with(&format!(".{}", popup_domain)) {
            return PopupClassification::UserInitiated;
        }
    }
    
    // Suspicious patterns
    let suspicious_patterns = [
        "pop", "ad.", "ads.", "advert",
        "click", "track", "pixel",
        "doubleclick", "googlesyndication",
        "popup", "pounder",
    ];
    
    for pattern in suspicious_patterns {
        if url_lower.contains(pattern) {
            return PopupClassification::Blocked;
        }
    }
    
    // Long query strings with tracking parameters
    if url.contains("utm_") && url.contains("&") && url.len() > 200 {
        return PopupClassification::Suspicious;
    }
    
    // window.open from script without user interaction
    if reason == PopupReason::Script {
        return PopupClassification::Suspicious;
    }
    
    // Default to suspicious for script-initiated
    if reason == PopupReason::WindowOpen {
        return PopupClassification::Suspicious;
    }
    
    PopupClassification::UserInitiated
}

fn extract_domain(url: &str) -> Option<&str> {
    let url = url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    url.split('/').next()?.split(':').next()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_captcha_detection() {
        assert!(is_probable_captcha("https://www.google.com/recaptcha/api2/anchor", "example.com"));
        assert!(is_probable_captcha("https://hcaptcha.com/1/api.js", "example.com"));
        assert!(is_probable_captcha("https://challenges.cloudflare.com/turnstile", "example.com"));
        assert!(!is_probable_captcha("https://ads.example.com/popup", "example.com"));
    }
    
    #[test]
    fn test_auth_detection() {
        assert!(is_probable_auth("https://accounts.google.com/signin/oauth"));
        assert!(is_probable_auth("https://github.com/login/oauth/authorize"));
        assert!(!is_probable_auth("https://example.com/page"));
    }
    
    #[test]
    fn test_popup_classification() {
        let mut manager = PopupManager::new();
        
        // Captcha should be allowed
        let decision = manager.evaluate(
            "https://www.google.com/recaptcha/api2/anchor?k=xxx",
            "https://example.com",
            PopupReason::Script,
            false,
        );
        assert!(decision.allow);
        
        // Ad should be blocked  
        let mut manager = PopupManager::new();
        let decision = manager.evaluate(
            "https://ads.doubleclick.net/popup",
            "https://example.com",
            PopupReason::Script,
            false,
        );
        assert!(!decision.allow);
    }

    #[test]
    fn test_popup_manager_lifecycle() {
        let mut pm = PopupManager::new();

        // Record a click interaction and ensure it affects evaluation timing
        pm.record_interaction(InteractionType::Click);

        // Add pending popup by simulating sandbox block
        let decision = pm.evaluate("https://suspicious.example/popup", "https://opener.example", PopupReason::Script, false);
        assert!(!decision.allow);
        assert!(pm.pending_count() >= 1);

        // Exercise PopupDecision::describe which reads show_notification
        let desc = decision.describe();
        assert!(desc.contains("notify="));

        // Exercise PendingPopup::describe
        if let Some(pending) = pm.pending_popups.first() {
            let pdesc = pending.describe();
            assert!(pdesc.contains("suspicious.example"));
        }

        // Allow pending
        let urls = pm.allow_all_pending();
        assert!(!urls.is_empty());

        // Domain allow/block
        pm.allow_domain("trusted.example");
        assert!(pm.allowed_domains.contains(&"trusted.example".to_string()));
        pm.block_domain("evil.example");
        assert!(pm.blocked_domains.contains(&"evil.example".to_string()));
        assert_eq!(pm.blocked_for("evil.example"), 0);
    }

    #[test]
    fn test_popup_reason_variants() {
        // Construct and label all PopupReason variants
        let reasons = vec![
            PopupReason::WindowOpen,
            PopupReason::TargetBlank,
            PopupReason::FormSubmit,
            PopupReason::UserClick,
            PopupReason::Script,
            PopupReason::Unknown,
        ];
        for r in &reasons {
            assert!(!r.label().is_empty());
        }
    }

    #[test]
    fn test_interaction_type_variants() {
        // Construct and label all InteractionType variants
        let types = vec![
            InteractionType::Click,
            InteractionType::KeyPress,
            InteractionType::FormSubmit,
            InteractionType::Scroll,
        ];
        for t in &types {
            assert!(!t.label().is_empty());
        }

        // Record all interaction types
        let mut pm = PopupManager::new();
        pm.record_interaction(InteractionType::KeyPress);
        pm.record_interaction(InteractionType::Scroll);
        pm.record_interaction(InteractionType::FormSubmit);
        assert!(pm.last_user_interaction.is_some());
    }

    #[test]
    fn test_popup_with_user_click_reason() {
        let mut pm = PopupManager::new();
        pm.record_interaction(InteractionType::Click);

        // UserClick-initiated popup should be allowed (with sandbox permission)
        let decision = pm.evaluate(
            "https://safe.example/page",
            "https://opener.example",
            PopupReason::UserClick,
            true,
        );
        assert!(decision.allow);

        // TargetBlank popup
        let decision2 = pm.evaluate(
            "https://safe.example/page2",
            "https://opener.example",
            PopupReason::TargetBlank,
            true,
        );
        assert!(decision2.allow);

        // FormSubmit popup
        let decision3 = pm.evaluate(
            "https://safe.example/submit",
            "https://opener.example",
            PopupReason::FormSubmit,
            true,
        );
        // FormSubmit without tracking patterns should be allowed as UserInitiated
        assert!(decision3.allow);

        // Unknown popup reason
        let decision4 = pm.evaluate(
            "https://safe.example/unknown",
            "https://opener.example",
            PopupReason::Unknown,
            true,
        );
        assert!(decision4.allow);
    }
}
