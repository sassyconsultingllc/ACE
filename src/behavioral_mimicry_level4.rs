//! Behavioral Mimicry Level 4 -- Self-Evolving Input Poisoning
//!
//! ADVANCED ANTI-FINGERPRINTING THROUGH ADAPTIVE BEHAVIORAL POISONING
//!
//! PURPOSE:
//! ---------------------------------------------------------------------------
//! Level 1 (`behavioral_mimicry.rs`) adds static noise to mouse, scroll, and
//! typing signals.  Level 4 goes further:
//!
//!   1. **Velocity-curve mouse paths** -- instead of flat jitter, the mouse
//!      follows a human-like velocity bell-curve with path deviation.
//!   2. **Inertia-profiled scrolling** -- multi-segment inertia that decays
//!      like a real finger-flick.
//!   3. **Distribution-sampled typing** -- keystroke delays drawn from a
//!      realistic delay distribution with configurable error rate.
//!   4. **Self-evolving poisoning** -- when a countermeasure is detected the
//!      noise parameters automatically escalate, and the evolution history is
//!      logged for audit.
//!   5. **MCP integration** -- evolution requests can be forwarded over an
//!      async channel to the MCP orchestrator for remote monitoring.
//!
//! ISOLATION:
//! ---------------------------------------------------------------------------
//! One `BehavioralMimicLevel4` per tab.  Each instance carries its own
//! `SessionEntropySeed` with a unique nonce so cross-tab correlation via
//! input timing is impossible.
//!
//! RELATIONSHIP TO OTHER MODULES:
//! ---------------------------------------------------------------------------
//! - `poisoning.rs`           -- API-level fingerprint poisoning (canvas, WebGL, ...)
//! - `behavioral_mimicry.rs`  -- Level 1 static noise (simple jitter)
//! - `detection.rs`           -- Honeypot / detection engine that feeds
//!                               countermeasure signals into `evolve_poison`.

use rand::{rngs::StdRng, Rng, SeedableRng};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

// =============================================================================
// SESSION ENTROPY SEED
// =============================================================================

/// Per-tab entropy context.  The `session_nonce` seeds the PRNG so every
/// session produces unique-but-reproducible noise.  `poison_variant` tracks
/// how many times the noise has been escalated via `evolve_poison`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntropySeed {
    pub tab_id: u64,
    pub session_nonce: u64,
    pub created_at: DateTime<Utc>,
    pub poison_variant: u32,
}

// =============================================================================
// EVOLUTION TYPES
// =============================================================================

/// Record of a single poison evolution step (for audit / MCP reporting).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoisonEvolution {
    pub timestamp: DateTime<Utc>,
    pub old_variant: u32,
    pub new_variant: u32,
    pub reason: String,
    pub patch_diff: String,
}

/// Sent over the MCP channel when a countermeasure is detected and the
/// poison parameters are escalated.
#[derive(Debug, Clone)]
pub struct EvolutionRequest {
    pub current_variant: u32,
    pub detected_countermeasure: String,
}

// =============================================================================
// SUB-MIMICS -- Mouse, Scroll, Typing
// =============================================================================

/// Advanced mouse mimic with velocity-curve shaping and path deviation.
#[derive(Debug, Clone)]
struct AdvancedMouseMimic {
    /// Normalized velocity curve sampled along the move duration.
    velocity_curve: Vec<f32>,
    /// Per-axis random jitter magnitude (pixels).
    micro_jitter: f32,
    /// Per-axis random path deviation magnitude (pixels).
    path_deviation: f32,
}

/// Scroll mimic with multi-segment inertia decay.
#[derive(Debug, Clone)]
struct HumanScrollMimic {
    /// Inertia multipliers sampled across the scroll decay.
    inertia_profile: Vec<f32>,
    /// Random jitter added on top of the inertia-scaled delta.
    jitter_amplitude: f32,
}

/// Typing mimic with distribution-sampled delays and deliberate errors.
#[derive(Debug, Clone)]
struct RealisticTypingMimic {
    /// Pool of plausible inter-key delays (ms) sampled uniformly.
    delay_distribution: Vec<u64>,
    /// Probability [0.0, 1.0] that any given keystroke is an error.
    error_rate: f32,
    /// Extra per-keystroke jitter added on top of the sampled delay (ms).
    jitter_ms: u64,
}

// =============================================================================
// BEHAVIORAL MIMIC LEVEL 4 -- Core engine
// =============================================================================

/// Level 4 behavioral mimicry engine with self-evolving noise parameters.
///
/// # Usage
///
/// ```ignore
/// use crate::behavioral_mimicry_level4::BehavioralMimicLevel4;
///
/// let mut mimic = BehavioralMimicLevel4::new(tab_id, None);
///
/// // Mouse:
/// let (dx, dy) = mimic.mimic_mouse_delta(raw_dx, raw_dy);
///
/// // Scroll:
/// let delta = mimic.mimic_scroll_delta(raw_delta);
///
/// // Typing:
/// let (ch, delay_ms) = mimic.mimic_typing_char("hello world", 0);
///
/// // When a countermeasure is detected:
/// mimic.evolve_poison("canvas_readback_timing");
/// ```
pub struct BehavioralMimicLevel4 {
    current_seed: SessionEntropySeed,
    rng: StdRng,
    mouse: AdvancedMouseMimic,
    scroll: HumanScrollMimic,
    typing: RealisticTypingMimic,
    /// Optional async channel for forwarding evolution requests to MCP.
    mcp_evolution_tx: Option<Sender<EvolutionRequest>>,
    /// Append-only log of every poison escalation.
    poison_history: Arc<Mutex<Vec<PoisonEvolution>>>,
}

impl BehavioralMimicLevel4 {
    /// Create a new Level 4 mimic for the given `tab_id`.
    ///
    /// If `mcp_tx` is provided, evolution requests are forwarded to the MCP
    /// orchestrator over the channel.
    pub fn new(tab_id: u64, mcp_tx: Option<Sender<EvolutionRequest>>) -> Self {
        let nonce = rand::thread_rng().gen::<u64>();
        let seed = SessionEntropySeed {
            tab_id,
            session_nonce: nonce,
            created_at: Utc::now(),
            poison_variant: 0,
        };
        let rng = StdRng::seed_from_u64(nonce);
        Self {
            current_seed: seed,
            rng,
            mouse: AdvancedMouseMimic {
                velocity_curve: vec![0.0, 0.4, 0.9, 1.0, 0.7, 0.3, 0.1, 0.0],
                micro_jitter: 1.5,
                path_deviation: 0.8,
            },
            scroll: HumanScrollMimic {
                inertia_profile: vec![1.2, 1.0, 0.8, 0.6, 0.4, 0.2, 0.1, 0.0],
                jitter_amplitude: 0.45,
            },
            typing: RealisticTypingMimic {
                delay_distribution: vec![40, 55, 70, 85, 100, 120, 150, 200],
                error_rate: 0.04,
                jitter_ms: 40,
            },
            mcp_evolution_tx: mcp_tx,
            poison_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    // -------------------------------------------------------------------------
    // Mouse -- velocity-curve shaped deltas
    // -------------------------------------------------------------------------

    /// Apply Level 4 noise to a raw mouse delta.
    ///
    /// The delta is scaled by a velocity-curve factor, then micro-jitter and
    /// path-deviation noise are added on each axis independently.
    pub fn mimic_mouse_delta(&mut self, raw_dx: f32, raw_dy: f32) -> (f32, f32) {
        let t = self.rng.gen_range(0.0..1.0f32);
        let curve_len = self.mouse.velocity_curve.len();
        let curve_pos = (t * (curve_len - 1) as f32) as usize;
        let velocity_factor = self.mouse.velocity_curve[curve_pos.min(curve_len - 1)];

        let jitter_x = self.rng.gen_range(-self.mouse.micro_jitter..self.mouse.micro_jitter);
        let jitter_y = self.rng.gen_range(-self.mouse.micro_jitter..self.mouse.micro_jitter);
        let deviation_x = self.rng.gen_range(-self.mouse.path_deviation..self.mouse.path_deviation);
        let deviation_y = self.rng.gen_range(-self.mouse.path_deviation..self.mouse.path_deviation);

        let poisoned_dx = raw_dx * velocity_factor + jitter_x + deviation_x;
        let poisoned_dy = raw_dy * velocity_factor + jitter_y + deviation_y;

        (poisoned_dx, poisoned_dy)
    }

    // -------------------------------------------------------------------------
    // Scroll -- inertia-profiled delta
    // -------------------------------------------------------------------------

    /// Apply Level 4 noise to a raw scroll delta.
    ///
    /// The delta is scaled by a randomly-sampled inertia factor from the
    /// decay profile, and jitter is added.
    pub fn mimic_scroll_delta(&mut self, raw_delta: f32) -> f32 {
        let profile_len = self.scroll.inertia_profile.len();
        let profile_pos =
            (self.rng.gen_range(0.0..1.0f32) * (profile_len - 1) as f32) as usize;
        let inertia = self.scroll.inertia_profile[profile_pos.min(profile_len - 1)];
        let jitter = self
            .rng
            .gen_range(-self.scroll.jitter_amplitude..self.scroll.jitter_amplitude);

        raw_delta * inertia + jitter
    }

    // -------------------------------------------------------------------------
    // Typing -- distribution-sampled with deliberate errors
    // -------------------------------------------------------------------------

    /// Produce the next character (or `None` if past the end) and the delay
    /// in milliseconds that should elapse before injecting it.
    ///
    /// Returns `('\x08', delay)` when simulating a typo-correction backspace.
    pub fn mimic_typing_char(
        &mut self,
        target: &str,
        current_pos: usize,
    ) -> (Option<char>, u64) {
        if current_pos >= target.len() {
            return (None, 0);
        }

        // Sample base delay from the distribution.
        let dist_len = self.typing.delay_distribution.len();
        let base_delay = self.typing.delay_distribution[self.rng.gen_range(0..dist_len)];
        let jitter = self.rng.gen_range(0..self.typing.jitter_ms);
        let delay = base_delay + jitter;

        // Deliberate error?
        if self.rng.gen_bool(self.typing.error_rate as f64) {
            // 60% chance: emit backspace (simulates correcting a typo).
            // 40% chance: emit the correct character anyway (error suppressed).
            if self.rng.gen_bool(0.6) {
                return (Some('\x08'), delay);
            }
        }

        let ch = target.chars().nth(current_pos);
        (ch, delay)
    }

    // -------------------------------------------------------------------------
    // Poison evolution -- self-adapting noise
    // -------------------------------------------------------------------------

    /// Escalate the noise parameters because a countermeasure was detected.
    ///
    /// - Increments `poison_variant` in the seed.
    /// - Increases mouse jitter and scroll amplitude proportionally.
    /// - Logs the change in `poison_history`.
    /// - Optionally sends an `EvolutionRequest` to the MCP channel.
    pub fn evolve_poison(&mut self, countermeasure_detected: &str) {
        // Notify MCP (best-effort, non-blocking).
        if let Some(tx) = &self.mcp_evolution_tx {
            let req = EvolutionRequest {
                current_variant: self.current_seed.poison_variant,
                detected_countermeasure: countermeasure_detected.to_string(),
            };
            let _ = tx.try_send(req);
        }

        // Bump the variant.
        self.current_seed.poison_variant += 1;
        let new_variant = self.current_seed.poison_variant;

        // Scale up noise parameters.
        self.mouse.micro_jitter += 0.3 * new_variant as f32;
        self.scroll.jitter_amplitude += 0.1 * new_variant as f32;

        // Record the evolution.
        if let Ok(mut history) = self.poison_history.lock() {
            history.push(PoisonEvolution {
                timestamp: Utc::now(),
                old_variant: new_variant.saturating_sub(1),
                new_variant,
                reason: countermeasure_detected.to_string(),
                patch_diff: format!(
                    "mouse_jitter={:.1}, scroll_amplitude={:.2}, variant={}",
                    self.mouse.micro_jitter, self.scroll.jitter_amplitude, new_variant,
                ),
            });
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Current session entropy seed (includes poison variant).
    pub fn seed(&self) -> &SessionEntropySeed {
        &self.current_seed
    }

    /// Shared handle to the append-only poison evolution log.
    pub fn poison_history(&self) -> Arc<Mutex<Vec<PoisonEvolution>>> {
        self.poison_history.clone()
    }

    /// Current poison variant number.
    pub fn poison_variant(&self) -> u32 {
        self.current_seed.poison_variant
    }
}

/// Create a Level 4 mimic instance for use in per-tab input handling.
/// This wires the module into the crate so all types are considered used.
pub fn create_level4_mimic(tab_id: u64) -> BehavioralMimicLevel4 {
    BehavioralMimicLevel4::new(tab_id, None)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- construction ---------------------------------------------------------

    #[test]
    fn test_new_creates_variant_zero() {
        let mimic = BehavioralMimicLevel4::new(1, None);
        assert_eq!(mimic.seed().poison_variant, 0);
        assert_eq!(mimic.seed().tab_id, 1);
    }

    #[test]
    fn test_different_tabs_different_nonces() {
        let m1 = BehavioralMimicLevel4::new(1, None);
        let m2 = BehavioralMimicLevel4::new(2, None);
        // Nonces are random -- overwhelmingly unlikely to collide.
        assert_ne!(m1.seed().session_nonce, m2.seed().session_nonce);
    }

    // -- mouse ----------------------------------------------------------------

    #[test]
    fn test_mouse_delta_adds_noise() {
        let mut mimic = BehavioralMimicLevel4::new(42, None);
        let (dx, dy) = mimic.mimic_mouse_delta(10.0, 10.0);
        // The output should differ from the raw input (noise added).
        // Exact equality is astronomically unlikely.
        let differs = (dx - 10.0).abs() > 0.0001 || (dy - 10.0).abs() > 0.0001;
        assert!(differs, "Mouse delta should have noise added");
    }

    #[test]
    fn test_mouse_delta_bounded() {
        let mut mimic = BehavioralMimicLevel4::new(42, None);
        // With default parameters the noise should not dwarf the signal.
        for _ in 0..100 {
            let (dx, _dy) = mimic.mimic_mouse_delta(10.0, 0.0);
            // velocity_factor is in [0, 1] and jitter+deviation are bounded
            // by micro_jitter (1.5) + path_deviation (0.8) = 2.3 per axis.
            assert!(dx.abs() < 15.0, "Mouse delta should be bounded");
        }
    }

    // -- scroll ---------------------------------------------------------------

    #[test]
    fn test_scroll_delta_applies_inertia() {
        let mut mimic = BehavioralMimicLevel4::new(42, None);
        let delta = mimic.mimic_scroll_delta(100.0);
        // The inertia profile values are in [0.0, 1.2], so the output
        // should be roughly 0..120 plus jitter.
        assert!(delta.abs() < 200.0, "Scroll delta should be reasonable");
    }

    #[test]
    fn test_scroll_zero_input_produces_jitter_only() {
        let mut mimic = BehavioralMimicLevel4::new(42, None);
        let delta = mimic.mimic_scroll_delta(0.0);
        // With zero input the output is purely jitter (bounded by 0.45).
        assert!(
            delta.abs() <= 0.45 + f32::EPSILON,
            "Zero scroll should produce only jitter"
        );
    }

    // -- typing ---------------------------------------------------------------

    #[test]
    fn test_typing_returns_correct_chars() {
        let mut mimic = BehavioralMimicLevel4::new(42, None);
        let target = "abc";
        // Collect non-backspace characters across all positions.
        let mut chars = Vec::new();
        for i in 0..target.len() {
            let (ch, delay) = mimic.mimic_typing_char(target, i);
            if let Some(c) = ch {
                if c != '\x08' {
                    chars.push(c);
                }
            }
            assert!(delay > 0, "Delay should be positive");
        }
        // At least some characters should match the target (error_rate is 4%).
        assert!(!chars.is_empty(), "Should produce at least one character");
    }

    #[test]
    fn test_typing_past_end_returns_none() {
        let mut mimic = BehavioralMimicLevel4::new(42, None);
        let (ch, delay) = mimic.mimic_typing_char("hi", 5);
        assert!(ch.is_none());
        assert_eq!(delay, 0);
    }

    #[test]
    fn test_typing_delay_from_distribution() {
        let mut mimic = BehavioralMimicLevel4::new(42, None);
        let (_, delay) = mimic.mimic_typing_char("x", 0);
        // delay = base (40..200) + jitter (0..40), so range is [40, 240).
        assert!(delay >= 40 && delay < 240, "Delay should be in expected range");
    }

    // -- poison evolution -----------------------------------------------------

    #[test]
    fn test_evolve_increments_variant() {
        let mut mimic = BehavioralMimicLevel4::new(1, None);
        assert_eq!(mimic.poison_variant(), 0);

        mimic.evolve_poison("canvas_readback");
        assert_eq!(mimic.poison_variant(), 1);

        mimic.evolve_poison("webgl_timing");
        assert_eq!(mimic.poison_variant(), 2);
    }

    #[test]
    fn test_evolve_increases_noise() {
        let mut mimic = BehavioralMimicLevel4::new(1, None);
        let initial_jitter = mimic.mouse.micro_jitter;
        let initial_amplitude = mimic.scroll.jitter_amplitude;

        mimic.evolve_poison("test");

        assert!(mimic.mouse.micro_jitter > initial_jitter);
        assert!(mimic.scroll.jitter_amplitude > initial_amplitude);
    }

    #[test]
    fn test_evolve_records_history() {
        let mut mimic = BehavioralMimicLevel4::new(1, None);
        mimic.evolve_poison("font_detection");
        mimic.evolve_poison("battery_api");

        let history = mimic.poison_history();
        let history = history.lock().unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].old_variant, 0);
        assert_eq!(history[0].new_variant, 1);
        assert_eq!(history[0].reason, "font_detection");
        assert_eq!(history[1].old_variant, 1);
        assert_eq!(history[1].new_variant, 2);
        assert_eq!(history[1].reason, "battery_api");
    }

    #[test]
    fn test_evolve_sends_mcp_request() {
        let (tx, mut rx) = mpsc::channel::<EvolutionRequest>(8);
        let mut mimic = BehavioralMimicLevel4::new(1, Some(tx));

        mimic.evolve_poison("sensor_api");

        let req = rx.try_recv().expect("Should receive an evolution request");
        assert_eq!(req.current_variant, 0); // sent before the bump
        assert_eq!(req.detected_countermeasure, "sensor_api");
    }

    // -- seed -----------------------------------------------------------------

    #[test]
    fn test_seed_reflects_tab_id() {
        let mimic = BehavioralMimicLevel4::new(0xDEAD, None);
        assert_eq!(mimic.seed().tab_id, 0xDEAD);
    }

    #[test]
    fn test_seed_created_at_is_recent() {
        let before = Utc::now();
        let mimic = BehavioralMimicLevel4::new(1, None);
        let after = Utc::now();
        assert!(mimic.seed().created_at >= before);
        assert!(mimic.seed().created_at <= after);
    }
}
