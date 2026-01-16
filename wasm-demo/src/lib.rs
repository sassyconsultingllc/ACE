//! WASM Entropy Visualization for Sassy Browser Marketing Site
//! 
//! Same cryptographic entropy collection as the real browser.
//! Compiles to WebAssembly - zero JavaScript runtime.
//! 
//! Build: wasm-pack build --target web --release

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, MouseEvent, TouchEvent};
use sha2::{Sha256, Digest};
use std::cell::RefCell;

const POOL_SIZE: usize = 32;
const TARGET_BITS: u32 = 256;

thread_local! {
    static STATE: RefCell<EntropyState> = RefCell::new(EntropyState::new());
}

/// Entropy collection state
struct EntropyState {
    pool: [u8; POOL_SIZE],
    samples: u32,
    last_x: i32,
    last_y: i32,
    last_time: f64,
    direction_changes: u32,
    last_dir: (i32, i32),
    entropy_bits: f64,
    total_distance: f64,
}

impl EntropyState {
    fn new() -> Self {
        Self {
            pool: [0u8; POOL_SIZE],
            samples: 0,
            last_x: 0,
            last_y: 0,
            last_time: 0.0,
            direction_changes: 0,
            last_dir: (0, 0),
            entropy_bits: 0.0,
            total_distance: 0.0,
        }
    }

    fn feed(&mut self, x: i32, y: i32, time: f64) {
        let dx = x - self.last_x;
        let dy = y - self.last_y;
        
        // Only count meaningful movement
        if dx.abs() > 2 || dy.abs() > 2 {
            self.samples += 1;
            
            // Track distance
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            self.total_distance += dist;
            
            // Track direction changes (high entropy)
            let dir = (dx.signum(), dy.signum());
            if dir != self.last_dir && self.last_dir != (0, 0) {
                self.direction_changes += 1;
            }
            self.last_dir = dir;
            
            // Mix into pool
            self.mix(dx, dy, time);
            
            // Update entropy estimate
            self.entropy_bits = (self.samples as f64) * 2.0 
                + (self.direction_changes as f64) * 1.5
                + (self.total_distance / 100.0).min(32.0);
        }
        
        self.last_x = x;
        self.last_y = y;
        self.last_time = time;
    }

    fn mix(&mut self, dx: i32, dy: i32, time: f64) {
        let time_bits = (time * 1000.0) as u64;
        let input: [u8; 16] = [
            (dx & 0xff) as u8,
            ((dx >> 8) & 0xff) as u8,
            (dy & 0xff) as u8,
            ((dy >> 8) & 0xff) as u8,
            (time_bits & 0xff) as u8,
            ((time_bits >> 8) & 0xff) as u8,
            ((time_bits >> 16) & 0xff) as u8,
            ((time_bits >> 24) & 0xff) as u8,
            ((time_bits >> 32) & 0xff) as u8,
            ((time_bits >> 40) & 0xff) as u8,
            self.direction_changes as u8,
            (self.samples & 0xff) as u8,
            ((self.samples >> 8) & 0xff) as u8,
            (self.total_distance as u32 & 0xff) as u8,
            ((self.total_distance as u32 >> 8) & 0xff) as u8,
            ((self.total_distance as u32 >> 16) & 0xff) as u8,
        ];

        // XOR into pool
        let pos = (self.samples as usize) % POOL_SIZE;
        for (i, byte) in input.iter().enumerate() {
            self.pool[(pos + i) % POOL_SIZE] ^= byte;
        }

        // Periodic SHA256 mixing for better distribution
        if self.samples % 16 == 0 {
            let mut hasher = Sha256::new();
            hasher.update(&self.pool);
            hasher.update(&input);
            let result = hasher.finalize();
            self.pool.copy_from_slice(&result[..POOL_SIZE]);
        }
    }

    fn progress(&self) -> f32 {
        (self.entropy_bits / TARGET_BITS as f64).min(1.0) as f32
    }

    fn is_complete(&self) -> bool {
        self.entropy_bits >= TARGET_BITS as f64
    }

    fn get_hex(&self, index: usize) -> String {
        if index < POOL_SIZE {
            format!("{:02X}", self.pool[index])
        } else {
            "00".to_string()
        }
    }

    fn active_cells(&self) -> usize {
        ((self.entropy_bits / 8.0) as usize).min(POOL_SIZE)
    }
}


// ============================================
// WASM BINDINGS - Exposed to the browser
// ============================================

#[wasm_bindgen]
pub fn init_entropy() {
    STATE.with(|s| {
        *s.borrow_mut() = EntropyState::new();
    });
}

#[wasm_bindgen]
pub fn feed_mouse(x: i32, y: i32, time: f64) {
    STATE.with(|s| {
        s.borrow_mut().feed(x, y, time);
    });
}

#[wasm_bindgen]
pub fn get_progress() -> f32 {
    STATE.with(|s| s.borrow().progress())
}

#[wasm_bindgen]
pub fn get_bits() -> u32 {
    STATE.with(|s| s.borrow().entropy_bits as u32)
}

#[wasm_bindgen]
pub fn get_samples() -> u32 {
    STATE.with(|s| s.borrow().samples)
}

#[wasm_bindgen]
pub fn is_complete() -> bool {
    STATE.with(|s| s.borrow().is_complete())
}

#[wasm_bindgen]
pub fn get_hex_cell(index: usize) -> String {
    STATE.with(|s| s.borrow().get_hex(index))
}

#[wasm_bindgen]
pub fn get_active_cells() -> usize {
    STATE.with(|s| s.borrow().active_cells())
}

// ============================================
// CANVAS RENDERING - Pure Rust, no JS
// ============================================

#[wasm_bindgen]
pub fn render(canvas_id: &str) {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
        .get_element_by_id(canvas_id)
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    
    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
        .unwrap();
    
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    
    STATE.with(|s| {
        let state = s.borrow();
        render_entropy(&ctx, width, height, &state);
    });
}

fn render_entropy(ctx: &CanvasRenderingContext2d, width: f64, height: f64, state: &EntropyState) {
    // Clear
    ctx.set_fill_style_str("#0a0e12");
    ctx.fill_rect(0.0, 0.0, width, height);
    
    // Grid background
    ctx.set_fill_style_str("#111820");
    round_rect(ctx, 20.0, 20.0, width - 40.0, 140.0, 8.0);
    ctx.fill();
    
    // Draw 32 hex cells (4 rows x 8 cols)
    let cell_w = (width - 80.0) / 8.0;
    let cell_h = 28.0;
    let gap = 4.0;
    let active = state.active_cells();
    
    for i in 0..32 {
        let col = (i % 8) as f64;
        let row = (i / 8) as f64;
        
        let cx = 30.0 + col * (cell_w + gap);
        let cy = 35.0 + row * (cell_h + gap);
        
        // Cell background
        let bg = if i < active { "#0f1a12" } else { "#0d1117" };
        ctx.set_fill_style_str(bg);
        round_rect(ctx, cx, cy, cell_w, cell_h, 3.0);
        ctx.fill();
        
        // Hex text
        let hex = state.get_hex(i);
        let color = if i < active { "#22c55e" } else { "#1a2a1a" };
        ctx.set_fill_style_str(color);
        ctx.set_font("14px monospace");
        ctx.set_text_align("center");
        let _ = ctx.fill_text(&hex, cx + cell_w / 2.0, cy + 20.0);
        
        // Glow effect for recently updated cells
        if i < active && i >= active.saturating_sub(4) {
            ctx.set_shadow_color("#22c55e");
            ctx.set_shadow_blur(8.0);
        } else {
            ctx.set_shadow_blur(0.0);
        }
    }
    ctx.set_shadow_blur(0.0);
    
    // Progress bar
    let bar_y = 175.0;
    let bar_w = width - 60.0;
    let progress = state.progress();
    
    // Bar background
    ctx.set_fill_style_str("#1a2a1a");
    round_rect(ctx, 30.0, bar_y, bar_w, 8.0, 4.0);
    ctx.fill();
    
    // Bar fill
    if progress > 0.0 {
        let color = if progress < 0.3 { "#ef4444" }
                   else if progress < 0.6 { "#f59e0b" }
                   else { "#22c55e" };
        ctx.set_fill_style_str(color);
        round_rect(ctx, 30.0, bar_y, bar_w * progress as f64, 8.0, 4.0);
        ctx.fill();
    }
    
    // Progress text
    ctx.set_fill_style_str("#6b7280");
    ctx.set_font("12px monospace");
    ctx.set_text_align("left");
    let bits_text = format!("{} / 256 bits", state.entropy_bits as u32);
    let _ = ctx.fill_text(&bits_text, 30.0, bar_y + 24.0);
    
    ctx.set_text_align("right");
    let samples_text = format!("{} samples", state.samples);
    let _ = ctx.fill_text(&samples_text, width - 30.0, bar_y + 24.0);
    
    // Keygen steps
    render_keygen_steps(ctx, 30.0, bar_y + 45.0, state);
    
    // Password comparison
    render_comparison(ctx, 30.0, bar_y + 180.0, state);
}

fn render_keygen_steps(ctx: &CanvasRenderingContext2d, x: f64, y: f64, state: &EntropyState) {
    let progress = state.progress();
    let active_step = if progress < 0.25 { 1 }
                     else if progress < 0.50 { 2 }
                     else if progress < 0.75 { 3 }
                     else { 4 };
    
    let steps = [
        ("1.", "Your mouse movements + clicks + timing", "feeds entropy pool"),
        ("2.", "32-byte Master Secret generated", "ChaCha20-Poly1305"),
        ("3.", "Ed25519 Key Pair derived", "same as Signal, SSH, Tor"),
        ("4.", "Recovery Key displayed", "WRITE THIS DOWN"),
    ];
    
    let mut cy = y;
    for (i, (num, label, detail)) in steps.iter().enumerate() {
        let step = (i + 1) as u8;
        let is_active = step <= active_step;
        let is_current = step == active_step;
        
        // Step number (blue)
        ctx.set_fill_style_str("#3b82f6");
        ctx.set_font("13px monospace");
        ctx.set_text_align("left");
        let _ = ctx.fill_text(num, x, cy);
        
        // Label
        let label_color = if is_current { "#22c55e" } 
                         else if is_active { "#e0ffe0" } 
                         else { "#4b5563" };
        ctx.set_fill_style_str(label_color);
        let _ = ctx.fill_text(label, x + 24.0, cy);
        
        // Detail
        ctx.set_fill_style_str("#6b7280");
        ctx.set_font("11px monospace");
        let _ = ctx.fill_text(&format!("↓ {}", detail), x + 24.0, cy + 14.0);
        
        cy += 32.0;
    }
}

fn render_comparison(ctx: &CanvasRenderingContext2d, x: f64, y: f64, state: &EntropyState) {
    ctx.set_font("11px monospace");
    ctx.set_text_align("left");
    
    ctx.set_fill_style_str("#6b7280");
    let _ = ctx.fill_text("vs common passwords:", x, y);
    
    let comparisons = [
        ("zebra123", "instant", "#ef4444"),
        ("P@ssw0rd!", "3 hours", "#f59e0b"),
        ("correct-horse-battery", "centuries", "#22c55e"),
    ];
    
    let mut cy = y + 18.0;
    for (pwd, time, color) in comparisons {
        ctx.set_fill_style_str(color);
        let _ = ctx.fill_text(&format!("\"{}\" = {}", pwd, time), x + 8.0, cy);
        cy += 16.0;
    }
    
    // Your key
    let bits = state.entropy_bits as u32;
    let your_time = if bits >= 256 { "heat death of universe" }
                   else if bits >= 128 { "billions of years" }
                   else if bits >= 64 { "centuries" }
                   else { "keep moving!" };
    
    ctx.set_fill_style_str("#22c55e");
    let _ = ctx.fill_text(&format!("Your {}-bit key = {}", bits, your_time), x + 8.0, cy + 4.0);
}

// Helper for rounded rectangles
fn round_rect(ctx: &CanvasRenderingContext2d, x: f64, y: f64, w: f64, h: f64, r: f64) {
    ctx.begin_path();
    ctx.move_to(x + r, y);
    ctx.line_to(x + w - r, y);
    ctx.arc_to(x + w, y, x + w, y + r, r).unwrap();
    ctx.line_to(x + w, y + h - r);
    ctx.arc_to(x + w, y + h, x + w - r, y + h, r).unwrap();
    ctx.line_to(x + r, y + h);
    ctx.arc_to(x, y + h, x, y + h - r, r).unwrap();
    ctx.line_to(x, y + r);
    ctx.arc_to(x, y, x + r, y, r).unwrap();
    ctx.close_path();
}
