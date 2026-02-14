//! MCP Panel - The AI Coding Interface
//!
//! A beautiful, integrated panel for conversational code editing.
//! Shows the multi-agent conversation, task progress, and pending edits.

 
use crate::mcp::{
    AgentRole, CodeEdit, EditOperation, McpMessage, McpOrchestrator,
    MessageRole, Task, TaskStatus,
};
use crate::style::Color;
use crate::syntax::{Language, SyntaxHighlighter};

/// Panel mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanelMode {
    Chat,
    Tasks,
    Edits,
    Settings,
}

/// The MCP Panel UI
pub struct McpPanel {
    pub orchestrator: McpOrchestrator,
    pub mode: PanelMode,
    pub input_text: String,
    pub input_cursor: usize,
    pub scroll_offset: f32,
    pub selected_edit: Option<usize>,
    pub is_visible: bool,
    pub width: f32,
    pub highlighter: SyntaxHighlighter,
    pub theme: McpTheme,
    pub dark_mode: bool,
}

impl McpPanel {
    pub fn new() -> Self {
        McpPanel {
            orchestrator: McpOrchestrator::new(),
            mode: PanelMode::Chat,
            input_text: String::new(),
            input_cursor: 0,
            scroll_offset: 0.0,
            selected_edit: None,
            is_visible: false,
            width: 400.0,
            highlighter: SyntaxHighlighter::new(),
            theme: McpTheme::dark(),
            dark_mode: true,
        }
    }
    
    /// Toggle panel visibility
    pub fn toggle(&mut self) {
        self.is_visible = !self.is_visible;
        if self.is_visible && !self.orchestrator.is_active {
            self.orchestrator.start_session();
        }
    }
    
    /// Handle key input
    pub fn handle_key(&mut self, key: char) {
        self.input_text.insert(self.input_cursor, key);
        self.input_cursor += 1;
    }
    
    /// Handle backspace
    pub fn handle_backspace(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor -= 1;
            self.input_text.remove(self.input_cursor);
        }
    }
    
    /// Handle enter - submit input
    pub fn handle_enter(&mut self) {
        if !self.input_text.is_empty() {
            let input = std::mem::take(&mut self.input_text);
            self.orchestrator.process_input(&input);
            self.input_cursor = 0;
        }
    }
    
    /// Switch mode
    pub fn set_mode(&mut self, mode: PanelMode) {
        self.mode = mode;
        self.scroll_offset = 0.0;
    }
    
    /// Approve all pending edits
    pub fn approve_edits(&mut self) -> Vec<CodeEdit> {
        self.orchestrator.approve_edits()
    }
    
    /// Reject all pending edits
    pub fn reject_edits(&mut self) {
        self.orchestrator.reject_edits();
    }

    /// Toggle between dark and light theme
    pub fn toggle_theme(&mut self) {
        self.dark_mode = !self.dark_mode;
        self.theme = if self.dark_mode {
            McpTheme::dark()
        } else {
            McpTheme::light()
        };
    }
    
    /// Approve a single edit
    pub fn approve_edit(&mut self, index: usize) -> Option<CodeEdit> {
        if index < self.orchestrator.pending_edits.len() {
            Some(self.orchestrator.pending_edits.remove(index))
        } else {
            None
        }
    }
    
    /// Render the panel to a string representation
    pub fn render(&self) -> PanelRender {
        match self.mode {
            PanelMode::Chat => self.render_chat(),
            PanelMode::Tasks => self.render_tasks(),
            PanelMode::Edits => self.render_edits(),
            PanelMode::Settings => self.render_settings(),
        }
    }
    
    fn render_chat(&self) -> PanelRender {
        let mut elements = Vec::new();

        // Header
        elements.push(RenderElement::Header {
            title: "AI Coding Assistant".to_string(),
            subtitle: Some(format!("Session: {}", &self.orchestrator.session_id[..8])),
        });

        // Agent status bar - use theme.agent_color() for each role
        let agents: Vec<AgentStatus> = vec![
            (AgentRole::Voice, "Grok"),
            (AgentRole::Orchestrator, "Manus"),
            (AgentRole::Coder, "Claude"),
            (AgentRole::Auditor, "Gemini"),
        ].into_iter().map(|(role, name)| {
            let _color = self.theme.agent_color(role);
            AgentStatus { role, online: true, name: name.to_string() }
        }).collect();
        elements.push(RenderElement::AgentBar { agents });

        // Messages
        for msg in self.orchestrator.history() {
            elements.push(RenderElement::Message {
                role: msg.role,
                agent: msg.agent,
                content: msg.content.clone(),
                timestamp: msg.timestamp.format("%H:%M").to_string(),
            });
        }

        // Pending edits notification
        let pending_count = self.orchestrator.pending_edits.len();
        if pending_count > 0 {
            elements.push(RenderElement::Notification {
                message: format!("* {} pending code changes - switch to Edits tab to review", pending_count),
                style: NotificationStyle::Warning,
            });
        }

        // Git status notification
        let git_summary = self.orchestrator.git_status_summary();
        if !git_summary.starts_with("Git:") {
            elements.push(RenderElement::Notification {
                message: git_summary,
                style: NotificationStyle::Info,
            });
        }

        // Session active notification
        if self.orchestrator.is_active {
            elements.push(RenderElement::Notification {
                message: "Session active - all agents online".to_string(),
                style: NotificationStyle::Success,
            });
        }

        // Show any task errors
        let failed_tasks: Vec<_> = self.orchestrator.tasks.values()
            .filter(|t| t.status == TaskStatus::Failed)
            .collect();
        if !failed_tasks.is_empty() {
            elements.push(RenderElement::Notification {
                message: format!("{} task(s) failed - check Tasks tab", failed_tasks.len()),
                style: NotificationStyle::Error,
            });
        }

        // Quick command hints from get_quick_commands()
        if self.orchestrator.conversation.is_empty() {
            let commands = get_quick_commands();
            let hints: Vec<String> = commands.iter().map(|c| format!("/{} - {} (e.g. \"{}\")", c.trigger, c.description, c.example)).collect();
            elements.push(RenderElement::EmptyState {
                icon: String::new(),
                message: "Start coding with AI".to_string(),
                hint: hints.join(" | "),
            });
        }

        // Input field
        elements.push(RenderElement::Input {
            placeholder: "Ask me to create, fix, or refactor code...".to_string(),
            value: self.input_text.clone(),
            cursor: self.input_cursor,
        });

        PanelRender {
            mode: self.mode,
            elements,
            width: self.width,
            scroll_offset: self.scroll_offset,
        }
    }
    
    fn render_tasks(&self) -> PanelRender {
        let mut elements = Vec::new();
        
        elements.push(RenderElement::Header {
            title: "Task Pipeline".to_string(),
            subtitle: Some(format!("{} tasks total", self.orchestrator.tasks.len())),
        });
        
        // Group tasks by status
        let pending: Vec<_> = self.orchestrator.tasks.values()
            .filter(|t| t.status == TaskStatus::Pending)
            .collect();
        let in_progress: Vec<_> = self.orchestrator.tasks.values()
            .filter(|t| t.status == TaskStatus::InProgress)
            .collect();
        let completed: Vec<_> = self.orchestrator.tasks.values()
            .filter(|t| t.status == TaskStatus::Completed)
            .collect();
        
        if !in_progress.is_empty() {
            elements.push(RenderElement::SectionHeader {
                title: "In Progress".to_string(),
            });
            for task in in_progress {
                elements.push(self.render_task(task));
            }
        }
        
        if !pending.is_empty() {
            elements.push(RenderElement::SectionHeader {
                title: "... Pending".to_string(),
            });
            for task in pending {
                elements.push(self.render_task(task));
            }
        }
        
        if !completed.is_empty() {
            elements.push(RenderElement::SectionHeader {
                title: "[OK] Completed".to_string(),
            });
            for task in completed {
                elements.push(self.render_task(task));
            }
        }
        
        PanelRender {
            mode: self.mode,
            elements,
            width: self.width,
            scroll_offset: self.scroll_offset,
        }
    }
    
    fn render_task(&self, task: &Task) -> RenderElement {
        RenderElement::Task {
            id: task.id,
            title: task.title.clone(),
            description: task.description.clone(),
            status: task.status,
            assigned_to: task.assigned_to,
            has_artifacts: !task.artifacts.is_empty(),
        }
    }
    
    fn render_edits(&self) -> PanelRender {
        let mut elements = Vec::new();
        
        let pending_count = self.orchestrator.pending_edits.len();
        
        elements.push(RenderElement::Header {
            title: "* Code Changes".to_string(),
            subtitle: Some(format!("{} pending", pending_count)),
        });
        
        if pending_count > 0 {
            // Bulk action buttons
            elements.push(RenderElement::ActionBar {
                actions: vec![
                    Action { label: "[OK] Approve All".to_string(), id: "approve_all".to_string(), style: ActionStyle::Primary },
                    Action { label: "View Diff".to_string(), id: "view_diff".to_string(), style: ActionStyle::Secondary },
                    Action { label: "[X] Reject All".to_string(), id: "reject_all".to_string(), style: ActionStyle::Danger },
                ],
            });
            
            // Individual edits
            for (i, edit) in self.orchestrator.pending_edits.iter().enumerate() {
                let is_selected = self.selected_edit == Some(i);
                elements.push(self.render_edit(edit, i, is_selected));
            }
        } else {
            elements.push(RenderElement::EmptyState {
                icon: "".to_string(),
                message: "No pending code changes".to_string(),
                hint: "Ask the AI to create or modify code".to_string(),
            });
        }
        
        PanelRender {
            mode: self.mode,
            elements,
            width: self.width,
            scroll_offset: self.scroll_offset,
        }
    }
    
    fn render_edit(&self, edit: &CodeEdit, index: usize, selected: bool) -> RenderElement {
        let language = detect_language(&edit.file_path);
        let highlighted = self.highlighter.highlight(&edit.new_content, language);
        
        // Flatten the highlighted tokens into (String, Color) pairs
        let preview: Vec<(String, Color)> = highlighted
            .into_iter()
            .flatten()
            .map(|token| (token.text, token.color))
            .collect();
        
        RenderElement::CodeEdit {
            index,
            file_path: edit.file_path.clone(),
            operation: edit.operation,
            description: edit.description.clone(),
            preview,
            selected,
        }
    }
    
    fn render_settings(&self) -> PanelRender {
        let mut elements = Vec::new();
        
        elements.push(RenderElement::Header {
            title: "(settings) MCP Settings".to_string(),
            subtitle: None,
        });
        
        // Agent configurations
        for (role, config) in &self.orchestrator.agents {
            elements.push(RenderElement::AgentConfig {
                role: *role,
                model: config.model.clone(),
                api_url: config.api_url.clone(),
                has_key: config.api_key.is_some(),
                enabled: config.enabled,
            });
        }
        
        // Session info
        elements.push(RenderElement::SectionHeader {
            title: "Session Info".to_string(),
        });
        
        elements.push(RenderElement::InfoRow {
            label: "Session ID".to_string(),
            value: self.orchestrator.session_id.clone(),
        });
        
        elements.push(RenderElement::InfoRow {
            label: "Started".to_string(),
            value: self.orchestrator.started_at.format("%Y-%m-%d %H:%M").to_string(),
        });
        
        elements.push(RenderElement::InfoRow {
            label: "Messages".to_string(),
            value: self.orchestrator.conversation.len().to_string(),
        });
        
        elements.push(RenderElement::InfoRow {
            label: "Tasks".to_string(),
            value: self.orchestrator.tasks.len().to_string(),
        });
        
        PanelRender {
            mode: self.mode,
            elements,
            width: self.width,
            scroll_offset: self.scroll_offset,
        }
    }
}

impl Default for McpPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Rendered panel output
pub struct PanelRender {
    pub mode: PanelMode,
    pub elements: Vec<RenderElement>,
    pub width: f32,
    pub scroll_offset: f32,
}

/// Individual render elements
#[derive(Debug, Clone)]
pub enum RenderElement {
    Header {
        title: String,
        subtitle: Option<String>,
    },
    SectionHeader {
        title: String,
    },
    AgentBar {
        agents: Vec<AgentStatus>,
    },
    Message {
        role: MessageRole,
        agent: Option<AgentRole>,
        content: String,
        timestamp: String,
    },
    Task {
        id: u64,
        title: String,
        description: String,
        status: TaskStatus,
        assigned_to: AgentRole,
        has_artifacts: bool,
    },
    CodeEdit {
        index: usize,
        file_path: String,
        operation: EditOperation,
        description: String,
        preview: Vec<(String, Color)>,
        selected: bool,
    },
    AgentConfig {
        role: AgentRole,
        model: String,
        api_url: String,
        has_key: bool,
        enabled: bool,
    },
    ActionBar {
        actions: Vec<Action>,
    },
    Input {
        placeholder: String,
        value: String,
        cursor: usize,
    },
    Notification {
        message: String,
        style: NotificationStyle,
    },
    EmptyState {
        icon: String,
        message: String,
        hint: String,
    },
    InfoRow {
        label: String,
        value: String,
    },
}

#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub role: AgentRole,
    pub name: String,
    pub online: bool,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub label: String,
    pub id: String,
    pub style: ActionStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum ActionStyle {
    Primary,
    Secondary,
    Danger,
}

#[derive(Debug, Clone, Copy)]
pub enum NotificationStyle {
    Info,
    Success,
    Warning,
    Error,
}

/// Theme colors for the MCP panel
pub struct McpTheme {
    pub background: Color,
    pub surface: Color,
    pub primary: Color,
    pub secondary: Color,
    pub text: Color,
    pub text_dim: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub border: Color,
    
    // Agent colors
    pub voice_color: Color,
    pub orchestrator_color: Color,
    pub coder_color: Color,
    pub auditor_color: Color,
    
    // Message colors
    pub user_bubble: Color,
    pub agent_bubble: Color,
    pub system_bubble: Color,
}

impl McpTheme {
    pub fn dark() -> Self {
        McpTheme {
            background: Color::new(30, 30, 35, 255),
            surface: Color::new(40, 42, 48, 255),
            primary: Color::new(130, 170, 255, 255),
            secondary: Color::new(160, 160, 180, 255),
            text: Color::new(230, 230, 235, 255),
            text_dim: Color::new(140, 140, 150, 255),
            accent: Color::new(255, 180, 100, 255),
            success: Color::new(100, 220, 140, 255),
            warning: Color::new(255, 200, 80, 255),
            error: Color::new(255, 100, 100, 255),
            border: Color::new(60, 62, 70, 255),
            
            voice_color: Color::new(120, 180, 255, 255),      // Blue for Grok
            orchestrator_color: Color::new(255, 160, 100, 255), // Orange for Manus
            coder_color: Color::new(180, 130, 255, 255),       // Purple for Claude
            auditor_color: Color::new(100, 220, 180, 255),     // Teal for Gemini
            
            user_bubble: Color::new(70, 100, 140, 255),
            agent_bubble: Color::new(50, 55, 65, 255),
            system_bubble: Color::new(45, 50, 60, 255),
        }
    }
    
    pub fn light() -> Self {
        McpTheme {
            background: Color::new(248, 249, 250, 255),
            surface: Color::new(255, 255, 255, 255),
            primary: Color::new(50, 100, 200, 255),
            secondary: Color::new(100, 100, 120, 255),
            text: Color::new(30, 30, 40, 255),
            text_dim: Color::new(120, 120, 140, 255),
            accent: Color::new(220, 140, 40, 255),
            success: Color::new(40, 180, 100, 255),
            warning: Color::new(220, 160, 40, 255),
            error: Color::new(220, 60, 60, 255),
            border: Color::new(220, 222, 230, 255),
            
            voice_color: Color::new(40, 120, 220, 255),
            orchestrator_color: Color::new(220, 120, 40, 255),
            coder_color: Color::new(140, 80, 220, 255),
            auditor_color: Color::new(40, 180, 150, 255),      // Teal for Gemini
            
            user_bubble: Color::new(220, 235, 255, 255),
            agent_bubble: Color::new(240, 242, 248, 255),
            system_bubble: Color::new(245, 247, 252, 255),
        }
    }
    
    /// Get color for agent role
    pub fn agent_color(&self, role: AgentRole) -> Color {
        match role {
            AgentRole::Voice => self.voice_color,
            AgentRole::Orchestrator => self.orchestrator_color,
            AgentRole::Coder => self.coder_color,
            AgentRole::Auditor => self.auditor_color,
        }
    }
}

/// Detect language from file extension
fn detect_language(path: &str) -> Language {
    if let Some(ext) = path.split('.').next_back() {
        match crate::fontcase::ascii_lower(ext).as_str() {
            "rs" => Language::Rust,
            "js" | "jsx" | "mjs" => Language::JavaScript,
            "ts" | "tsx" => Language::TypeScript,
            "py" => Language::Python,
            "html" | "htm" => Language::Html,
            "css" | "scss" | "sass" => Language::Css,
            _ => Language::JavaScript, // Default
        }
    } else {
        Language::JavaScript
    }
}

/// Quick commands for the MCP panel
pub struct QuickCommand {
    pub trigger: String,
    pub description: String,
    pub example: String,
}

pub fn get_quick_commands() -> Vec<QuickCommand> {
    vec![
        QuickCommand {
            trigger: "create".to_string(),
            description: "Create new code or files".to_string(),
            example: "create a function to parse JSON".to_string(),
        },
        QuickCommand {
            trigger: "fix".to_string(),
            description: "Fix bugs or errors".to_string(),
            example: "fix the null pointer in parser.rs".to_string(),
        },
        QuickCommand {
            trigger: "refactor".to_string(),
            description: "Improve code structure".to_string(),
            example: "refactor this to use async/await".to_string(),
        },
        QuickCommand {
            trigger: "explain".to_string(),
            description: "Get code explanation".to_string(),
            example: "explain how the layout engine works".to_string(),
        },
        QuickCommand {
            trigger: "test".to_string(),
            description: "Generate tests".to_string(),
            example: "test the cookie parser".to_string(),
        },
        QuickCommand {
            trigger: "document".to_string(),
            description: "Add documentation".to_string(),
            example: "document the MCP module".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_panel_creation() {
        let panel = McpPanel::new();
        assert!(!panel.is_visible);
        assert_eq!(panel.mode, PanelMode::Chat);
    }
    
    #[test]
    fn test_panel_toggle() {
        let mut panel = McpPanel::new();
        panel.toggle();
        assert!(panel.is_visible);
        panel.toggle();
        assert!(!panel.is_visible);
    }
    
    #[test]
    fn test_language_detection() {
        assert_eq!(detect_language("test.rs"), Language::Rust);
        assert_eq!(detect_language("app.js"), Language::JavaScript);
        assert_eq!(detect_language("main.py"), Language::Python);
    }
}
