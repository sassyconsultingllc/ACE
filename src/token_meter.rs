//! Token Meter — AI Context Window Usage Dashboard
//!
//! Tracks token consumption, cost, and context window utilization
//! across all MCP AI agents. Provides real-time visibility into:
//! - Per-agent token usage (prompt vs completion)
//! - Running cost estimates based on provider pricing
//! - Context window fill percentage with visual bar
//! - Session totals and per-request averages
//! - Rate limiting and budget alerts

use crate::mcp::{AgentRole, TokenUsage};
use crate::style::Color;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Provider pricing (per 1M tokens, as of Feb 2026)
// ---------------------------------------------------------------------------

/// Cost per million tokens for each provider/model combination
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub context_window: u32,
}

impl ModelPricing {
    /// Get pricing for a known model
    pub fn for_model(model: &str) -> Self {
        match model {
            // xAI / Grok
            m if m.contains("grok-2") => ModelPricing {
                input_per_million: 2.00,
                output_per_million: 10.00,
                context_window: 131_072,
            },
            m if m.contains("grok-3") => ModelPricing {
                input_per_million: 3.00,
                output_per_million: 15.00,
                context_window: 131_072,
            },

            // Anthropic / Claude
            m if m.contains("claude-opus") || m.contains("opus-5") => ModelPricing {
                input_per_million: 15.00,
                output_per_million: 75.00,
                context_window: 200_000,
            },
            m if m.contains("claude-sonnet") || m.contains("sonnet-4") => ModelPricing {
                input_per_million: 3.00,
                output_per_million: 15.00,
                context_window: 200_000,
            },
            m if m.contains("claude-haiku") || m.contains("haiku") => ModelPricing {
                input_per_million: 0.25,
                output_per_million: 1.25,
                context_window: 200_000,
            },

            // Google / Gemini
            m if m.contains("gemini-2.5-pro") => ModelPricing {
                input_per_million: 1.25,
                output_per_million: 10.00,
                context_window: 1_000_000,
            },
            m if m.contains("gemini-2.0") || m.contains("gemini-2") => ModelPricing {
                input_per_million: 0.10,
                output_per_million: 0.40,
                context_window: 1_000_000,
            },
            m if m.contains("gemini") => ModelPricing {
                input_per_million: 0.50,
                output_per_million: 1.50,
                context_window: 1_000_000,
            },

            // Together.ai / Open models
            m if m.contains("mixtral") || m.contains("Mixtral") => ModelPricing {
                input_per_million: 0.60,
                output_per_million: 0.60,
                context_window: 65_536,
            },
            m if m.contains("llama") || m.contains("Llama") => ModelPricing {
                input_per_million: 0.20,
                output_per_million: 0.20,
                context_window: 131_072,
            },

            // OpenAI
            m if m.contains("gpt-4o") => ModelPricing {
                input_per_million: 2.50,
                output_per_million: 10.00,
                context_window: 128_000,
            },
            m if m.contains("gpt-4-turbo") || m.contains("gpt-4") => ModelPricing {
                input_per_million: 10.00,
                output_per_million: 30.00,
                context_window: 128_000,
            },
            m if m.contains("o1") || m.contains("o3") || m.contains("o4") => ModelPricing {
                input_per_million: 15.00,
                output_per_million: 60.00,
                context_window: 200_000,
            },

            // Ollama / Local (free)
            m if m.contains("ollama") || m.contains("local") => ModelPricing {
                input_per_million: 0.0,
                output_per_million: 0.0,
                context_window: 32_768,
            },

            // Unknown — conservative estimate
            _ => ModelPricing {
                input_per_million: 1.00,
                output_per_million: 3.00,
                context_window: 128_000,
            },
        }
    }

    /// Calculate cost for a token usage
    pub fn cost(&self, usage: &TokenUsage) -> f64 {
        let input_cost = (usage.prompt_tokens as f64 / 1_000_000.0) * self.input_per_million;
        let output_cost = (usage.completion_tokens as f64 / 1_000_000.0) * self.output_per_million;
        input_cost + output_cost
    }

    /// Context window fill percentage
    pub fn context_fill_pct(&self, total_tokens: u32) -> f32 {
        if self.context_window == 0 {
            return 0.0;
        }
        (total_tokens as f32 / self.context_window as f32 * 100.0).min(100.0)
    }
}

// ---------------------------------------------------------------------------
// Per-agent usage tracking
// ---------------------------------------------------------------------------

/// Usage record for a single API call
#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub timestamp: DateTime<Utc>,
    pub agent: AgentRole,
    pub model: String,
    pub usage: TokenUsage,
    pub cost: f64,
    pub latency_ms: u64,
}

/// Accumulated stats for one agent
#[derive(Debug, Clone)]
pub struct AgentStats {
    pub role: AgentRole,
    pub model: String,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub request_count: u32,
    pub avg_latency_ms: u64,
    pub context_window: u32,
    pub last_request: Option<DateTime<Utc>>,
    pub peak_single_request: u32,
}

impl AgentStats {
    pub fn new(role: AgentRole, model: &str) -> Self {
        let pricing = ModelPricing::for_model(model);
        AgentStats {
            role,
            model: model.to_string(),
            total_prompt_tokens: 0,
            total_completion_tokens: 0,
            total_tokens: 0,
            total_cost: 0.0,
            request_count: 0,
            avg_latency_ms: 0,
            context_window: pricing.context_window,
            last_request: None,
            peak_single_request: 0,
        }
    }

    pub fn record(&mut self, usage: &TokenUsage, cost: f64, latency_ms: u64) {
        self.total_prompt_tokens += usage.prompt_tokens as u64;
        self.total_completion_tokens += usage.completion_tokens as u64;
        self.total_tokens += usage.total_tokens as u64;
        self.total_cost += cost;
        self.request_count += 1;
        self.last_request = Some(Utc::now());
        self.peak_single_request = self.peak_single_request.max(usage.total_tokens);

        // Rolling average latency
        if self.request_count == 1 {
            self.avg_latency_ms = latency_ms;
        } else {
            self.avg_latency_ms = (self.avg_latency_ms * (self.request_count as u64 - 1)
                + latency_ms)
                / self.request_count as u64;
        }
    }

    /// Tokens per request average
    pub fn avg_tokens_per_request(&self) -> u32 {
        if self.request_count == 0 {
            return 0;
        }
        (self.total_tokens / self.request_count as u64) as u32
    }

    /// Context window fill percentage (based on cumulative conversation tokens)
    pub fn context_fill_pct(&self) -> f32 {
        if self.context_window == 0 {
            return 0.0;
        }
        // Use peak single request as approximation of current context usage
        (self.peak_single_request as f32 / self.context_window as f32 * 100.0).min(100.0)
    }

    /// Cost per request average
    pub fn avg_cost_per_request(&self) -> f64 {
        if self.request_count == 0 {
            return 0.0;
        }
        self.total_cost / self.request_count as f64
    }
}

// ---------------------------------------------------------------------------
// Token Meter — the main tracking engine
// ---------------------------------------------------------------------------

/// Budget alert threshold
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BudgetStatus {
    /// Under 50% of budget
    Normal,
    /// 50-80% of budget
    Warning,
    /// 80-100% of budget
    Critical,
    /// Over budget
    Exceeded,
}

impl BudgetStatus {
    pub fn color(&self) -> Color {
        match self {
            BudgetStatus::Normal => Color::new(100, 220, 140, 255), // Green
            BudgetStatus::Warning => Color::new(255, 200, 80, 255), // Yellow
            BudgetStatus::Critical => Color::new(255, 140, 60, 255), // Orange
            BudgetStatus::Exceeded => Color::new(255, 80, 80, 255), // Red
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            BudgetStatus::Normal => "Normal",
            BudgetStatus::Warning => "Warning",
            BudgetStatus::Critical => "Critical",
            BudgetStatus::Exceeded => "OVER BUDGET",
        }
    }
}

/// The Token Meter — tracks all AI usage across the session
pub struct TokenMeter {
    /// Per-agent accumulated stats
    pub agent_stats: HashMap<AgentRole, AgentStats>,

    /// Full request history (capped at 500)
    pub history: Vec<UsageRecord>,
    pub max_history: usize,

    /// Session-level totals
    pub session_start: DateTime<Utc>,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub total_requests: u32,

    /// Budget controls
    pub session_budget: f64, // Max $ for this session (0 = unlimited)
    pub monthly_budget: f64, // Max $ per month (0 = unlimited)
    pub monthly_spent: f64,  // Accumulated monthly spend

    /// Rate limiting
    pub requests_per_minute_limit: u32, // 0 = unlimited
    recent_request_times: Vec<Instant>,

    /// Display preferences
    pub show_cost: bool,
    pub show_tokens: bool,
    pub show_context_bars: bool,
    pub compact_mode: bool,
}

impl TokenMeter {
    pub fn new() -> Self {
        TokenMeter {
            agent_stats: HashMap::new(),
            history: Vec::new(),
            max_history: 500,
            session_start: Utc::now(),
            total_tokens: 0,
            total_cost: 0.0,
            total_requests: 0,
            session_budget: 0.0,
            monthly_budget: 0.0,
            monthly_spent: 0.0,
            requests_per_minute_limit: 0,
            recent_request_times: Vec::new(),
            show_cost: true,
            show_tokens: true,
            show_context_bars: true,
            compact_mode: false,
        }
    }

    /// Record a completed API call
    pub fn record_usage(
        &mut self,
        agent: AgentRole,
        model: &str,
        usage: TokenUsage,
        latency_ms: u64,
    ) {
        let pricing = ModelPricing::for_model(model);
        let cost = pricing.cost(&usage);

        // Update agent stats
        let stats = self
            .agent_stats
            .entry(agent)
            .or_insert_with(|| AgentStats::new(agent, model));
        stats.record(&usage, cost, latency_ms);

        // Update session totals
        self.total_tokens += usage.total_tokens as u64;
        self.total_cost += cost;
        self.total_requests += 1;
        self.monthly_spent += cost;

        // Record in history
        let record = UsageRecord {
            timestamp: Utc::now(),
            agent,
            model: model.to_string(),
            usage,
            cost,
            latency_ms,
        };

        self.history.push(record);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        // Track for rate limiting
        self.recent_request_times.push(Instant::now());
        self.prune_rate_window();
    }

    /// Check if we're within rate limits
    pub fn is_rate_limited(&mut self) -> bool {
        if self.requests_per_minute_limit == 0 {
            return false;
        }
        self.prune_rate_window();
        self.recent_request_times.len() as u32 >= self.requests_per_minute_limit
    }

    /// Clean up old rate tracking entries
    fn prune_rate_window(&mut self) {
        let one_minute_ago = Instant::now() - std::time::Duration::from_secs(60);
        self.recent_request_times.retain(|t| *t > one_minute_ago);
    }

    /// Current budget status
    pub fn budget_status(&self) -> BudgetStatus {
        if self.session_budget <= 0.0 {
            return BudgetStatus::Normal;
        }
        let pct = self.total_cost / self.session_budget;
        if pct >= 1.0 {
            BudgetStatus::Exceeded
        } else if pct >= 0.8 {
            BudgetStatus::Critical
        } else if pct >= 0.5 {
            BudgetStatus::Warning
        } else {
            BudgetStatus::Normal
        }
    }

    /// Monthly budget status
    pub fn monthly_budget_status(&self) -> BudgetStatus {
        if self.monthly_budget <= 0.0 {
            return BudgetStatus::Normal;
        }
        let pct = self.monthly_spent / self.monthly_budget;
        if pct >= 1.0 {
            BudgetStatus::Exceeded
        } else if pct >= 0.8 {
            BudgetStatus::Critical
        } else if pct >= 0.5 {
            BudgetStatus::Warning
        } else {
            BudgetStatus::Normal
        }
    }

    /// Session duration as a human-readable string
    pub fn session_duration(&self) -> String {
        let dur = Utc::now() - self.session_start;
        let hours = dur.num_hours();
        let mins = dur.num_minutes() % 60;
        let secs = dur.num_seconds() % 60;
        if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else if mins > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}s", secs)
        }
    }

    /// Cost formatted as dollars
    pub fn format_cost(cost: f64) -> String {
        if cost < 0.01 {
            format!("${:.4}", cost)
        } else if cost < 1.0 {
            format!("${:.3}", cost)
        } else {
            format!("${:.2}", cost)
        }
    }

    /// Token count formatted with K/M suffix
    pub fn format_tokens(tokens: u64) -> String {
        if tokens >= 1_000_000 {
            format!("{:.1}M", tokens as f64 / 1_000_000.0)
        } else if tokens >= 1_000 {
            format!("{:.1}K", tokens as f64 / 1_000.0)
        } else {
            format!("{}", tokens)
        }
    }

    /// Requests per minute (current rate)
    pub fn current_rpm(&mut self) -> u32 {
        self.prune_rate_window();
        self.recent_request_times.len() as u32
    }

    /// Get the most expensive agent
    pub fn most_expensive_agent(&self) -> Option<(&AgentRole, &AgentStats)> {
        self.agent_stats.iter().max_by(|a, b| {
            a.1.total_cost
                .partial_cmp(&b.1.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Get the most token-hungry agent
    pub fn most_tokens_agent(&self) -> Option<(&AgentRole, &AgentStats)> {
        self.agent_stats
            .iter()
            .max_by_key(|(_role, stats)| stats.total_tokens)
    }

    // -----------------------------------------------------------------------
    // Render methods — produce structured output for the UI
    // -----------------------------------------------------------------------

    /// Render the full token meter panel
    pub fn render(&mut self) -> TokenMeterRender {
        let mut sections = Vec::new();

        // --- Session Overview ---
        sections.push(MeterSection::SessionOverview {
            duration: self.session_duration(),
            total_requests: self.total_requests,
            total_tokens: Self::format_tokens(self.total_tokens),
            total_cost: Self::format_cost(self.total_cost),
            budget_status: self.budget_status(),
            rpm: self.current_rpm(),
        });

        // --- Per-Agent Breakdown ---
        let mut agent_meters: Vec<AgentMeter> = Vec::new();
        let agent_order = [
            AgentRole::Voice,
            AgentRole::Coder,
            AgentRole::Orchestrator,
            AgentRole::Auditor,
        ];

        for role in &agent_order {
            if let Some(stats) = self.agent_stats.get(role) {
                agent_meters.push(AgentMeter {
                    role: *role,
                    name: role_display_name(*role).to_string(),
                    model: stats.model.clone(),
                    prompt_tokens: Self::format_tokens(stats.total_prompt_tokens),
                    completion_tokens: Self::format_tokens(stats.total_completion_tokens),
                    total_tokens: Self::format_tokens(stats.total_tokens),
                    cost: Self::format_cost(stats.total_cost),
                    requests: stats.request_count,
                    avg_latency_ms: stats.avg_latency_ms,
                    context_fill_pct: stats.context_fill_pct(),
                    context_window: Self::format_tokens(stats.context_window as u64),
                });
            }
        }
        sections.push(MeterSection::AgentBreakdown {
            agents: agent_meters,
        });

        // --- Budget Gauge ---
        if self.session_budget > 0.0 || self.monthly_budget > 0.0 {
            sections.push(MeterSection::BudgetGauge {
                session_spent: Self::format_cost(self.total_cost),
                session_budget: if self.session_budget > 0.0 {
                    Some(Self::format_cost(self.session_budget))
                } else {
                    None
                },
                session_pct: if self.session_budget > 0.0 {
                    (self.total_cost / self.session_budget * 100.0).min(100.0) as f32
                } else {
                    0.0
                },
                monthly_spent: Self::format_cost(self.monthly_spent),
                monthly_budget: if self.monthly_budget > 0.0 {
                    Some(Self::format_cost(self.monthly_budget))
                } else {
                    None
                },
                monthly_pct: if self.monthly_budget > 0.0 {
                    (self.monthly_spent / self.monthly_budget * 100.0).min(100.0) as f32
                } else {
                    0.0
                },
                status: self.budget_status(),
            });
        }

        // --- Recent Requests ---
        let recent: Vec<RecentRequest> = self
            .history
            .iter()
            .rev()
            .take(10)
            .map(|r| RecentRequest {
                timestamp: r.timestamp.format("%H:%M:%S").to_string(),
                agent: role_display_name(r.agent).to_string(),
                tokens: Self::format_tokens(r.usage.total_tokens as u64),
                cost: Self::format_cost(r.cost),
                latency_ms: r.latency_ms,
            })
            .collect();

        if !recent.is_empty() {
            sections.push(MeterSection::RecentRequests { requests: recent });
        }

        TokenMeterRender { sections }
    }

    /// Render a compact one-line summary (for status bar)
    pub fn render_compact(&mut self) -> String {
        let budget_indicator = match self.budget_status() {
            BudgetStatus::Normal => "",
            BudgetStatus::Warning => " [!]",
            BudgetStatus::Critical => " [!!]",
            BudgetStatus::Exceeded => " [OVER]",
        };

        format!(
            "AI: {} req | {} tokens | {}{}",
            self.total_requests,
            Self::format_tokens(self.total_tokens),
            Self::format_cost(self.total_cost),
            budget_indicator,
        )
    }
}

impl Default for TokenMeter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Render types — structured output for UI consumption
// ---------------------------------------------------------------------------

/// Full panel render output
pub struct TokenMeterRender {
    pub sections: Vec<MeterSection>,
}

/// Sections of the token meter panel
#[derive(Debug, Clone)]
pub enum MeterSection {
    SessionOverview {
        duration: String,
        total_requests: u32,
        total_tokens: String,
        total_cost: String,
        budget_status: BudgetStatus,
        rpm: u32,
    },
    AgentBreakdown {
        agents: Vec<AgentMeter>,
    },
    BudgetGauge {
        session_spent: String,
        session_budget: Option<String>,
        session_pct: f32,
        monthly_spent: String,
        monthly_budget: Option<String>,
        monthly_pct: f32,
        status: BudgetStatus,
    },
    RecentRequests {
        requests: Vec<RecentRequest>,
    },
}

/// Per-agent meter data
#[derive(Debug, Clone)]
pub struct AgentMeter {
    pub role: AgentRole,
    pub name: String,
    pub model: String,
    pub prompt_tokens: String,
    pub completion_tokens: String,
    pub total_tokens: String,
    pub cost: String,
    pub requests: u32,
    pub avg_latency_ms: u64,
    pub context_fill_pct: f32,
    pub context_window: String,
}

impl AgentMeter {
    /// Get display name for an agent role
    pub fn default_name(role: AgentRole) -> &'static str {
        role_display_name(role)
    }
}

/// Recent request entry
#[derive(Debug, Clone)]
pub struct RecentRequest {
    pub timestamp: String,
    pub agent: String,
    pub tokens: String,
    pub cost: String,
    pub latency_ms: u64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Human-readable agent name
fn role_display_name(role: AgentRole) -> &'static str {
    match role {
        AgentRole::Voice => "Grok",
        AgentRole::Orchestrator => "Manus",
        AgentRole::Coder => "Claude",
        AgentRole::Auditor => "Gemini",
    }
}

/// Generate a context bar string (for text-mode rendering)
/// Example: [████████░░░░░░░░] 52%
pub fn context_bar(fill_pct: f32, width: usize) -> String {
    let filled = ((fill_pct / 100.0) * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);
    let bar: String = std::iter::repeat('█')
        .take(filled)
        .chain(std::iter::repeat('░').take(empty))
        .collect();
    format!("[{}] {:.0}%", bar, fill_pct)
}

/// Color for context fill percentage
pub fn context_fill_color(pct: f32) -> Color {
    if pct >= 90.0 {
        Color::new(255, 80, 80, 255) // Red — almost full
    } else if pct >= 70.0 {
        Color::new(255, 180, 60, 255) // Orange — getting full
    } else if pct >= 50.0 {
        Color::new(255, 220, 80, 255) // Yellow — halfway
    } else {
        Color::new(100, 220, 140, 255) // Green — plenty of room
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_usage(prompt: u32, completion: u32) -> TokenUsage {
        TokenUsage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }

    #[test]
    fn test_token_meter_creation() {
        let meter = TokenMeter::new();
        assert_eq!(meter.total_tokens, 0);
        assert_eq!(meter.total_cost, 0.0);
        assert_eq!(meter.total_requests, 0);
        assert!(meter.agent_stats.is_empty());
        assert!(meter.history.is_empty());
    }

    #[test]
    fn test_record_usage() {
        let mut meter = TokenMeter::new();
        let usage = make_usage(1000, 500);
        meter.record_usage(AgentRole::Coder, "claude-sonnet-4", usage, 250);

        assert_eq!(meter.total_requests, 1);
        assert_eq!(meter.total_tokens, 1500);
        assert!(meter.total_cost > 0.0);
        assert_eq!(meter.agent_stats.len(), 1);
        assert!(meter.agent_stats.contains_key(&AgentRole::Coder));
    }

    #[test]
    fn test_multi_agent_tracking() {
        let mut meter = TokenMeter::new();

        meter.record_usage(AgentRole::Voice, "grok-2", make_usage(500, 200), 100);
        meter.record_usage(
            AgentRole::Coder,
            "claude-sonnet-4",
            make_usage(2000, 800),
            300,
        );
        meter.record_usage(
            AgentRole::Auditor,
            "gemini-2.5-pro",
            make_usage(3000, 1000),
            500,
        );

        assert_eq!(meter.total_requests, 3);
        assert_eq!(meter.total_tokens, 500 + 200 + 2000 + 800 + 3000 + 1000);
        assert_eq!(meter.agent_stats.len(), 3);
    }

    #[test]
    fn test_pricing_claude_sonnet() {
        let pricing = ModelPricing::for_model("claude-sonnet-4");
        assert_eq!(pricing.input_per_million, 3.00);
        assert_eq!(pricing.output_per_million, 15.00);
        assert_eq!(pricing.context_window, 200_000);

        let usage = make_usage(1_000_000, 100_000);
        let cost = pricing.cost(&usage);
        // 1M input * $3/M + 100K output * $15/M = $3.00 + $1.50 = $4.50
        assert!((cost - 4.50).abs() < 0.001);
    }

    #[test]
    fn test_pricing_grok() {
        let pricing = ModelPricing::for_model("grok-2");
        assert_eq!(pricing.input_per_million, 2.00);
        assert_eq!(pricing.output_per_million, 10.00);
    }

    #[test]
    fn test_pricing_gemini() {
        let pricing = ModelPricing::for_model("gemini-2.5-pro");
        assert_eq!(pricing.input_per_million, 1.25);
        assert_eq!(pricing.context_window, 1_000_000);
    }

    #[test]
    fn test_pricing_ollama_free() {
        let pricing = ModelPricing::for_model("ollama-llama3");
        assert_eq!(pricing.input_per_million, 0.0);
        assert_eq!(pricing.output_per_million, 0.0);
    }

    #[test]
    fn test_pricing_unknown_model() {
        let pricing = ModelPricing::for_model("some-unknown-model-v9");
        assert!(pricing.input_per_million > 0.0);
        assert!(pricing.context_window > 0);
    }

    #[test]
    fn test_budget_status() {
        let mut meter = TokenMeter::new();
        meter.session_budget = 1.00; // $1 budget

        assert_eq!(meter.budget_status(), BudgetStatus::Normal);

        meter.total_cost = 0.49;
        assert_eq!(meter.budget_status(), BudgetStatus::Normal);

        meter.total_cost = 0.50;
        assert_eq!(meter.budget_status(), BudgetStatus::Warning);

        meter.total_cost = 0.80;
        assert_eq!(meter.budget_status(), BudgetStatus::Critical);

        meter.total_cost = 1.50;
        assert_eq!(meter.budget_status(), BudgetStatus::Exceeded);
    }

    #[test]
    fn test_budget_unlimited() {
        let meter = TokenMeter::new();
        // No budget set = always Normal
        assert_eq!(meter.budget_status(), BudgetStatus::Normal);
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(TokenMeter::format_tokens(500), "500");
        assert_eq!(TokenMeter::format_tokens(1_500), "1.5K");
        assert_eq!(TokenMeter::format_tokens(25_000), "25.0K");
        assert_eq!(TokenMeter::format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn test_format_cost() {
        assert_eq!(TokenMeter::format_cost(0.0001), "$0.0001");
        assert_eq!(TokenMeter::format_cost(0.005), "$0.005");
        assert_eq!(TokenMeter::format_cost(0.15), "$0.150");
        assert_eq!(TokenMeter::format_cost(3.50), "$3.50");
        assert_eq!(TokenMeter::format_cost(125.99), "$125.99");
    }

    #[test]
    fn test_context_bar() {
        let bar = context_bar(50.0, 16);
        assert!(bar.contains("50%"));
        assert!(bar.starts_with('['));
        assert!(bar.contains('█'));
        assert!(bar.contains('░'));
    }

    #[test]
    fn test_context_fill_color() {
        let green = context_fill_color(30.0);
        assert_eq!(green.r(), 100); // Green

        let red = context_fill_color(95.0);
        assert_eq!(red.r(), 255); // Red
    }

    #[test]
    fn test_agent_stats_averaging() {
        let mut stats = AgentStats::new(AgentRole::Coder, "claude-sonnet-4");

        stats.record(&make_usage(1000, 500), 0.01, 200);
        stats.record(&make_usage(2000, 1000), 0.02, 400);

        assert_eq!(stats.request_count, 2);
        assert_eq!(stats.total_prompt_tokens, 3000);
        assert_eq!(stats.total_completion_tokens, 1500);
        assert_eq!(stats.avg_tokens_per_request(), 2250);
        assert_eq!(stats.avg_latency_ms, 300); // (200 + 400) / 2
    }

    #[test]
    fn test_compact_render() {
        let mut meter = TokenMeter::new();
        meter.record_usage(
            AgentRole::Coder,
            "claude-sonnet-4",
            make_usage(5000, 2000),
            300,
        );

        let compact = meter.render_compact();
        assert!(compact.contains("1 req"));
        assert!(compact.contains("7.0K tokens"));
        assert!(compact.contains("$"));
    }

    #[test]
    fn test_full_render() {
        let mut meter = TokenMeter::new();
        meter.record_usage(
            AgentRole::Coder,
            "claude-sonnet-4",
            make_usage(5000, 2000),
            300,
        );
        meter.record_usage(AgentRole::Voice, "grok-2", make_usage(1000, 500), 100);

        let render = meter.render();
        assert!(!render.sections.is_empty());

        // Should have at least SessionOverview and AgentBreakdown
        let has_overview = render
            .sections
            .iter()
            .any(|s| matches!(s, MeterSection::SessionOverview { .. }));
        let has_agents = render
            .sections
            .iter()
            .any(|s| matches!(s, MeterSection::AgentBreakdown { .. }));
        assert!(has_overview);
        assert!(has_agents);
    }

    #[test]
    fn test_history_cap() {
        let mut meter = TokenMeter::new();
        meter.max_history = 5;

        for _ in 0..10 {
            meter.record_usage(AgentRole::Coder, "claude-sonnet-4", make_usage(100, 50), 50);
        }

        assert_eq!(meter.history.len(), 5);
        assert_eq!(meter.total_requests, 10); // Total still tracked
    }

    #[test]
    fn test_rate_limiting() {
        let mut meter = TokenMeter::new();
        meter.requests_per_minute_limit = 3;

        assert!(!meter.is_rate_limited());

        meter.record_usage(AgentRole::Coder, "claude-sonnet-4", make_usage(100, 50), 50);
        meter.record_usage(AgentRole::Coder, "claude-sonnet-4", make_usage(100, 50), 50);
        assert!(!meter.is_rate_limited());

        meter.record_usage(AgentRole::Coder, "claude-sonnet-4", make_usage(100, 50), 50);
        assert!(meter.is_rate_limited());
    }

    #[test]
    fn test_peak_single_request() {
        let mut stats = AgentStats::new(AgentRole::Coder, "claude-sonnet-4");
        stats.record(&make_usage(1000, 500), 0.01, 200);
        stats.record(&make_usage(5000, 3000), 0.05, 400);
        stats.record(&make_usage(200, 100), 0.001, 50);

        assert_eq!(stats.peak_single_request, 8000); // 5000 + 3000
    }

    #[test]
    fn test_context_fill_percentage() {
        let pricing = ModelPricing::for_model("claude-sonnet-4");
        assert_eq!(pricing.context_window, 200_000);

        let pct = pricing.context_fill_pct(100_000);
        assert!((pct - 50.0).abs() < 0.1);

        let pct_full = pricing.context_fill_pct(200_000);
        assert!((pct_full - 100.0).abs() < 0.1);

        // Over 100% should cap at 100
        let pct_over = pricing.context_fill_pct(300_000);
        assert!((pct_over - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_monthly_budget_status() {
        let mut meter = TokenMeter::new();
        meter.monthly_budget = 50.0;
        meter.monthly_spent = 10.0;
        assert_eq!(meter.monthly_budget_status(), BudgetStatus::Normal);

        meter.monthly_spent = 42.0;
        assert_eq!(meter.monthly_budget_status(), BudgetStatus::Critical);

        meter.monthly_spent = 55.0;
        assert_eq!(meter.monthly_budget_status(), BudgetStatus::Exceeded);
    }
}
