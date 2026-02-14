#![allow(dead_code)]
//! Stealth Victories — Silent Anti-Tracking Warfare
//!
//! FOUR SILENT VICTORIES:
//! ─────────────────────────────────────────────────────────────────────────
//! 1. Silent Kill — Corrupt tracking data without any visible trace
//! 2. Honey-Pot Redirect — Redirect tracking pixels to internal sink
//! 3. Fake Tracking ID — Return plausible but useless IDs
//! 4. Entropy Bomb — Flood trackers with garbage entropy
//!
//! No leaks to trackers. Only internal logging & counter.
//! The user sees "Sites poisoned: 47" in the status bar — that's it.
//!
//! All data persisted to disk between sessions for the running tally.

use rand::Rng;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufWriter, Read};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════════
// STATS — Persistent counters
// ═══════════════════════════════════════════════════════════════════════════════

/// Persistent stealth statistics — serialized to disk between sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealthStats {
    /// Domain → count of times we poisoned tracking for this domain
    pub poisoned_sites: HashMap<String, u32>,
    /// Total poisoning events across all domains
    pub total_poisoned: u32,
    /// Timestamp of last poisoning event (epoch seconds)
    pub last_poison_epoch: Option<u64>,
}

impl Default for StealthStats {
    fn default() -> Self {
        Self {
            poisoned_sites: HashMap::new(),
            total_poisoned: 0,
            last_poison_epoch: None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// STEALTH VICTORIES — Main engine
// ═══════════════════════════════════════════════════════════════════════════════

pub struct StealthVictories {
    stats: Arc<Mutex<StealthStats>>,
    data_path: PathBuf,
    rng: Mutex<rand::rngs::ThreadRng>,
}

impl StealthVictories {
    /// Create and load previous stats from disk (if available)
    pub fn new() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("./data"))
            .join("sassy-browser");
        fs::create_dir_all(&data_dir).ok();

        let data_path = data_dir.join("stealth_stats.json");

        let mut stats = StealthStats::default();

        // Load previous stats from disk
        if data_path.exists() {
            if let Ok(mut file) = File::open(&data_path) {
                let mut contents = String::new();
                if file.read_to_string(&mut contents).is_ok() {
                    if let Ok(loaded) = serde_json::from_str::<StealthStats>(&contents) {
                        stats = loaded;
                    }
                }
            }
        }

        Self {
            stats: Arc::new(Mutex::new(stats)),
            data_path,
            rng: Mutex::new(rand::thread_rng()),
        }
    }

    /// Persist stats to disk
    pub fn save(&self) -> io::Result<()> {
        let file = File::create(&self.data_path)?;
        let writer = BufWriter::new(file);
        let stats = self.stats.lock().unwrap();
        serde_json::to_writer_pretty(writer, &*stats)?;
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────
    // VICTORY 1: Silent Kill
    // Corrupt tracking data without any visible trace
    // ─────────────────────────────────────────────────────────────────────

    /// Record a silent kill — tracking data was corrupted for this URL.
    /// Called after poisoning.rs does its work. This just counts silently.
    pub fn silent_kill(&self, url: &str) {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(domain) = parsed.host_str() {
                let mut stats = self.stats.lock().unwrap();
                *stats.poisoned_sites.entry(domain.to_string()).or_insert(0) += 1;
                stats.total_poisoned += 1;
                stats.last_poison_epoch = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                );
                drop(stats); // Unlock before I/O
                let _ = self.save();
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // VICTORY 2: Honey-Pot Redirect
    // Redirect tracking pixels to internal sink (return blank GIF / 204)
    // ─────────────────────────────────────────────────────────────────────

    /// Check if a URL is a known tracking pixel/request.
    /// Returns true if it should be intercepted (e.g. return blank GIF or 204).
    pub fn honey_pot_redirect(&self, url: &str) -> bool {
        let lower = url.to_lowercase();
        if lower.contains("pixel.gif")
            || lower.contains("/track")
            || lower.contains("/analytics")
            || lower.contains("beacon.gif")
            || lower.contains("__utm.gif")
            || lower.contains("collect?v=")
            || lower.contains("pixel?")
        {
            self.silent_kill(url);
            return true;
        }
        false
    }

    // ─────────────────────────────────────────────────────────────────────
    // VICTORY 3: Fake Tracking ID
    // Return plausible but useless tracking IDs
    // ─────────────────────────────────────────────────────────────────────

    /// Mutate a tracking ID to break cross-site linking.
    /// Returns a plausible-looking but different ID.
    pub fn fake_tracking_id(&self, original_id: &str) -> String {
        let mut id = original_id.to_string();
        if id.len() > 6 {
            let pos = id.len() - 4;
            let random_suffix = {
                let mut rng = self.rng.lock().unwrap();
                format!("{:04x}", rng.gen::<u16>())
            };
            id.replace_range(pos.., &random_suffix);
        }
        id
    }

    // ─────────────────────────────────────────────────────────────────────
    // VICTORY 4: Entropy Bomb
    // Flood trackers with garbage entropy via JS injection
    // ─────────────────────────────────────────────────────────────────────

    /// Generate the JS code for an entropy bomb.
    /// Injects noise into Math.random and performance.now — making
    /// entropy-based fingerprinting collect useless data.
    pub fn entropy_bomb_script() -> &'static str {
        r#"
        // Entropy bomb — flood trackers with garbage
        (function() {
            // Corrupt Math.random with micro-noise
            const _realRandom = Math.random;
            Math.random = function() {
                return _realRandom() + (_realRandom() - 0.5) * 0.0001;
            };

            // Corrupt performance.now with sub-ms jitter
            const _realNow = performance.now.bind(performance);
            performance.now = function() {
                return _realNow() + (Math.random() - 0.5) * 0.8;
            };

            // Corrupt Date.now with micro-jitter
            const _realDateNow = Date.now;
            Date.now = function() {
                return _realDateNow() + Math.floor((Math.random() - 0.5) * 2);
            };
        })();
        "#
    }

    /// Execute the entropy bomb on a JS interpreter.
    /// Call this on page load when aggressive mode is active.
    pub fn execute_entropy_bomb(&self, js: &mut crate::js::interpreter::JsInterpreter, url: &str) {
        let script = Self::entropy_bomb_script();
        let _ = js.execute(script);
        self.silent_kill(url);
    }

    // ─────────────────────────────────────────────────────────────────────
    // QUERY — For UI display
    // ─────────────────────────────────────────────────────────────────────

    /// Get total count for UI display ("Sites poisoned: 47")
    pub fn poisoned_count(&self) -> u32 {
        self.stats.lock().unwrap().total_poisoned
    }

    /// Get per-domain count
    pub fn poisoned_for_domain(&self, domain: &str) -> u32 {
        self.stats.lock().unwrap()
            .poisoned_sites
            .get(domain)
            .copied()
            .unwrap_or(0)
    }

    /// Get the shared stats handle (for UI rendering)
    pub fn stats(&self) -> Arc<Mutex<StealthStats>> {
        self.stats.clone()
    }

    /// Get top N poisoned domains (for UI display)
    pub fn top_poisoned_domains(&self, n: usize) -> Vec<(String, u32)> {
        let stats = self.stats.lock().unwrap();
        let mut domains: Vec<_> = stats.poisoned_sites.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        domains.sort_by(|a, b| b.1.cmp(&a.1));
        domains.truncate(n);
        domains
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silent_kill_increments() {
        let sv = StealthVictories::new();
        sv.silent_kill("https://facebook.com/pixel");
        sv.silent_kill("https://facebook.com/track");
        sv.silent_kill("https://google.com/analytics");

        assert_eq!(sv.poisoned_count(), 3);
        assert_eq!(sv.poisoned_for_domain("facebook.com"), 2);
        assert_eq!(sv.poisoned_for_domain("google.com"), 1);
    }

    #[test]
    fn test_honey_pot_redirect() {
        let sv = StealthVictories::new();

        assert!(sv.honey_pot_redirect("https://tracker.com/pixel.gif?id=abc"));
        assert!(sv.honey_pot_redirect("https://analytics.com/track"));
        assert!(!sv.honey_pot_redirect("https://example.com/index.html"));
    }

    #[test]
    fn test_fake_tracking_id() {
        let sv = StealthVictories::new();
        let original = "abc123def456";
        let faked = sv.fake_tracking_id(original);

        // Same length
        assert_eq!(faked.len(), original.len());
        // Last 4 chars should be different
        assert_ne!(&faked[faked.len()-4..], &original[original.len()-4..]);
    }

    #[test]
    fn test_top_poisoned_domains() {
        let sv = StealthVictories::new();
        for _ in 0..5 { sv.silent_kill("https://facebook.com/pixel"); }
        for _ in 0..3 { sv.silent_kill("https://google.com/analytics"); }
        sv.silent_kill("https://twitter.com/track");

        let top = sv.top_poisoned_domains(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "facebook.com");
        assert_eq!(top[0].1, 5);
        assert_eq!(top[1].0, "google.com");
    }
}
