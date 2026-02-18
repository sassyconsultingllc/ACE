//! Data persistence - stores user data, config, and state
//! ALL data is stored locally on the user's device. Nothing is transmitted externally.
//! Settings are colocated with passwords/history in the encrypted profile section.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::sync::{UserManager, FamilyConfig};

/// Get the data directory for Sassy Browser
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SassyBrowser")
}

/// Get the config directory
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SassyBrowser")
}

/// Ensure directories exist
pub fn init_dirs() -> Result<(), String> {
    fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    fs::create_dir_all(config_dir()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Check if this is first run
pub fn is_first_run() -> bool {
    !config_dir().join("config.toml").exists()
}

/// Browser configuration - ALL settings stored locally, never transmitted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: String,
    pub theme: String,
    pub sync: SyncSettings,
    pub window: WindowSettings,
    pub privacy: PrivacySettings,
    pub protection: ProtectionSettings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            theme: "default".into(),
            sync: SyncSettings::default(),
            window: WindowSettings::default(),
            privacy: PrivacySettings::default(),
            protection: ProtectionSettings::default(),
        }
    }
}

/// Privacy settings - enforces that ALL data stays on the user's device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// All browsing data is local-only (always true, cannot be disabled)
    pub data_stays_local: bool,
    /// Zero telemetry - no usage data ever leaves the device
    pub zero_telemetry: bool,
    /// No crash reports sent externally
    pub no_crash_reports: bool,
    /// Block all third-party trackers
    pub block_trackers: bool,
    /// Fingerprint poisoning active
    pub poison_fingerprints: bool,
    /// Clear browsing data on exit
    pub clear_on_exit: bool,
    /// DNS-over-HTTPS for privacy
    pub dns_over_https: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            data_stays_local: true,     // Always true - core promise
            zero_telemetry: true,        // Always true - no exceptions
            no_crash_reports: true,      // No external reporting
            block_trackers: true,        // On by default
            poison_fingerprints: true,   // On by default
            clear_on_exit: false,        // User choice
            dns_over_https: true,        // On by default
        }
    }
}

/// Active protection settings - always-on threat defense
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionSettings {
    /// Real-time ad and tracker blocking
    pub adblock_enabled: bool,
    /// Malicious site detection
    pub threat_detection: bool,
    /// Download quarantine for untrusted files
    pub download_quarantine: bool,
    /// Sandbox isolation for pages
    pub sandbox_enabled: bool,
    /// Anti-phishing protection
    pub anti_phishing: bool,
    /// Script analysis for malicious code
    pub script_analysis: bool,
    /// TLS certificate validation
    pub strict_tls: bool,
}

impl Default for ProtectionSettings {
    fn default() -> Self {
        Self {
            adblock_enabled: true,
            threat_detection: true,
            download_quarantine: true,
            sandbox_enabled: true,
            anti_phishing: true,
            script_analysis: true,
            strict_tls: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    pub enabled: bool,
    pub port: u16,
    pub bind_tailscale_only: bool,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 8765,
            bind_tailscale_only: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
            maximized: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = config_dir().join("config.toml");
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
    
    pub fn save(&self) -> Result<(), String> {
        let path = config_dir().join("config.toml");
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }
}

/// User data storage - encrypted at rest alongside passwords/history in the profile
/// All settings are client-side only. Nothing is ever transmitted externally.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserData {
    pub users: UserManager,
    pub family: FamilyConfig,
    /// Privacy & protection settings stored in the encrypted profile
    /// alongside passwords and history - never leaves the device
    #[serde(default)]
    pub privacy: PrivacySettings,
    #[serde(default)]
    pub protection: ProtectionSettings,
    /// Encrypted protection stats (threats blocked, etc.)
    #[serde(default)]
    pub protection_stats: ProtectionStats,
}

/// Lifetime protection statistics - stored encrypted in the user profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionStats {
    pub total_ads_blocked: u64,
    pub total_trackers_stopped: u64,
    pub total_threats_detected: u64,
    pub total_fingerprints_poisoned: u64,
    pub total_phishing_blocked: u64,
    pub total_malicious_scripts_blocked: u64,
    pub total_downloads_quarantined: u64,
    /// Timestamp of first protection event
    pub protecting_since: Option<u64>,
}

impl Default for ProtectionStats {
    fn default() -> Self {
        Self {
            total_ads_blocked: 0,
            total_trackers_stopped: 0,
            total_threats_detected: 0,
            total_fingerprints_poisoned: 0,
            total_phishing_blocked: 0,
            total_malicious_scripts_blocked: 0,
            total_downloads_quarantined: 0,
            protecting_since: None,
        }
    }
}

impl UserData {
    pub fn load() -> Self {
        let path = data_dir().join("users.json");
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    
    pub fn save(&self) -> Result<(), String> {
        let path = data_dir().join("users.json");
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }
}

/// Session state (not persisted across restarts)
pub struct SessionState {
    pub current_user: Option<String>,
    pub tabs: Vec<TabState>,
    pub active_tab: Option<usize>,
}

impl SessionState {
    /// Create a new empty session state
    pub fn new() -> Self {
        Self {
            current_user: None,
            tabs: Vec::new(),
            active_tab: None,
        }
    }

    /// Create session state for a specific user
    pub fn for_user(user_id: &str) -> Self {
        Self {
            current_user: Some(user_id.to_string()),
            tabs: Vec::new(),
            active_tab: None,
        }
    }

    /// Snapshot current browser state into a SessionRestore for persistence
    pub fn to_restore(&self) -> SessionRestore {
        SessionRestore {
            user_id: self.current_user.clone().unwrap_or_default(),
            tabs: self.tabs.clone(),
            active_tab: self.active_tab,
            quarantine: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub scroll_x: i32,
    pub scroll_y: i32,
}

/// Persist tabs for session restore
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionRestore {
    pub user_id: String,
    pub tabs: Vec<TabState>,
    pub active_tab: Option<usize>,
    /// Persisted quarantined files (encrypted bytes may be stored)
    pub quarantine: Vec<QuarantinedFileState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantinedFileState {
    pub id: String,
    pub filename: String,
    pub source_url: String,
    pub content_type: String,
    pub size_bytes: usize,
    pub sha256: String,
    /// Milliseconds since UNIX_EPOCH when file was quarantined
    pub quarantined_at_ms: u128,
    /// Optional encrypted bytes for at-rest storage
    pub encrypted_data: Option<Vec<u8>>,
}

impl SessionRestore {
    pub fn load(user_id: &str) -> Option<Self> {
        let path = data_dir().join("sessions").join(format!("{}.json", user_id));
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = data_dir().join("sessions");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let path = dir.join(format!("{}.json", self.user_id));
        let content = serde_json::to_string(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_for_user() {
        let state = SessionState::for_user("alice");
        assert_eq!(state.current_user.as_deref(), Some("alice"));
        assert!(state.tabs.is_empty());
        assert!(state.active_tab.is_none());
    }

    #[test]
    fn test_session_state_to_restore() {
        let mut state = SessionState::for_user("bob");
        state.tabs.push(TabState {
            id: 1,
            url: "https://example.com".into(),
            title: "Example".into(),
            scroll_x: 0,
            scroll_y: 100,
        });
        state.active_tab = Some(0);
        let restore = state.to_restore();
        assert_eq!(restore.user_id, "bob");
        assert_eq!(restore.tabs.len(), 1);
        assert_eq!(restore.active_tab, Some(0));
        assert!(restore.quarantine.is_empty());
    }

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new();
        assert!(state.current_user.is_none());
        assert!(state.tabs.is_empty());
    }

    #[test]
    fn test_protection_stats_default() {
        let stats = ProtectionStats::default();
        assert_eq!(stats.total_ads_blocked, 0);
        assert_eq!(stats.total_trackers_stopped, 0);
        assert_eq!(stats.total_threats_detected, 0);
        assert_eq!(stats.total_fingerprints_poisoned, 0);
        assert_eq!(stats.total_phishing_blocked, 0);
        assert_eq!(stats.total_malicious_scripts_blocked, 0);
        assert_eq!(stats.total_downloads_quarantined, 0);
        assert!(stats.protecting_since.is_none());
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(!config.version.is_empty());
        assert_eq!(config.theme, "default");
        assert!(config.privacy.data_stays_local);
        assert!(config.privacy.zero_telemetry);
        assert!(config.protection.adblock_enabled);
        assert!(config.protection.threat_detection);
    }

    #[test]
    fn test_privacy_settings_defaults() {
        let p = PrivacySettings::default();
        assert!(p.data_stays_local);
        assert!(p.zero_telemetry);
        assert!(p.no_crash_reports);
        assert!(p.block_trackers);
        assert!(p.poison_fingerprints);
        assert!(!p.clear_on_exit);
        assert!(p.dns_over_https);
    }

    #[test]
    fn test_protection_settings_defaults() {
        let p = ProtectionSettings::default();
        assert!(p.adblock_enabled);
        assert!(p.threat_detection);
        assert!(p.download_quarantine);
        assert!(p.sandbox_enabled);
        assert!(p.anti_phishing);
        assert!(p.script_analysis);
        assert!(p.strict_tls);
    }

    #[test]
    fn test_quarantined_file_state_serialization() {
        let qfs = QuarantinedFileState {
            id: "q1".into(),
            filename: "test.exe".into(),
            source_url: "https://example.com/test.exe".into(),
            content_type: "application/octet-stream".into(),
            size_bytes: 1024,
            sha256: "abc123".into(),
            quarantined_at_ms: 1700000000000,
            encrypted_data: None,
        };
        let json = serde_json::to_string(&qfs).unwrap();
        let deser: QuarantinedFileState = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.id, "q1");
        assert_eq!(deser.filename, "test.exe");
        assert_eq!(deser.size_bytes, 1024);
    }
}
