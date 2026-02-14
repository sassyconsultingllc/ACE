//! Theme system for Sassy Browser
//! Handles loading, parsing, and applying themes from TOML config


use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub meta: ThemeMeta,
    pub colors: ThemeColors,
    pub typography: Typography,
    pub spacing: Spacing,
    pub borders: Borders,
    pub shadows: Shadows,
    pub layout: Layout,
    pub animations: Animations,
    pub phone_sync: PhoneSyncConfig,
    pub panels: PanelConfig,
    pub tab_tiles: TabTileConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMeta {
    pub name: String,
    pub version: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    // Base
    pub background: String,
    pub surface: String,
    pub surface_elevated: String,
    pub border: String,
    
    // Text
    pub text_primary: String,
    pub text_secondary: String,
    pub text_muted: String,
    
    // Accent
    pub accent: String,
    pub accent_hover: String,
    pub accent_active: String,
    
    // Status
    pub success: String,
    pub warning: String,
    pub error: String,
    pub info: String,
    
    // Tabs
    pub tab_active: String,
    pub tab_inactive: String,
    pub tab_hover: String,
    
    // Sidebar
    pub sidebar_bg: String,
    pub sidebar_border: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Typography {
    pub font_family: String,
    pub font_mono: String,
    pub font_size_xs: u32,
    pub font_size_sm: u32,
    pub font_size_base: u32,
    pub font_size_lg: u32,
    pub font_size_xl: u32,
    pub font_size_2xl: u32,
    pub line_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spacing {
    pub xs: u32,
    pub sm: u32,
    pub md: u32,
    pub lg: u32,
    pub xl: u32,
    pub xxl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Borders {
    pub radius_sm: u32,
    pub radius_md: u32,
    pub radius_lg: u32,
    pub radius_full: u32,
    pub width: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shadows {
    pub sm: String,
    pub md: String,
    pub lg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub sidebar_top: SidebarState,
    pub sidebar_right: SidebarState,
    pub sidebar_bottom: SidebarState,
    pub sidebar_left: SidebarState,
    
    pub sidebar_top_height: u32,
    pub sidebar_bottom_height: u32,
    pub sidebar_left_width: u32,
    pub sidebar_right_width: u32,
    
    pub sidebar_collapsed_size: u32,
    
    pub tab_tile_min_width: u32,
    pub tab_tile_max_width: u32,
    pub tab_tile_aspect_ratio: f32,
    pub tab_tile_gap: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SidebarState {
    Hidden,
    Collapsed,
    Expanded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Animations {
    pub duration_fast: u32,
    pub duration_normal: u32,
    pub duration_slow: u32,
    pub easing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneSyncConfig {
    pub enabled: bool,
    pub port: u16,
    pub auto_connect: bool,
    pub show_qr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    pub address_bar: bool,
    pub navigation_buttons: bool,
    pub tab_bar: bool,
    pub bookmarks_bar: bool,
    pub status_bar: bool,
    pub dev_tools: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabTileConfig {
    pub columns_auto: bool,
    pub columns_min: u32,
    pub columns_max: u32,
    pub show_favicon: bool,
    pub show_title: bool,
    pub show_url: bool,
    pub show_preview: bool,
    pub preview_quality: PreviewQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PreviewQuality {
    Low,
    Medium,
    High,
}

impl Theme {
    /// Load theme from TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read theme file: {}", e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse theme: {}", e))
    }
    
    /// Save theme to TOML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize theme: {}", e))?;
        fs::write(path, content)
            .map_err(|e| format!("Failed to write theme: {}", e))
    }
    
    /// Get default dark theme
    pub fn dark() -> Self {
        Self {
            meta: ThemeMeta {
                name: "Sassy Dark".into(),
                version: "2.0".into(),
                author: "SassyBrowser".into(),
            },
            colors: ThemeColors {
                // Brand palette:
                // Brand Purple: #6C63FF - Primary accent
                // Dark Blue: #101E32 - Primary background
                // Dark Gray: #2E384B - Panels/surfaces
                // Yellow: #FEC337 - Highlights/warnings
                // Light Gray: #F6F6F6 - Text on dark
                background: "#101E32".into(),
                surface: "#2E384B".into(),
                surface_elevated: "#384458".into(),
                border: "#445066".into(),
                text_primary: "#F6F6F6".into(),
                text_secondary: "#C0C8D4".into(),
                text_muted: "#808A9E".into(),
                accent: "#6C63FF".into(),
                accent_hover: "#7D75FF".into(),
                accent_active: "#5B53E6".into(),
                success: "#3fb950".into(),
                warning: "#FEC337".into(),
                error: "#f85149".into(),
                info: "#6C63FF".into(),
                tab_active: "#384458".into(),
                tab_inactive: "#101E32".into(),
                tab_hover: "#2E384B".into(),
                sidebar_bg: "#101E32".into(),
                sidebar_border: "#2E384B".into(),
            },
            typography: Typography {
                // Azo Sans preferred, with Space Grotesk and system fallbacks
                font_family: "Azo Sans, Space Grotesk, Inter, system-ui, sans-serif".into(),
                font_mono: "JetBrains Mono, monospace".into(),
                // Increase base and related sizes for better legibility
                font_size_xs: 12,
                font_size_sm: 14,
                font_size_base: 16,
                font_size_lg: 18,
                font_size_xl: 22,
                font_size_2xl: 26,
                line_height: 1.5,
            },
            spacing: Spacing {
                xs: 4, sm: 8, md: 16, lg: 24, xl: 32, xxl: 48,
            },
            borders: Borders {
                radius_sm: 4, radius_md: 8, radius_lg: 12, radius_full: 9999, width: 1,
            },
            shadows: Shadows {
                sm: "0 1px 2px rgba(0,0,0,0.3)".into(),
                md: "0 4px 6px rgba(0,0,0,0.4)".into(),
                lg: "0 10px 15px rgba(0,0,0,0.5)".into(),
            },
            layout: Layout {
                sidebar_top: SidebarState::Expanded,
                sidebar_right: SidebarState::Collapsed,
                sidebar_bottom: SidebarState::Hidden,
                sidebar_left: SidebarState::Expanded,
                sidebar_top_height: 48,
                sidebar_bottom_height: 32,
                sidebar_left_width: 280,
                sidebar_right_width: 320,
                sidebar_collapsed_size: 48,
                tab_tile_min_width: 200,
                tab_tile_max_width: 300,
                tab_tile_aspect_ratio: 0.75,
                tab_tile_gap: 12,
            },
            animations: Animations {
                duration_fast: 150,
                duration_normal: 250,
                duration_slow: 400,
                easing: "ease-out".into(),
            },
            phone_sync: PhoneSyncConfig {
                enabled: true,
                port: 8765,
                auto_connect: true,
                show_qr: true,
            },
            panels: PanelConfig {
                address_bar: true,
                navigation_buttons: true,
                tab_bar: true,
                bookmarks_bar: false,
                status_bar: true,
                dev_tools: false,
            },
            tab_tiles: TabTileConfig {
                columns_auto: true,
                columns_min: 2,
                columns_max: 6,
                show_favicon: true,
                show_title: true,
                show_url: false,
                show_preview: true,
                preview_quality: PreviewQuality::Medium,
            },
        }
    }
    
    /// Get default light theme
    pub fn light() -> Self {
        let mut theme = Self::dark();
        theme.meta.name = "Sassy Light".into();
        theme.colors = ThemeColors {
            // Light mode using brand colors
            // Light Gray background, Dark Blue text, Brand Purple accents
            background: "#F6F6F6".into(),
            surface: "#EEEEF2".into(),
            surface_elevated: "#FFFFFF".into(),
            border: "#D0D4DE".into(),
            text_primary: "#101E32".into(),  // Dark Blue
            text_secondary: "#2E384B".into(), // Dark Gray
            text_muted: "#606878".into(),
            accent: "#6C63FF".into(),         // Brand Purple
            accent_hover: "#5B53E6".into(),
            accent_active: "#4A44CC".into(),
            success: "#1a7f37".into(),
            warning: "#B08A00".into(),        // Darker yellow for light bg
            error: "#cf222e".into(),
            info: "#6C63FF".into(),
            tab_active: "#FFFFFF".into(),
            tab_inactive: "#F6F6F6".into(),
            tab_hover: "#E8E8EC".into(),
            sidebar_bg: "#F6F6F6".into(),
            sidebar_border: "#D0D4DE".into(),
        };
        theme
    }
    
    /// Parse hex color to RGBA
    pub fn parse_color(hex: &str) -> (u8, u8, u8, u8) {
        let hex = hex.trim_start_matches('#');
        let len = hex.len();
        
        match len {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                (r, g, b, 255)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
                (r, g, b, a)
            }
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0);
                (r, g, b, 255)
            }
            _ => (0, 0, 0, 255),
        }
    }
    
    /// Convert color to u32 for framebuffer
    pub fn color_to_u32(hex: &str) -> u32 {
        let (r, g, b, _) = Self::parse_color(hex);
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}

/// Theme manager for runtime theme switching
pub struct ThemeManager {
    current: Theme,
    themes: HashMap<String, Theme>,
    custom_path: Option<String>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut themes = HashMap::new();
        themes.insert("dark".into(), Theme::dark());
        themes.insert("light".into(), Theme::light());
        
        Self {
            current: Theme::dark(),
            themes,
            custom_path: None,
        }
    }
    
    pub fn load_custom<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let theme = Theme::load(&path)?;
        let name = theme.meta.name.clone();
        self.themes.insert(name.clone(), theme);
        self.custom_path = Some(path.as_ref().to_string_lossy().into());
        Ok(())
    }
    
    pub fn switch(&mut self, name: &str) -> Result<(), String> {
        if let Some(theme) = self.themes.get(name) {
            self.current = theme.clone();
            Ok(())
        } else {
            Err(format!("Theme '{}' not found", name))
        }
    }
    
    pub fn current(&self) -> &Theme {
        &self.current
    }
    
    pub fn current_mut(&mut self) -> &mut Theme {
        &mut self.current
    }
    
    pub fn list_themes(&self) -> Vec<&str> {
        self.themes.keys().map(|s| s.as_str()).collect()
    }
    
    pub fn save_current(&self) -> Result<(), String> {
        if let Some(ref path) = self.custom_path {
            self.current.save(path)
        } else {
            Err("No custom theme path set".into())
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_defaults_and_color_parse() {
        let dark = Theme::dark();
        assert!(dark.meta.name.contains("Sassy"));

        let light = Theme::light();
        assert!(light.meta.name.contains("Light"));

        // 6-digit hex
        let rgba = Theme::parse_color("#112233");
        assert_eq!(rgba, (0x11, 0x22, 0x33, 255));

        // 3-digit hex
        let rgba2 = Theme::parse_color("#abc");
        assert_eq!(rgba2, (0xaa, 0xbb, 0xcc, 255));

        // color_to_u32 basic check
        let v = Theme::color_to_u32("#010203");
        assert_eq!(v, 0x010203);
    }
}
