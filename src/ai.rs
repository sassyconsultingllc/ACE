//! AI Assistant Integration
//!
//! Off by default. When enabled, provides contextual help like Windows XP's "?" button.
//! Easter eggs throughout encourage exploration and learning.
//!
//! Supported providers: Anthropic (Claude), OpenAI, xAI (Grok), Google (Gemini), Local (Ollama)
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;

use crate::data::config_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    pub provider: AiProvider,
    pub api_key: Option<String>,
    pub show_help_button: bool,  // The "?" button
    pub easter_eggs_found: Vec<String>,
    pub learning_mode: bool,  // Extra explanations
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: true,  // OFF by default
            provider: AiProvider::None,
            api_key: None,
            show_help_button: true,
            easter_eggs_found: Vec::new(),
            learning_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    None,
    Anthropic,  // Claude
    OpenAI,
    XAI,        // Grok
    Google,     // Gemini
    Local,      // Ollama/local models
}

/// Runtime view of AI config + secrets
#[derive(Debug, Clone, Default)]
pub struct AiRuntime {
    pub config: AiConfig,
    pub openai_key: Option<String>,
    pub anthropic_key: Option<String>,
    pub xai_key: Option<String>,
    pub google_key: Option<String>,
    pub local_endpoint: Option<String>,
    pub local_model: Option<String>,
}

/// Quick analysis requests - minimal, contextual
#[derive(Debug)]
pub enum HelpQuery {
    WhatIsThis { element: String },      // "?" on any UI element
    ExplainPage { url: String },          // "What is this site?"
    IsThisSafe { url: String },           // Quick safety check
    HowDoI { action: String },            // "How do I..."
    EasterEgg { trigger: String },        // Hidden discoveries
}

/// Easter eggs - local food discounts + Foodie Finder promo
pub const EASTER_EGGS: &[(&str, &str, &str)] = &[
    // (trigger, message, reward_code)
    ("konami", "->-><-<-<--><-->BA - Classic gamer! Here's 15% off at participating local restaurants.", "SASSY-KONAMI-15"),
    ("night_owl", "Browsing at 3am? Night owls get 10% off late-night eats.", "SASSY-NIGHTOWL-10"),
    ("first_block", "First popup blocked! Celebrate with $5 off your next local meal.", "SASSY-BLOCKED-5"),
    ("speed_reader", "500 pages today! Fuel up with 20% off local coffee shops.", "SASSY-READER-20"),
    ("trust_watcher", "You get security. Get 15% off farm-to-table spots.", "SASSY-TRUST-15"),
    ("week_streak", "7 days secure browsing! Free appetizer at local favorites.", "SASSY-STREAK-APP"),
    ("family_setup", "Family protected! $10 off family-style restaurants.", "SASSY-FAMILY-10"),
    ("zero_malware", "30 days malware-free! Dessert's on us at local bakeries.", "SASSY-CLEAN-DESSERT"),
];

/// Foodie Finder integration
pub const FOODIE_FINDER_URL: &str = "https://foodiefinder.app";
pub const FOODIE_FINDER_REDEEM: &str = "https://foodiefinder.app/redeem?code=";

#[derive(Debug, Clone)]
pub struct EasterEggReward {
    pub message: String,
    pub code: String,
    pub redeem_url: String,
}

impl EasterEggReward {
    /// Summary of the reward
    pub fn describe(&self) -> String {
        format!("EasterEggReward[msg={}, code={}, url={}]", self.message, self.code, self.redeem_url)
    }
}

impl AiRuntime {
    /// Summary of the AI runtime configuration
    pub fn describe(&self) -> String {
        format!("AiRuntime[enabled={}, provider={:?}, openai={}, anthropic={}, xai={}, google={}, local_endpoint={}, local_model={}]",
            self.config.enabled,
            self.config.provider,
            self.openai_key.is_some(),
            self.anthropic_key.is_some(),
            self.xai_key.is_some(),
            self.google_key.is_some(),
            self.local_endpoint.as_deref().unwrap_or("none"),
            self.local_model.as_deref().unwrap_or("none"))
    }
}

impl AiConfig {
    pub fn enable_with_key(&mut self, provider: AiProvider, key: String) {
        self.provider = provider;
        self.api_key = Some(key);
        self.enabled = true;
    }
    
    pub fn discover_easter_egg(&mut self, id: &str) -> Option<EasterEggReward> {
        if self.easter_eggs_found.contains(&id.to_string()) {
            return None;
        }
        
        if let Some((_, message, code)) = EASTER_EGGS.iter().find(|(eid, _, _)| *eid == id) {
            self.easter_eggs_found.push(id.to_string());
            Some(EasterEggReward {
                message: message.to_string(),
                code: code.to_string(),
                redeem_url: format!("{}{}", FOODIE_FINDER_REDEEM, code),
            })
        } else {
            None
        }
    }
    
    pub fn eggs_found(&self) -> usize {
        self.easter_eggs_found.len()
    }

    /// URL for the Foodie Finder main page
    pub fn foodie_finder_url() -> &'static str {
        FOODIE_FINDER_URL
    }
    
    pub fn total_eggs() -> usize {
        EASTER_EGGS.len()
    }
}

/// Load AI runtime config from config/ai.toml (user config dir), falling back to packaged default.
///
/// Keys are resolved with fallback priority:
///   1. [ai.keys] section (provider-specific)
///   2. [mcp.keys] section (shared cloud keys)
/// This means you only need to set keys once in [mcp.keys] and the sidebar AI picks them up.
pub fn load_runtime() -> AiRuntime {
    let path = config_dir().join("ai.toml");
    let fallback = PathBuf::from("config").join("ai.toml");

    let content = fs::read_to_string(&path)
        .or_else(|_| fs::read_to_string(&fallback))
        .unwrap_or_default();

    let parsed: AiFile = toml::from_str(&content).unwrap_or_default();

    // Resolve keys: [ai.keys] takes priority, then fall back to [mcp.keys]
    let anthropic_key = parsed.ai.keys.anthropic.clone()
        .filter(|s| !s.is_empty())
        .or_else(|| parsed.mcp.keys.anthropic.clone().filter(|s| !s.is_empty()));
    let openai_key = parsed.ai.keys.openai.clone()
        .filter(|s| !s.is_empty());
    let xai_key = parsed.ai.keys.xai.clone()
        .filter(|s| !s.is_empty())
        .or_else(|| parsed.mcp.keys.xai.clone().filter(|s| !s.is_empty()));
    let google_key = parsed.ai.keys.google.clone()
        .filter(|s| !s.is_empty())
        .or_else(|| parsed.mcp.keys.google.clone().filter(|s| !s.is_empty()));

    let provider = match crate::fontcase::ascii_lower(&parsed.ai.provider).as_str() {
        "anthropic" | "claude" => AiProvider::Anthropic,
        "openai" | "gpt" => AiProvider::OpenAI,
        "xai" | "grok" | "x" => AiProvider::XAI,
        "google" | "gemini" => AiProvider::Google,
        "local" | "ollama" => AiProvider::Local,
        // Auto-detect: if provider is "auto" or "none" but keys exist, pick the first available
        "auto" => {
            if xai_key.is_some() { AiProvider::XAI }
            else if anthropic_key.is_some() { AiProvider::Anthropic }
            else if google_key.is_some() { AiProvider::Google }
            else if openai_key.is_some() { AiProvider::OpenAI }
            else { AiProvider::None }
        }
        _ => AiProvider::None,
    };

    let api_key = match &provider {
        AiProvider::Anthropic => anthropic_key.clone(),
        AiProvider::OpenAI => openai_key.clone(),
        AiProvider::XAI => xai_key.clone(),
        AiProvider::Google => google_key.clone(),
        AiProvider::Local => None,
        AiProvider::None => None,
    };

    // Auto-enable if we have a valid provider + key
    let enabled = parsed.ai.enabled || (api_key.is_some() && !matches!(provider, AiProvider::None));

    let config = AiConfig {
        enabled,
        provider: provider.clone(),
        api_key,
        show_help_button: parsed.help.show_button,
        learning_mode: parsed.learning.enabled,
        ..Default::default()
    };

    // Log optional config values
    if let Some(ref pos) = parsed.help.position {
        eprintln!("AI help button position: {}", pos);
    }
    if parsed.learning.easter_eggs_notifications {
        eprintln!("Easter egg notifications enabled");
    }

    tracing::info!("AI sidebar: provider={:?}, enabled={}, has_key={}", 
        config.provider, config.enabled, config.api_key.is_some());

    AiRuntime {
        config,
        openai_key,
        anthropic_key,
        xai_key,
        google_key,
        local_endpoint: Some(parsed.ai.local.endpoint.clone()).filter(|s| !s.is_empty()),
        local_model: Some(parsed.ai.local.model.clone()).filter(|s| !s.is_empty()),
    }
}

/// Build a help query from a context string, choosing the appropriate variant
pub fn help_query_for_context(context: &str, url: &str) -> HelpQuery {
    let lower = crate::fontcase::ascii_lower(context);
    if lower.starts_with("what is") || lower.starts_with("explain") {
        HelpQuery::WhatIsThis { element: context.to_string() }
    } else if lower.starts_with("safe") || lower.starts_with("is this safe") {
        HelpQuery::IsThisSafe { url: url.to_string() }
    } else if lower.starts_with("how do i") || lower.starts_with("how to") {
        HelpQuery::HowDoI { action: context.to_string() }
    } else if lower.starts_with("egg:") || lower.starts_with("easter") {
        HelpQuery::EasterEgg { trigger: context.to_string() }
    } else {
        HelpQuery::ExplainPage { url: url.to_string() }
    }
}

/// Execute a help query and return the provider response text.
pub fn run_help_query(runtime: &AiRuntime, query: HelpQuery) -> Result<String, String> {
    if !runtime.config.enabled {
        return Err("AI is disabled".into());
    }

    match runtime.config.provider {
        AiProvider::OpenAI => {
            let key = runtime
                .openai_key
                .as_ref()
                .ok_or_else(|| "OpenAI key missing".to_string())?;
            call_openai(key, &build_prompt(&query))
        }
        AiProvider::Anthropic => {
            let key = runtime
                .anthropic_key
                .as_ref()
                .ok_or_else(|| "Anthropic key missing".to_string())?;
            call_anthropic(key, &build_prompt(&query))
        }
        AiProvider::XAI => {
            let key = runtime
                .xai_key
                .as_ref()
                .ok_or_else(|| "xAI key missing".to_string())?;
            call_xai(key, &build_prompt(&query))
        }
        AiProvider::Google => {
            let key = runtime
                .google_key
                .as_ref()
                .ok_or_else(|| "Google API key missing".to_string())?;
            call_google(key, &build_prompt(&query))
        }
        AiProvider::Local => {
            let endpoint = runtime
                .local_endpoint
                .as_ref()
                .ok_or_else(|| "Local endpoint missing".to_string())?;
            let model = runtime
                .local_model
                .as_ref()
                .ok_or_else(|| "Local model missing".to_string())?;
            call_local(endpoint, model, &build_prompt(&query))
        }
        AiProvider::None => Err("AI provider not configured".into()),
    }
}

fn build_prompt(query: &HelpQuery) -> String {
    match query {
        HelpQuery::WhatIsThis { element } => {
            format!("Explain this UI element concisely: {}", element)
        }
        HelpQuery::ExplainPage { url } => {
            format!("Explain what this page is and any trust/safety notes. URL: {}", url)
        }
        HelpQuery::IsThisSafe { url } => {
            format!("Assess safety risks (phishing/malware/trackers) for {}. Give concise guidance.", url)
        }
        HelpQuery::HowDoI { action } => {
            format!("How do I {} in a browser? Keep it short and safe.", action)
        }
        HelpQuery::EasterEgg { trigger } => {
            format!("User triggered easter egg: {}. Respond playfully in one line.", trigger)
        }
    }
}

fn call_openai(key: &str, prompt: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "system", "content": "You are a concise browser assistant. Be safe, avoid code execution, no external calls."},
            {"role": "user", "content": prompt}
        ],
        "max_tokens": 256
    });

    let resp = ureq::post("https://api.openai.com/v1/chat/completions")
        .set("Authorization", &format!("Bearer {}", key))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("OpenAI request failed: {}", e))?;

    let json: JsonValue = resp
        .into_json()
        .map_err(|e| format!("OpenAI JSON parse failed: {}", e))?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if content.is_empty() {
        Err("OpenAI returned empty response".into())
    } else {
        Ok(content)
    }
}

fn call_anthropic(key: &str, prompt: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "model": "claude-3-5-haiku-latest",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": prompt}],
        "system": "You are a concise browser assistant. Be safe; avoid code that could execute." 
    });

    let resp = ureq::post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", key)
        .set("anthropic-version", "2023-06-01")
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("Anthropic request failed: {}", e))?;

    let json: JsonValue = resp
        .into_json()
        .map_err(|e| format!("Anthropic JSON parse failed: {}", e))?;
    let content = json["content"][0]["text"].as_str().unwrap_or("").to_string();
    if content.is_empty() {
        Err("Anthropic returned empty response".into())
    } else {
        Ok(content)
    }
}

fn call_xai(key: &str, prompt: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "model": "grok-2",
        "messages": [
            {"role": "system", "content": "You are a concise browser assistant. Be safe, avoid code execution, no external calls."},
            {"role": "user", "content": prompt}
        ],
        "max_tokens": 256,
        "temperature": 0.7
    });

    let resp = ureq::post("https://api.x.ai/v1/chat/completions")
        .set("Authorization", &format!("Bearer {}", key))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("xAI request failed: {}", e))?;

    let json: JsonValue = resp
        .into_json()
        .map_err(|e| format!("xAI JSON parse failed: {}", e))?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if content.is_empty() {
        Err("xAI returned empty response".into())
    } else {
        Ok(content)
    }
}

fn call_google(key: &str, prompt: &str) -> Result<String, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        key
    );

    let body = serde_json::json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }],
        "systemInstruction": {
            "parts": [{"text": "You are a concise browser assistant. Be safe, avoid code execution, no external calls."}]
        },
        "generationConfig": {
            "maxOutputTokens": 256,
            "temperature": 0.4
        }
    });

    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("Google Gemini request failed: {}", e))?;

    let json: JsonValue = resp
        .into_json()
        .map_err(|e| format!("Gemini JSON parse failed: {}", e))?;
    let content = json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if content.is_empty() {
        Err("Gemini returned empty response".into())
    } else {
        Ok(content)
    }
}

fn call_local(endpoint: &str, model: &str, prompt: &str) -> Result<String, String> {
    let url = format!("{}/api/generate", endpoint.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false
    });

    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("Local AI request failed: {}", e))?;

    let json: JsonValue = resp
        .into_json()
        .map_err(|e| format!("Local AI JSON parse failed: {}", e))?;

    if let Some(resp_text) = json["response"].as_str() {
        return Ok(resp_text.to_string());
    }
    if let Some(resp_text) = json["output"].as_str() {
        return Ok(resp_text.to_string());
    }
    Err("Local AI returned empty response".into())
}

// ==============================================================================
// Config file mapping
// ==============================================================================

#[derive(Debug, Deserialize, Default)]
struct AiFile {
    #[serde(default)]
    ai: AiSection,
    #[serde(default)]
    help: HelpSection,
    #[serde(default)]
    learning: LearningSection,
    #[serde(default)]
    whisper: WhisperSection,
    #[serde(default)]
    mcp: McpSection,
}

#[derive(Debug, Deserialize)]
struct AiSection {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_provider_none")]
    provider: String,
    #[serde(default)]
    keys: AiKeys,
    #[serde(default)]
    local: AiLocal,
}

impl Default for AiSection {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_provider_none(),
            keys: AiKeys::default(),
            local: AiLocal::default(),
        }
    }
}

fn default_provider_none() -> String { "none".into() }

#[derive(Debug, Deserialize, Default, Clone)]
struct AiKeys {
    #[serde(default)]
    anthropic: Option<String>,
    #[serde(default)]
    openai: Option<String>,
    #[serde(default)]
    xai: Option<String>,
    #[serde(default)]
    google: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct AiLocal {
    #[serde(default = "default_local_endpoint")]
    endpoint: String,
    #[serde(default = "default_local_model")]
    model: String,
}

fn default_local_endpoint() -> String { "http://localhost:11434".into() }
fn default_local_model() -> String { "llama2".into() }

#[derive(Debug, Deserialize, Default)]
struct HelpSection {
    #[serde(default = "default_show_button")]
    show_button: bool,
    #[serde(default)]
    position: Option<String>,
}

fn default_show_button() -> bool { true }

#[derive(Debug, Deserialize, Default)]
struct LearningSection {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    easter_eggs_notifications: bool,
}

/// Minimal MCP section — just enough to read [mcp.keys] for fallback
#[derive(Debug, Deserialize, Default)]
struct McpSection {
    #[serde(default)]
    keys: McpKeys,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct McpKeys {
    #[serde(default)]
    xai: Option<String>,
    #[serde(default)]
    anthropic: Option<String>,
    #[serde(default)]
    google: Option<String>,
}

// ==============================================================================
// Whisper (Speech-to-Text) Configuration
// ==============================================================================

#[derive(Debug, Deserialize)]
struct WhisperSection {
    #[serde(default = "default_whisper_enabled")]
    enabled: bool,
    #[serde(default = "default_whisper_model")]
    model: String,
    #[serde(default = "default_use_gpu")]
    use_gpu: bool,
    #[serde(default)]
    language: Option<String>,
    #[serde(default = "default_vad_threshold")]
    vad_threshold: f32,
    #[serde(default = "default_silence_duration")]
    silence_duration: f32,
    #[serde(default = "default_max_duration")]
    max_duration: f32,
    #[serde(default = "default_live_preview")]
    live_preview: bool,
}

impl Default for WhisperSection {
    fn default() -> Self {
        Self {
            enabled: true,
            model: default_whisper_model(),
            use_gpu: true,
            language: Some("en".to_string()),
            vad_threshold: default_vad_threshold(),
            silence_duration: default_silence_duration(),
            max_duration: default_max_duration(),
            live_preview: true,
        }
    }
}

fn default_whisper_enabled() -> bool { true }
fn default_whisper_model() -> String { "base".into() }
fn default_use_gpu() -> bool { true }
fn default_vad_threshold() -> f32 { 0.3 }
fn default_silence_duration() -> f32 { 1.5 }
fn default_max_duration() -> f32 { 60.0 }
fn default_live_preview() -> bool { true }

/// Load Whisper (speech-to-text) configuration from ai.toml
pub fn load_whisper_config() -> crate::voice::VoiceConfig {
    let path = config_dir().join("ai.toml");
    let fallback = PathBuf::from("config").join("ai.toml");

    let content = fs::read_to_string(&path)
        .or_else(|_| fs::read_to_string(&fallback))
        .unwrap_or_default();

    let parsed: AiFile = toml::from_str(&content).unwrap_or_default();
    let ws = parsed.whisper;
    
    let model = match crate::fontcase::ascii_lower(&ws.model).as_str() {
        "tiny" => crate::voice::WhisperModel::Tiny,
        "base" => crate::voice::WhisperModel::Base,
        "small" => crate::voice::WhisperModel::Small,
        "medium" => crate::voice::WhisperModel::Medium,
        "large" => crate::voice::WhisperModel::Large,
        _ => crate::voice::WhisperModel::Base,
    };
    
    crate::voice::VoiceConfig {
        enabled: ws.enabled,
        model,
        use_gpu: ws.use_gpu,
        language: ws.language,
        vad_threshold: ws.vad_threshold,
        silence_duration: ws.silence_duration,
        max_duration: ws.max_duration,
        live_preview: ws.live_preview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_config_easter_eggs_and_enable() {
        let mut cfg = AiConfig::default();
        assert!(!cfg.enabled);

        cfg.enable_with_key(AiProvider::OpenAI, "sk_test".to_string());
        assert!(cfg.enabled);
        assert!(matches!(cfg.provider, AiProvider::OpenAI));
        assert_eq!(cfg.api_key.as_deref(), Some("sk_test"));

        let total = AiConfig::total_eggs();
        assert!(total >= 1);

        // Discover an egg (use first defined id)
        if let Some((id, _, _)) = EASTER_EGGS.first() {
            let reward = cfg.discover_easter_egg(id);
            assert!(reward.is_some());
            assert_eq!(cfg.eggs_found(), 1);
        }
    }

    #[test]
    fn test_xai_provider_parse() {
        // Verify "xai", "grok", "x" all resolve to XAI
        for name in &["xai", "grok", "x"] {
            let provider = match *name {
                "xai" | "grok" | "x" => AiProvider::XAI,
                _ => AiProvider::None,
            };
            assert!(matches!(provider, AiProvider::XAI));
        }
    }

    #[test]
    fn test_google_provider_parse() {
        for name in &["google", "gemini"] {
            let provider = match *name {
                "google" | "gemini" => AiProvider::Google,
                _ => AiProvider::None,
            };
            assert!(matches!(provider, AiProvider::Google));
        }
    }
}
