//! UI Rendering - Production quality text, shapes, UI components
//! Uses fontdue for real text rendering
#![allow(dead_code)]
#![allow(unused_variables)]

use crate::ui::{Theme, Edge, SidebarLayout, Rect, TabManager};
use crate::ui::network_bar::{NetworkBar, NetworkBarColors, RequestState};
use crate::ai::{AiConfig, AiProvider};
use fontdue::{Font, FontSettings};
use std::sync::OnceLock;

// Re-export helpers for external use
#[allow(unused_imports)]
pub use crate::ui::network_bar::{format_bytes, format_bytes_per_sec, truncate_url, truncate_host, state_text};

// Global font instance
static FONT: OnceLock<Font> = OnceLock::new();

fn get_font() -> &'static Font {
    FONT.get_or_init(|| {
        // Try to load DejaVu Sans Mono, fall back to embedded minimal font
        // Use bundled font if DejaVu is not available
        let font_data = include_bytes!("../../assets/fonts/Metamorphous-7wZ4.ttf");
        Font::from_bytes(font_data.as_slice(), FontSettings::default())
            .expect("Failed to load font")
    })
}

pub fn hex_to_u32(hex: &str) -> u32 {
    let hex = hex.trim_start_matches('#');
    u32::from_str_radix(hex, 16).unwrap_or(0xFF000000) | 0xFF000000
}

pub fn u32_to_rgb(color: u32) -> (u8, u8, u8) {
    let r = ((color >> 16) & 0xFF) as u8;
    let g = ((color >> 8) & 0xFF) as u8;
    let b = (color & 0xFF) as u8;
    (r, g, b)
}

pub fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

pub fn blend_colors(bg: u32, fg: u32, alpha: f32) -> u32 {
    let (br, bg_g, bb) = u32_to_rgb(bg);
    let (fr, fg_g, fb) = u32_to_rgb(fg);
    
    let r = (br as f32 * (1.0 - alpha) + fr as f32 * alpha) as u8;
    let g = (bg_g as f32 * (1.0 - alpha) + fg_g as f32 * alpha) as u8;
    let b = (bb as f32 * (1.0 - alpha) + fb as f32 * alpha) as u8;
    
    rgb_to_u32(r, g, b)
}

pub struct UIRenderer {
    pub width: u32,
    pub height: u32,
}

impl UIRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        // Ensure font is loaded
        let _ = get_font();
        Self { width, height }
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
    
    pub fn fill_rect(&self, buffer: &mut [u32], x: i32, y: i32, w: u32, h: u32, color: u32) {
        let alpha = ((color >> 24) & 0xFF) as f32 / 255.0;
        let is_opaque = alpha >= 0.99;
        
        for dy in 0..h {
            let py = y + dy as i32;
            if py < 0 || py >= self.height as i32 { continue; }
            
            for dx in 0..w {
                let px = x + dx as i32;
                if px < 0 || px >= self.width as i32 { continue; }
                
                let idx = (py as u32 * self.width + px as u32) as usize;
                if idx < buffer.len() {
                    if is_opaque {
                        buffer[idx] = color | 0xFF000000;
                    } else {
                        buffer[idx] = blend_colors(buffer[idx], color, alpha);
                    }
                }
            }
        }
    }
    
    pub fn stroke_rect(&self, buffer: &mut [u32], x: i32, y: i32, w: u32, h: u32, color: u32, thickness: u32) {
        self.fill_rect(buffer, x, y, w, thickness, color); // Top
        self.fill_rect(buffer, x, y + h as i32 - thickness as i32, w, thickness, color); // Bottom
        self.fill_rect(buffer, x, y, thickness, h, color); // Left
        self.fill_rect(buffer, x + w as i32 - thickness as i32, y, thickness, h, color); // Right
    }
    
    pub fn fill_rounded_rect(&self, buffer: &mut [u32], x: i32, y: i32, w: u32, h: u32, color: u32, radius: u32) {
        let r = radius.min(w / 2).min(h / 2);
        
        for dy in 0..h {
            for dx in 0..w {
                let px = x + dx as i32;
                let py = y + dy as i32;
                
                if px < 0 || py < 0 || px >= self.width as i32 || py >= self.height as i32 {
                    continue;
                }
                
                // Check if in corner regions
                let in_corner = |cx: u32, cy: u32| -> bool {
                    let cdx = if dx < r { r - dx } else if dx >= w - r { dx - (w - r - 1) } else { 0 };
                    let cdy = if dy < r { r - dy } else if dy >= h - r { dy - (h - r - 1) } else { 0 };
                    
                    if cdx > 0 && cdy > 0 {
                        let dist_sq = cdx * cdx + cdy * cdy;
                        dist_sq > r * r
                    } else {
                        false
                    }
                };
                
                if !in_corner(dx, dy) {
                    let idx = (py as u32 * self.width + px as u32) as usize;
                    if idx < buffer.len() {
                        buffer[idx] = color | 0xFF000000;
                    }
                }
            }
        }
    }
    
    fn draw_char(&self, buffer: &mut [u32], c: char, x: i32, y: i32, size: f32, color: u32) -> u32 {
        let font = get_font();
        let (metrics, bitmap) = font.rasterize(c, size);
        
        let (cr, cg, cb) = u32_to_rgb(color);
        
        for row in 0..metrics.height {
            let py = y + row as i32 - metrics.height as i32 + metrics.ymin as i32 + (size * 0.8) as i32;
            if py < 0 || py >= self.height as i32 { continue; }
            
            for col in 0..metrics.width {
                let px = x + col as i32;
                if px < 0 || px >= self.width as i32 { continue; }
                
                let alpha = bitmap[row * metrics.width + col] as f32 / 255.0;
                if alpha < 0.01 { continue; }
                
                let idx = (py as u32 * self.width + px as u32) as usize;
                if idx < buffer.len() {
                    if alpha > 0.99 {
                        buffer[idx] = color | 0xFF000000;
                    } else {
                        buffer[idx] = blend_colors(buffer[idx], color, alpha);
                    }
                }
            }
        }
        
        metrics.advance_width as u32
    }
    
    pub fn draw_text(&self, buffer: &mut [u32], text: &str, x: i32, y: i32, size: f32, color: u32) -> u32 {
        let mut cursor_x = x;
        for c in text.chars() {
            let advance = self.draw_char(buffer, c, cursor_x, y, size, color);
            cursor_x += advance as i32;
        }
        (cursor_x - x) as u32
    }
    
    pub fn draw_text_truncated(&self, buffer: &mut [u32], text: &str, x: i32, y: i32, 
                                max_width: u32, size: f32, color: u32) -> u32 {
        let font = get_font();
        let mut chars: Vec<char> = text.chars().collect();
        
        // Measure full text
        let full_width: f32 = chars.iter()
            .map(|c| font.metrics(*c, size).advance_width)
            .sum();
        
        if (full_width as u32) <= max_width {
            return self.draw_text(buffer, text, x, y, size, color);
        }
        
        // Need to truncate with ellipsis
        let ellipsis_width: f32 = "...".chars()
            .map(|c| font.metrics(c, size).advance_width)
            .sum();
        
        let target_width = max_width as f32 - ellipsis_width;
        let mut width: f32 = 0.0;
        let mut truncate_at = chars.len();
        
        for (i, c) in chars.iter().enumerate() {
            let char_width = font.metrics(*c, size).advance_width;
            if width + char_width > target_width {
                truncate_at = i;
                break;
            }
            width += char_width;
        }
        
        chars.truncate(truncate_at);
        let truncated: String = chars.into_iter().chain("...".chars()).collect();
        self.draw_text(buffer, &truncated, x, y, size, color)
    }
    
    pub fn measure_text(&self, text: &str, size: f32) -> u32 {
        let font = get_font();
        text.chars()
            .map(|c| font.metrics(c, size).advance_width as u32)
            .sum()
    }
    
    pub fn blit_preview(&self, buffer: &mut [u32], preview: &[u32], 
                         src_w: u32, src_h: u32,
                         dst_x: i32, dst_y: i32, dst_w: u32, dst_h: u32) {
        let scale_x = src_w as f32 / dst_w as f32;
        let scale_y = src_h as f32 / dst_h as f32;
        
        for dy in 0..dst_h {
            let py = dst_y + dy as i32;
            if py < 0 || py >= self.height as i32 { continue; }
            
            let src_y = (dy as f32 * scale_y) as u32;
            if src_y >= src_h { continue; }
            
            for dx in 0..dst_w {
                let px = dst_x + dx as i32;
                if px < 0 || px >= self.width as i32 { continue; }
                
                let src_x = (dx as f32 * scale_x) as u32;
                if src_x >= src_w { continue; }
                
                let src_idx = (src_y * src_w + src_x) as usize;
                let dst_idx = (py as u32 * self.width + px as u32) as usize;
                
                if src_idx < preview.len() && dst_idx < buffer.len() {
                    buffer[dst_idx] = preview[src_idx];
                }
            }
        }
    }
    
    pub fn draw_nav_bar(&self, buffer: &mut [u32], bounds: Rect, theme: &Theme,
                         url: &str, can_back: bool, can_forward: bool, loading: bool,
                         show_help_button: bool, help_enabled: bool, help_open: bool) {
        let bg = hex_to_u32(&theme.colors.surface);
        let text_color = hex_to_u32(&theme.colors.text_primary);
        let text_dim = hex_to_u32(&theme.colors.text_secondary);
        let accent = hex_to_u32(&theme.colors.accent);
        let border = hex_to_u32(&theme.colors.border);
        
        // Background
        self.fill_rect(buffer, bounds.x as i32, bounds.y as i32, bounds.width, bounds.height, bg);
        
        // Bottom border
        self.fill_rect(buffer, bounds.x as i32, (bounds.y + bounds.height - 1) as i32, bounds.width, 1, border);
        
        // Navigation buttons
        let btn_size = 32u32;
        let btn_y = bounds.y + (bounds.height - btn_size) / 2;
        let mut x = bounds.x + 8;
        let network_width = 100u32;
        let network_x = bounds.x as i32 + bounds.width as i32 - (network_width as i32 + 10);
        let mut address_right = network_x - 8;
        
        // Back button
        let back_color = if can_back { text_color } else { text_dim };
        self.fill_rounded_rect(buffer, x as i32, btn_y as i32, btn_size, btn_size, 
                                hex_to_u32(&theme.colors.surface_elevated), 4);
        self.draw_text(buffer, "←", (x + 8) as i32, (btn_y + 22) as i32, 16.0, back_color);
        x += btn_size + 4;
        
        // Forward button
        let fwd_color = if can_forward { text_color } else { text_dim };
        self.fill_rounded_rect(buffer, x as i32, btn_y as i32, btn_size, btn_size,
                                hex_to_u32(&theme.colors.surface_elevated), 4);
        self.draw_text(buffer, "→", (x + 8) as i32, (btn_y + 22) as i32, 16.0, fwd_color);
        x += btn_size + 4;
        
        // Refresh button
        self.fill_rounded_rect(buffer, x as i32, btn_y as i32, btn_size, btn_size,
                                hex_to_u32(&theme.colors.surface_elevated), 4);
        let refresh_char = if loading { "◌" } else { "↻" };
        self.draw_text(buffer, refresh_char, (x + 8) as i32, (btn_y + 22) as i32, 16.0, text_color);
        x += btn_size + 12;

        // Help button near network indicator
        if show_help_button {
            let help_x = network_x - btn_size as i32 - 8;
            let help_bg = if help_open {
                accent
            } else {
                hex_to_u32(&theme.colors.surface_elevated)
            };
            let help_fg = if help_open {
                0xFFffffff
            } else if help_enabled {
                text_color
            } else {
                text_dim
            };
            self.fill_rounded_rect(buffer, help_x, btn_y as i32, btn_size, btn_size, help_bg, 4);
            self.draw_text(buffer, "?", help_x + 10, (btn_y + 22) as i32, 16.0, help_fg);
            address_right = help_x - 8;
        }
        
        // Address bar
        let addr_width = (address_right - x as i32).max(200) as u32;
        self.fill_rounded_rect(buffer, x as i32, btn_y as i32, addr_width, btn_size,
                                hex_to_u32(&theme.colors.surface_elevated), 4);
        
        // URL text
        let url_display = if url.is_empty() { "Search or enter URL" } else { url };
        let url_color = if url.is_empty() { text_dim } else { text_color };
        self.draw_text_truncated(buffer, url_display, (x + 8) as i32, (btn_y + 22) as i32, 
                                  addr_width - 16, 14.0, url_color);
        
        // Loading indicator
        if loading {
            let progress = (std::time::Instant::now().elapsed().as_millis() % 2000) as f32 / 2000.0;
            let bar_width = (addr_width as f32 * progress) as u32;
            self.fill_rect(buffer, x as i32, (btn_y + btn_size - 3) as i32, bar_width, 2, accent);
        }
    }
    
    pub fn draw_tab_list(&self, buffer: &mut [u32], bounds: Rect, 
                          tab_manager: &TabManager, theme: &Theme) {
        let bg = hex_to_u32(&theme.colors.surface);
        let border = hex_to_u32(&theme.colors.border);
        let text_color = hex_to_u32(&theme.colors.text_primary);
        let text_dim = hex_to_u32(&theme.colors.text_secondary);
        let accent = hex_to_u32(&theme.colors.accent);
        
        // Background
        self.fill_rect(buffer, bounds.x as i32, bounds.y as i32, bounds.width, bounds.height, bg);
        
        // Right border
        self.fill_rect(buffer, (bounds.x + bounds.width - 1) as i32, bounds.y as i32, 1, bounds.height, border);
        
        let tab_height = 40u32;
        let mut y = bounds.y + 8;
        
        let active_id = tab_manager.active_tab().map(|t| t.id);
        
        for tab in tab_manager.tabs() {
            if y + tab_height > bounds.y + bounds.height { break; }
            
            let is_active = Some(tab.id) == active_id;
            
            // Tab background
            if is_active {
                self.fill_rounded_rect(buffer, (bounds.x + 4) as i32, y as i32, 
                                        bounds.width - 8, tab_height, accent, 4);
            }
            
            // Favicon placeholder
            let favicon_size = 16u32;
            let favicon_x = bounds.x + 12;
            let favicon_y = y + (tab_height - favicon_size) / 2;
            
            if tab.loading {
                // Loading spinner placeholder
                let spinner_color = if is_active { 0xFFffffff } else { accent };
                self.fill_rounded_rect(buffer, favicon_x as i32, favicon_y as i32,
                                        favicon_size, favicon_size, spinner_color, 8);
            } else {
                // Globe icon placeholder
                let icon_color = if is_active { 0xFFffffff } else { text_dim };
                self.draw_text(buffer, "○", favicon_x as i32, (favicon_y + 14) as i32, 14.0, icon_color);
            }
            
            // Title
            let title_x = favicon_x + favicon_size + 8;
            let title_width = bounds.width - (title_x - bounds.x) - 28;
            let title_color = if is_active { 0xFFffffff } else { text_color };
            self.draw_text_truncated(buffer, &tab.title, title_x as i32, (y + 26) as i32,
                                      title_width, 13.0, title_color);
            
            // Trust indicator dot
            let trust_color = tab.trust_color();
            let dot_x = bounds.x + bounds.width - 20;
            self.fill_rounded_rect(buffer, dot_x as i32, (y + 14) as i32, 10, 10, trust_color, 5);
            
            // Close button (X)
            if !tab.pinned {
                let close_x = bounds.x + bounds.width - 24;
                let close_color = if is_active { 0xCCffffff } else { text_dim };
                self.draw_text(buffer, "×", close_x as i32, (y + 24) as i32, 14.0, close_color);
            }
            
            y += tab_height + 4;
        }
        
        // New tab button
        if y + 36 < bounds.y + bounds.height {
            self.fill_rounded_rect(buffer, (bounds.x + 4) as i32, y as i32,
                                    bounds.width - 8, 32, hex_to_u32(&theme.colors.surface_elevated), 4);
            self.draw_text(buffer, "+ New Tab", (bounds.x + 12) as i32, (y + 22) as i32, 13.0, text_dim);
        }
    }
    
    pub fn draw_tab_tiles(&self, buffer: &mut [u32], tab_manager: &TabManager, 
                           theme: &Theme, content_rect: Rect) {
        if !tab_manager.tile_view_active { return; }
        
        // Overlay background
        let overlay = 0xE0000000;
        self.fill_rect(buffer, content_rect.x as i32, content_rect.y as i32,
                        content_rect.width, content_rect.height, overlay);
        
        // Calculate tile layout
        let tabs = tab_manager.filtered_tabs();
        if tabs.is_empty() { return; }
        
        let tile_layout = crate::ui::TileLayout::calculate(
            content_rect.width,
            content_rect.height,
            tabs.len(),
            200, 400, 0.75, 16
        );
        
        // Draw tiles
        for (i, tab) in tabs.iter().enumerate() {
            let (tile_x, tile_y, tile_w, tile_h) = tile_layout.tile_rect(i);
            let x = content_rect.x + tile_x;
            let y = content_rect.y + tile_y;
            
            let is_selected = tab_manager.selected_index == Some(i);
            
            // Tile background
            let bg = if is_selected {
                hex_to_u32(&theme.colors.accent)
            } else {
                hex_to_u32(&theme.colors.surface_elevated)
            };
            self.fill_rounded_rect(buffer, x as i32, y as i32, tile_w, tile_h, bg, 8);
            
            // Preview area
            let preview_h = tile_h - 50;
            if let Some(ref preview) = tab.preview {
                self.blit_preview(buffer, &preview.data, preview.width, preview.height,
                                  (x + 4) as i32, (y + 4) as i32, tile_w - 8, preview_h - 8);
            } else {
                // Placeholder
                let placeholder = hex_to_u32(&theme.colors.surface);
                self.fill_rect(buffer, (x + 4) as i32, (y + 4) as i32, tile_w - 8, preview_h - 8, placeholder);
            }
            
            // Title
            let title_color = if is_selected { 0xFFffffff } else { hex_to_u32(&theme.colors.text_primary) };
            self.draw_text_truncated(buffer, &tab.title, (x + 8) as i32, (y + preview_h + 20) as i32,
                                      tile_w - 16, 13.0, title_color);
            
            // Trust indicator
            let trust_color = tab.trust_color();
            self.fill_rounded_rect(buffer, (x + tile_w - 18) as i32, (y + preview_h + 8) as i32, 
                                    10, 10, trust_color, 5);
            
            // Keyboard hint
            if i < 9 {
                let hint = format!("{}", i + 1);
                let hint_bg = 0x80000000;
                self.fill_rounded_rect(buffer, (x + 8) as i32, (y + 8) as i32, 20, 20, hint_bg, 4);
                self.draw_text(buffer, &hint, (x + 13) as i32, (y + 22) as i32, 12.0, 0xFFffffff);
            }
        }
        
        // Search bar at top
        let search_y = content_rect.y + 20;
        let search_w = 400u32.min(content_rect.width - 40);
        let search_x = content_rect.x + (content_rect.width - search_w) / 2;
        
        self.fill_rounded_rect(buffer, search_x as i32, search_y as i32, search_w, 40,
                                hex_to_u32(&theme.colors.surface), 20);
        self.draw_text(buffer, "Type to search tabs...", (search_x + 16) as i32, (search_y + 26) as i32,
                        14.0, hex_to_u32(&theme.colors.text_secondary));
    }
    
    pub fn draw_sidebar(&self, buffer: &mut [u32], edge: Edge, 
                         sidebar_layout: &SidebarLayout, theme: &Theme) {
        if let Some(sidebar) = sidebar_layout.get(edge) {
            if !sidebar.is_visible() { return; }
            
            let bounds = sidebar.bounds(self.width, self.height, sidebar_layout);
            let bg = hex_to_u32(&theme.colors.surface);
            
            self.fill_rect(buffer, bounds.x as i32, bounds.y as i32, bounds.width, bounds.height, bg);
        }
    }

    /// Draw detailed network activity bar with status text and per-request info
    pub fn draw_network_bar_detailed(&self, buffer: &mut [u32], bounds: Rect, 
                                      network_bar: &NetworkBar, is_dark: bool) {
        let colors = if is_dark { NetworkBarColors::dark() } else { NetworkBarColors::light() };
        
        // Background
        self.fill_rounded_rect(buffer, bounds.x as i32, bounds.y as i32, 
                               bounds.width, bounds.height, colors.background, 4);
        
        // Activity level bar
        let activity = network_bar.activity_level();
        if activity > 0.0 {
            let bar_width = ((bounds.width - 8) as f32 * activity) as u32;
            let bar_color = if network_bar.is_active { colors.active } else { colors.receiving };
            self.fill_rounded_rect(buffer, (bounds.x + 4) as i32, (bounds.y + bounds.height - 6) as i32,
                                   bar_width, 4, bar_color, 2);
        }
        
        // Status text
        let status = network_bar.status_text();
        self.draw_text_truncated(buffer, &status, (bounds.x + 8) as i32, (bounds.y + 14) as i32,
                                 bounds.width - 16, 11.0, colors.text);
        
        // If expanded, show individual requests
        if network_bar.expanded && bounds.height > 30 {
            let mut y = bounds.y as i32 + 24;
            for req in network_bar.visible_requests() {
                if y as u32 > bounds.y + bounds.height - 16 { break; }
                
                let state_icon = match req.state {
                    RequestState::Connecting => "🔄",
                    RequestState::Sending => "📤",
                    RequestState::Waiting => "⏳",
                    RequestState::Receiving => "📥",
                    _ => "✓",
                };
                let line = format!("{} {}", state_icon, truncate_host(&req.host, 20));
                self.draw_text(buffer, &line, (bounds.x + 8) as i32, y, 10.0, colors.text_dim);
                y += 14;
            }
        }
    }
    
    pub fn draw_sync_status(&self, buffer: &mut [u32], x: i32, y: i32, 
                             client_count: usize, theme: &Theme) {
        let bg = hex_to_u32(&theme.colors.surface_elevated);
        let text_color = hex_to_u32(&theme.colors.text_secondary);
        
        let label = if client_count == 0 {
            "📱 No phones".to_string()
        } else if client_count == 1 {
            "📱 1 phone".to_string()
        } else {
            format!("📱 {} phones", client_count)
        };
        
        let width = self.measure_text(&label, 12.0) + 16;
        self.fill_rounded_rect(buffer, x, y, width, 24, bg, 12);
        self.draw_text(buffer, &label, x + 8, y + 16, 12.0, text_color);
    }

    pub fn draw_help_pane(&self, buffer: &mut [u32], bounds: Rect, theme: &Theme, ai: &AiConfig,
                          response: Option<&str>, error: Option<&str>) {
        let bg = hex_to_u32(&theme.colors.surface_elevated);
        let border = hex_to_u32(&theme.colors.border);
        let text = hex_to_u32(&theme.colors.text_primary);
        let dim = hex_to_u32(&theme.colors.text_secondary);
        let accent = hex_to_u32(&theme.colors.accent);

        self.fill_rect(buffer, bounds.x as i32, bounds.y as i32, bounds.width, bounds.height, bg);
        self.stroke_rect(buffer, bounds.x as i32, bounds.y as i32, bounds.width, bounds.height, border, 1);

        let mut y = bounds.y as i32 + 24;
        let x = bounds.x as i32 + 14;

        self.draw_text(buffer, "AI Help", x, y, 16.0, text);
        y += 22;

        let status = if ai.enabled { "Status: Enabled" } else { "Status: Disabled" };
        self.draw_text(buffer, status, x, y, 13.0, dim);
        y += 18;

        let provider = match ai.provider {
            AiProvider::None => "Provider: not set",
            AiProvider::Anthropic => "Provider: Anthropic (Claude)",
            AiProvider::OpenAI => "Provider: OpenAI",
            AiProvider::Local => "Provider: Local (Ollama)",
        };
        self.draw_text(buffer, provider, x, y, 13.0, dim);
        y += 22;

        let hint_title = "What can I ask?";
        self.draw_text(buffer, hint_title, x, y, 13.0, text);
        y += 18;
        self.draw_text(buffer, "• Explain the current page", x, y, 12.0, dim);
        y += 16;
        self.draw_text(buffer, "• Is this site safe?", x, y, 12.0, dim);
        y += 16;
        self.draw_text(buffer, "• How do I do ...?", x, y, 12.0, dim);
        y += 24;

        let footer = "Configured in config/ai.toml";
        self.draw_text(buffer, footer, x, y, 12.0, accent);
        y += 20;

        if let Some(err) = error {
            self.draw_text(buffer, "Last request failed:", x, y, 12.0, accent);
            y += 16;
            for line in wrap_lines(err, 40) {
                self.draw_text(buffer, &line, x, y, 12.0, dim);
                y += 14;
                if y as u32 > bounds.y + bounds.height { break; }
            }
        } else if let Some(resp) = response {
            self.draw_text(buffer, "AI reply:", x, y, 12.0, accent);
            y += 16;
            for line in wrap_lines(resp, 42) {
                self.draw_text(buffer, &line, x, y, 12.0, text);
                y += 14;
                if y as u32 > bounds.y + bounds.height { break; }
            }
        }
    }
}

/// Naive word-wrap for short UI text
fn wrap_lines(input: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in input.split_whitespace() {
        if current.len() + word.len() + 1 > max_width {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(input.to_string());
    }
    lines
}
