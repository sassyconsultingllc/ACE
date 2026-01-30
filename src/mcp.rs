//! Model Context Protocol (MCP) - Multi-Agent AI Orchestration
//!
//! Built-in conversational code editing through coordinated AI agents:
//! - **Grok**: Voice and logic - understands intent, speaks to user
//! - **Manus**: Flow orchestration - breaks down tasks, manages workflow  
//! - **Claude Opus 5**: Code pusher - writes and edits actual code

#![allow(unused_variables)]
//! - **Gemini**: Auditor - reviews feasibility, ensures project coherence
//!
//! Supports multiple hosting modes:
//! - **Cloud**: xAI, Anthropic, Google, Together.ai
//! - **Self-hosted**: Hugging Face Inference Endpoints
//! - **Local**: Ollama (with optional ngrok tunneling)
//! - **Hybrid**: Mix cloud and local for cost/privacy balance
//!
//! "VS Code is a sandcastle. This is Vlad's castle."

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Utc};

/// Hosting mode for MCP agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum HostingMode {
    /// Use cloud APIs (xAI, Anthropic, Google)
    #[default]
    Cloud,
    /// Use self-hosted endpoints (Hugging Face Inference)
    SelfHosted,
    /// Mix of cloud and local
    Hybrid,
    /// All local via Ollama
    Local,
}


/// Provider for AI model hosting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
    /// xAI (Grok)
    #[serde(rename = "XAI")]
    Xai,
    /// Anthropic (Claude)
    Anthropic,
    /// Google (Gemini)
    Google,
    /// Together.ai (various open models)
    Together,
    /// Hugging Face Inference Endpoints
    HuggingFace,
    /// Local Ollama
    Ollama,
    /// OpenAI-compatible endpoint
    OpenAI,
    /// Custom endpoint
    Custom,
}

impl Provider {
    pub fn name(&self) -> &'static str {
        match self {
            Provider::Xai => "xAI",
            Provider::Anthropic => "Anthropic",
            Provider::Google => "Google",
            Provider::Together => "Together.ai",
            Provider::HuggingFace => "Hugging Face",
            Provider::Ollama => "Ollama",
            Provider::OpenAI => "OpenAI",
            Provider::Custom => "Custom",
        }
    }
    
    pub fn is_local(&self) -> bool {
        matches!(self, Provider::Ollama)
    }
    
    pub fn is_openai_compatible(&self) -> bool {
        matches!(self, Provider::Together | Provider::OpenAI | Provider::Xai)
    }
}

/// Agent role in the MCP system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    /// Grok - Conversational AI, understands user intent
    Voice,
    /// Manus - Task orchestration, workflow management
    Orchestrator,
    /// Claude Opus 5 - Code generation and editing
    Coder,
    /// Gemini - Reviews feasibility, ensures project coherence
    Auditor,
}

impl AgentRole {
    pub fn name(&self) -> &'static str {
        match self {
            AgentRole::Voice => "Grok",
            AgentRole::Orchestrator => "Manus",
            AgentRole::Coder => "Claude Opus 5",
            AgentRole::Auditor => "Gemini",
        }
    }
    
    pub fn icon(&self) -> &'static str {
        match self {
            AgentRole::Voice => "",
            AgentRole::Orchestrator => "",
            AgentRole::Coder => "âš¡",
            AgentRole::Auditor => "",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            AgentRole::Voice => "Understands your intent, speaks naturally",
            AgentRole::Orchestrator => "Plans tasks, manages workflow",
            AgentRole::Coder => "Writes and edits code with precision",
            AgentRole::Auditor => "Reviews feasibility, ensures coherence",
        }
    }
}

/// Configuration for an AI agent endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub role: AgentRole,
    pub provider: Provider,
    pub api_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub enabled: bool,
}

impl AgentConfig {
    pub fn grok_default() -> Self {
        AgentConfig {
            role: AgentRole::Voice,
            provider: Provider::Xai,
            api_url: "https://api.x.ai/v1/chat/completions".to_string(),
            api_key: None,
            model: "grok-2".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            enabled: true,
        }
    }
    
    pub fn manus_default() -> Self {
        AgentConfig {
            role: AgentRole::Orchestrator,
            provider: Provider::Together,
            api_url: "https://api.together.xyz/v1/chat/completions".to_string(),
            api_key: None,
            model: "mistralai/Mixtral-8x22B-Instruct-v0.1".to_string(),
            max_tokens: 8192,
            temperature: 0.3,
            enabled: true,
        }
    }
    
    pub fn claude_default() -> Self {
        AgentConfig {
            role: AgentRole::Coder,
            provider: Provider::Anthropic,
            api_url: "https://api.anthropic.com/v1/messages".to_string(),
            api_key: None,
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 16384,
            temperature: 0.1,
            enabled: true,
        }
    }
    
    pub fn gemini_default() -> Self {
        AgentConfig {
            role: AgentRole::Auditor,
            provider: Provider::Google,
            api_url: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
            api_key: None,
            model: "gemini-2.5-pro-preview-06-05".to_string(),
            max_tokens: 65536,
            temperature: 0.2,
            enabled: true,
        }
    }
    
    // =========================================================================
    // Alternative Provider Configurations
    // =========================================================================
    
    /// Voice via Together.ai (Llama 3.3 70B)
    pub fn voice_together() -> Self {
        AgentConfig {
            role: AgentRole::Voice,
            provider: Provider::Together,
            api_url: "https://api.together.xyz/v1/chat/completions".to_string(),
            api_key: None,
            model: "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            enabled: true,
        }
    }
    
    /// Coder via Together.ai (Qwen 2.5 Coder 32B)
    pub fn coder_together() -> Self {
        AgentConfig {
            role: AgentRole::Coder,
            provider: Provider::Together,
            api_url: "https://api.together.xyz/v1/chat/completions".to_string(),
            api_key: None,
            model: "Qwen/Qwen2.5-Coder-32B-Instruct".to_string(),
            max_tokens: 8192,
            temperature: 0.3,
            enabled: true,
        }
    }
    
    /// Auditor via Together.ai (DeepSeek V3)
    pub fn auditor_together() -> Self {
        AgentConfig {
            role: AgentRole::Auditor,
            provider: Provider::Together,
            api_url: "https://api.together.xyz/v1/chat/completions".to_string(),
            api_key: None,
            model: "deepseek-ai/DeepSeek-V3".to_string(),
            max_tokens: 32768,
            temperature: 0.2,
            enabled: true,
        }
    }
    
    // =========================================================================
    // Local Ollama Configurations
    // =========================================================================
    
    /// Voice via Ollama (Llama 3.3)
    pub fn voice_ollama() -> Self {
        AgentConfig {
            role: AgentRole::Voice,
            provider: Provider::Ollama,
            api_url: "http://localhost:11434/api/chat".to_string(),
            api_key: None,
            model: "llama3.3:70b".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            enabled: true,
        }
    }
    
    /// Orchestrator via Ollama (Mixtral)
    pub fn orchestrator_ollama() -> Self {
        AgentConfig {
            role: AgentRole::Orchestrator,
            provider: Provider::Ollama,
            api_url: "http://localhost:11434/api/chat".to_string(),
            api_key: None,
            model: "mixtral:8x22b".to_string(),
            max_tokens: 8192,
            temperature: 0.3,
            enabled: true,
        }
    }
    
    /// Coder via Ollama (Qwen 2.5 Coder)
    pub fn coder_ollama() -> Self {
        AgentConfig {
            role: AgentRole::Coder,
            provider: Provider::Ollama,
            api_url: "http://localhost:11434/api/chat".to_string(),
            api_key: None,
            model: "qwen2.5-coder:32b".to_string(),
            max_tokens: 8192,
            temperature: 0.3,
            enabled: true,
        }
    }
    
    /// Auditor via Ollama (DeepSeek Coder V2)
    pub fn auditor_ollama() -> Self {
        AgentConfig {
            role: AgentRole::Auditor,
            provider: Provider::Ollama,
            api_url: "http://localhost:11434/api/chat".to_string(),
            api_key: None,
            model: "deepseek-coder-v2:latest".to_string(),
            max_tokens: 32768,
            temperature: 0.2,
            enabled: true,
        }
    }
    
    // =========================================================================
    // Hugging Face Inference Endpoint Configurations
    // =========================================================================
    
    /// Create a Hugging Face Inference Endpoint config
    pub fn huggingface(role: AgentRole, endpoint_url: &str, model: &str) -> Self {
        let (max_tokens, temperature) = match role {
            AgentRole::Voice => (4096, 0.7),
            AgentRole::Orchestrator => (8192, 0.3),
            AgentRole::Coder => (8192, 0.3),
            AgentRole::Auditor => (32768, 0.2),
        };
        
        AgentConfig {
            role,
            provider: Provider::HuggingFace,
            api_url: endpoint_url.to_string(),
            api_key: None,
            model: model.to_string(),
            max_tokens,
            temperature,
            enabled: true,
        }
    }
    
    /// Set custom endpoint URL (for ngrok or self-hosted)
    pub fn with_url(mut self, url: &str) -> Self {
        self.api_url = url.to_string();
        self
    }
    
    /// Set API key
    pub fn with_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_string());
        self
    }
    
    /// Set custom model
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }
}

/// Message in the MCP conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMessage {
    pub id: u64,
    pub role: MessageRole,
    pub agent: Option<AgentRole>,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Agent,
    System,
}

/// A task in the workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub assigned_to: AgentRole,
    pub parent_id: Option<u64>,
    pub subtasks: Vec<u64>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Review,
    Completed,
    Failed,
    Blocked,
}

impl TaskStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "â³",
            TaskStatus::InProgress => "",
            TaskStatus::Review => "",
            TaskStatus::Completed => "âœ…",
            TaskStatus::Failed => "âŒ",
            TaskStatus::Blocked => "",
        }
    }
}

/// Audit result from Gemini
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub id: u64,
    pub verdict: AuditVerdict,
    pub feasibility_score: f32,      // 0.0 - 1.0
    pub compatibility_score: f32,    // 0.0 - 1.0
    pub issues: Vec<AuditIssue>,
    pub suggestions: Vec<String>,
    pub affected_files: Vec<String>,
    pub estimated_impact: ImpactLevel,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditVerdict {
    /// Good to go, no issues found
    Approved,
    /// Minor issues, can proceed with caution
    ApprovedWithWarnings,
    /// Needs revision before proceeding
    NeedsRevision,
    /// Fundamental issues, should not proceed
    Rejected,
}

impl AuditVerdict {
    pub fn icon(&self) -> &'static str {
        match self {
            AuditVerdict::Approved => "âœ…",
            AuditVerdict::ApprovedWithWarnings => "âš ï¸",
            AuditVerdict::NeedsRevision => "",
            AuditVerdict::Rejected => "âŒ",
        }
    }
    
    pub fn can_proceed(&self) -> bool {
        matches!(self, AuditVerdict::Approved | AuditVerdict::ApprovedWithWarnings)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub description: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl IssueSeverity {
    pub fn icon(&self) -> &'static str {
        match self {
            IssueSeverity::Info => "â„¹ï¸",
            IssueSeverity::Warning => "âš ï¸",
            IssueSeverity::Error => "âŒ",
            IssueSeverity::Critical => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueCategory {
    /// Code won't compile or run
    Syntax,
    /// Breaks existing functionality
    Breaking,
    /// Inconsistent with project style
    Style,
    /// Security vulnerability
    Security,
    /// Performance concern
    Performance,
    /// Missing tests or docs
    Coverage,
    /// Architectural concern
    Architecture,
    /// Dependency issue
    Dependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpactLevel {
    /// No impact on existing code
    None,
    /// Affects only the new code
    Isolated,
    /// Affects a few related files
    Localized,
    /// Affects a module/feature
    Moderate,
    /// Affects multiple modules
    Significant,
    /// Affects the entire project
    Pervasive,
}

/// Artifact produced by a task (code, file, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: u64,
    pub artifact_type: ArtifactType,
    pub path: Option<String>,
    pub content: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ArtifactType {
    Code,
    File,
    Diff,
    Command,
    Documentation,
    Test,
}

/// Code edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEdit {
    pub file_path: String,
    pub operation: EditOperation,
    pub old_content: Option<String>,
    pub new_content: String,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EditOperation {
    Create,
    Replace,
    Insert,
    Delete,
    Append,
}

/// Project context for the AI agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub root_path: String,
    pub language: String,
    pub framework: Option<String>,
    pub files: Vec<FileInfo>,
    pub dependencies: Vec<String>,
    pub git_branch: Option<String>,
    pub recent_changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub language: String,
    pub size_bytes: u64,
    pub last_modified: DateTime<Utc>,
    pub summary: Option<String>,
}

/// The MCP Orchestrator - coordinates all agents
pub struct McpOrchestrator {
    /// Agent configurations
    pub agents: HashMap<AgentRole, AgentConfig>,
    
    /// Conversation history
    pub conversation: VecDeque<McpMessage>,
    pub max_history: usize,
    
    /// Active tasks
    pub tasks: HashMap<u64, Task>,
    pub task_queue: VecDeque<u64>,
    pub next_task_id: u64,
    
    /// Project context
    pub context: Option<ProjectContext>,
    
    /// Pending code edits awaiting approval
    pub pending_edits: Vec<CodeEdit>,
    
    /// Audit results from Gemini
    pub audit_history: Vec<AuditResult>,
    pub last_audit: Option<AuditResult>,
    next_audit_id: u64,
    
    /// Session state
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub is_active: bool,
    
    /// Message counter
    next_message_id: u64,
    next_artifact_id: u64,
}

impl McpOrchestrator {
    pub fn new() -> Self {
        let mut agents = HashMap::new();
        agents.insert(AgentRole::Voice, AgentConfig::grok_default());
        agents.insert(AgentRole::Orchestrator, AgentConfig::manus_default());
        agents.insert(AgentRole::Coder, AgentConfig::claude_default());
        agents.insert(AgentRole::Auditor, AgentConfig::gemini_default());
        
        McpOrchestrator {
            agents,
            conversation: VecDeque::new(),
            max_history: 100,
            tasks: HashMap::new(),
            task_queue: VecDeque::new(),
            next_task_id: 1,
            context: None,
            pending_edits: Vec::new(),
            audit_history: Vec::new(),
            last_audit: None,
            next_audit_id: 1,
            session_id: generate_session_id(),
            started_at: Utc::now(),
            is_active: false,
            next_message_id: 1,
            next_artifact_id: 1,
        }
    }
    
    /// Configure an agent
    pub fn configure_agent(&mut self, config: AgentConfig) {
        self.agents.insert(config.role, config);
    }
    
    /// Set API key for an agent
    pub fn set_api_key(&mut self, role: AgentRole, key: String) {
        if let Some(agent) = self.agents.get_mut(&role) {
            agent.api_key = Some(key);
        }
    }
    
    /// Start a new session
    pub fn start_session(&mut self) {
        self.session_id = generate_session_id();
        self.started_at = Utc::now();
        self.is_active = true;
        self.conversation.clear();
        self.tasks.clear();
        self.task_queue.clear();
        self.pending_edits.clear();
        self.audit_history.clear();
        self.last_audit = None;
        
        // Add system message
        self.add_system_message(
            "MCP Session started. Agents online:\n\
             Grok (Voice) - Ready to understand your intent\n\
             Manus (Orchestrator) - Ready to plan your workflow\n\
             âš¡ Claude Opus 5 (Coder) - Ready to write code\n\
             Gemini (Auditor) - Ready to review feasibility"
        );
    }
    
    /// Process user input - the main entry point
    pub fn process_input(&mut self, input: &str) -> Vec<McpMessage> {
        let mut responses = Vec::new();
        
        // Add user message
        let user_msg = self.add_user_message(input);
        responses.push(user_msg);
        
        // Phase 1: Grok understands intent
        let intent = self.voice_understand(input);
        responses.push(self.add_agent_message(
            AgentRole::Voice,
            &format!("I understand you want to: {}", intent.summary)
        ));
        
        // Phase 2: Manus creates task plan
        let tasks = self.orchestrator_plan(&intent);
        responses.push(self.add_agent_message(
            AgentRole::Orchestrator,
            &format!("I've created {} tasks to accomplish this:\n{}", 
                tasks.len(),
                tasks.iter().map(|t| format!("  {} {}", t.status.icon(), t.title)).collect::<Vec<_>>().join("\n")
            )
        ));
        
        // Phase 3: Claude executes code tasks
        for task in &tasks {
            if task.assigned_to == AgentRole::Coder {
                let edits = self.coder_execute(task);
                if !edits.is_empty() {
                    self.pending_edits.extend(edits.clone());
                    responses.push(self.add_agent_message(
                        AgentRole::Coder,
                        &format!("I've prepared {} code changes for task '{}':\n{}",
                            edits.len(),
                            task.title,
                            edits.iter().map(|e| format!("  â€¢ {} ({})", e.file_path, format_operation(&e.operation))).collect::<Vec<_>>().join("\n")
                        )
                    ));
                }
            }
        }
        
        // Phase 4: Gemini audits the proposed changes
        if !self.pending_edits.is_empty() {
            let audit = self.auditor_review(&self.pending_edits.clone(), &tasks);
            responses.push(self.add_agent_message(
                AgentRole::Auditor,
                &format!("{} Audit complete: {}\n  Feasibility: {:.0}% | Compatibility: {:.0}%\n  {}",
                    audit.verdict.icon(),
                    match audit.verdict {
                        AuditVerdict::Approved => "All clear, ready to apply",
                        AuditVerdict::ApprovedWithWarnings => "Can proceed with minor concerns",
                        AuditVerdict::NeedsRevision => "Issues found, revision recommended",
                        AuditVerdict::Rejected => "Fundamental problems detected",
                    },
                    audit.feasibility_score * 100.0,
                    audit.compatibility_score * 100.0,
                    if audit.issues.is_empty() {
                        "No issues found".to_string()
                    } else {
                        format!("{} issue(s) found", audit.issues.len())
                    }
                )
            ));
            
            self.last_audit = Some(audit.clone());
            self.audit_history.push(audit);
        }
        
        // Add tasks to queue
        for task in tasks {
            let id = task.id;
            self.tasks.insert(id, task);
            self.task_queue.push_back(id);
        }
        
        responses
    }
    
    /// Voice agent understands user intent
    fn voice_understand(&self, input: &str) -> UserIntent {
        // In real implementation, this calls Grok API
        // For now, parse intent locally
        
        let input_lower = crate::fontcase::ascii_lower(input);
        
        let intent_type = if input_lower.contains("create") || input_lower.contains("new") || input_lower.contains("add") {
            IntentType::Create
        } else if input_lower.contains("fix") || input_lower.contains("bug") || input_lower.contains("error") {
            IntentType::Fix
        } else if input_lower.contains("refactor") || input_lower.contains("improve") || input_lower.contains("clean") {
            IntentType::Refactor
        } else if input_lower.contains("explain") || input_lower.contains("what") || input_lower.contains("how") {
            IntentType::Explain
        } else if input_lower.contains("test") {
            IntentType::Test
        } else if input_lower.contains("document") || input_lower.contains("comment") {
            IntentType::Document
        } else {
            IntentType::General
        };
        
        UserIntent {
            summary: input.to_string(),
            intent_type,
            entities: extract_entities(input),
            confidence: 0.85,
        }
    }
    
    /// Orchestrator creates task plan
    fn orchestrator_plan(&mut self, intent: &UserIntent) -> Vec<Task> {
        // In real implementation, this calls Manus API
        // For now, create tasks based on intent
        
        let mut tasks = Vec::new();
        
        match intent.intent_type {
            IntentType::Create => {
                tasks.push(self.create_task(
                    "Analyze requirements",
                    "Understand what needs to be created",
                    AgentRole::Voice
                ));
                tasks.push(self.create_task(
                    "Design structure",
                    "Plan the code structure and interfaces",
                    AgentRole::Orchestrator
                ));
                tasks.push(self.create_task(
                    "Implement code",
                    &intent.summary,
                    AgentRole::Coder
                ));
                tasks.push(self.create_task(
                    "Add tests",
                    "Create unit tests for new code",
                    AgentRole::Coder
                ));
            }
            IntentType::Fix => {
                tasks.push(self.create_task(
                    "Identify bug",
                    "Locate the source of the issue",
                    AgentRole::Voice
                ));
                tasks.push(self.create_task(
                    "Create fix",
                    &intent.summary,
                    AgentRole::Coder
                ));
                tasks.push(self.create_task(
                    "Verify fix",
                    "Ensure the fix works correctly",
                    AgentRole::Coder
                ));
            }
            IntentType::Refactor => {
                tasks.push(self.create_task(
                    "Analyze current code",
                    "Understand what needs refactoring",
                    AgentRole::Voice
                ));
                tasks.push(self.create_task(
                    "Plan refactor",
                    "Design the improved structure",
                    AgentRole::Orchestrator
                ));
                tasks.push(self.create_task(
                    "Apply refactor",
                    &intent.summary,
                    AgentRole::Coder
                ));
            }
            IntentType::Test => {
                tasks.push(self.create_task(
                    "Identify test cases",
                    "Determine what needs testing",
                    AgentRole::Orchestrator
                ));
                tasks.push(self.create_task(
                    "Write tests",
                    &intent.summary,
                    AgentRole::Coder
                ));
            }
            IntentType::Document => {
                tasks.push(self.create_task(
                    "Analyze code",
                    "Understand code to document",
                    AgentRole::Voice
                ));
                tasks.push(self.create_task(
                    "Write documentation",
                    &intent.summary,
                    AgentRole::Coder
                ));
            }
            IntentType::Explain | IntentType::General => {
                tasks.push(self.create_task(
                    "Research",
                    &intent.summary,
                    AgentRole::Voice
                ));
            }
        }
        
        tasks
    }
    
    /// Coder executes a task and produces code edits
    fn coder_execute(&mut self, task: &Task) -> Vec<CodeEdit> {
        // In real implementation, this calls Claude API
        // For now, return placeholder edits
        
        let artifact_id = self.next_artifact_id;
        self.next_artifact_id += 1;
        
        vec![CodeEdit {
            file_path: "src/new_feature.rs".to_string(),
            operation: EditOperation::Create,
            old_content: None,
            new_content: format!(
                "// Auto-generated for task: {}\n\
                 // TODO: Implement {}\n\n\
                 pub fn placeholder() {{\n    \
                     unimplemented!(\"Generated by Claude Opus 5\")\n\
                 }}\n",
                task.title,
                task.description
            ),
            line_start: None,
            line_end: None,
            description: task.description.clone(),
        }]
    }
    
    /// Auditor reviews proposed changes for feasibility and compatibility
    fn auditor_review(&mut self, edits: &[CodeEdit], tasks: &[Task]) -> AuditResult {
        // In real implementation, this calls Gemini API with full project context
        // For now, perform basic static analysis
        
        let id = self.next_audit_id;
        self.next_audit_id += 1;
        
        let mut issues = Vec::new();
        let mut feasibility_score = 1.0_f32;
        let mut compatibility_score = 1.0_f32;
        let mut affected_files: Vec<String> = Vec::new();
        
        for edit in edits {
            affected_files.push(edit.file_path.clone());
            
            // Check for potential issues
            let content = &edit.new_content;
            
            // Check for unimplemented/todo markers
            if content.contains("unimplemented!") || content.contains("todo!") {
                issues.push(AuditIssue {
                    severity: IssueSeverity::Warning,
                    category: IssueCategory::Coverage,
                    description: "Code contains unimplemented sections".to_string(),
                    file_path: Some(edit.file_path.clone()),
                    line_number: None,
                    suggestion: Some("Complete implementation before deploying".to_string()),
                });
                feasibility_score -= 0.1;
            }
            
            // Check for unwrap() calls (potential panics)
            if content.contains(".unwrap()") {
                issues.push(AuditIssue {
                    severity: IssueSeverity::Warning,
                    category: IssueCategory::Security,
                    description: "Code uses .unwrap() which may panic".to_string(),
                    file_path: Some(edit.file_path.clone()),
                    line_number: None,
                    suggestion: Some("Consider using .ok()?, .expect(), or proper error handling".to_string()),
                });
                compatibility_score -= 0.05;
            }
            
            // Check for unsafe blocks
            if content.contains("unsafe {") || content.contains("unsafe{") {
                issues.push(AuditIssue {
                    severity: IssueSeverity::Error,
                    category: IssueCategory::Security,
                    description: "Code contains unsafe blocks".to_string(),
                    file_path: Some(edit.file_path.clone()),
                    line_number: None,
                    suggestion: Some("Verify unsafe code is necessary and properly documented".to_string()),
                });
                compatibility_score -= 0.15;
            }
            
            // Check for missing error handling patterns
            if content.contains("panic!") {
                issues.push(AuditIssue {
                    severity: IssueSeverity::Warning,
                    category: IssueCategory::Architecture,
                    description: "Code uses panic! for error handling".to_string(),
                    file_path: Some(edit.file_path.clone()),
                    line_number: None,
                    suggestion: Some("Consider returning Result<T, E> instead".to_string()),
                });
                feasibility_score -= 0.1;
            }
            
            // Check for very long functions (complexity)
            let line_count = content.lines().count();
            if line_count > 100 {
                issues.push(AuditIssue {
                    severity: IssueSeverity::Info,
                    category: IssueCategory::Architecture,
                    description: format!("Generated code is {} lines - consider breaking into smaller functions", line_count),
                    file_path: Some(edit.file_path.clone()),
                    line_number: None,
                    suggestion: Some("Extract helper functions for better maintainability".to_string()),
                });
            }
            
            // Check for hardcoded secrets/credentials patterns
            let secrets_patterns = ["password", "secret", "api_key", "token", "credential"];
            for pattern in secrets_patterns {
                if crate::fontcase::ascii_lower(content).contains(pattern) && content.contains("\"") {
                    issues.push(AuditIssue {
                        severity: IssueSeverity::Critical,
                        category: IssueCategory::Security,
                        description: format!("Possible hardcoded {} detected", pattern),
                        file_path: Some(edit.file_path.clone()),
                        line_number: None,
                        suggestion: Some("Use environment variables or secure configuration".to_string()),
                    });
                    compatibility_score -= 0.2;
                }
            }
        }
        
        // Determine verdict based on scores and issues
        let has_critical = issues.iter().any(|i| i.severity == IssueSeverity::Critical);
        let has_errors = issues.iter().any(|i| i.severity == IssueSeverity::Error);
        let warning_count = issues.iter().filter(|i| i.severity == IssueSeverity::Warning).count();
        
        let verdict = if has_critical {
            AuditVerdict::Rejected
        } else if has_errors || feasibility_score < 0.6 || compatibility_score < 0.6 {
            AuditVerdict::NeedsRevision
        } else if warning_count > 0 || feasibility_score < 0.9 || compatibility_score < 0.9 {
            AuditVerdict::ApprovedWithWarnings
        } else {
            AuditVerdict::Approved
        };
        
        // Calculate impact level
        let impact = if affected_files.is_empty() {
            ImpactLevel::None
        } else if affected_files.len() == 1 {
            ImpactLevel::Isolated
        } else if affected_files.len() <= 3 {
            ImpactLevel::Localized
        } else if affected_files.len() <= 10 {
            ImpactLevel::Moderate
        } else {
            ImpactLevel::Significant
        };
        
        // Generate suggestions
        let mut suggestions = Vec::new();
        if !issues.is_empty() {
            suggestions.push("Review the issues above before applying changes".to_string());
        }
        if tasks.len() > 5 {
            suggestions.push("Consider breaking this into smaller incremental changes".to_string());
        }
        if affected_files.len() > 3 {
            suggestions.push("Multiple files affected - test thoroughly".to_string());
        }
        
        AuditResult {
            id,
            verdict,
            feasibility_score: feasibility_score.clamp(0.0, 1.0),
            compatibility_score: compatibility_score.clamp(0.0, 1.0),
            issues,
            suggestions,
            affected_files,
            estimated_impact: impact,
            timestamp: Utc::now(),
        }
    }
    
    /// Get the last audit result
    pub fn last_audit_result(&self) -> Option<&AuditResult> {
        self.last_audit.as_ref()
    }
    
    /// Check if pending edits passed audit
    pub fn edits_approved(&self) -> bool {
        self.last_audit.as_ref()
            .map(|a| a.verdict.can_proceed())
            .unwrap_or(false)
    }
    
    /// Create a new task
    fn create_task(&mut self, title: &str, description: &str, assigned_to: AgentRole) -> Task {
        let id = self.next_task_id;
        self.next_task_id += 1;
        
        Task {
            id,
            title: title.to_string(),
            description: description.to_string(),
            status: TaskStatus::Pending,
            assigned_to,
            parent_id: None,
            subtasks: Vec::new(),
            created_at: Utc::now(),
            completed_at: None,
            artifacts: Vec::new(),
        }
    }
    
    /// Add a user message
    fn add_user_message(&mut self, content: &str) -> McpMessage {
        let msg = McpMessage {
            id: self.next_message_id,
            role: MessageRole::User,
            agent: None,
            content: content.to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        self.next_message_id += 1;
        self.conversation.push_back(msg.clone());
        self.trim_history();
        msg
    }
    
    /// Add an agent message
    fn add_agent_message(&mut self, agent: AgentRole, content: &str) -> McpMessage {
        let msg = McpMessage {
            id: self.next_message_id,
            role: MessageRole::Agent,
            agent: Some(agent),
            content: content.to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        self.next_message_id += 1;
        self.conversation.push_back(msg.clone());
        self.trim_history();
        msg
    }
    
    /// Add a system message
    fn add_system_message(&mut self, content: &str) -> McpMessage {
        let msg = McpMessage {
            id: self.next_message_id,
            role: MessageRole::System,
            agent: None,
            content: content.to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        self.next_message_id += 1;
        self.conversation.push_back(msg.clone());
        msg
    }
    
    fn trim_history(&mut self) {
        while self.conversation.len() > self.max_history {
            self.conversation.pop_front();
        }
    }
    
    /// Approve pending edits
    pub fn approve_edits(&mut self) -> Vec<CodeEdit> {
        let edits = std::mem::take(&mut self.pending_edits);
        
        // Mark related tasks as completed
        for task in self.tasks.values_mut() {
            if task.status == TaskStatus::InProgress {
                task.status = TaskStatus::Completed;
                task.completed_at = Some(Utc::now());
            }
        }
        
        edits
    }
    
    /// Reject pending edits
    pub fn reject_edits(&mut self) {
        self.pending_edits.clear();
        
        self.add_agent_message(
            AgentRole::Coder,
            "Edits rejected. Let me know how you'd like me to revise them."
        );
    }
    
    /// Get task by ID
    pub fn get_task(&self, id: u64) -> Option<&Task> {
        self.tasks.get(&id)
    }
    
    /// Update task status
    pub fn update_task_status(&mut self, id: u64, status: TaskStatus) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.status = status;
            if status == TaskStatus::Completed {
                task.completed_at = Some(Utc::now());
            }
        }
    }
    
    /// Get all pending tasks
    pub fn pending_tasks(&self) -> Vec<&Task> {
        self.tasks.values()
            .filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::InProgress)
            .collect()
    }
    
    /// Get conversation history
    pub fn history(&self) -> Vec<&McpMessage> {
        self.conversation.iter().collect()
    }
    
    /// Load project context
    pub fn load_context(&mut self, root_path: &str) {
        // In real implementation, scan project files
        self.context = Some(ProjectContext {
            root_path: root_path.to_string(),
            language: "rust".to_string(),
            framework: Some("sassy-browser".to_string()),
            files: Vec::new(),
            dependencies: Vec::new(),
            git_branch: Some("main".to_string()),
            recent_changes: Vec::new(),
        });
    }
    
    /// Build API request for an agent
    pub fn build_request(&self, agent: AgentRole, messages: &[McpMessage]) -> Option<AgentRequest> {
        let config = self.agents.get(&agent)?;
        
        if !config.enabled || config.api_key.is_none() {
            return None;
        }
        
        Some(AgentRequest {
            model: config.model.clone(),
            messages: messages.iter().map(|m| RequestMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Agent => "assistant".to_string(),
                    MessageRole::System => "system".to_string(),
                },
                content: m.content.clone(),
            }).collect(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        })
    }
}

impl Default for McpOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// User intent parsed by voice agent
#[derive(Debug, Clone)]
pub struct UserIntent {
    pub summary: String,
    pub intent_type: IntentType,
    pub entities: Vec<Entity>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntentType {
    Create,
    Fix,
    Refactor,
    Explain,
    Test,
    Document,
    General,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_type: String,
    pub value: String,
    pub start: usize,
    pub end: usize,
}

/// API request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub model: String,
    pub messages: Vec<RequestMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMessage {
    pub role: String,
    pub content: String,
}

/// API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub content: String,
    pub finish_reason: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// Helper functions

fn generate_session_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 8] = rng.r#gen();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn extract_entities(input: &str) -> Vec<Entity> {
    let mut entities = Vec::new();
    
    // Extract file paths
    for (i, word) in input.split_whitespace().enumerate() {
        if word.contains('.') && (word.ends_with(".rs") || word.ends_with(".js") || word.ends_with(".py") || word.ends_with(".ts")) {
            entities.push(Entity {
                entity_type: "file".to_string(),
                value: word.to_string(),
                start: i,
                end: i + 1,
            });
        }
        
        // Extract function names (snake_case or camelCase patterns)
        if word.contains('_') || (word.chars().any(|c| c.is_lowercase()) && word.chars().any(|c| c.is_uppercase())) {
            entities.push(Entity {
                entity_type: "identifier".to_string(),
                value: word.to_string(),
                start: i,
                end: i + 1,
            });
        }
    }
    
    entities
}

fn format_operation(op: &EditOperation) -> &'static str {
    match op {
        EditOperation::Create => "create",
        EditOperation::Replace => "replace",
        EditOperation::Insert => "insert",
        EditOperation::Delete => "delete",
        EditOperation::Append => "append",
    }
}

/// MCP Server - handles incoming connections
pub struct McpServer {
    pub orchestrator: McpOrchestrator,
    pub port: u16,
    pub is_running: bool,
}

impl McpServer {
    pub fn new(port: u16) -> Self {
        McpServer {
            orchestrator: McpOrchestrator::new(),
            port,
            is_running: false,
        }
    }
    
    /// Start the MCP server
    pub fn start(&mut self) {
        self.is_running = true;
        self.orchestrator.start_session();
    }
    
    /// Stop the MCP server
    pub fn stop(&mut self) {
        self.is_running = false;
        self.orchestrator.is_active = false;
    }
    
    /// Handle incoming request
    pub fn handle_request(&mut self, request: &str) -> String {
        let responses = self.orchestrator.process_input(request);
        
        // Format responses
        responses.iter()
            .filter(|m| m.role == MessageRole::Agent)
            .map(|m| {
                let agent = m.agent.map(|a| a.icon()).unwrap_or("");
                format!("{} {}", agent, m.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_orchestrator_creation() {
        let mcp = McpOrchestrator::new();
        assert!(mcp.agents.contains_key(&AgentRole::Voice));
        assert!(mcp.agents.contains_key(&AgentRole::Orchestrator));
        assert!(mcp.agents.contains_key(&AgentRole::Coder));
    }
    
    #[test]
    fn test_process_input() {
        let mut mcp = McpOrchestrator::new();
        mcp.start_session();
        
        let responses = mcp.process_input("Create a new function to parse JSON");
        assert!(!responses.is_empty());
        assert!(!mcp.tasks.is_empty());
    }
    
    #[test]
    fn test_intent_parsing() {
        let mcp = McpOrchestrator::new();
        let intent = mcp.voice_understand("Fix the bug in parser.rs");
        assert_eq!(intent.intent_type, IntentType::Fix);
    }
}
