//! Behavioral Mimicry Engine — Human-Like Input Simulation
//!
//! ANTI-FINGERPRINTING THROUGH BEHAVIORAL POISONING
#![allow(dead_code)]
//!
//! PURPOSE:
//! ─────────────────────────────────────────────────────────────────────────
//! Fingerprinters don't just read static hardware properties — they also
//! analyze behavioral patterns (mouse entropy, scroll dynamics, typing
//! cadence) to identify users across sessions. This module poisons those
//! behavioral signals with human-like noise.
//!
//! WHAT IT DOES:
//! ─────────────────────────────────────────────────────────────────────────
//! 1. Mouse movement: Adds micro-jitter + path smoothing (1-3px random)
//! 2. Scroll: Adds inertia + variable acceleration (human-like damping)
//! 3. Typing: Variable delay + occasional backspace (realistic cadence)
//!
//! ISOLATION:
//! ─────────────────────────────────────────────────────────────────────────
//! One BehavioralMimic instance per tab — each tab has its own seed,
//! so cross-tab fingerprinting via input patterns is impossible.

use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

// ═══════════════════════════════════════════════════════════════════════════════
// BEHAVIORAL MIMIC — Core engine
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct BehavioralMimic {
    rng: StdRng,
    mouse: MouseSimulator,
    scroll: ScrollSimulator,
    typing: TypingSimulator,
}

#[derive(Debug, Clone)]
struct MouseSimulator {
    last_pos: (f32, f32),
    path_buffer: VecDeque<(f32, f32, Instant)>,
    max_path_len: usize,
}

#[derive(Debug, Clone)]
struct ScrollSimulator {
    current_velocity: f32,
    inertia_factor: f32,
}

#[derive(Debug, Clone)]
struct TypingSimulator {
    next_char_time: Instant,
    base_delay_ms: u64,
    jitter_ms: u64,
    backspace_prob: f32,
    /// Index into the current target string
    current_index: usize,
    /// Whether a backspace is pending (to simulate typo correction)
    backspace_pending: bool,
}

impl BehavioralMimic {
    /// Create new mimicry engine with a seed for reproducibility.
    /// Each tab should get a unique seed (e.g., tab_id ^ nonce).
    pub fn new(seed: u64) -> Self {
        let rng = StdRng::seed_from_u64(seed);
        Self {
            rng,
            mouse: MouseSimulator {
                last_pos: (0.0, 0.0),
                path_buffer: VecDeque::with_capacity(8),
                max_path_len: 8,
            },
            scroll: ScrollSimulator {
                current_velocity: 0.0,
                inertia_factor: 0.82, // human-like damping
            },
            typing: TypingSimulator {
                next_char_time: Instant::now(),
                base_delay_ms: 70,
                jitter_ms: 45,
                backspace_prob: 0.035,
                current_index: 0,
                backspace_pending: false,
            },
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // MOUSE — Micro-jitter + path smoothing
    // ─────────────────────────────────────────────────────────────────────

    /// Mimic mouse movement — adds human-like jitter & path smoothing.
    /// Returns poisoned delta (dx, dy).
    pub fn mimic_mouse_move(&mut self, raw_dx: f32, raw_dy: f32) -> (f32, f32) {
        let now = Instant::now();

        // Human micro-jitter (1–3 pixels random)
        let jitter_x = self.rng.gen_range(-1.8..1.8);
        let jitter_y = self.rng.gen_range(-1.8..1.8);

        // Simple bezier-like smoothing over last 5–8 moves
        self.mouse.path_buffer.push_back((jitter_x, jitter_y, now));
        if self.mouse.path_buffer.len() > self.mouse.max_path_len {
            self.mouse.path_buffer.pop_front();
        }

        let len = self.mouse.path_buffer.len() as f32;
        let avg_jitter_x = self.mouse.path_buffer.iter()
            .map(|(x, _, _)| *x)
            .sum::<f32>() / len;
        let avg_jitter_y = self.mouse.path_buffer.iter()
            .map(|(_, y, _)| *y)
            .sum::<f32>() / len;

        let poisoned_dx = raw_dx + avg_jitter_x;
        let poisoned_dy = raw_dy + avg_jitter_y;

        self.mouse.last_pos.0 += poisoned_dx;
        self.mouse.last_pos.1 += poisoned_dy;

        (poisoned_dx, poisoned_dy)
    }

    /// Get the current (poisoned) mouse position.
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse.last_pos
    }

    // ─────────────────────────────────────────────────────────────────────
    // SCROLL — Inertia + variable acceleration
    // ─────────────────────────────────────────────────────────────────────

    /// Mimic scroll — adds inertia & variable acceleration.
    /// Returns poisoned delta_y.
    pub fn mimic_scroll(&mut self, raw_delta_y: f32) -> f32 {
        // Human scroll acceleration/deceleration
        let jitter = self.rng.gen_range(-0.35..0.35);
        let accelerated = raw_delta_y * (1.0 + jitter);

        // Apply inertia
        self.scroll.current_velocity += accelerated * 0.28;
        self.scroll.current_velocity *= self.scroll.inertia_factor;

        accelerated + self.scroll.current_velocity
    }

    /// Get current scroll velocity (for frame-based inertia updates).
    pub fn scroll_velocity(&self) -> f32 {
        self.scroll.current_velocity
    }

    // ─────────────────────────────────────────────────────────────────────
    // TYPING — Variable delay + occasional backspace
    // ─────────────────────────────────────────────────────────────────────

    /// Mimic typing — variable delay + occasional backspace.
    /// Call repeatedly with the full target string; returns next char
    /// (or None if waiting for the next keystroke delay).
    ///
    /// Returns `\x08` (backspace) when simulating a typo correction.
    pub fn mimic_typing(&mut self, target_text: &str) -> Option<char> {
        let now = Instant::now();

        if now < self.typing.next_char_time {
            return None;
        }

        // Fake backspace (typo correction)
        if self.typing.backspace_pending {
            self.typing.backspace_pending = false;
            self.typing.next_char_time = now + Duration::from_millis(self.typing.base_delay_ms / 2);
            return Some('\x08'); // backspace
        }

        // Get next character from target
        if let Some(c) = target_text.chars().nth(self.typing.current_index) {
            self.typing.current_index += 1;

            // Random chance of backspace next (simulates mistype → correction)
            if self.rng.gen_bool(self.typing.backspace_prob as f64) {
                self.typing.backspace_pending = true;
            }

            // Variable delay between keystrokes
            let delay = self.typing.base_delay_ms + self.rng.gen_range(0..self.typing.jitter_ms);
            self.typing.next_char_time = now + Duration::from_millis(delay);

            Some(c)
        } else {
            // Reset when finished typing the string
            self.typing.current_index = 0;
            None
        }
    }

    /// Reset the typing simulator for a new input string.
    pub fn reset_typing(&mut self) {
        self.typing.current_index = 0;
        self.typing.backspace_pending = false;
    }

    // ─────────────────────────────────────────────────────────────────────
    // RESET — New tab/session
    // ─────────────────────────────────────────────────────────────────────

    /// Reset all state (new tab, new session, different seed).
    pub fn reset(&mut self, new_seed: u64) {
        self.rng = StdRng::seed_from_u64(new_seed);
        self.mouse = MouseSimulator {
            last_pos: (0.0, 0.0),
            path_buffer: VecDeque::with_capacity(8),
            max_path_len: 8,
        };
        self.scroll = ScrollSimulator {
            current_velocity: 0.0,
            inertia_factor: 0.82,
        };
        self.typing = TypingSimulator {
            next_char_time: Instant::now(),
            base_delay_ms: 70,
            jitter_ms: 45,
            backspace_prob: 0.035,
            current_index: 0,
            backspace_pending: false,
        };
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TAB INPUT HANDLER — Per-tab wrapper for integration
// ═══════════════════════════════════════════════════════════════════════════════

/// Per-tab input handler that wraps BehavioralMimic.
/// Each tab gets its own instance with a unique seed.
///
/// Usage in BrowserEngine:
/// ```ignore
/// // Mouse move:
/// let (dx, dy) = tab.input_handler.handle_mouse_move(raw_dx, raw_dy);
///
/// // Scroll:
/// let delta = tab.input_handler.handle_scroll(raw_delta);
///
/// // Typing:
/// while let Some(c) = tab.input_handler.handle_typing(&text) {
///     if c == '\x08' { backspace(); } else { insert(c); }
/// }
///
/// // Reset on tab close/reopen:
/// tab.input_handler.reset_mimic(new_seed);
/// ```
#[derive(Debug, Clone)]
pub struct TabInputHandler {
    mimic: BehavioralMimic,
}

impl TabInputHandler {
    pub fn new(tab_seed: u64) -> Self {
        Self {
            mimic: BehavioralMimic::new(tab_seed),
        }
    }

    pub fn handle_mouse_move(&mut self, raw_dx: f32, raw_dy: f32) -> (f32, f32) {
        self.mimic.mimic_mouse_move(raw_dx, raw_dy)
    }

    pub fn handle_scroll(&mut self, raw_delta_y: f32) -> f32 {
        self.mimic.mimic_scroll(raw_delta_y)
    }

    pub fn handle_typing(&mut self, target: &str) -> Option<char> {
        self.mimic.mimic_typing(target)
    }

    pub fn reset_typing(&mut self) {
        self.mimic.reset_typing();
    }

    pub fn reset_mimic(&mut self, new_seed: u64) {
        self.mimic.reset(new_seed);
    }

    /// Get the underlying mimic for advanced usage
    pub fn mimic(&self) -> &BehavioralMimic {
        &self.mimic
    }

    pub fn mimic_mut(&mut self) -> &mut BehavioralMimic {
        &mut self.mimic
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_jitter_adds_noise() {
        let mut mimic = BehavioralMimic::new(42);

        // Move exactly 10 pixels right
        let (dx, _dy) = mimic.mimic_mouse_move(10.0, 0.0);

        // Should be close to 10 but NOT exactly 10 (jitter added)
        assert!((dx - 10.0).abs() < 5.0, "Mouse jitter should be subtle");
        assert!((dx - 10.0).abs() > 0.001, "Mouse should have some jitter");
    }

    #[test]
    fn test_scroll_inertia() {
        let mut mimic = BehavioralMimic::new(42);

        // Initial scroll
        let delta1 = mimic.mimic_scroll(100.0);
        assert!(delta1.abs() > 50.0, "Scroll should respond to input");

        // Scroll with zero input — inertia should still produce movement
        let delta2 = mimic.mimic_scroll(0.0);
        assert!(delta2.abs() > 0.1, "Scroll inertia should continue briefly");
    }

    #[test]
    fn test_typing_produces_characters() {
        let mut mimic = BehavioralMimic::new(42);

        // Force the time to be "ready"
        mimic.typing.next_char_time = Instant::now() - Duration::from_millis(100);

        let c = mimic.mimic_typing("hello");
        assert_eq!(c, Some('h'));
    }

    #[test]
    fn test_reset_clears_state() {
        let mut mimic = BehavioralMimic::new(42);

        // Move the mouse around
        mimic.mimic_mouse_move(100.0, 200.0);
        mimic.mimic_scroll(50.0);

        // Reset
        mimic.reset(99);

        assert_eq!(mimic.mouse.last_pos, (0.0, 0.0));
        assert_eq!(mimic.scroll.current_velocity, 0.0);
        assert_eq!(mimic.typing.current_index, 0);
    }

    #[test]
    fn test_different_seeds_different_jitter() {
        let mut m1 = BehavioralMimic::new(1);
        let mut m2 = BehavioralMimic::new(2);

        let (dx1, _) = m1.mimic_mouse_move(10.0, 0.0);
        let (dx2, _) = m2.mimic_mouse_move(10.0, 0.0);

        // Different seeds should produce different jitter
        assert!((dx1 - dx2).abs() > 0.001, "Different seeds should produce different jitter");
    }

    #[test]
    fn test_tab_input_handler() {
        let mut handler = TabInputHandler::new(0xdeadbeef);

        let (dx, dy) = handler.handle_mouse_move(5.0, 5.0);
        assert!(dx.abs() > 0.0);
        assert!(dy.abs() > 0.0);

        let scroll = handler.handle_scroll(10.0);
        assert!(scroll.abs() > 0.0);
    }
}
