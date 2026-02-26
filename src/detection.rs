//! Client-Side Detection Engine with Honeypot System
//!
//! ACTIVE ON TRUST LEVELS 0-1 (Untrusted, Acknowledged) ONLY
//! Disengages on level 2+ to avoid performance impact on trusted pages.
//!
//! ARCHITECTURE:
//! ─────────────────────────────────────────────────────────────────────────
//! 1. HONEYPOTS — Invisible traps injected into untrusted page sandboxes
//!    - Hidden form fields (detect credential harvesters)
//!    - Invisible links (detect automated crawlers/scrapers)
//!    - Fake localStorage/sessionStorage keys (detect fingerprinting)
//!    - Canvas/WebGL bait (detect fingerprint scripts)
//!    - Fake cookie values (detect cookie exfiltration)
//!
//! 2. BEHAVIOR ANALYSIS — Pattern detection across page interactions
//!    - Known malicious tracker domains
//!    - Canvas/WebGL fingerprinting attempts
//!    - Battery API abuse
//!    - Credential phishing indicators
//!    - Crypto miner script patterns
//!    - Clipboard hijacking attempts
//!
//! 3. ALERT PIPELINE — Detection → Violation → MCP notification
//!    - Alerts feed into SecurityContext violations
//!    - Critical alerts immediately downgrade trust to Untrusted
//!    - Alerts forwarded to MCP server for remote monitoring
//!
//! WHY HONEYPOTS ONLY ON LEVELS 0-1:
//! ─────────────────────────────────────────────────────────────────────────
//! - Untrusted pages are the attack surface
//! - Legitimate pages earn trust through interaction — no honeypot needed
//! - Zero overhead on trusted/established pages
//! - A page that trips a honeypot proves it was doing something it shouldn't

use crate::sandbox::{TrustLevel, ViolationSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ─────────────────────────────────────────────────────────────────────────────
// Suspicion Level — maps to our ViolationSeverity system
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuspicionLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl SuspicionLevel {
    pub fn score(&self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Low => 0.3,
            Self::Medium => 0.7,
            Self::High => 1.5,
            Self::Critical => 3.0,
        }
    }

    /// Convert to our existing ViolationSeverity for sandbox integration
    pub fn to_violation_severity(&self) -> Option<ViolationSeverity> {
        match self {
            Self::None => None,
            Self::Low => Some(ViolationSeverity::Low),
            Self::Medium => Some(ViolationSeverity::Medium),
            Self::High => Some(ViolationSeverity::High),
            Self::Critical => Some(ViolationSeverity::Critical),
        }
    }

    /// Convert to u8 for wire transport
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Detection Alert — emitted by rules and honeypots
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DetectionAlert {
    pub rule_name: String,
    pub level: SuspicionLevel,
    pub description: String,
    pub url: String,
    pub domain: String,
    pub timestamp: Instant,
    pub score: f32,
    pub honeypot_triggered: bool,
    pub action: DetectionAction,
}

impl DetectionAlert {
    /// Serialize for MCP transport (Instant → epoch millis)
    pub fn to_mcp_payload(&self) -> DetectionAlertPayload {
        DetectionAlertPayload {
            rule_name: self.rule_name.clone(),
            level: self.level,
            description: self.description.clone(),
            url: self.url.clone(),
            domain: self.domain.clone(),
            score: self.score,
            honeypot_triggered: self.honeypot_triggered,
            action: self.action,
        }
    }
}

/// Serializable version of DetectionAlert for MCP/JSON transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionAlertPayload {
    pub rule_name: String,
    pub level: SuspicionLevel,
    pub description: String,
    pub url: String,
    pub domain: String,
    pub score: f32,
    pub honeypot_triggered: bool,
    pub action: DetectionAction,
}

// ─────────────────────────────────────────────────────────────────────────────
// Behavior Tracking — per-rule cumulative scoring
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BehaviorRecord {
    pub first_seen: Instant,
    pub count: u32,
    pub total_score: f32,
    pub last_event: Instant,
}

// ─────────────────────────────────────────────────────────────────────────────
// Page Context — snapshot of page state for analysis
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PageContext {
    pub url: String,
    pub domain: String,
    pub trust_level: TrustLevel,
    pub headers: HashMap<String, String>,
    pub scripts_src: Vec<String>,
    pub inline_script_hashes: Vec<String>,
    pub iframes: Vec<String>,
    pub canvas_calls: bool,
    pub webgl_calls: bool,
    pub battery_api_calls: bool,
    pub clipboard_read_attempts: u32,
    pub known_trackers: Vec<String>,
    pub login_form_detected: bool,
    pub phishing_keywords: bool,
    pub crypto_miner_indicators: bool,
    pub honeypot_form_touched: bool,
    pub honeypot_link_followed: bool,
    pub honeypot_storage_read: bool,
    pub honeypot_cookie_exfiltrated: bool,
    pub honeypot_canvas_probed: bool,
    pub redirect_chain_length: u32,
    pub last_updated: Instant,
}

impl Default for PageContext {
    fn default() -> Self {
        Self {
            url: String::new(),
            domain: String::new(),
            trust_level: TrustLevel::Untrusted,
            headers: HashMap::new(),
            scripts_src: Vec::new(),
            inline_script_hashes: Vec::new(),
            iframes: Vec::new(),
            canvas_calls: false,
            webgl_calls: false,
            battery_api_calls: false,
            clipboard_read_attempts: 0,
            known_trackers: Vec::new(),
            login_form_detected: false,
            phishing_keywords: false,
            crypto_miner_indicators: false,
            honeypot_form_touched: false,
            honeypot_link_followed: false,
            honeypot_storage_read: false,
            honeypot_cookie_exfiltrated: false,
            honeypot_canvas_probed: false,
            redirect_chain_length: 0,
            last_updated: Instant::now(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Detection Actions — what to do when a rule fires
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionAction {
    LogOnly,
    WarnUser,
    BlockResource,
    QuarantinePage,
    ResetTrust,
    NotifyMcp,
}

impl DetectionAction {
    /// Convert to u8 for wire transport
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::LogOnly => 0,
            Self::WarnUser => 1,
            Self::BlockResource => 2,
            Self::QuarantinePage => 3,
            Self::ResetTrust => 4,
            Self::NotifyMcp => 5,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Honeypot Configuration — what traps to inject for trust levels 0-1
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct HoneypotConfig {
    /// Inject invisible form field that catches credential harvesters
    pub inject_hidden_form: bool,
    /// Inject invisible link that catches automated crawlers
    pub inject_invisible_link: bool,
    /// Plant fake localStorage key to detect storage scrapers
    pub inject_fake_storage: bool,
    /// Plant canary cookie to detect cookie exfiltration
    pub inject_canary_cookie: bool,
    /// Set up canvas bait to detect fingerprint scripts
    pub inject_canvas_bait: bool,
    /// Unique per-page honeypot token (to correlate triggers)
    pub session_token: String,
}

impl Default for HoneypotConfig {
    fn default() -> Self {
        Self {
            inject_hidden_form: true,
            inject_invisible_link: true,
            inject_fake_storage: true,
            inject_canary_cookie: true,
            inject_canvas_bait: true,
            session_token: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl HoneypotConfig {
    /// Generate the HTML/JS honeypot payload to inject into page sandbox
    pub fn generate_injection_html(&self) -> String {
        let mut html = String::new();

        if self.inject_hidden_form {
            // Hidden form field — invisible to users, irresistible to bots
            html.push_str(&format!(r#"
<div style="position:absolute;left:-9999px;top:-9999px;width:1px;height:1px;overflow:hidden;">
  <form id="sassy_hp_{tok}" autocomplete="off">
    <input type="text" name="username" id="sassy_hp_user_{tok}" tabindex="-1" autocomplete="off" />
    <input type="password" name="password" id="sassy_hp_pass_{tok}" tabindex="-1" autocomplete="off" />
    <input type="email" name="email" id="sassy_hp_email_{tok}" tabindex="-1" autocomplete="off" />
  </form>
</div>"#, tok = &self.session_token[..8]));
        }

        if self.inject_invisible_link {
            // Invisible link — legitimate users can't see or click it
            html.push_str(&format!(r#"
<a href="sassy://honeypot/link/{tok}" id="sassy_hp_link_{tok}"
   style="position:absolute;left:-9999px;top:-9999px;width:1px;height:1px;overflow:hidden;opacity:0;pointer-events:auto;">
   Click here for free rewards
</a>"#, tok = &self.session_token[..8]));
        }

        html
    }

    /// Generate JavaScript honeypot traps
    pub fn generate_injection_js(&self) -> String {
        let tok = &self.session_token[..8];
        let mut js = String::new();

        if self.inject_fake_storage {
            // Plant fake keys that fingerprinters will try to read
            js.push_str(&format!(
                r#"
(function() {{
    try {{
        var _hp_key = 'sassy_session_{tok}';
        localStorage.setItem(_hp_key, 'canary_{tok}');
        sessionStorage.setItem(_hp_key, 'canary_{tok}');
        // Monitor reads via proxy (SassyScript intercepts)
        window.__sassy_hp_storage_token = _hp_key;
    }} catch(e) {{}}
}})();"#,
                tok = tok
            ));
        }

        if self.inject_canary_cookie {
            // Plant canary cookie
            js.push_str(&format!(
                r#"
document.cookie = 'sassy_hp_{tok}=canary_{tok};path=/;SameSite=Strict';"#,
                tok = tok
            ));
        }

        if self.inject_canvas_bait {
            // Canvas bait — creates a hidden canvas with known content
            js.push_str(&format!(
                r#"
(function() {{
    try {{
        var c = document.createElement('canvas');
        c.width = 1; c.height = 1;
        c.id = 'sassy_hp_canvas_{tok}';
        c.style.position = 'absolute';
        c.style.left = '-9999px';
        document.body.appendChild(c);
        window.__sassy_hp_canvas_id = c.id;
    }} catch(e) {{}}
}})();"#,
                tok = tok
            ));
        }

        js
    }

    /// Check if honeypots should be active for this trust level
    pub fn should_activate(trust_level: TrustLevel) -> bool {
        matches!(
            trust_level,
            TrustLevel::Untrusted | TrustLevel::Acknowledged
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Detection Rules — pattern matching functions
// ─────────────────────────────────────────────────────────────────────────────

pub type RulePattern = fn(&PageContext) -> Option<SuspicionLevel>;

#[derive(Clone)]
pub struct DetectionRule {
    pub name: &'static str,
    pub pattern: RulePattern,
    pub description: &'static str,
    pub action: DetectionAction,
}

// ═══════════════════════════════════════════════════════════════════════════════
// BUILT-IN DETECTION RULES
// ═══════════════════════════════════════════════════════════════════════════════

/// Known malicious tracker / ad network domains
const MALICIOUS_DOMAINS: &[&str] = &[
    "doubleclick.net",
    "googletagservices.com",
    "adservice.google.com",
    "scorecardresearch.com",
    "quantserve.com",
    "googlesyndication.com",
    "2mdn.net",
    "moatads.com",
    "taboola.com",
    "outbrain.com",
];

/// Crypto miner domains
const MINER_DOMAINS: &[&str] = &[
    "coinhive.com",
    "jsecoin.com",
    "cryptoloot.pro",
    "coin-hive.com",
    "authedmine.com",
    "minero.cc",
    "webmine.cz",
];

const BUILT_IN_RULES: &[DetectionRule] = &[
    // ─── HONEYPOT RULES (only fire on levels 0-1) ───
    DetectionRule {
        name: "honeypot_form_touched",
        pattern: |ctx| {
            if !HoneypotConfig::should_activate(ctx.trust_level) {
                return None;
            }
            if ctx.honeypot_form_touched {
                Some(SuspicionLevel::Critical) // Only bots touch hidden forms
            } else {
                None
            }
        },
        description:
            "Hidden honeypot form field was filled — automated credential harvester detected",
        action: DetectionAction::QuarantinePage,
    },
    DetectionRule {
        name: "honeypot_link_followed",
        pattern: |ctx| {
            if !HoneypotConfig::should_activate(ctx.trust_level) {
                return None;
            }
            if ctx.honeypot_link_followed {
                Some(SuspicionLevel::High)
            } else {
                None
            }
        },
        description: "Invisible honeypot link was followed — automated crawler/scraper detected",
        action: DetectionAction::ResetTrust,
    },
    DetectionRule {
        name: "honeypot_storage_scraped",
        pattern: |ctx| {
            if !HoneypotConfig::should_activate(ctx.trust_level) {
                return None;
            }
            if ctx.honeypot_storage_read {
                Some(SuspicionLevel::High)
            } else {
                None
            }
        },
        description:
            "Honeypot localStorage/sessionStorage key was read — fingerprint script detected",
        action: DetectionAction::WarnUser,
    },
    DetectionRule {
        name: "honeypot_cookie_exfiltrated",
        pattern: |ctx| {
            if !HoneypotConfig::should_activate(ctx.trust_level) {
                return None;
            }
            if ctx.honeypot_cookie_exfiltrated {
                Some(SuspicionLevel::Critical)
            } else {
                None
            }
        },
        description: "Canary cookie was sent to external domain — cookie theft detected",
        action: DetectionAction::QuarantinePage,
    },
    DetectionRule {
        name: "honeypot_canvas_probed",
        pattern: |ctx| {
            if !HoneypotConfig::should_activate(ctx.trust_level) {
                return None;
            }
            if ctx.honeypot_canvas_probed {
                Some(SuspicionLevel::Medium)
            } else {
                None
            }
        },
        description: "Honeypot canvas element was read — canvas fingerprinting script detected",
        action: DetectionAction::WarnUser,
    },
    // ─── BEHAVIORAL RULES (fire at any trust level) ───
    DetectionRule {
        name: "known_malicious_tracker",
        pattern: |ctx| {
            if ctx
                .known_trackers
                .iter()
                .any(|t| MALICIOUS_DOMAINS.contains(&t.as_str()))
            {
                Some(SuspicionLevel::High)
            } else {
                None
            }
        },
        description: "Known malicious or heavy-tracking domain detected",
        action: DetectionAction::WarnUser,
    },
    DetectionRule {
        name: "canvas_fingerprinting",
        pattern: |ctx| {
            if ctx.canvas_calls {
                Some(SuspicionLevel::Medium)
            } else {
                None
            }
        },
        description: "Canvas fingerprinting attempt detected",
        action: DetectionAction::LogOnly,
    },
    DetectionRule {
        name: "webgl_fingerprinting",
        pattern: |ctx| {
            if ctx.webgl_calls {
                Some(SuspicionLevel::Medium)
            } else {
                None
            }
        },
        description: "WebGL fingerprinting attempt detected",
        action: DetectionAction::LogOnly,
    },
    DetectionRule {
        name: "battery_api_abuse",
        pattern: |ctx| {
            if ctx.battery_api_calls {
                Some(SuspicionLevel::Low)
            } else {
                None
            }
        },
        description: "Battery API used for potential fingerprinting",
        action: DetectionAction::LogOnly,
    },
    DetectionRule {
        name: "clipboard_hijack_attempt",
        pattern: |ctx| {
            if ctx.clipboard_read_attempts > 2 {
                Some(SuspicionLevel::High)
            } else if ctx.clipboard_read_attempts > 0 {
                Some(SuspicionLevel::Medium)
            } else {
                None
            }
        },
        description: "Page attempted to access clipboard without user gesture",
        action: DetectionAction::BlockResource,
    },
    DetectionRule {
        name: "credential_phishing_form",
        pattern: |ctx| {
            if ctx.login_form_detected && ctx.phishing_keywords {
                Some(SuspicionLevel::Critical)
            } else {
                None
            }
        },
        description: "Suspicious login form on untrusted domain — potential phishing",
        action: DetectionAction::QuarantinePage,
    },
    DetectionRule {
        name: "crypto_miner_detected",
        pattern: |ctx| {
            if ctx.crypto_miner_indicators {
                Some(SuspicionLevel::High)
            } else if ctx
                .scripts_src
                .iter()
                .any(|s| MINER_DOMAINS.iter().any(|d| s.contains(d)))
            {
                Some(SuspicionLevel::Critical)
            } else {
                None
            }
        },
        description: "Cryptocurrency miner script behavior detected",
        action: DetectionAction::BlockResource,
    },
    DetectionRule {
        name: "excessive_redirects",
        pattern: |ctx| {
            if ctx.redirect_chain_length > 5 {
                Some(SuspicionLevel::High)
            } else if ctx.redirect_chain_length > 3 {
                Some(SuspicionLevel::Medium)
            } else {
                None
            }
        },
        description: "Excessive redirect chain — possible malicious redirect loop",
        action: DetectionAction::WarnUser,
    },
    DetectionRule {
        name: "iframe_overload",
        pattern: |ctx| {
            if ctx.iframes.len() > 10 {
                Some(SuspicionLevel::High)
            } else if ctx.iframes.len() > 5 {
                Some(SuspicionLevel::Medium)
            } else {
                None
            }
        },
        description: "Excessive iframes detected — possible clickjacking or ad injection",
        action: DetectionAction::WarnUser,
    },
];

// ─────────────────────────────────────────────────────────────────────────────
// Detection Engine — synchronous (no tokio required in main loop)
// ─────────────────────────────────────────────────────────────────────────────
//
// Design decision: We use synchronous analysis in the egui render loop
// (which is single-threaded) and push alerts to a shared Vec. The MCP
// server (async) can poll/consume alerts via Arc<Mutex<>>.

pub struct DetectionEngine {
    rules: Vec<DetectionRule>,
    behaviors: HashMap<String, BehaviorRecord>,
    /// All alerts generated this session, newest first
    alerts: Vec<DetectionAlert>,
    /// Shared alert queue for MCP server consumption
    shared_alerts: Arc<Mutex<Vec<DetectionAlertPayload>>>,
    /// Per-tab honeypot configurations (tab_id → config)
    honeypots: HashMap<u64, HoneypotConfig>,
    /// Cooldown: minimum time between alerts for same rule
    cooldown: Duration,
    /// Last alert time per rule name
    last_alert_time: HashMap<String, Instant>,
    /// Total alerts emitted
    total_alerts: u64,
    /// Whether engine is enabled
    pub enabled: bool,
}

impl DetectionEngine {
    pub fn new() -> Self {
        Self {
            rules: BUILT_IN_RULES.to_vec(),
            behaviors: HashMap::new(),
            alerts: Vec::new(),
            shared_alerts: Arc::new(Mutex::new(Vec::new())),
            honeypots: HashMap::new(),
            cooldown: Duration::from_secs(10),
            last_alert_time: HashMap::new(),
            total_alerts: 0,
            enabled: true,
        }
    }

    /// Get Arc handle for MCP server to consume alerts
    pub fn shared_alerts(&self) -> Arc<Mutex<Vec<DetectionAlertPayload>>> {
        self.shared_alerts.clone()
    }

    /// Analyze a page context and return any triggered alerts.
    /// This is called synchronously from the render loop.
    pub fn analyze(&mut self, ctx: &PageContext) -> Vec<DetectionAlert> {
        if !self.enabled {
            return Vec::new();
        }

        let now = Instant::now();
        let mut new_alerts = Vec::new();

        for rule in &self.rules {
            if let Some(level) = (rule.pattern)(ctx) {
                // Cooldown check
                if let Some(last) = self.last_alert_time.get(rule.name) {
                    if now.duration_since(*last) < self.cooldown {
                        continue;
                    }
                }

                let score = level.score();
                let alert = DetectionAlert {
                    rule_name: rule.name.to_string(),
                    level,
                    description: rule.description.to_string(),
                    url: ctx.url.clone(),
                    domain: ctx.domain.clone(),
                    timestamp: now,
                    score,
                    honeypot_triggered: rule.name.starts_with("honeypot_"),
                    action: rule.action,
                };

                // Update behavior record
                let entry = self
                    .behaviors
                    .entry(rule.name.to_string())
                    .or_insert(BehaviorRecord {
                        first_seen: now,
                        count: 0,
                        total_score: 0.0,
                        last_event: now,
                    });
                entry.count += 1;
                entry.total_score += score;
                entry.last_event = now;

                // Log first-seen age for persistent tracking
                let _age_secs = now.duration_since(entry.first_seen).as_secs();

                // Update cooldown
                self.last_alert_time.insert(rule.name.to_string(), now);

                // Push to shared MCP queue
                if let Ok(mut shared) = self.shared_alerts.lock() {
                    shared.push(alert.to_mcp_payload());
                }

                self.total_alerts += 1;
                new_alerts.push(alert);
            }
        }

        // Store alerts
        self.alerts.extend(new_alerts.clone());

        // Cap stored alerts at 1000
        if self.alerts.len() > 1000 {
            self.alerts.drain(0..self.alerts.len() - 1000);
        }

        new_alerts
    }

    /// Set up honeypots for a tab at trust levels 0-1
    pub fn setup_honeypots(
        &mut self,
        tab_id: u64,
        trust_level: TrustLevel,
    ) -> Option<HoneypotConfig> {
        if HoneypotConfig::should_activate(trust_level) {
            let config = HoneypotConfig::default();
            self.honeypots.insert(tab_id, config.clone());
            Some(config)
        } else {
            // Remove honeypots when trust increases
            self.honeypots.remove(&tab_id);
            None
        }
    }

    /// Remove honeypots for a closed tab
    pub fn remove_tab(&mut self, tab_id: u64) {
        self.honeypots.remove(&tab_id);
    }

    /// Get honeypot config for a tab
    pub fn get_honeypot(&self, tab_id: u64) -> Option<&HoneypotConfig> {
        self.honeypots.get(&tab_id)
    }

    /// Get recent alerts (for UI display)
    pub fn recent_alerts(&self, count: usize) -> &[DetectionAlert] {
        let start = self.alerts.len().saturating_sub(count);
        &self.alerts[start..]
    }

    /// Get active alert count
    pub fn active_alert_count(&self) -> usize {
        let cutoff = Instant::now() - Duration::from_secs(300); // 5 min window
        self.alerts.iter().filter(|a| a.timestamp > cutoff).count()
    }

    /// Get behavior record for a rule
    pub fn get_behavior(&self, rule_name: &str) -> Option<&BehaviorRecord> {
        self.behaviors.get(rule_name)
    }

    /// Get cumulative score across all behaviors
    pub fn cumulative_score(&self) -> f32 {
        self.behaviors.values().map(|b| b.total_score).sum()
    }

    /// Clear all data (e.g., on new session)
    pub fn clear(&mut self) {
        self.behaviors.clear();
        self.alerts.clear();
        self.honeypots.clear();
        self.last_alert_time.clear();
        self.total_alerts = 0;
    }

    /// Get total alerts emitted since engine start
    pub fn total_alerts(&self) -> u64 {
        self.total_alerts
    }

    /// Drain shared alerts (called by MCP server)
    pub fn drain_mcp_alerts(&self) -> Vec<DetectionAlertPayload> {
        if let Ok(mut shared) = self.shared_alerts.lock() {
            std::mem::take(&mut *shared)
        } else {
            Vec::new()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UI rendering for detection banner (called from app.rs)
// ─────────────────────────────────────────────────────────────────────────────

impl DetectionEngine {
    /// Render security alert banner in the browser UI.
    /// Returns true if there are active alerts shown.
    pub fn render_alert_banner(&mut self, ui: &mut eframe::egui::Ui) -> bool {
        use eframe::egui::{Color32, RichText};

        let cutoff = Instant::now() - Duration::from_secs(60);

        // Collect alert display data into owned values BEFORE entering the UI closure.
        // This avoids the E0500 borrow conflict (recent_alerts borrows self.alerts
        // immutably, but the dismiss button closure needs mutable access).
        struct AlertSnapshot {
            rule_name: String,
            description: String,
            level: SuspicionLevel,
            honeypot_triggered: bool,
        }

        let snapshots: Vec<AlertSnapshot> = {
            let active = self.recent_alerts(5);
            active
                .iter()
                .filter(|a| a.timestamp > cutoff)
                .map(|a| AlertSnapshot {
                    rule_name: a.rule_name.clone(),
                    description: a.description.clone(),
                    level: a.level,
                    honeypot_triggered: a.honeypot_triggered,
                })
                .collect()
        };
        // At this point the borrow from recent_alerts() is dropped

        if snapshots.is_empty() {
            return false;
        }

        let banner_color = if snapshots
            .iter()
            .any(|a| a.level == SuspicionLevel::Critical)
        {
            Color32::from_rgb(180, 40, 40) // Red for critical
        } else if snapshots.iter().any(|a| a.level == SuspicionLevel::High) {
            Color32::from_rgb(200, 120, 40) // Orange for high
        } else {
            Color32::from_rgb(180, 160, 40) // Yellow for medium/low
        };

        let text_color = Color32::WHITE;
        let snap_len = snapshots.len();

        ui.horizontal(|ui| {
            ui.colored_label(
                banner_color,
                RichText::new("SECURITY").strong().color(text_color),
            );

            for alert in snapshots.iter().take(3) {
                let prefix = if alert.honeypot_triggered {
                    "[HP] "
                } else {
                    ""
                };
                ui.separator();
                ui.colored_label(
                    banner_color,
                    RichText::new(format!(
                        "{}{}: {}",
                        prefix, alert.rule_name, alert.description
                    ))
                    .color(text_color)
                    .small(),
                );
            }

            if snap_len > 3 {
                ui.separator();
                ui.colored_label(
                    banner_color,
                    RichText::new(format!("+{} more", snap_len - 3))
                        .color(text_color)
                        .small(),
                );
            }

            // Dismiss button — safe to mutate self.alerts now (no outstanding borrows)
            if ui.small_button("X").clicked() {
                let cutoff_idx = self
                    .alerts
                    .iter()
                    .position(|a| a.timestamp > cutoff)
                    .unwrap_or(self.alerts.len());
                self.alerts.drain(cutoff_idx..);
            }
        });

        true
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn untrusted_ctx() -> PageContext {
        PageContext {
            url: "https://phish-site.com/login".to_string(),
            domain: "phish-site.com".to_string(),
            trust_level: TrustLevel::Untrusted,
            ..Default::default()
        }
    }

    #[test]
    fn test_honeypot_form_triggers_critical() {
        let mut engine = DetectionEngine::new();
        let mut ctx = untrusted_ctx();
        ctx.honeypot_form_touched = true;
        let alerts = engine.analyze(&ctx);
        assert!(alerts
            .iter()
            .any(|a| a.rule_name == "honeypot_form_touched"));
        assert!(alerts.iter().any(|a| a.level == SuspicionLevel::Critical));
    }

    #[test]
    fn test_honeypot_disabled_on_trusted() {
        let mut engine = DetectionEngine::new();
        let mut ctx = untrusted_ctx();
        ctx.trust_level = TrustLevel::Reviewed; // Level 2 — honeypots off
        ctx.honeypot_form_touched = true;
        let alerts = engine.analyze(&ctx);
        assert!(alerts
            .iter()
            .all(|a| a.rule_name != "honeypot_form_touched"));
    }

    #[test]
    fn test_phishing_detection() {
        let mut engine = DetectionEngine::new();
        let mut ctx = untrusted_ctx();
        ctx.login_form_detected = true;
        ctx.phishing_keywords = true;
        let alerts = engine.analyze(&ctx);
        assert!(alerts
            .iter()
            .any(|a| a.rule_name == "credential_phishing_form"));
    }

    #[test]
    fn test_behavior_tracking() {
        let mut engine = DetectionEngine::new();
        // Disable cooldown for test
        engine.cooldown = Duration::from_millis(0);

        let mut ctx = untrusted_ctx();
        ctx.known_trackers = vec!["doubleclick.net".to_string()];

        engine.analyze(&ctx);
        engine.analyze(&ctx);

        let record = engine.get_behavior("known_malicious_tracker").unwrap();
        assert_eq!(record.count, 2);
        assert!(record.total_score > 2.9);
    }

    #[test]
    fn test_cooldown_prevents_spam() {
        let mut engine = DetectionEngine::new();
        engine.cooldown = Duration::from_secs(60); // Long cooldown

        let mut ctx = untrusted_ctx();
        ctx.canvas_calls = true;

        let alerts1 = engine.analyze(&ctx);
        let alerts2 = engine.analyze(&ctx); // Should be cooldown-blocked

        assert!(!alerts1.is_empty());
        // Second call should have fewer alerts (cooldown active)
        assert!(alerts2.len() < alerts1.len() || alerts2.is_empty());
    }

    #[test]
    fn test_shared_alerts_for_mcp() {
        let mut engine = DetectionEngine::new();
        let shared = engine.shared_alerts();

        let mut ctx = untrusted_ctx();
        ctx.honeypot_link_followed = true;
        engine.analyze(&ctx);

        let mcp_alerts = shared.lock().unwrap();
        assert!(!mcp_alerts.is_empty());
        assert!(mcp_alerts[0].honeypot_triggered);
    }

    #[test]
    fn test_suspicion_level_to_u8() {
        assert_eq!(SuspicionLevel::None.to_u8(), 0);
        assert_eq!(SuspicionLevel::Low.to_u8(), 1);
        assert_eq!(SuspicionLevel::Medium.to_u8(), 2);
        assert_eq!(SuspicionLevel::High.to_u8(), 3);
        assert_eq!(SuspicionLevel::Critical.to_u8(), 4);
    }

    #[test]
    fn test_detection_action_to_u8() {
        assert_eq!(DetectionAction::LogOnly.to_u8(), 0);
        assert_eq!(DetectionAction::WarnUser.to_u8(), 1);
        assert_eq!(DetectionAction::BlockResource.to_u8(), 2);
        assert_eq!(DetectionAction::QuarantinePage.to_u8(), 3);
        assert_eq!(DetectionAction::ResetTrust.to_u8(), 4);
        assert_eq!(DetectionAction::NotifyMcp.to_u8(), 5);
    }

    #[test]
    fn test_detection_alert_payload_fields() {
        let mut engine = DetectionEngine::new();
        engine.cooldown = Duration::from_millis(0);
        let mut ctx = untrusted_ctx();
        ctx.honeypot_form_touched = true;
        let alerts = engine.analyze(&ctx);
        let payload = alerts[0].to_mcp_payload();
        // Exercise all DetectionAlertPayload fields
        assert!(!payload.rule_name.is_empty());
        assert_eq!(payload.level, SuspicionLevel::Critical);
        assert!(!payload.description.is_empty());
        assert!(!payload.url.is_empty());
        assert!(!payload.domain.is_empty());
        assert!(payload.score > 0.0);
        assert!(payload.honeypot_triggered);
        let _ = payload.action;
    }

    #[test]
    fn test_behavior_record_fields() {
        let mut engine = DetectionEngine::new();
        engine.cooldown = Duration::from_millis(0);
        let mut ctx = untrusted_ctx();
        ctx.known_trackers = vec!["doubleclick.net".to_string()];
        engine.analyze(&ctx);
        let record = engine.get_behavior("known_malicious_tracker").unwrap();
        // Exercise all BehaviorRecord fields
        let _ = record.first_seen;
        assert!(record.count > 0);
        assert!(record.total_score > 0.0);
        let _ = record.last_event;
    }

    #[test]
    fn test_page_context_all_fields() {
        let mut ctx = PageContext::default();
        // Exercise all PageContext fields
        ctx.url = "https://test.com".to_string();
        ctx.domain = "test.com".to_string();
        ctx.trust_level = TrustLevel::Untrusted;
        ctx.headers
            .insert("Content-Type".into(), "text/html".into());
        ctx.scripts_src
            .push("https://cdn.example.com/script.js".into());
        ctx.inline_script_hashes.push("sha256-abc".into());
        ctx.iframes.push("https://frame.example.com".into());
        ctx.canvas_calls = true;
        ctx.webgl_calls = true;
        ctx.battery_api_calls = true;
        ctx.clipboard_read_attempts = 3;
        ctx.known_trackers.push("tracker.example.com".into());
        ctx.login_form_detected = true;
        ctx.phishing_keywords = true;
        ctx.crypto_miner_indicators = true;
        ctx.honeypot_form_touched = true;
        ctx.honeypot_link_followed = true;
        ctx.honeypot_storage_read = true;
        ctx.honeypot_cookie_exfiltrated = true;
        ctx.honeypot_canvas_probed = true;
        ctx.redirect_chain_length = 6;
        let _ = ctx.last_updated;
    }

    #[test]
    fn test_honeypot_config_generate_html_js() {
        let config = HoneypotConfig::default();
        let html = config.generate_injection_html();
        assert!(html.contains("sassy_hp_"));
        let js = config.generate_injection_js();
        assert!(js.contains("sassy_hp_"));
        // Test should_activate
        assert!(HoneypotConfig::should_activate(TrustLevel::Untrusted));
        assert!(HoneypotConfig::should_activate(TrustLevel::Acknowledged));
        assert!(!HoneypotConfig::should_activate(TrustLevel::Reviewed));
    }

    #[test]
    fn test_drain_mcp_alerts() {
        let mut engine = DetectionEngine::new();
        let mut ctx = untrusted_ctx();
        ctx.honeypot_canvas_probed = true;
        engine.analyze(&ctx);
        let drained = engine.drain_mcp_alerts();
        assert!(!drained.is_empty());
        // Second drain should be empty
        let drained2 = engine.drain_mcp_alerts();
        assert!(drained2.is_empty());
    }

    #[test]
    fn test_engine_clear_and_total() {
        let mut engine = DetectionEngine::new();
        let mut ctx = untrusted_ctx();
        ctx.canvas_calls = true;
        engine.analyze(&ctx);
        assert!(engine.total_alerts() > 0);
        assert!(engine.cumulative_score() > 0.0);
        assert!(engine.active_alert_count() > 0);
        let _ = engine.recent_alerts(10);
        engine.clear();
        assert_eq!(engine.total_alerts(), 0);
    }

    #[test]
    fn test_honeypot_setup_and_get() {
        let mut engine = DetectionEngine::new();
        let config = engine.setup_honeypots(42, TrustLevel::Untrusted);
        assert!(config.is_some());
        assert!(engine.get_honeypot(42).is_some());
        // Removes on established level (trusted enough to disengage honeypots)
        let config2 = engine.setup_honeypots(42, TrustLevel::Established);
        assert!(config2.is_none());
        assert!(engine.get_honeypot(42).is_none());
        // Remove tab
        engine.setup_honeypots(99, TrustLevel::Untrusted);
        engine.remove_tab(99);
        assert!(engine.get_honeypot(99).is_none());
    }

    #[test]
    fn test_suspicion_level_to_violation_severity() {
        // None maps to None
        assert!(SuspicionLevel::None.to_violation_severity().is_none());
        // Low maps to Some(Low)
        let low = SuspicionLevel::Low.to_violation_severity();
        assert!(low.is_some());
        // Medium maps to Some(Medium)
        let med = SuspicionLevel::Medium.to_violation_severity();
        assert!(med.is_some());
        // High maps to Some(High)
        let high = SuspicionLevel::High.to_violation_severity();
        assert!(high.is_some());
        // Critical maps to Some(Critical)
        let crit = SuspicionLevel::Critical.to_violation_severity();
        assert!(crit.is_some());
    }
}
