//! Hunter Mode -- Active Anti-Tracker Warfare
//!
//! NOTE: This module is NOT wired into main.rs. It exists as a standalone
//! implementation ready to be activated when the feature is integrated.
//!
//! FIVE HUNT ACTIONS:
//! ---------------------------------------------------------------------------
//! 1. Payload Poisoned    -- Flip bits in tracker payloads to corrupt data
//! 2. Honey-Pot Redirect  -- Redirect tracker URLs to HTTP 204 (no content)
//! 3. Session Token Mutated -- Mutate the last 4 chars of tracking tokens
//! 4. Entropy Flooded     -- Inject JS to corrupt Math.random, performance.now
//! 5. Tracker Blocked     -- Outright block known tracker domains
//!
//! All hunt events are logged to disk with timestamps and per-domain tallies.
//! The UI can display "Trackers hunted: 142" in the status bar.
//!
//! Unlike `stealth_victories` which operates passively, Hunter Mode is an
//! opt-in aggressive stance the user explicitly enables.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufWriter, Read};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use url::Url;

/// Type alias for a simple message channel sender.
/// Stands in for a full MCP bridge -- messages are plain strings
/// describing hunt events (e.g. "HUNT:PayloadPoisoned:example.com").
pub type McpBridgeSender = tokio::sync::mpsc::Sender<String>;

// =============================================================================
// KNOWN TRACKER DOMAINS
// =============================================================================

/// Hardcoded list of tracker domains. Checked via suffix match so that
/// subdomains (e.g. "px.ads.linkedin.com") are also caught.
const KNOWN_TRACKER_DOMAINS: &[&str] = &[
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "google-analytics.com",
    "googletagmanager.com",
    "facebook.net",
    "facebook.com/tr",
    "analytics.twitter.com",
    "ads.linkedin.com",
    "bat.bing.com",
    "pixel.quantserve.com",
    "scorecardresearch.com",
    "adnxs.com",
    "criteo.com",
    "taboola.com",
    "outbrain.com",
    "amazon-adsystem.com",
    "hotjar.com",
    "clarity.ms",
    "newrelic.com",
    "segment.io",
    "mixpanel.com",
    "amplitude.com",
    "fullstory.com",
    "mouseflow.com",
    "crazyegg.com",
];

// =============================================================================
// HUNT ACTION
// =============================================================================

/// The type of anti-tracker action that was taken.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HuntAction {
    /// Tracker payload data was bit-flipped to corrupt it
    PayloadPoisoned,
    /// Tracker URL was redirected to a 204 No Content sink
    HoneyPotRedirected,
    /// Session/tracking token had its tail mutated
    SessionTokenMutated,
    /// JS entropy sources were flooded with noise
    EntropyFlooded,
    /// Tracker request was outright blocked
    TrackerBlocked,
}

impl std::fmt::Display for HuntAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HuntAction::PayloadPoisoned => write!(f, "PayloadPoisoned"),
            HuntAction::HoneyPotRedirected => write!(f, "HoneyPotRedirected"),
            HuntAction::SessionTokenMutated => write!(f, "SessionTokenMutated"),
            HuntAction::EntropyFlooded => write!(f, "EntropyFlooded"),
            HuntAction::TrackerBlocked => write!(f, "TrackerBlocked"),
        }
    }
}

// =============================================================================
// HUNT RECORD
// =============================================================================

/// A single recorded hunt event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuntRecord {
    /// The domain that was hunted (e.g. "doubleclick.net")
    pub domain: String,
    /// Unix epoch seconds when the hunt occurred
    pub timestamp: u64,
    /// Which action was taken
    pub action: HuntAction,
    /// The full URL that triggered the hunt
    pub target_url: String,
    /// Whether the action succeeded
    pub success: bool,
}

// =============================================================================
// HUNTER MODE -- Main engine
// =============================================================================

/// Active anti-tracker warfare engine.
///
/// When enabled, intercepts requests to known tracker domains and applies
/// one of five hunt actions. All events are logged to disk and optionally
/// forwarded over an MCP message channel.
pub struct HunterMode {
    /// Whether hunter mode is currently active
    enabled: bool,
    /// Set of domains the user has explicitly targeted (in addition to defaults)
    hunted_domains: HashSet<String>,
    /// Full hunt history for the current session, plus loaded from disk
    hunt_history: Arc<Mutex<Vec<HuntRecord>>>,
    /// Running total of successful hunts (cached for fast UI reads)
    hunt_counter: Arc<Mutex<u64>>,
    /// Path to the JSON file where hunt history is persisted
    data_path: PathBuf,
    /// Optional MCP bridge sender for forwarding hunt events
    mcp_sender: Option<McpBridgeSender>,
    /// Thread-local RNG behind a mutex for cross-thread safety
    rng: Mutex<rand::rngs::ThreadRng>,
}

impl HunterMode {
    /// Create a new HunterMode instance.
    ///
    /// Loads previous hunt history from disk (if available) and initialises
    /// the running counter. Hunter mode starts **disabled** -- call
    /// [`enable()`](Self::enable) to activate.
    pub fn new(mcp_sender: Option<McpBridgeSender>) -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("./data"))
            .join("sassy-browser");
        fs::create_dir_all(&data_dir).ok();

        let data_path = data_dir.join("hunter_mode.json");

        let mut history: Vec<HuntRecord> = Vec::new();

        // Load previous history from disk
        if data_path.exists() {
            if let Ok(mut file) = File::open(&data_path) {
                let mut contents = String::new();
                if file.read_to_string(&mut contents).is_ok() {
                    if let Ok(loaded) = serde_json::from_str::<Vec<HuntRecord>>(&contents) {
                        history = loaded;
                    }
                }
            }
        }

        let counter = history.iter().filter(|r| r.success).count() as u64;

        Self {
            enabled: false,
            hunted_domains: HashSet::new(),
            hunt_history: Arc::new(Mutex::new(history)),
            hunt_counter: Arc::new(Mutex::new(counter)),
            data_path,
            mcp_sender,
            rng: Mutex::new(rand::thread_rng()),
        }
    }

    // -------------------------------------------------------------------------
    // Enable / Disable
    // -------------------------------------------------------------------------

    /// Activate hunter mode. All subsequent requests will be checked against
    /// the tracker list and hunted if they match.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Deactivate hunter mode. No further hunt actions will be taken.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check whether hunter mode is currently active.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    // -------------------------------------------------------------------------
    // Persistence
    // -------------------------------------------------------------------------

    /// Persist the full hunt history to disk as pretty-printed JSON.
    pub fn save(&self) -> io::Result<()> {
        let file = File::create(&self.data_path)?;
        let writer = BufWriter::new(file);
        let history = self.hunt_history.lock().unwrap();
        serde_json::to_writer_pretty(writer, &*history)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Logging
    // -------------------------------------------------------------------------

    /// Record a hunt event in history, bump the counter, persist to disk,
    /// and optionally forward a message over the MCP bridge.
    fn log_hunt(&self, record: HuntRecord) {
        if record.success {
            let mut counter = self.hunt_counter.lock().unwrap();
            *counter += 1;
        }

        // Build MCP message before acquiring history lock
        let msg = format!(
            "HUNT:{}:{}:{}",
            record.action, record.domain, record.success
        );

        {
            let mut history = self.hunt_history.lock().unwrap();
            history.push(record);
        }

        let _ = self.save();

        // Fire-and-forget on the MCP channel
        if let Some(ref sender) = self.mcp_sender {
            let _ = sender.try_send(msg);
        }
    }

    // -------------------------------------------------------------------------
    // Queries (for UI)
    // -------------------------------------------------------------------------

    /// Total number of successful hunt actions (for status bar display).
    pub fn poisoned_count(&self) -> u64 {
        *self.hunt_counter.lock().unwrap()
    }

    // -------------------------------------------------------------------------
    // Tracker detection helper
    // -------------------------------------------------------------------------

    /// Check if a URL belongs to a known tracker domain.
    ///
    /// Matches against the hardcoded tracker list AND any user-added domains
    /// in `hunted_domains`. Uses suffix matching so subdomains are caught.
    pub fn is_known_tracker(&self, url: &str) -> bool {
        let domain = match extract_domain(url) {
            Some(d) => d,
            None => return false,
        };

        let lower = domain.to_lowercase();

        // Check hardcoded list
        for tracker in KNOWN_TRACKER_DOMAINS {
            if lower.ends_with(tracker) {
                return true;
            }
        }

        // Check user-added domains
        for custom in &self.hunted_domains {
            if lower.ends_with(custom.as_str()) {
                return true;
            }
        }

        false
    }

    /// Add a custom domain to the hunt list.
    pub fn add_hunted_domain(&mut self, domain: String) {
        self.hunted_domains.insert(domain.to_lowercase());
    }

    // =========================================================================
    // VICTORY 1: Poison Tracker Payload
    // Flip a single bit in the tracker payload to silently corrupt it.
    // =========================================================================

    /// Flip one random bit in a tracker payload byte slice.
    ///
    /// The mutation is tiny enough that the tracker endpoint accepts the
    /// request (no HTTP 400), but the recorded data is garbage. Returns
    /// the modified payload and whether the operation succeeded.
    ///
    /// This is a synchronous operation -- no async needed since we only
    /// mutate bytes in memory.
    pub fn poison_tracker_payload(&self, url: &str, payload: &mut [u8]) -> bool {
        if !self.enabled || payload.is_empty() {
            return false;
        }

        let success = if !payload.is_empty() {
            let mut rng = self.rng.lock().unwrap();
            let byte_idx = rng.gen_range(0..payload.len());
            let bit_idx = rng.gen_range(0..8u8);
            payload[byte_idx] ^= 1 << bit_idx;
            true
        } else {
            false
        };

        let domain = extract_domain(url).unwrap_or_default();
        self.log_hunt(HuntRecord {
            domain,
            timestamp: epoch_now(),
            action: HuntAction::PayloadPoisoned,
            target_url: url.to_string(),
            success,
        });

        success
    }

    // =========================================================================
    // VICTORY 2: Honey-Pot Intercept
    // Redirect tracker URLs to a 204 No Content response.
    // =========================================================================

    /// Check if a URL should be honey-pot intercepted.
    ///
    /// Returns `true` if the URL is a known tracker and should receive a
    /// 204 No Content response instead of reaching the real server. The
    /// caller is responsible for actually returning the 204.
    pub fn honey_pot_intercept(&self, url: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let is_tracker = self.is_known_tracker(url);
        if !is_tracker {
            return false;
        }

        let domain = extract_domain(url).unwrap_or_default();
        self.log_hunt(HuntRecord {
            domain,
            timestamp: epoch_now(),
            action: HuntAction::HoneyPotRedirected,
            target_url: url.to_string(),
            success: true,
        });

        true
    }

    // =========================================================================
    // VICTORY 3: Mutate Tracking Token
    // Replace the last 4 characters of a tracking token with random hex.
    // =========================================================================

    /// Mutate a tracking token to break cross-site linking.
    ///
    /// Replaces the final 4 characters with random hex digits. The token
    /// still looks plausible to the tracker endpoint but links to nobody.
    /// Returns `None` if the token is too short to mutate (< 5 chars).
    pub fn mutate_tracking_token(&self, url: &str, token: &str) -> Option<String> {
        if !self.enabled || token.len() < 5 {
            return None;
        }

        let mut mutated = token.to_string();
        let pos = mutated.len() - 4;
        let random_suffix = {
            let mut rng = self.rng.lock().unwrap();
            format!("{:04x}", rng.gen::<u16>())
        };
        mutated.replace_range(pos.., &random_suffix);

        let domain = extract_domain(url).unwrap_or_default();
        self.log_hunt(HuntRecord {
            domain,
            timestamp: epoch_now(),
            action: HuntAction::SessionTokenMutated,
            target_url: url.to_string(),
            success: true,
        });

        Some(mutated)
    }

    // =========================================================================
    // VICTORY 4: Entropy Bomb
    // Inject JS that corrupts Math.random and performance.now.
    // =========================================================================

    /// The raw JS payload for the entropy bomb. Replaces `Math.random` and
    /// `performance.now` with noisy wrappers that feed garbage to any
    /// entropy-based fingerprinting script.
    const ENTROPY_BOMB_JS: &'static str = r#"
        (function() {
            var _realRandom = Math.random;
            Math.random = function() {
                return _realRandom() + (_realRandom() - 0.5) * 0.0001;
            };

            var _realNow = performance.now.bind(performance);
            performance.now = function() {
                return _realNow() + (Math.random() - 0.5) * 0.8;
            };

            var _realDateNow = Date.now;
            Date.now = function() {
                return _realDateNow() + Math.floor((Math.random() - 0.5) * 2);
            };
        })();
    "#;

    /// Execute the entropy bomb on a SassyScript JS interpreter instance.
    ///
    /// Injects noise into `Math.random`, `performance.now`, and `Date.now`
    /// so that entropy-based fingerprinting collects useless data. Logs the
    /// event regardless of whether the JS execution succeeded.
    pub fn entropy_bomb(
        &self,
        js: &mut crate::js::interpreter::JsInterpreter,
        url: &str,
    ) {
        if !self.enabled {
            return;
        }

        let success = js.execute(Self::ENTROPY_BOMB_JS).is_ok();

        let domain = extract_domain(url).unwrap_or_default();
        self.log_hunt(HuntRecord {
            domain,
            timestamp: epoch_now(),
            action: HuntAction::EntropyFlooded,
            target_url: url.to_string(),
            success,
        });
    }

    // =========================================================================
    // VICTORY 5 (implicit): Tracker Blocked
    // Used when a tracker is outright dropped rather than poisoned.
    // =========================================================================

    /// Log a plain tracker block (request was dropped entirely).
    pub fn log_tracker_blocked(&self, url: &str) {
        if !self.enabled {
            return;
        }

        let domain = extract_domain(url).unwrap_or_default();
        self.log_hunt(HuntRecord {
            domain,
            timestamp: epoch_now(),
            action: HuntAction::TrackerBlocked,
            target_url: url.to_string(),
            success: true,
        });
    }
}

// =============================================================================
// HELPERS
// =============================================================================

/// Extract the host/domain string from a URL. Returns `None` for invalid URLs
/// or URLs without a host (e.g. `data:` URIs).
fn extract_domain(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
}

/// Current Unix epoch time in seconds.
fn epoch_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hunter() -> HunterMode {
        let mut h = HunterMode::new(None);
        h.enable();
        h
    }

    #[test]
    fn test_known_tracker_detection() {
        let h = make_hunter();
        assert!(h.is_known_tracker("https://www.googletagmanager.com/gtag/js?id=UA-12345"));
        assert!(h.is_known_tracker("https://pixel.quantserve.com/pixel"));
        assert!(!h.is_known_tracker("https://example.com/page"));
        assert!(!h.is_known_tracker("https://rust-lang.org/"));
    }

    #[test]
    fn test_custom_domain() {
        let mut h = make_hunter();
        assert!(!h.is_known_tracker("https://evil-tracker.example.org/t.gif"));
        h.add_hunted_domain("evil-tracker.example.org".to_string());
        assert!(h.is_known_tracker("https://evil-tracker.example.org/t.gif"));
    }

    #[test]
    fn test_poison_payload() {
        let h = make_hunter();
        let mut payload = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let original = payload.clone();
        let ok = h.poison_tracker_payload("https://doubleclick.net/track", &mut payload);
        assert!(ok);
        // At least one byte should differ (extremely unlikely to flip back)
        assert_ne!(payload, original);
    }

    #[test]
    fn test_poison_empty_payload() {
        let h = make_hunter();
        let mut payload: Vec<u8> = vec![];
        let ok = h.poison_tracker_payload("https://doubleclick.net/track", &mut payload);
        assert!(!ok);
    }

    #[test]
    fn test_honey_pot_intercept() {
        let h = make_hunter();
        assert!(h.honey_pot_intercept("https://doubleclick.net/ad?q=1"));
        assert!(!h.honey_pot_intercept("https://example.com/page"));
    }

    #[test]
    fn test_mutate_token() {
        let h = make_hunter();
        let token = "abc123def456";
        let mutated = h.mutate_tracking_token("https://criteo.com/t", token);
        assert!(mutated.is_some());
        let m = mutated.unwrap();
        assert_eq!(m.len(), token.len());
        // First 8 chars should be unchanged
        assert_eq!(&m[..8], &token[..8]);
        // Last 4 should differ (with overwhelming probability)
        assert_ne!(&m[8..], &token[8..]);
    }

    #[test]
    fn test_mutate_short_token() {
        let h = make_hunter();
        assert!(h.mutate_tracking_token("https://criteo.com/t", "ab").is_none());
    }

    #[test]
    fn test_disabled_does_nothing() {
        let mut h = HunterMode::new(None);
        // Not enabled
        assert!(!h.honey_pot_intercept("https://doubleclick.net/ad"));
        let mut payload = vec![0x01];
        assert!(!h.poison_tracker_payload("https://doubleclick.net/t", &mut payload));
        assert_eq!(payload, vec![0x01]); // unchanged
        assert!(h.mutate_tracking_token("https://criteo.com/t", "abcdefgh").is_none());
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://www.example.com/path?q=1"),
            Some("www.example.com".to_string())
        );
        assert_eq!(extract_domain("not a url"), None);
    }

    #[test]
    fn test_poisoned_count_increments() {
        let h = make_hunter();
        assert_eq!(h.poisoned_count(), 0);
        h.honey_pot_intercept("https://doubleclick.net/ad");
        assert_eq!(h.poisoned_count(), 1);
        h.log_tracker_blocked("https://criteo.com/pixel");
        assert_eq!(h.poisoned_count(), 2);
    }
}
