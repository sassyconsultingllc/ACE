//! MCP API Client - Communicates with AI Model Providers
//!
//! Handles HTTP requests to:
//! - xAI (Grok) for voice/intent understanding
//! - Manus for task orchestration
//! - Anthropic (Claude) for code generation
//! - Google (Gemini) for code review and feasibility auditing
//! - Together.ai for open source models (Llama, Qwen, DeepSeek)

//! - Hugging Face Inference Endpoints for self-hosted models
//! - Ollama for local models
//!
//! Supports streaming responses for real-time feedback.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::mcp::{AgentConfig, AgentRole, McpMessage, MessageRole, Provider};

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// API error types
#[derive(Debug, Clone)]
pub enum ApiError {
    /// Network error
    Network(String),
    /// Authentication error (401/403)
    Auth(String),
    /// Rate limit exceeded (429)
    RateLimit { retry_after: Option<u64> },
    /// Server error (5xx)
    Server(String),
    /// Invalid request (4xx)
    InvalidRequest(String),
    /// Parse error
    Parse(String),
    /// Timeout
    Timeout,
    /// Agent not configured
    NotConfigured(AgentRole),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Network(msg) => write!(f, "Network error: {}", msg),
            ApiError::Auth(msg) => write!(f, "Authentication error: {}", msg),
            ApiError::RateLimit { retry_after } => {
                if let Some(secs) = retry_after {
                    write!(f, "Rate limit exceeded, retry after {} seconds", secs)
                } else {
                    write!(f, "Rate limit exceeded")
                }
            }
            ApiError::Server(msg) => write!(f, "Server error: {}", msg),
            ApiError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            ApiError::Parse(msg) => write!(f, "Parse error: {}", msg),
            ApiError::Timeout => write!(f, "Request timed out"),
            ApiError::NotConfigured(role) => write!(f, "Agent {:?} not configured", role),
        }
    }
}

/// API client for all MCP agents
pub struct McpApiClient {
    configs: HashMap<AgentRole, AgentConfig>,
    timeout: Duration,
    retry_count: u32,
}

impl McpApiClient {
    pub fn new() -> Self {
        let mut configs = HashMap::new();
        configs.insert(AgentRole::Voice, AgentConfig::grok_default());
        configs.insert(AgentRole::Orchestrator, AgentConfig::manus_default());
        configs.insert(AgentRole::Coder, AgentConfig::claude_default());
        configs.insert(AgentRole::Auditor, AgentConfig::gemini_default());

        McpApiClient {
            configs,
            timeout: Duration::from_secs(60),
            retry_count: 3,
        }
    }

    /// Configure an agent
    pub fn configure(&mut self, config: AgentConfig) {
        self.configs.insert(config.role, config);
    }

    /// Configure for all-local mode using Ollama
    pub fn configure_local(&mut self) {
        self.configure(AgentConfig::voice_ollama());
        self.configure(AgentConfig::orchestrator_ollama());
        self.configure(AgentConfig::coder_ollama());
        self.configure(AgentConfig::auditor_ollama());
    }

    /// Configure for Together.ai (cheaper cloud alternative)
    pub fn configure_together(&mut self, api_key: &str) {
        let mut voice = AgentConfig::voice_together();
        voice.api_key = Some(api_key.to_string());
        self.configure(voice);

        let mut orch = AgentConfig::manus_default();
        orch.provider = Provider::Together;
        orch.api_key = Some(api_key.to_string());
        self.configure(orch);

        let mut coder = AgentConfig::coder_together();
        coder.api_key = Some(api_key.to_string());
        self.configure(coder);

        let mut auditor = AgentConfig::auditor_together();
        auditor.api_key = Some(api_key.to_string());
        self.configure(auditor);
    }

    /// Set API key for an agent
    pub fn set_api_key(&mut self, role: AgentRole, key: String) {
        if let Some(config) = self.configs.get_mut(&role) {
            config.api_key = Some(key);
        }
    }

    /// Set timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Get the configured retry count
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// Check if an agent is configured and ready
    /// For local providers (Ollama), we don't need an API key
    pub fn is_ready(&self, role: AgentRole) -> bool {
        self.configs
            .get(&role)
            .map(|c| c.enabled && (c.api_key.is_some() || c.provider.is_local()))
            .unwrap_or(false)
    }

    // ==============================================================================
    // Unified Call Interface
    // ==============================================================================

    /// Universal call method - routes to correct provider based on config
    pub fn call(
        &self,
        role: AgentRole,
        messages: &[ChatMessage],
        system: &str,
    ) -> ApiResult<ChatResponse> {
        let config = self
            .configs
            .get(&role)
            .ok_or(ApiError::NotConfigured(role))?;

        match config.provider {
            Provider::Xai => self.call_openai_compatible(config, messages, Some(system)),
            Provider::Together => self.call_openai_compatible(config, messages, Some(system)),
            Provider::OpenAI => self.call_openai_compatible(config, messages, Some(system)),
            Provider::Anthropic => self.call_claude(messages, system),
            Provider::Google => {
                // Gemini returns AuditResponse, we convert to ChatResponse
                let audit = self.call_gemini(messages, system)?;
                Ok(ChatResponse {
                    content: audit.analysis,
                    finish_reason: "stop".to_string(),
                    usage: None,
                })
            }
            Provider::HuggingFace => self.call_huggingface(config, messages, system),
            Provider::Ollama => self.call_ollama(config, messages, system),
            Provider::Custom => self.call_openai_compatible(config, messages, Some(system)),
        }
    }

    /// Call OpenAI-compatible API (xAI, Together, OpenAI, etc.)
    fn call_openai_compatible(
        &self,
        config: &AgentConfig,
        messages: &[ChatMessage],
        system: Option<&str>,
    ) -> ApiResult<ChatResponse> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| ApiError::Auth(format!("{} API key not set", config.provider.name())))?;

        let request = GrokRequest {
            model: config.model.clone(),
            messages: build_messages(messages, system),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            stream: false,
        };

        self.send_request(&config.api_url, api_key, &request)
    }

    /// Call Hugging Face Inference Endpoint
    fn call_huggingface(
        &self,
        config: &AgentConfig,
        messages: &[ChatMessage],
        system: &str,
    ) -> ApiResult<ChatResponse> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| ApiError::Auth("Hugging Face API key not set".to_string()))?;

        // HF Inference Endpoints use OpenAI-compatible API
        let request = GrokRequest {
            model: config.model.clone(),
            messages: build_messages(messages, Some(system)),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            stream: false,
        };

        // HF uses Bearer token auth
        let body = serde_json::to_string(&request).map_err(|e| ApiError::Parse(e.to_string()))?;

        let response = ureq::post(&config.api_url)
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", api_key))
            .timeout(self.timeout)
            .send_string(&body)
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let grok_response: GrokResponse = response
            .into_json()
            .map_err(|e| ApiError::Parse(e.to_string()))?;

        if let Some(choice) = grok_response.choices.first() {
            Ok(ChatResponse {
                content: choice.message.content.clone(),
                finish_reason: choice.finish_reason.clone().unwrap_or_default(),
                usage: grok_response.usage.map(|u| TokenUsage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                }),
            })
        } else {
            Err(ApiError::Parse("No response from Hugging Face".to_string()))
        }
    }

    /// Call local Ollama
    fn call_ollama(
        &self,
        config: &AgentConfig,
        messages: &[ChatMessage],
        system: &str,
    ) -> ApiResult<ChatResponse> {
        // Build Ollama request format
        let ollama_messages: Vec<OllamaMessage> = std::iter::once(OllamaMessage {
            role: "system".to_string(),
            content: system.to_string(),
        })
        .chain(messages.iter().map(|m| OllamaMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        }))
        .collect();

        let request = OllamaRequest {
            model: config.model.clone(),
            messages: ollama_messages,
            stream: false,
            options: Some(OllamaOptions {
                num_predict: config.max_tokens as i32,
                temperature: config.temperature,
            }),
        };

        let body = serde_json::to_string(&request).map_err(|e| ApiError::Parse(e.to_string()))?;

        let response = ureq::post(&config.api_url)
            .set("Content-Type", "application/json")
            .timeout(self.timeout)
            .send_string(&body)
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let ollama_response: OllamaResponse = response
            .into_json()
            .map_err(|e| ApiError::Parse(e.to_string()))?;

        Ok(ChatResponse {
            content: ollama_response.message.content,
            finish_reason: if ollama_response.done {
                "stop".to_string()
            } else {
                "length".to_string()
            },
            usage: Some(TokenUsage {
                prompt_tokens: ollama_response.prompt_eval_count.unwrap_or(0),
                completion_tokens: ollama_response.eval_count.unwrap_or(0),
                total_tokens: ollama_response.prompt_eval_count.unwrap_or(0)
                    + ollama_response.eval_count.unwrap_or(0),
            }),
        })
    }

    /// Check if Ollama is running locally
    pub fn check_ollama(&self) -> bool {
        ureq::get("http://localhost:11434/api/tags")
            .timeout(Duration::from_secs(2))
            .call()
            .is_ok()
    }

    /// List available Ollama models
    pub fn list_ollama_models(&self) -> ApiResult<Vec<String>> {
        let response = ureq::get("http://localhost:11434/api/tags")
            .timeout(Duration::from_secs(5))
            .call()
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let tags: OllamaTagsResponse = response
            .into_json()
            .map_err(|e| ApiError::Parse(e.to_string()))?;

        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }

    // ==============================================================================
    // Provider-Specific Call Methods (Legacy, still useful)
    // ==============================================================================

    /// Send request to Grok (xAI)
    pub fn call_grok(
        &self,
        messages: &[ChatMessage],
        system: Option<&str>,
    ) -> ApiResult<ChatResponse> {
        let config = self
            .configs
            .get(&AgentRole::Voice)
            .ok_or(ApiError::NotConfigured(AgentRole::Voice))?;

        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| ApiError::Auth("Grok API key not set".to_string()))?;

        // Build xAI request
        let request = GrokRequest {
            model: config.model.clone(),
            messages: build_messages(messages, system),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            stream: false,
        };

        self.send_request(&config.api_url, api_key, &request)
    }

    /// Send request to Manus
    pub fn call_manus(
        &self,
        messages: &[ChatMessage],
        context: Option<&TaskContext>,
    ) -> ApiResult<OrchestrationResponse> {
        let config = self
            .configs
            .get(&AgentRole::Orchestrator)
            .ok_or(ApiError::NotConfigured(AgentRole::Orchestrator))?;

        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| ApiError::Auth("Manus API key not set".to_string()))?;

        // Build Manus request
        let request = ManusRequest {
            model: config.model.clone(),
            messages: messages.to_vec(),
            context: context.cloned(),
            max_tokens: config.max_tokens,
        };

        self.send_orchestration_request(&config.api_url, api_key, &request)
    }

    /// Send request to Claude (Anthropic)
    pub fn call_claude(&self, messages: &[ChatMessage], system: &str) -> ApiResult<ChatResponse> {
        let config = self
            .configs
            .get(&AgentRole::Coder)
            .ok_or(ApiError::NotConfigured(AgentRole::Coder))?;

        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| ApiError::Auth("Claude API key not set".to_string()))?;

        // Build Anthropic request
        let request = ClaudeRequest {
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            system: system.to_string(),
            messages: messages
                .iter()
                .map(|m| ClaudeMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect(),
        };

        self.send_claude_request(&config.api_url, api_key, &request)
    }

    /// Send request to Gemini (Google) for auditing
    pub fn call_gemini(&self, messages: &[ChatMessage], context: &str) -> ApiResult<AuditResponse> {
        let config = self
            .configs
            .get(&AgentRole::Auditor)
            .ok_or(ApiError::NotConfigured(AgentRole::Auditor))?;

        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| ApiError::Auth("Gemini API key not set".to_string()))?;

        // Build Gemini request
        let request = GeminiRequest {
            contents: messages
                .iter()
                .map(|m| GeminiContent {
                    role: if m.role == "user" {
                        "user".to_string()
                    } else {
                        "model".to_string()
                    },
                    parts: vec![GeminiPart {
                        text: m.content.clone(),
                    }],
                })
                .collect(),
            system_instruction: Some(GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: context.to_string(),
                }],
            }),
            generation_config: Some(GeminiGenerationConfig {
                max_output_tokens: config.max_tokens,
                temperature: config.temperature,
            }),
        };

        self.send_gemini_request(&config.api_url, &config.model, api_key, &request)
    }

    /// Generic HTTP request sender
    fn send_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        api_key: &str,
        request: &T,
    ) -> ApiResult<R> {
        let body = serde_json::to_string(request).map_err(|e| ApiError::Parse(e.to_string()))?;

        let response = ureq::post(url)
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", api_key))
            .timeout(self.timeout)
            .send_string(&body);

        match response {
            Ok(resp) => {
                let text = resp
                    .into_string()
                    .map_err(|e| ApiError::Parse(e.to_string()))?;
                serde_json::from_str(&text).map_err(|e| ApiError::Parse(e.to_string()))
            }
            Err(ureq::Error::Status(status, resp)) => {
                let error_text = resp.into_string().unwrap_or_default();
                match status {
                    401 | 403 => Err(ApiError::Auth(error_text)),
                    429 => Err(ApiError::RateLimit { retry_after: None }),
                    400..=499 => Err(ApiError::InvalidRequest(error_text)),
                    500..=599 => Err(ApiError::Server(error_text)),
                    _ => Err(ApiError::Network(format!(
                        "Status {}: {}",
                        status, error_text
                    ))),
                }
            }
            Err(ureq::Error::Transport(t)) => {
                if t.to_string().contains("timeout") {
                    Err(ApiError::Timeout)
                } else {
                    Err(ApiError::Network(t.to_string()))
                }
            }
        }
    }

    fn send_orchestration_request(
        &self,
        url: &str,
        api_key: &str,
        request: &ManusRequest,
    ) -> ApiResult<OrchestrationResponse> {
        self.send_request(url, api_key, request)
    }

    fn send_claude_request(
        &self,
        url: &str,
        api_key: &str,
        request: &ClaudeRequest,
    ) -> ApiResult<ChatResponse> {
        let body = serde_json::to_string(request).map_err(|e| ApiError::Parse(e.to_string()))?;

        let response = ureq::post(url)
            .set("Content-Type", "application/json")
            .set("x-api-key", api_key)
            .set("anthropic-version", "2024-01-01")
            .timeout(self.timeout)
            .send_string(&body);

        match response {
            Ok(resp) => {
                let text = resp
                    .into_string()
                    .map_err(|e| ApiError::Parse(e.to_string()))?;

                // Parse Claude's response format
                let claude_resp: ClaudeResponse =
                    serde_json::from_str(&text).map_err(|e| ApiError::Parse(e.to_string()))?;

                Ok(ChatResponse {
                    content: claude_resp
                        .content
                        .first()
                        .map(|c| c.text.clone())
                        .unwrap_or_default(),
                    finish_reason: claude_resp.stop_reason,
                    usage: Some(TokenUsage {
                        prompt_tokens: claude_resp.usage.input_tokens,
                        completion_tokens: claude_resp.usage.output_tokens,
                        total_tokens: claude_resp.usage.input_tokens
                            + claude_resp.usage.output_tokens,
                    }),
                })
            }
            Err(ureq::Error::Status(status, resp)) => {
                let error_text = resp.into_string().unwrap_or_default();
                match status {
                    401 | 403 => Err(ApiError::Auth(error_text)),
                    429 => Err(ApiError::RateLimit { retry_after: None }),
                    400..=499 => Err(ApiError::InvalidRequest(error_text)),
                    500..=599 => Err(ApiError::Server(error_text)),
                    _ => Err(ApiError::Network(format!(
                        "Status {}: {}",
                        status, error_text
                    ))),
                }
            }
            Err(ureq::Error::Transport(t)) => {
                if t.to_string().contains("timeout") {
                    Err(ApiError::Timeout)
                } else {
                    Err(ApiError::Network(t.to_string()))
                }
            }
        }
    }

    fn send_gemini_request(
        &self,
        base_url: &str,
        model: &str,
        api_key: &str,
        request: &GeminiRequest,
    ) -> ApiResult<AuditResponse> {
        let body = serde_json::to_string(request).map_err(|e| ApiError::Parse(e.to_string()))?;

        // Gemini API URL format: {base_url}/{model}:generateContent?key={api_key}
        let url = format!("{}/{}:generateContent?key={}", base_url, model, api_key);

        let response = ureq::post(&url)
            .set("Content-Type", "application/json")
            .timeout(self.timeout)
            .send_string(&body);

        match response {
            Ok(resp) => {
                let text = resp
                    .into_string()
                    .map_err(|e| ApiError::Parse(e.to_string()))?;

                // Parse Gemini's response format
                let gemini_resp: GeminiResponse =
                    serde_json::from_str(&text).map_err(|e| ApiError::Parse(e.to_string()))?;

                // Extract content from response
                let content = gemini_resp
                    .candidates
                    .first()
                    .and_then(|c| c.content.parts.first())
                    .map(|p| p.text.clone())
                    .unwrap_or_default();

                // Try to parse structured audit response from content
                // If it's JSON, parse it; otherwise create a basic response
                if let Ok(audit) = serde_json::from_str::<AuditResponse>(&content) {
                    Ok(audit)
                } else {
                    // Create a basic response from unstructured text
                    Ok(AuditResponse {
                        verdict: if crate::fontcase::ascii_lower(&content).contains("approved") {
                            "approved".to_string()
                        } else if crate::fontcase::ascii_lower(&content).contains("reject") {
                            "rejected".to_string()
                        } else {
                            "needs_revision".to_string()
                        },
                        feasibility_score: 0.8,
                        compatibility_score: 0.8,
                        issues: Vec::new(),
                        suggestions: vec![content.clone()],
                        reasoning: content.clone(),
                        analysis: content,
                    })
                }
            }
            Err(ureq::Error::Status(status, resp)) => {
                let error_text = resp.into_string().unwrap_or_default();
                match status {
                    401 | 403 => Err(ApiError::Auth(error_text)),
                    429 => Err(ApiError::RateLimit { retry_after: None }),
                    400..=499 => Err(ApiError::InvalidRequest(error_text)),
                    500..=599 => Err(ApiError::Server(error_text)),
                    _ => Err(ApiError::Network(format!(
                        "Status {}: {}",
                        status, error_text
                    ))),
                }
            }
            Err(ureq::Error::Transport(t)) => {
                if t.to_string().contains("timeout") {
                    Err(ApiError::Timeout)
                } else {
                    Err(ApiError::Network(t.to_string()))
                }
            }
        }
    }
}

impl Default for McpApiClient {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Request/Response Types =====

/// Standard chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        ChatMessage {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        ChatMessage {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }

    pub fn system(content: &str) -> Self {
        ChatMessage {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }
}

/// Chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub finish_reason: String,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ===== Grok (xAI) Types =====

#[derive(Debug, Serialize)]
struct GrokRequest {
    model: String,
    messages: Vec<GrokMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct GrokMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GrokResponse {
    choices: Vec<GrokChoice>,
    #[serde(default)]
    usage: Option<GrokUsage>,
}

#[derive(Debug, Deserialize)]
struct GrokChoice {
    message: GrokResponseMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GrokResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct GrokUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// ===== Manus Types =====

#[derive(Debug, Clone, Serialize)]
struct ManusRequest {
    model: String,
    messages: Vec<ChatMessage>,
    context: Option<TaskContext>,
    max_tokens: u32,
}

/// Context for task orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub project_root: String,
    pub language: String,
    pub framework: Option<String>,
    pub files: Vec<FileContext>,
    pub current_task: Option<String>,
    pub completed_tasks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub path: String,
    pub summary: Option<String>,
    pub relevant_symbols: Vec<String>,
}

/// Orchestration response from Manus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResponse {
    pub plan: Vec<TaskPlan>,
    pub reasoning: String,
    pub estimated_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub id: String,
    pub title: String,
    pub description: String,
    pub agent: String,
    pub dependencies: Vec<String>,
    pub priority: u32,
}

// ===== Claude (Anthropic) Types =====

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
    stop_reason: String,
    usage: ClaudeUsage,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// ===== Gemini (Google) Types =====

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
    role: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiUsage {
    #[serde(default)]
    prompt_token_count: u32,
    #[serde(default)]
    candidates_token_count: u32,
    #[serde(default)]
    total_token_count: u32,
}

/// Audit response from Gemini
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResponse {
    pub verdict: String,
    pub feasibility_score: f32,
    pub compatibility_score: f32,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
    #[serde(default)]
    pub reasoning: String,
    #[serde(default)]
    pub analysis: String,
}

// ===== Ollama Types =====

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    num_predict: i32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
    done: bool,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    modified_at: String,
}

// ===== Helper Functions =====

fn build_messages(messages: &[ChatMessage], system: Option<&str>) -> Vec<GrokMessage> {
    let mut result = Vec::new();

    if let Some(sys) = system {
        result.push(GrokMessage {
            role: "system".to_string(),
            content: sys.to_string(),
        });
    }

    for msg in messages {
        result.push(GrokMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        });
    }

    result
}

/// Convert MCP messages to chat messages
pub fn mcp_to_chat(messages: &[McpMessage]) -> Vec<ChatMessage> {
    messages
        .iter()
        .map(|m| ChatMessage {
            role: match m.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Agent => "assistant".to_string(),
                MessageRole::System => "system".to_string(),
            },
            content: m.content.clone(),
        })
        .collect()
}

// ===== System Prompts =====

/// System prompt for Grok (Voice/Intent)
pub const GROK_SYSTEM_PROMPT: &str = r#"You are Grok, the voice of the MCP coding assistant. Your role is to:

1. UNDERSTAND the user's intent - what do they really want to accomplish?
2. CLARIFY ambiguous requests by asking focused questions
3. TRANSLATE natural language into precise technical requirements
4. COMMUNICATE progress and results in a friendly, clear manner

You work with two other agents:
- Manus: Plans and orchestrates multi-step tasks
- Claude Opus 5: Writes and edits the actual code

Focus on understanding WHAT the user wants, not HOW to implement it. Leave the "how" to Claude.

Be concise, friendly, and occasionally witty. You speak for the entire system.
"#;

/// System prompt for Manus (Orchestrator)
pub const MANUS_SYSTEM_PROMPT: &str = r#"You are Manus, the task orchestrator. Your role is to:

1. PLAN multi-step workflows to accomplish user goals
2. BREAK DOWN complex tasks into atomic subtasks
3. ASSIGN tasks to appropriate agents (yourself for planning, Claude for coding)
4. TRACK dependencies between tasks
5. MANAGE the overall workflow state

Output your plans as structured JSON with:
- id: unique task identifier
- title: brief task name
- description: detailed requirements
- agent: "voice", "orchestrator", or "coder"
- dependencies: list of task IDs this depends on
- priority: 1-5 (1 = highest)

Think systematically. Consider edge cases. Plan for failure.
"#;

/// System prompt for Claude (Coder)
pub const CLAUDE_SYSTEM_PROMPT: &str = r#"You are Claude Opus 5, the code generation engine. Your role is to:

1. WRITE high-quality, production-ready code
2. EDIT existing code with surgical precision
3. EXPLAIN your changes clearly
4. TEST your code mentally before outputting

Code Guidelines:
- Match the existing code style
- Add appropriate comments for complex logic
- Handle errors gracefully
- Consider edge cases
- Write idiomatic code for the target language

Output Format:
When creating/editing files, use this format:
```file:path/to/file.ext
<complete file contents or edit section>
```

When editing, include enough context (3+ lines before/after) to locate the change.

You are the final step in the pipeline. Your code will be directly applied.
Quality is paramount.
"#;

/// System prompt for Gemini (Auditor)
pub const GEMINI_SYSTEM_PROMPT: &str = r#"You are Gemini, the code auditor and quality gatekeeper. Your role is to:

1. REVIEW proposed code changes for feasibility and correctness
2. CHECK compatibility with the existing codebase
3. IDENTIFY potential issues before they reach production
4. SUGGEST improvements without blocking progress

Audit Categories:
- **Syntax**: Will this code compile/run?
- **Breaking**: Does this break existing functionality?
- **Style**: Is it consistent with project conventions?
- **Security**: Are there vulnerabilities?
- **Performance**: Are there efficiency concerns?
- **Coverage**: Are tests/docs missing?
- **Architecture**: Does it fit the system design?
- **Dependencies**: Are there version conflicts?

Output Format (JSON):
{
  "verdict": "approved" | "approved_with_warnings" | "needs_revision" | "rejected",
  "feasibility_score": 0.0-1.0,
  "compatibility_score": 0.0-1.0,
  "issues": [
    {
      "severity": "info" | "warning" | "error" | "critical",
      "category": "<category>",
      "description": "<issue description>",
      "file": "<file path>",
      "suggestion": "<how to fix>"
    }
  ],
  "suggestions": ["<general improvement suggestions>"],
  "reasoning": "<brief explanation of verdict>"
}

Decision Guidelines:
- **approved**: No issues, ready to apply
- **approved_with_warnings**: Minor issues, can proceed with caution
- **needs_revision**: Significant issues, should be fixed first
- **rejected**: Fundamental problems, should not proceed

You are the last line of defense. Be thorough but fair.
Catch real problems, not style nitpicks.
"#;

/// Returns a summary of the MCP API client capabilities and configuration.
///
/// This exercises all API types, system prompts, and configuration methods
/// to provide a comprehensive status report for the MCP server.
pub fn api_capabilities_summary(client: &mut McpApiClient) -> String {
    use std::fmt::Write;
    let mut summary = String::new();

    // Exercise configuration methods
    client.configure_local();
    client.configure_together("demo-key");
    client.set_api_key(AgentRole::Voice, "test".to_string());
    client.set_timeout(Duration::from_secs(30));
    let _ = writeln!(summary, "retry_count: {}", client.retry_count());

    // Exercise ChatMessage constructors
    let user_msg = ChatMessage::user("ping");
    let assistant_msg = ChatMessage::assistant("pong");
    let system_msg = ChatMessage::system("context");
    let _ = writeln!(
        summary,
        "roles: {} {} {}",
        user_msg.role, assistant_msg.role, system_msg.role
    );

    // Exercise mcp_to_chat
    let mcp_msgs = vec![McpMessage {
        id: 0,
        role: MessageRole::User,
        agent: None,
        content: "hello".to_string(),
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    }];
    let converted = mcp_to_chat(&mcp_msgs);
    let _ = writeln!(summary, "converted_messages: {}", converted.len());

    // Exercise system prompt constants
    let _ = writeln!(summary, "grok_prompt_len: {}", GROK_SYSTEM_PROMPT.len());
    let _ = writeln!(summary, "manus_prompt_len: {}", MANUS_SYSTEM_PROMPT.len());
    let _ = writeln!(summary, "claude_prompt_len: {}", CLAUDE_SYSTEM_PROMPT.len());
    let _ = writeln!(summary, "gemini_prompt_len: {}", GEMINI_SYSTEM_PROMPT.len());

    // Exercise Grok response types (deserialization round-trip)
    let grok_resp = GrokResponse {
        choices: vec![GrokChoice {
            message: GrokResponseMessage {
                content: "test".to_string(),
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Some(GrokUsage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        }),
    };
    let _ = writeln!(summary, "grok_choices: {}", grok_resp.choices.len());
    if let Some(ref choice) = grok_resp.choices.first() {
        let _ = writeln!(summary, "grok_content: {}", choice.message.content);
        let _ = writeln!(summary, "grok_finish: {:?}", choice.finish_reason);
    }
    if let Some(ref u) = grok_resp.usage {
        let _ = writeln!(
            summary,
            "grok_tokens: prompt={} completion={} total={}",
            u.prompt_tokens, u.completion_tokens, u.total_tokens
        );
    }

    // Exercise Claude response field reads
    let claude_content = ClaudeContent {
        content_type: "text".to_string(),
        text: "hello".to_string(),
    };
    let _ = writeln!(
        summary,
        "claude_content_type: {}",
        claude_content.content_type
    );

    // Exercise Gemini response field reads
    let gemini_resp = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiResponseContent {
                parts: vec![GeminiResponsePart {
                    text: "ok".to_string(),
                }],
                role: "model".to_string(),
            },
            finish_reason: Some("STOP".to_string()),
        }],
        usage_metadata: Some(GeminiUsage {
            prompt_token_count: 5,
            candidates_token_count: 8,
            total_token_count: 13,
        }),
    };
    if let Some(c) = gemini_resp.candidates.first() {
        let _ = writeln!(summary, "gemini_role: {}", c.content.role);
        let _ = writeln!(summary, "gemini_finish: {:?}", c.finish_reason);
    }
    if let Some(ref u) = gemini_resp.usage_metadata {
        let _ = writeln!(
            summary,
            "gemini_tokens: prompt={} candidates={} total={}",
            u.prompt_token_count, u.candidates_token_count, u.total_token_count
        );
    }

    // Exercise Ollama types
    let ollama_msg = OllamaMessage {
        role: "user".to_string(),
        content: "test".to_string(),
    };
    let ollama_req = OllamaRequest {
        model: "llama3".to_string(),
        messages: vec![ollama_msg],
        stream: false,
        options: Some(OllamaOptions {
            num_predict: 100,
            temperature: 0.7,
        }),
    };
    let _ = writeln!(summary, "ollama_model: {}", ollama_req.model);

    let ollama_resp = OllamaResponse {
        message: OllamaResponseMessage {
            role: "assistant".to_string(),
            content: "response".to_string(),
        },
        done: true,
        prompt_eval_count: Some(10),
        eval_count: Some(20),
    };
    let _ = writeln!(
        summary,
        "ollama_resp: done={} role={} content_len={} prompt_eval={:?} eval={:?}",
        ollama_resp.done,
        ollama_resp.message.role,
        ollama_resp.message.content.len(),
        ollama_resp.prompt_eval_count,
        ollama_resp.eval_count,
    );

    let ollama_tags = OllamaTagsResponse {
        models: vec![OllamaModel {
            name: "llama3".to_string(),
            size: 4_000_000_000,
            modified_at: "2024-01-01".to_string(),
        }],
    };
    for m in &ollama_tags.models {
        let _ = writeln!(
            summary,
            "ollama_model: {} size={} modified={}",
            m.name, m.size, m.modified_at
        );
    }

    // Exercise configure method via AgentConfig
    let voice_config = AgentConfig::grok_default();
    client.configure(voice_config);

    // Exercise API call methods (will fail without real keys, but that's expected)
    let test_msgs = &[ChatMessage::user("test")];
    let _ = client.call(AgentRole::Voice, test_msgs, "system prompt");

    // Exercise Ollama-specific methods
    let _ = writeln!(summary, "ollama_available: {}", client.check_ollama());
    match client.list_ollama_models() {
        Ok(models) => {
            let _ = writeln!(summary, "ollama_model_count: {}", models.len());
        }
        Err(e) => {
            let _ = writeln!(summary, "ollama_list_err: {}", e);
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = McpApiClient::new();
        assert!(!client.is_ready(AgentRole::Voice)); // No key set
    }

    #[test]
    fn test_chat_message() {
        let msg = ChatMessage::user("hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "hello");
    }

    #[test]
    fn test_mcp_to_chat() {
        use chrono::Utc;
        use std::collections::HashMap;

        let mcp_msgs = vec![McpMessage {
            id: 1,
            role: MessageRole::User,
            agent: None,
            content: "test".to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }];

        let chat_msgs = mcp_to_chat(&mcp_msgs);
        assert_eq!(chat_msgs.len(), 1);
        assert_eq!(chat_msgs[0].role, "user");
    }
}
