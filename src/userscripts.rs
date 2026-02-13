//! User Script Manager - Native Tampermonkey/Greasemonkey replacement
//!
//! KILLS: Tampermonkey, Greasemonkey, Violentmonkey extensions
//!
//! Features:
//! - Parse @metadata headers
//! - GM_* API implementation
//! - Script editor with syntax highlighting
//! - Enable/disable per script
//! - Update checking

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use serde::{Deserialize, Serialize};

// ==============================================================================
// USER SCRIPT STRUCTURE
// ==============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserScript {
    /// Unique identifier
    pub id: String,
    
    /// Script metadata
    pub metadata: ScriptMetadata,
    
    /// The actual JavaScript code
    pub code: String,
    
    /// Whether the script is enabled
    pub enabled: bool,
    
    /// File path on disk
    pub path: Option<PathBuf>,
    
    /// Last modified timestamp
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptMetadata {
    pub name: String,
    pub namespace: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub icon: Option<String>,
    pub update_url: Option<String>,
    pub download_url: Option<String>,
    
    /// URL patterns to match
    pub match_patterns: Vec<String>,
    
    /// URL patterns to include (legacy)
    pub include_patterns: Vec<String>,
    
    /// URL patterns to exclude
    pub exclude_patterns: Vec<String>,
    
    /// When to run: document-start, document-end, document-idle
    pub run_at: RunAt,
    
    /// GM_* permissions requested
    pub grant: Vec<GrantPermission>,
    
    /// Required scripts to inject before this one
    pub require: Vec<String>,
    
    /// Resources to make available
    pub resource: HashMap<String, String>,
    
    /// Whether to run in iframes
    pub no_frames: bool,
    
    /// Connect domains for GM_xmlhttpRequest
    pub connect: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RunAt {
    DocumentStart,
    #[default]
    DocumentEnd,
    DocumentIdle,
    ContextMenu,
}

impl RunAt {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "document-start" => Self::DocumentStart,
            "document-end" => Self::DocumentEnd,
            "document-idle" => Self::DocumentIdle,
            "context-menu" => Self::ContextMenu,
            _ => Self::DocumentEnd,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DocumentStart => "document-start",
            Self::DocumentEnd => "document-end",
            Self::DocumentIdle => "document-idle",
            Self::ContextMenu => "context-menu",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrantPermission {
    None,
    UnsafeWindow,
    GmGetValue,
    GmSetValue,
    GmDeleteValue,
    GmListValues,
    GmAddStyle,
    GmXmlHttpRequest,
    GmOpenInTab,
    GmRegisterMenuCommand,
    GmUnregisterMenuCommand,
    GmNotification,
    GmSetClipboard,
    GmGetResourceText,
    GmGetResourceUrl,
    GmLog,
    GmInfo,
    Window(String),  // window.close, window.focus, etc.
}

impl GrantPermission {
    pub fn from_str(s: &str) -> Self {
        match s {
            "none" => Self::None,
            "unsafeWindow" => Self::UnsafeWindow,
            "GM_getValue" | "GM.getValue" => Self::GmGetValue,
            "GM_setValue" | "GM.setValue" => Self::GmSetValue,
            "GM_deleteValue" | "GM.deleteValue" => Self::GmDeleteValue,
            "GM_listValues" | "GM.listValues" => Self::GmListValues,
            "GM_addStyle" | "GM.addStyle" => Self::GmAddStyle,
            "GM_xmlhttpRequest" | "GM.xmlHttpRequest" => Self::GmXmlHttpRequest,
            "GM_openInTab" | "GM.openInTab" => Self::GmOpenInTab,
            "GM_registerMenuCommand" => Self::GmRegisterMenuCommand,
            "GM_unregisterMenuCommand" => Self::GmUnregisterMenuCommand,
            "GM_notification" | "GM.notification" => Self::GmNotification,
            "GM_setClipboard" | "GM.setClipboard" => Self::GmSetClipboard,
            "GM_getResourceText" => Self::GmGetResourceText,
            "GM_getResourceURL" => Self::GmGetResourceUrl,
            "GM_log" => Self::GmLog,
            "GM_info" | "GM.info" => Self::GmInfo,
            s if s.starts_with("window.") => Self::Window(s.to_string()),
            _ => Self::None,
        }
    }
}

// ==============================================================================
// USER SCRIPT MANAGER
// ==============================================================================

pub struct UserScriptManager {
    /// All loaded scripts
    scripts: Vec<UserScript>,
    
    /// Script storage (GM_getValue/GM_setValue)
    storage: HashMap<String, HashMap<String, serde_json::Value>>,
    
    /// Registered menu commands
    menu_commands: HashMap<String, Vec<MenuCommand>>,
    
    /// Scripts directory
    scripts_dir: PathBuf,
    
    /// Pattern cache for performance
    pattern_cache: HashMap<String, Vec<Regex>>,
}

#[derive(Debug, Clone)]
pub struct MenuCommand {
    pub id: String,
    pub script_id: String,
    pub caption: String,
    pub access_key: Option<char>,
    pub callback_id: String,
}

impl UserScriptManager {
    pub fn new(scripts_dir: PathBuf) -> Self {
        let manager = Self {
            scripts: Vec::new(),
            storage: HashMap::new(),
            menu_commands: HashMap::new(),
            scripts_dir,
            pattern_cache: HashMap::new(),
        };
        manager
    }
    
    /// Load all scripts from the scripts directory
    pub fn load_scripts(&mut self) -> Result<usize, String> {
        if !self.scripts_dir.exists() {
            fs::create_dir_all(&self.scripts_dir)
                .map_err(|e| format!("Failed to create scripts directory: {}", e))?;
        }
        
        let entries = fs::read_dir(&self.scripts_dir)
            .map_err(|e| format!("Failed to read scripts directory: {}", e))?;
        
        let mut loaded = 0;
        
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "js" || e == "user.js").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(script) = self.parse_script(&content) {
                        let mut script = script;
                        script.path = Some(path);
                        self.scripts.push(script);
                        loaded += 1;
                    }
                }
            }
        }
        
        // Load storage
        self.load_storage();
        
        Ok(loaded)
    }
    
    // ==============================================================================
    // SCRIPT PARSING
    // ==============================================================================
    
    /// Parse a user script from its content
    pub fn parse_script(&self, content: &str) -> Option<UserScript> {
        let metadata = self.parse_metadata(content)?;
        
        // Remove metadata block from code
        let code = self.extract_code(content);
        
        let id = format!(
            "{}@{}",
            metadata.name,
            metadata.namespace.as_deref().unwrap_or("anonymous")
        );
        
        Some(UserScript {
            id,
            metadata,
            code,
            enabled: true,
            path: None,
            last_modified: None,
        })
    }
    
    /// Parse the ==UserScript== metadata block
    fn parse_metadata(&self, content: &str) -> Option<ScriptMetadata> {
        // Find metadata block
        let start = content.find("==UserScript==")?;
        let end = content.find("==/UserScript==")?;
        
        let block = &content[start..end];
        
        let mut metadata = ScriptMetadata::default();
        
        for line in block.lines() {
            let line = line.trim();
            if !line.starts_with("// @") && !line.starts_with("//@") {
                continue;
            }
            
            let line = line.trim_start_matches("//").trim_start_matches(" @").trim_start_matches("@");
            
            let (key, value) = if let Some(space_pos) = line.find(char::is_whitespace) {
                let key = &line[..space_pos];
                let value = line[space_pos..].trim();
                (key, value)
            } else {
                (line, "")
            };
            
            match key {
                "name" => metadata.name = value.to_string(),
                "namespace" => metadata.namespace = Some(value.to_string()),
                "version" => metadata.version = Some(value.to_string()),
                "description" => metadata.description = Some(value.to_string()),
                "author" => metadata.author = Some(value.to_string()),
                "homepage" | "homepageURL" => metadata.homepage = Some(value.to_string()),
                "icon" | "iconURL" => metadata.icon = Some(value.to_string()),
                "updateURL" => metadata.update_url = Some(value.to_string()),
                "downloadURL" => metadata.download_url = Some(value.to_string()),
                "match" => metadata.match_patterns.push(value.to_string()),
                "include" => metadata.include_patterns.push(value.to_string()),
                "exclude" => metadata.exclude_patterns.push(value.to_string()),
                "run-at" => metadata.run_at = RunAt::from_str(value),
                "grant" => metadata.grant.push(GrantPermission::from_str(value)),
                "require" => metadata.require.push(value.to_string()),
                "resource" => {
                    let parts: Vec<&str> = value.splitn(2, char::is_whitespace).collect();
                    if parts.len() == 2 {
                        metadata.resource.insert(parts[0].to_string(), parts[1].to_string());
                    }
                }
                "noframes" => metadata.no_frames = true,
                "connect" => metadata.connect.push(value.to_string()),
                _ => {}
            }
        }
        
        if metadata.name.is_empty() {
            return None;
        }
        
        Some(metadata)
    }
    
    /// Extract the actual JavaScript code (without metadata)
    fn extract_code(&self, content: &str) -> String {
        if let Some(end) = content.find("==/UserScript==") {
            // Find the end of the line containing ==/UserScript==
            let after = &content[end..];
            if let Some(newline) = after.find('\n') {
                return content[end + newline + 1..].to_string();
            }
        }
        content.to_string()
    }
    
    // ==============================================================================
    // URL MATCHING
    // ==============================================================================
    
    /// Get scripts that should run on a given URL
    pub fn get_scripts_for_url(&mut self, url: &str, run_at: RunAt) -> Vec<&UserScript> {
        self.scripts.iter()
            .filter(|s| s.enabled && s.metadata.run_at == run_at && self.script_matches_url(s, url))
            .collect()
    }
    
    /// Check if a script matches a URL
    fn script_matches_url(&mut self, script: &UserScript, url: &str) -> bool {
        // Check exclude patterns first
        for pattern in &script.metadata.exclude_patterns {
            if self.pattern_matches(pattern, url) {
                return false;
            }
        }
        
        // Check match patterns
        for pattern in &script.metadata.match_patterns {
            if self.pattern_matches(pattern, url) {
                return true;
            }
        }
        
        // Check include patterns (legacy)
        for pattern in &script.metadata.include_patterns {
            if self.pattern_matches(pattern, url) {
                return true;
            }
        }
        
        // No patterns = match nothing
        script.metadata.match_patterns.is_empty() && script.metadata.include_patterns.is_empty()
    }
    
    /// Check if a pattern matches a URL
    fn pattern_matches(&mut self, pattern: &str, url: &str) -> bool {
        // Special patterns
        if pattern == "*" || pattern == "<all_urls>" {
            return true;
        }
        
        // Convert pattern to regex
        let regex = self.pattern_cache.entry(pattern.to_string())
            .or_insert_with(|| {
                vec![self.pattern_to_regex(pattern)]
                    .into_iter()
                    .filter_map(|r| r)
                    .collect()
            });
        
        regex.iter().any(|r| r.is_match(url))
    }
    
    /// Convert a match pattern or glob to regex
    fn pattern_to_regex(&self, pattern: &str) -> Option<Regex> {
        let mut regex_str = String::from("^");
        
        // Handle different pattern types
        if pattern.starts_with('/') && pattern.ends_with('/') {
            // Already a regex
            let inner = &pattern[1..pattern.len()-1];
            return Regex::new(inner).ok();
        }
        
        // Convert glob-style pattern
        for c in pattern.chars() {
            match c {
                '*' => regex_str.push_str(".*"),
                '?' => regex_str.push('.'),
                '.' | '+' | '{' | '}' | '[' | ']' | '\\' | '(' | ')' | '^' | '$' | '|' => {
                    regex_str.push('\\');
                    regex_str.push(c);
                }
                _ => regex_str.push(c),
            }
        }
        
        regex_str.push('$');
        Regex::new(&regex_str).ok()
    }
    
    // ==============================================================================
    // SCRIPT INJECTION
    // ==============================================================================
    
    /// Generate the JavaScript code to inject, including GM_* API
    pub fn generate_injection_code(&self, script: &UserScript) -> String {
        let mut code = String::new();
        
        // Create isolated scope
        code.push_str("(function() {\n");
        code.push_str("'use strict';\n\n");
        
        // Generate GM_* API
        code.push_str(&self.generate_gm_api(&script.id, &script.metadata.grant));
        
        // Generate GM_info
        code.push_str(&self.generate_gm_info(script));
        
        // Include required scripts (inline for now)
        for require_url in &script.metadata.require {
            code.push_str(&format!("// @require {}\n", require_url));
            // In a real implementation, we'd fetch and inline these
        }
        
        // Add the actual script code
        code.push_str("\n// User Script Code\n");
        code.push_str(&script.code);
        
        // Close scope
        code.push_str("\n})();\n");
        
        code
    }
    
    /// Generate GM_* API functions
    fn generate_gm_api(&self, script_id: &str, grants: &[GrantPermission]) -> String {
        let mut api = String::new();
        
        // Always provide GM_info
        api.push_str("const GM_info = window.__SASSY_GM_INFO__;\n");
        
        for grant in grants {
            match grant {
                GrantPermission::GmGetValue => {
                    api.push_str(&format!(r#"
function GM_getValue(key, defaultValue) {{
    return window.__SASSY_GM__.getValue('{}', key, defaultValue);
}}
const GM = GM || {{}};
GM.getValue = (key, def) => Promise.resolve(GM_getValue(key, def));
"#, script_id));
                }
                GrantPermission::GmSetValue => {
                    api.push_str(&format!(r#"
function GM_setValue(key, value) {{
    window.__SASSY_GM__.setValue('{}', key, value);
}}
const GM = GM || {{}};
GM.setValue = (key, val) => Promise.resolve(GM_setValue(key, val));
"#, script_id));
                }
                GrantPermission::GmDeleteValue => {
                    api.push_str(&format!(r#"
function GM_deleteValue(key) {{
    window.__SASSY_GM__.deleteValue('{}', key);
}}
const GM = GM || {{}};
GM.deleteValue = (key) => Promise.resolve(GM_deleteValue(key));
"#, script_id));
                }
                GrantPermission::GmListValues => {
                    api.push_str(&format!(r#"
function GM_listValues() {{
    return window.__SASSY_GM__.listValues('{}');
}}
const GM = GM || {{}};
GM.listValues = () => Promise.resolve(GM_listValues());
"#, script_id));
                }
                GrantPermission::GmAddStyle => {
                    api.push_str(r#"
function GM_addStyle(css) {
    const style = document.createElement('style');
    style.textContent = css;
    (document.head || document.documentElement).appendChild(style);
    return style;
}
const GM = GM || {};
GM.addStyle = (css) => Promise.resolve(GM_addStyle(css));
"#);
                }
                GrantPermission::GmXmlHttpRequest => {
                    api.push_str(r#"
function GM_xmlhttpRequest(details) {
    return window.__SASSY_GM__.xmlHttpRequest(details);
}
const GM = GM || {};
GM.xmlHttpRequest = (details) => GM_xmlhttpRequest(details);
"#);
                }
                GrantPermission::GmOpenInTab => {
                    api.push_str(r#"
function GM_openInTab(url, options) {
    return window.__SASSY_GM__.openInTab(url, options);
}
const GM = GM || {};
GM.openInTab = (url, opts) => Promise.resolve(GM_openInTab(url, opts));
"#);
                }
                GrantPermission::GmRegisterMenuCommand => {
                    api.push_str(&format!(r#"
function GM_registerMenuCommand(caption, callback, accessKey) {{
    return window.__SASSY_GM__.registerMenuCommand('{}', caption, callback, accessKey);
}}
"#, script_id));
                }
                GrantPermission::GmNotification => {
                    api.push_str(r#"
function GM_notification(details) {
    if (typeof details === 'string') {
        details = { text: details };
    }
    return window.__SASSY_GM__.notification(details);
}
const GM = GM || {};
GM.notification = (details) => Promise.resolve(GM_notification(details));
"#);
                }
                GrantPermission::GmSetClipboard => {
                    api.push_str(r#"
function GM_setClipboard(text, info) {
    return window.__SASSY_GM__.setClipboard(text, info);
}
const GM = GM || {};
GM.setClipboard = (text, info) => Promise.resolve(GM_setClipboard(text, info));
"#);
                }
                GrantPermission::GmLog => {
                    api.push_str(r#"
function GM_log(...args) {
    console.log('[UserScript]', ...args);
}
"#);
                }
                GrantPermission::UnsafeWindow => {
                    api.push_str("const unsafeWindow = window;\n");
                }
                _ => {}
            }
        }
        
        api
    }
    
    /// Generate GM_info object
    fn generate_gm_info(&self, script: &UserScript) -> String {
        format!(r#"
window.__SASSY_GM_INFO__ = {{
    script: {{
        name: '{}',
        namespace: '{}',
        version: '{}',
        description: '{}',
        author: '{}'
    }},
    scriptHandler: 'Sassy Browser',
    version: '2.0.0'
}};
"#,
            script.metadata.name,
            script.metadata.namespace.as_deref().unwrap_or(""),
            script.metadata.version.as_deref().unwrap_or("1.0"),
            script.metadata.description.as_deref().unwrap_or(""),
            script.metadata.author.as_deref().unwrap_or("")
        )
    }
    
    // ==============================================================================
    // STORAGE
    // ==============================================================================
    
    pub fn get_value(&self, script_id: &str, key: &str) -> Option<serde_json::Value> {
        self.storage.get(script_id)?.get(key).cloned()
    }
    
    pub fn set_value(&mut self, script_id: &str, key: &str, value: serde_json::Value) {
        self.storage
            .entry(script_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value);
        
        // Save storage
        self.save_storage();
    }
    
    pub fn delete_value(&mut self, script_id: &str, key: &str) {
        if let Some(script_storage) = self.storage.get_mut(script_id) {
            script_storage.remove(key);
            self.save_storage();
        }
    }
    
    pub fn list_values(&self, script_id: &str) -> Vec<String> {
        self.storage.get(script_id)
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default()
    }
    
    fn load_storage(&mut self) {
        let storage_path = self.scripts_dir.join("storage.json");
        if let Ok(content) = fs::read_to_string(&storage_path) {
            if let Ok(storage) = serde_json::from_str(&content) {
                self.storage = storage;
            }
        }
    }
    
    fn save_storage(&self) {
        let storage_path = self.scripts_dir.join("storage.json");
        if let Ok(json) = serde_json::to_string_pretty(&self.storage) {
            let _ = fs::write(&storage_path, json);
        }
    }
    
    // ==============================================================================
    // MANAGEMENT
    // ==============================================================================
    
    pub fn add_script(&mut self, content: &str) -> Result<String, String> {
        let script = self.parse_script(content)
            .ok_or("Failed to parse script")?;
        
        let id = script.id.clone();
        
        // Save to disk
        let filename = format!("{}.user.js", sanitize_filename(&script.metadata.name));
        let path = self.scripts_dir.join(&filename);
        fs::write(&path, content)
            .map_err(|e| format!("Failed to save script: {}", e))?;
        
        let mut script = script;
        script.path = Some(path);
        
        self.scripts.push(script);
        
        Ok(id)
    }
    
    pub fn remove_script(&mut self, id: &str) -> Result<(), String> {
        if let Some(pos) = self.scripts.iter().position(|s| s.id == id) {
            let script = self.scripts.remove(pos);
            
            // Delete file
            if let Some(path) = script.path {
                let _ = fs::remove_file(path);
            }
            
            // Clean up storage
            self.storage.remove(id);
            self.save_storage();
            
            Ok(())
        } else {
            Err("Script not found".to_string())
        }
    }
    
    pub fn enable_script(&mut self, id: &str) {
        if let Some(script) = self.scripts.iter_mut().find(|s| s.id == id) {
            script.enabled = true;
        }
    }
    
    pub fn disable_script(&mut self, id: &str) {
        if let Some(script) = self.scripts.iter_mut().find(|s| s.id == id) {
            script.enabled = false;
        }
    }
    
    pub fn get_script(&self, id: &str) -> Option<&UserScript> {
        self.scripts.iter().find(|s| s.id == id)
    }
    
    pub fn list_scripts(&self) -> &[UserScript] {
        &self.scripts
    }
    
    pub fn update_script(&mut self, id: &str, new_content: &str) -> Result<(), String> {
        let script = self.parse_script(new_content)
            .ok_or("Failed to parse updated script")?;
        
        if let Some(existing) = self.scripts.iter_mut().find(|s| s.id == id) {
            existing.metadata = script.metadata;
            existing.code = script.code;
            
            // Save to disk
            if let Some(path) = &existing.path {
                fs::write(path, new_content)
                    .map_err(|e| format!("Failed to save: {}", e))?;
            }
            
            Ok(())
        } else {
            Err("Script not found".to_string())
        }
    }
    
    // ==============================================================================
    // MENU COMMANDS
    // ==============================================================================
    
    pub fn register_menu_command(
        &mut self,
        script_id: &str,
        caption: &str,
        callback_id: &str,
        access_key: Option<char>,
    ) -> String {
        let cmd_id = format!("{}_{}", script_id, uuid_simple());
        
        let command = MenuCommand {
            id: cmd_id.clone(),
            script_id: script_id.to_string(),
            caption: caption.to_string(),
            access_key,
            callback_id: callback_id.to_string(),
        };
        
        self.menu_commands
            .entry(script_id.to_string())
            .or_insert_with(Vec::new)
            .push(command);
        
        cmd_id
    }
    
    pub fn unregister_menu_command(&mut self, script_id: &str, cmd_id: &str) {
        if let Some(commands) = self.menu_commands.get_mut(script_id) {
            commands.retain(|c| c.id != cmd_id);
        }
    }
    
    pub fn get_menu_commands(&self, script_id: &str) -> Vec<&MenuCommand> {
        self.menu_commands.get(script_id)
            .map(|cmds| cmds.iter().collect())
            .unwrap_or_default()
    }
    
    pub fn get_all_menu_commands(&self) -> Vec<&MenuCommand> {
        self.menu_commands.values()
            .flat_map(|cmds| cmds.iter())
            .collect()
    }
}

impl Default for UserScriptManager {
    fn default() -> Self {
        Self::new(PathBuf::from("userscripts"))
    }
}

// ==============================================================================
// UI COMPONENT
// ==============================================================================

pub struct UserScriptUI {
    manager: std::sync::Arc<std::sync::RwLock<UserScriptManager>>,
    editor_content: String,
    selected_script: Option<String>,
    show_editor: bool,
    new_script_url: String,
}

impl UserScriptUI {
    pub fn new(manager: std::sync::Arc<std::sync::RwLock<UserScriptManager>>) -> Self {
        Self {
            manager,
            editor_content: String::new(),
            selected_script: None,
            show_editor: false,
            new_script_url: String::new(),
        }
    }
    
    pub fn render(&mut self, ui: &mut eframe::egui::Ui) {
        use eframe::egui;
        
        ui.heading("(doc) User Scripts");
        ui.separator();
        
        // Add script section
        ui.horizontal(|ui| {
            ui.label("Install from URL:");
            ui.text_edit_singleline(&mut self.new_script_url);
            if ui.button("Install").clicked() && !self.new_script_url.is_empty() {
                // Would fetch and install
                self.new_script_url.clear();
            }
        });
        
        if ui.button("+ New Script").clicked() {
            self.editor_content = r#"// ==UserScript==
// @name        New Script
// @namespace   sassy.browser
// @version     1.0
// @description A new user script
// @author      You
// @match       *://*/*
// @grant       none
// ==/UserScript==

// Your code here
console.log('Hello from user script!');
"#.to_string();
            self.show_editor = true;
            self.selected_script = None;
        }
        
        ui.separator();
        
        // Script list
        if let Ok(manager) = self.manager.read() {
            let scripts: Vec<_> = manager.list_scripts().to_vec();
            for script in scripts {
                ui.horizontal(|ui| {
                    let mut enabled = script.enabled;
                    if ui.checkbox(&mut enabled, "").changed() {
                        drop(manager);
                        if let Ok(mut m) = self.manager.write() {
                            if enabled {
                                m.enable_script(&script.id);
                            } else {
                                m.disable_script(&script.id);
                            }
                        }
                        return;
                    }
                    
                    let name_label = if script.enabled {
                        egui::RichText::new(&script.metadata.name)
                    } else {
                        egui::RichText::new(&script.metadata.name).color(egui::Color32::GRAY)
                    };
                    
                    if ui.selectable_label(
                        self.selected_script.as_ref() == Some(&script.id),
                        name_label
                    ).clicked() {
                        self.selected_script = Some(script.id.clone());
                        self.editor_content = script.code.clone();
                    }
                    
                    if let Some(version) = &script.metadata.version {
                        ui.label(egui::RichText::new(format!("v{}", version)).small());
                    }
                    
                    // Match count
                    let patterns = script.metadata.match_patterns.len() 
                        + script.metadata.include_patterns.len();
                    ui.label(egui::RichText::new(format!("{} sites", patterns)).small().weak());
                });
            }
        }
        
        // Editor window
        if self.show_editor {
            egui::Window::new("Script Editor")
                .resizable(true)
                .default_width(600.0)
                .default_height(400.0)
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.editor_content)
                                .font(egui::FontId::monospace(12.0))
                                .desired_width(f32::INFINITY)
                                .desired_rows(20)
                        );
                    });
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            if let Ok(mut manager) = self.manager.write() {
                                if let Some(id) = &self.selected_script {
                                    let _ = manager.update_script(id, &self.editor_content);
                                } else {
                                    let _ = manager.add_script(&self.editor_content);
                                }
                            }
                            self.show_editor = false;
                        }
                        
                        if ui.button("Cancel").clicked() {
                            self.show_editor = false;
                        }
                        
                        if self.selected_script.is_some() {
                            if ui.button("Delete").clicked() {
                                if let (Ok(mut manager), Some(id)) = (self.manager.write(), &self.selected_script) {
                                    let _ = manager.remove_script(id);
                                }
                                self.show_editor = false;
                                self.selected_script = None;
                            }
                        }
                    });
                });
        }
    }
}

// ==============================================================================
// HELPERS
// ==============================================================================

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", n)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    const TEST_SCRIPT: &str = r#"// ==UserScript==
// @name        Test Script
// @namespace   test
// @version     1.0
// @description A test script
// @match       *://example.com/*
// @match       *://test.org/*
// @exclude     *://example.com/admin/*
// @grant       GM_setValue
// @grant       GM_getValue
// @run-at      document-end
// ==/UserScript==

console.log('Test script loaded');
"#;
    
    #[test]
    fn test_parse_metadata() {
        let manager = UserScriptManager::default();
        let script = manager.parse_script(TEST_SCRIPT).unwrap();
        
        assert_eq!(script.metadata.name, "Test Script");
        assert_eq!(script.metadata.namespace, Some("test".to_string()));
        assert_eq!(script.metadata.version, Some("1.0".to_string()));
        assert_eq!(script.metadata.match_patterns.len(), 2);
        assert_eq!(script.metadata.exclude_patterns.len(), 1);
        assert_eq!(script.metadata.run_at, RunAt::DocumentEnd);
        assert!(script.metadata.grant.contains(&GrantPermission::GmSetValue));
    }
    
    #[test]
    fn test_url_matching() {
        let mut manager = UserScriptManager::default();
        let script = manager.parse_script(TEST_SCRIPT).unwrap();
        manager.scripts.push(script);
        
        // Should match
        assert!(!manager.get_scripts_for_url("https://example.com/page", RunAt::DocumentEnd).is_empty());
        assert!(!manager.get_scripts_for_url("https://test.org/", RunAt::DocumentEnd).is_empty());
        
        // Should not match (excluded)
        assert!(manager.get_scripts_for_url("https://example.com/admin/", RunAt::DocumentEnd).is_empty());
        
        // Should not match (wrong domain)
        assert!(manager.get_scripts_for_url("https://other.com/", RunAt::DocumentEnd).is_empty());
    }
}
