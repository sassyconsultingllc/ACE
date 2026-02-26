//! Self-healing health watchdog for the browser
//!
//! Monitors browser health metrics every 12 seconds, detects anomalies,
//! and applies corrective actions automatically. Inspired by BitDefender-style
//! always-on protection, but for browser internals.
//!
//! ALL data stays local. No telemetry. No crash reports. No phone-home.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

// ──────────────────────────────────────────────────────────────────────────────
// Health Snapshot — lightweight point-in-time health reading
// ──────────────────────────────────────────────────────────────────────────────

/// A lightweight snapshot of browser health at a point in time.
/// Designed to be cheap to collect (no allocations, no I/O).
#[derive(Clone, Debug)]
pub struct HealthSnapshot {
    pub timestamp: Instant,
    pub active_tabs: usize,
    pub crashed_tabs_last_5min: u32,
    pub renderer_stalls_last_5min: u32,
    pub memory_estimate_mb: u64,
    pub http_cache_size_mb: u64,
    pub recent_violations: u32,
    pub adblock_blocked_last_5min: u32,
    pub poison_applied_last_5min: u32,
    pub pending_downloads: u32,
    pub quarantined_files: u32,
    pub detection_alerts_last_5min: u32,
}

impl HealthSnapshot {
    /// Create a snapshot with all zeroes (initial state)
    pub fn empty() -> Self {
        Self {
            timestamp: Instant::now(),
            active_tabs: 0,
            crashed_tabs_last_5min: 0,
            renderer_stalls_last_5min: 0,
            memory_estimate_mb: 0,
            http_cache_size_mb: 0,
            recent_violations: 0,
            adblock_blocked_last_5min: 0,
            poison_applied_last_5min: 0,
            pending_downloads: 0,
            quarantined_files: 0,
            detection_alerts_last_5min: 0,
        }
    }

    /// Quick health score: 100 = perfect, 0 = critical
    pub fn health_score(&self) -> u8 {
        let mut score: i32 = 100;

        // Tab crashes are serious
        score -= (self.crashed_tabs_last_5min as i32) * 15;

        // Renderer stalls degrade experience
        score -= (self.renderer_stalls_last_5min as i32) * 8;

        // High memory is concerning
        if self.memory_estimate_mb > 4096 {
            score -= 20;
        } else if self.memory_estimate_mb > 2048 {
            score -= 10;
        }

        // Too many tabs
        if self.active_tabs > 50 {
            score -= 15;
        } else if self.active_tabs > 30 {
            score -= 5;
        }

        // Sandbox violations
        score -= (self.recent_violations as i32).min(20) * 2;

        // Detection alerts
        score -= (self.detection_alerts_last_5min as i32).min(10) * 3;

        score.clamp(0, 100) as u8
    }

    /// Human-readable status
    pub fn status_label(&self) -> &'static str {
        match self.health_score() {
            90..=100 => "Excellent",
            70..=89 => "Good",
            50..=69 => "Fair",
            25..=49 => "Degraded",
            _ => "Critical",
        }
    }

    /// Status color for UI (R, G, B)
    pub fn status_color(&self) -> (u8, u8, u8) {
        match self.health_score() {
            90..=100 => (60, 200, 80), // Green
            70..=89 => (120, 200, 60), // Yellow-green
            50..=69 => (240, 180, 40), // Yellow
            25..=49 => (240, 120, 40), // Orange
            _ => (220, 50, 50),        // Red
        }
    }

    /// Describe the snapshot for logging/diagnostics
    pub fn describe(&self) -> String {
        format!(
            "Health[score={}, tabs={}, crashes={}, stalls={}, mem={}MB, violations={}, alerts={}]",
            self.health_score(),
            self.active_tabs,
            self.crashed_tabs_last_5min,
            self.renderer_stalls_last_5min,
            self.memory_estimate_mb,
            self.recent_violations,
            self.detection_alerts_last_5min,
        )
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Healing Decision — what the watchdog decides to do
// ──────────────────────────────────────────────────────────────────────────────

/// A healing action decided by the watchdog
#[derive(Clone, Debug)]
pub struct HealingDecision {
    pub action: HealingAction,
    pub target: Option<String>,
    pub rationale: String,
    pub urgency: u8,     // 1=low ... 5=critical
    pub confidence: f32, // 0.0..1.0
    pub decided_at: Instant,
}

impl HealingDecision {
    pub fn new(
        action: HealingAction,
        rationale: impl Into<String>,
        urgency: u8,
        confidence: f32,
    ) -> Self {
        Self {
            action,
            target: None,
            rationale: rationale.into(),
            urgency: urgency.clamp(1, 5),
            confidence: confidence.clamp(0.0, 1.0),
            decided_at: Instant::now(),
        }
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn describe(&self) -> String {
        format!(
            "Heal[{:?} urgency={} conf={:.0}% target={} reason={}]",
            self.action,
            self.urgency,
            self.confidence * 100.0,
            self.target.as_deref().unwrap_or("*"),
            self.rationale,
        )
    }
}

/// Enumeration of all possible healing actions
#[derive(Clone, Debug, PartialEq)]
pub enum HealingAction {
    /// Clear HTTP cache to free memory
    ClearHttpCache,
    /// Close crashed/unresponsive tabs
    CloseCrashedTabs,
    /// Reload ad-block filter lists
    ReloadAdblockFilters,
    /// Reapply fingerprint poisoning to current page
    ReapplyPoisoning,
    /// Reduce concurrent network loads
    ReduceConcurrentLoads,
    /// Force garbage collection on script engines
    ForceScriptGc,
    /// Restart the rendering pipeline
    RestartRenderer,
    /// Suspend background tabs to free resources
    SuspendBackgroundTabs,
    /// Flush DNS cache
    FlushDnsCache,
    /// Tighten sandbox restrictions temporarily
    TightenSandbox,
    /// No action needed — everything is healthy
    NoAction,
}

impl HealingAction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClearHttpCache => "Clear HTTP Cache",
            Self::CloseCrashedTabs => "Close Crashed Tabs",
            Self::ReloadAdblockFilters => "Reload Ad-Block Filters",
            Self::ReapplyPoisoning => "Reapply Fingerprint Poisoning",
            Self::ReduceConcurrentLoads => "Reduce Concurrent Loads",
            Self::ForceScriptGc => "Force Script GC",
            Self::RestartRenderer => "Restart Renderer",
            Self::SuspendBackgroundTabs => "Suspend Background Tabs",
            Self::FlushDnsCache => "Flush DNS Cache",
            Self::TightenSandbox => "Tighten Sandbox",
            Self::NoAction => "No Action Needed",
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Health Watchdog — the core monitoring engine
// ──────────────────────────────────────────────────────────────────────────────

/// Watches browser health and makes healing decisions.
/// Runs synchronously in the update loop (cheap — no async needed).
pub struct HealthWatchdog {
    /// How often to check health (default: 12 seconds)
    pub check_interval: Duration,
    /// Last time we checked
    last_check: Instant,
    /// Last time we applied a healing action (cooldown)
    last_action_time: Option<Instant>,
    /// Minimum time between healing actions (prevent flapping)
    pub action_cooldown: Duration,
    /// Rolling window of recent snapshots (last 30 = ~6 minutes)
    snapshot_history: VecDeque<HealthSnapshot>,
    /// Maximum snapshots to keep
    max_history: usize,
    /// Log of healing actions taken
    healing_log: VecDeque<HealingLogEntry>,
    /// Maximum log entries to keep
    max_log: usize,
    /// Whether the watchdog is enabled
    pub enabled: bool,
    /// Consecutive unhealthy readings (for escalation)
    consecutive_unhealthy: u32,
    /// Total healing actions applied since startup
    pub total_actions_applied: u64,
}

/// A record of a healing action that was applied
#[derive(Clone, Debug)]
pub struct HealingLogEntry {
    pub timestamp: Instant,
    pub decision: HealingDecision,
    pub snapshot_before: HealthSnapshot,
    pub success: bool,
}

impl HealingLogEntry {
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }

    pub fn describe(&self) -> String {
        format!(
            "[{:.1}s ago] {:?} → {} (score was {})",
            self.age().as_secs_f32(),
            self.decision.action,
            if self.success { "OK" } else { "FAILED" },
            self.snapshot_before.health_score(),
        )
    }
}

impl HealthWatchdog {
    pub fn new() -> Self {
        Self {
            check_interval: Duration::from_secs(12),
            last_check: Instant::now(),
            last_action_time: None,
            action_cooldown: Duration::from_secs(60), // 1 minute between actions
            snapshot_history: VecDeque::with_capacity(30),
            max_history: 30,
            healing_log: VecDeque::with_capacity(50),
            max_log: 50,
            enabled: true,
            consecutive_unhealthy: 0,
            total_actions_applied: 0,
        }
    }

    /// Check if it's time to run a health check
    pub fn should_check(&self) -> bool {
        self.enabled && self.last_check.elapsed() >= self.check_interval
    }

    /// Record a health snapshot and decide if healing is needed
    pub fn evaluate(&mut self, snapshot: HealthSnapshot) -> Option<HealingDecision> {
        self.last_check = Instant::now();

        let score = snapshot.health_score();

        // Store in history
        self.snapshot_history.push_back(snapshot.clone());
        if self.snapshot_history.len() > self.max_history {
            self.snapshot_history.pop_front();
        }

        // Track consecutive unhealthy readings
        if score < 70 {
            self.consecutive_unhealthy += 1;
        } else {
            self.consecutive_unhealthy = 0;
        }

        // Don't act if we're healthy
        if score >= 85 {
            return Some(HealingDecision::new(
                HealingAction::NoAction,
                "System healthy",
                1,
                1.0,
            ));
        }

        // Don't act if we're in cooldown
        if let Some(last) = self.last_action_time {
            if last.elapsed() < self.action_cooldown {
                return None; // Still cooling down
            }
        }

        // Decide what to do based on the snapshot
        let decision = self.diagnose(&snapshot);

        if decision.action != HealingAction::NoAction && decision.confidence >= 0.65 {
            Some(decision)
        } else {
            None
        }
    }

    /// Analyze the snapshot and decide the best healing action
    fn diagnose(&self, snap: &HealthSnapshot) -> HealingDecision {
        // Priority 1: Crashed tabs (most visible to user)
        if snap.crashed_tabs_last_5min >= 3 {
            return HealingDecision::new(
                HealingAction::CloseCrashedTabs,
                format!("{} tabs crashed in last 5 min", snap.crashed_tabs_last_5min),
                4,
                0.95,
            );
        }

        // Priority 2: Memory pressure
        if snap.memory_estimate_mb > 5200 {
            return HealingDecision::new(
                HealingAction::ClearHttpCache,
                format!("Memory at {}MB — clearing cache", snap.memory_estimate_mb),
                4,
                0.90,
            );
        }

        // Priority 3: Too many tabs (suspend background ones)
        if snap.active_tabs > 35 {
            return HealingDecision::new(
                HealingAction::SuspendBackgroundTabs,
                format!(
                    "{} tabs open — suspending background tabs",
                    snap.active_tabs
                ),
                3,
                0.85,
            );
        }

        // Priority 4: Renderer stalls
        if snap.renderer_stalls_last_5min >= 4 {
            return HealingDecision::new(
                HealingAction::RestartRenderer,
                format!(
                    "{} renderer stalls — restarting pipeline",
                    snap.renderer_stalls_last_5min
                ),
                3,
                0.80,
            );
        }

        // Priority 5: Sandbox violations — tighten security
        if snap.recent_violations >= 5 {
            return HealingDecision::new(
                HealingAction::TightenSandbox,
                format!(
                    "{} sandbox violations — tightening restrictions",
                    snap.recent_violations
                ),
                3,
                0.85,
            );
        }

        // Priority 6: Detection alerts — reapply poisoning
        if snap.detection_alerts_last_5min >= 5 {
            return HealingDecision::new(
                HealingAction::ReapplyPoisoning,
                format!(
                    "{} tracking attempts detected — reapplying poisoning",
                    snap.detection_alerts_last_5min
                ),
                2,
                0.80,
            );
        }

        // Priority 7: Moderate memory (2-5GB) + many tabs
        if snap.memory_estimate_mb > 2048 && snap.active_tabs > 20 {
            return HealingDecision::new(
                HealingAction::SuspendBackgroundTabs,
                format!(
                    "Memory {}MB with {} tabs — preemptive suspension",
                    snap.memory_estimate_mb, snap.active_tabs
                ),
                2,
                0.70,
            );
        }

        // Escalation: consecutive unhealthy readings
        if self.consecutive_unhealthy >= 5 {
            return HealingDecision::new(
                HealingAction::ForceScriptGc,
                format!(
                    "{} consecutive unhealthy checks — forcing GC",
                    self.consecutive_unhealthy
                ),
                3,
                0.75,
            );
        }

        HealingDecision::new(HealingAction::NoAction, "No action warranted", 1, 1.0)
    }

    /// Record that a healing action was applied
    pub fn record_action(
        &mut self,
        decision: HealingDecision,
        snapshot: HealthSnapshot,
        success: bool,
    ) {
        self.last_action_time = Some(Instant::now());
        self.total_actions_applied += 1;

        self.healing_log.push_back(HealingLogEntry {
            timestamp: Instant::now(),
            decision,
            snapshot_before: snapshot,
            success,
        });

        if self.healing_log.len() > self.max_log {
            self.healing_log.pop_front();
        }
    }

    /// Get the most recent health score (or 100 if no data yet)
    pub fn latest_score(&self) -> u8 {
        self.snapshot_history
            .back()
            .map(|s| s.health_score())
            .unwrap_or(100)
    }

    /// Get the latest snapshot
    pub fn latest_snapshot(&self) -> Option<&HealthSnapshot> {
        self.snapshot_history.back()
    }

    /// Get trend: positive = improving, negative = degrading
    pub fn trend(&self) -> i8 {
        if self.snapshot_history.len() < 3 {
            return 0;
        }

        let recent: Vec<u8> = self
            .snapshot_history
            .iter()
            .rev()
            .take(5)
            .map(|s| s.health_score())
            .collect();
        let older: Vec<u8> = self
            .snapshot_history
            .iter()
            .rev()
            .skip(5)
            .take(5)
            .map(|s| s.health_score())
            .collect();

        if older.is_empty() {
            return 0;
        }

        let recent_avg: f32 = recent.iter().map(|&s| s as f32).sum::<f32>() / recent.len() as f32;
        let older_avg: f32 = older.iter().map(|&s| s as f32).sum::<f32>() / older.len() as f32;

        let diff = recent_avg - older_avg;
        if diff > 5.0 {
            1
        } else if diff < -5.0 {
            -1
        } else {
            0
        }
    }

    /// Get the trend emoji for UI display
    pub fn trend_indicator(&self) -> &'static str {
        match self.trend() {
            1 => "↑",  // Improving
            -1 => "↓", // Degrading
            _ => "→",  // Stable
        }
    }

    /// Get recent healing log entries
    pub fn recent_log(&self) -> &VecDeque<HealingLogEntry> {
        &self.healing_log
    }

    /// Get all snapshots for charting/history
    pub fn history(&self) -> &VecDeque<HealthSnapshot> {
        &self.snapshot_history
    }

    /// How long since the last health check
    pub fn time_since_last_check(&self) -> Duration {
        self.last_check.elapsed()
    }

    /// Summary for status bar display
    pub fn status_summary(&self) -> String {
        let score = self.latest_score();
        let trend = self.trend_indicator();
        let label = self
            .snapshot_history
            .back()
            .map(|s| s.status_label())
            .unwrap_or("Starting");
        format!("{}{} {}", score, trend, label)
    }
}

impl Default for HealthWatchdog {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_snapshot_is_healthy() {
        let snap = HealthSnapshot::empty();
        assert_eq!(snap.health_score(), 100);
        assert_eq!(snap.status_label(), "Excellent");
        let (r, g, b) = snap.status_color();
        assert_eq!((r, g, b), (60, 200, 80)); // Green
    }

    #[test]
    fn test_crashed_tabs_reduce_score() {
        let mut snap = HealthSnapshot::empty();
        snap.crashed_tabs_last_5min = 3;
        assert!(snap.health_score() < 60);
    }

    #[test]
    fn test_high_memory_reduces_score() {
        let mut snap = HealthSnapshot::empty();
        snap.memory_estimate_mb = 5000;
        assert!(snap.health_score() <= 80);
    }

    #[test]
    fn test_many_tabs_reduces_score() {
        let mut snap = HealthSnapshot::empty();
        snap.active_tabs = 60;
        assert!(snap.health_score() <= 85);
    }

    #[test]
    fn test_watchdog_new() {
        let wd = HealthWatchdog::new();
        assert!(wd.enabled);
        assert_eq!(wd.latest_score(), 100);
        assert_eq!(wd.trend(), 0);
        assert_eq!(wd.total_actions_applied, 0);
    }

    #[test]
    fn test_watchdog_healthy_returns_no_action() {
        let mut wd = HealthWatchdog::new();
        let snap = HealthSnapshot::empty();
        let decision = wd.evaluate(snap);
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().action, HealingAction::NoAction);
    }

    #[test]
    fn test_watchdog_crashes_trigger_close() {
        let mut wd = HealthWatchdog::new();
        let mut snap = HealthSnapshot::empty();
        snap.crashed_tabs_last_5min = 4;
        let decision = wd.evaluate(snap);
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().action, HealingAction::CloseCrashedTabs);
    }

    #[test]
    fn test_watchdog_high_memory_triggers_cache_clear() {
        let mut wd = HealthWatchdog::new();
        let mut snap = HealthSnapshot::empty();
        snap.memory_estimate_mb = 6000;
        let decision = wd.evaluate(snap);
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().action, HealingAction::ClearHttpCache);
    }

    #[test]
    fn test_watchdog_cooldown_prevents_flapping() {
        let mut wd = HealthWatchdog::new();

        // First unhealthy reading — should get action
        let mut snap = HealthSnapshot::empty();
        snap.crashed_tabs_last_5min = 5;
        let d1 = wd.evaluate(snap.clone());
        assert!(d1.is_some());
        assert_ne!(d1.as_ref().unwrap().action, HealingAction::NoAction);

        // Record the action
        wd.record_action(d1.unwrap(), snap.clone(), true);

        // Second unhealthy reading — should be in cooldown
        let d2 = wd.evaluate(snap);
        assert!(d2.is_none()); // Cooldown active
    }

    #[test]
    fn test_healing_decision_describe() {
        let d = HealingDecision::new(HealingAction::ClearHttpCache, "Memory too high", 3, 0.9)
            .with_target("all-caches");
        let desc = d.describe();
        assert!(desc.contains("ClearHttpCache"));
        assert!(desc.contains("all-caches"));
    }

    #[test]
    fn test_healing_action_labels() {
        assert_eq!(HealingAction::ClearHttpCache.label(), "Clear HTTP Cache");
        assert_eq!(HealingAction::NoAction.label(), "No Action Needed");
        assert_eq!(HealingAction::TightenSandbox.label(), "Tighten Sandbox");
    }

    #[test]
    fn test_snapshot_describe() {
        let snap = HealthSnapshot::empty();
        let desc = snap.describe();
        assert!(desc.contains("score=100"));
        assert!(desc.contains("tabs=0"));
    }

    #[test]
    fn test_log_entry_describe() {
        let entry = HealingLogEntry {
            timestamp: Instant::now(),
            decision: HealingDecision::new(HealingAction::ForceScriptGc, "test", 2, 0.8),
            snapshot_before: HealthSnapshot::empty(),
            success: true,
        };
        let desc = entry.describe();
        assert!(desc.contains("ForceScriptGc"));
        assert!(desc.contains("OK"));
    }

    #[test]
    fn test_status_summary() {
        let mut wd = HealthWatchdog::new();
        let snap = HealthSnapshot::empty();
        let _ = wd.evaluate(snap);
        let summary = wd.status_summary();
        assert!(summary.contains("100"));
        assert!(summary.contains("Excellent"));
    }

    #[test]
    fn test_trend_stable_with_few_readings() {
        let mut wd = HealthWatchdog::new();
        assert_eq!(wd.trend(), 0);
        assert_eq!(wd.trend_indicator(), "→");

        // Add a couple readings
        let _ = wd.evaluate(HealthSnapshot::empty());
        let _ = wd.evaluate(HealthSnapshot::empty());
        assert_eq!(wd.trend(), 0); // Not enough data
    }

    #[test]
    fn test_sandbox_violations_trigger_tighten() {
        let mut wd = HealthWatchdog::new();
        let mut snap = HealthSnapshot::empty();
        snap.recent_violations = 8;
        let decision = wd.evaluate(snap);
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().action, HealingAction::TightenSandbox);
    }

    #[test]
    fn test_detection_alerts_trigger_poisoning() {
        let mut wd = HealthWatchdog::new();
        let mut snap = HealthSnapshot::empty();
        snap.detection_alerts_last_5min = 10;
        let decision = wd.evaluate(snap);
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().action, HealingAction::ReapplyPoisoning);
    }
}
