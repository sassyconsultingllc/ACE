//! Data persistence - stores user data, config, and state

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[allow(unused_imports)]
use crate::sync::{UserManager, UserProfile, FamilyConfig};

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

/// Browser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: String,
    pub theme: String,
    pub sync: SyncSettings,
    pub window: WindowSettings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            theme: "default".into(),
            sync: SyncSettings::default(),
            window: WindowSettings::default(),
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

/// User data storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserData {
    pub users: UserManager,
    pub family: FamilyConfig,
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
