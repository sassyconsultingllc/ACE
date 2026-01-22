#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! Video Viewer - MP4, WebM, AVI, MKV playback with thumbnails
//! 
//! Features:
//! - Video thumbnail/poster display
//! - Metadata display (codec, resolution, framerate, duration)
//! - Basic playback controls (placeholder for rodio/gstreamer integration)
//! - Frame-by-frame navigation (when implemented)

use crate::file_handler::{OpenFile, VideoContent};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

/// Video playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

/// Video viewer for various video formats
pub struct VideoViewer {
    state: PlaybackState,
    current_time: f64,
    volume: f32,
    muted: bool,
    loop_enabled: bool,
    fullscreen: bool,
}

impl VideoViewer {
    pub fn new() -> Self {
        Self {
            state: PlaybackState::Stopped,
            current_time: 0.0,
            volume: 0.8,
            muted: false,
            loop_enabled: false,
            fullscreen: false,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        let available = ui.available_size();
        
        egui::ScrollArea::both().show(ui, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("ðŸŽ¬ Video Player").size(20.0));
                    ui.add_space(10.0);
                    ui.label(RichText::new(&file.name).monospace());
                });
                
                ui.separator();
                
                // Main layout - video area + info panel
                ui.horizontal(|ui| {
                    // Video display area
                    let video_width = (available.x - 320.0).max(400.0);
                    let video_height = (video_width * 9.0 / 16.0).min(available.y - 200.0);
                    
                    egui::Frame::none()
                        .fill(Color32::BLACK)
                        .stroke(Stroke::new(1.0, Color32::from_gray(60)))
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(video_width, video_height));
                            
                            // Show thumbnail or placeholder
                            if let Some(ref video) = file.video {
                                if let Some(ref thumb_data) = video.thumbnail {
                                    // TODO: Decode and display thumbnail
                                    ui.centered_and_justified(|ui| {
                                        ui.label(RichText::new("ðŸŽ¬").size(80.0).color(Color32::WHITE));
                                        ui.label(RichText::new("Thumbnail available").color(Color32::GRAY));
                                    });
                                } else {
                                    self.render_placeholder(ui, video_width, video_height);
                                }
                            } else {
                                self.render_placeholder(ui, video_width, video_height);
                            }
                        });
                    
                    ui.add_space(16.0);
                    
                    // Info panel
                    egui::Frame::none()
                        .fill(Color32::from_rgb(30, 33, 40))
                        .rounding(8.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.set_min_width(280.0);
                            self.render_info_panel(ui, file);
                        });
                });
                
                ui.add_space(8.0);
                
                // Playback controls
                self.render_controls(ui, file);
            });
        });
    }
    
    fn render_placeholder(&self, ui: &mut egui::Ui, width: f32, height: f32) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(height / 3.0);
                ui.label(RichText::new("ðŸŽ¬").size(64.0).color(Color32::from_gray(100)));
                ui.add_space(10.0);
                ui.label(RichText::new("Video Preview").size(16.0).color(Color32::from_gray(120)));
                ui.add_space(5.0);
                ui.label(RichText::new("(Native playback coming soon)").size(12.0).color(Color32::from_gray(80)));
            });
        });
    }
    
    fn render_info_panel(&mut self, ui: &mut egui::Ui, file: &OpenFile) {
        ui.heading(RichText::new("Video Information").size(16.0));
        if let Some(ref video) = file.video {
            egui::Grid::new("video_info_grid")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    // Resolution
                    ui.label(RichText::new("Resolution:").color(Color32::GRAY));
                    ui.label(format!("{}Ã—{}", video.width, video.height));
                    ui.end_row();
                    
                    // Frame rate
                    ui.label(RichText::new("Frame Rate:").color(Color32::GRAY));
                    ui.label(format!("{:.2} fps", video.frame_rate));
                    ui.end_row();
                    
                    // Duration
                    ui.label(RichText::new("Duration:").color(Color32::GRAY));
                    ui.label(format_duration(video.duration));
                    ui.end_row();
                    
                    // Video codec
                    if let Some(ref codec) = video.video_codec {
                        ui.label(RichText::new("Video Codec:").color(Color32::GRAY));
                        ui.label(codec);
                        ui.end_row();
                    }
                    
                    // Audio codec
                    if let Some(ref codec) = video.audio_codec {
                        ui.label(RichText::new("Audio Codec:").color(Color32::GRAY));
                        ui.label(codec);
                        ui.end_row();
                    }
                    
                    // Bitrate
                    if let Some(bitrate) = video.bitrate {
                        ui.label(RichText::new("Bitrate:").color(Color32::GRAY));
                        ui.label(format!("{} kbps", bitrate / 1000));
                        ui.end_row();
                    }
                });
            
            // Aspect ratio badge
            ui.add_space(16.0);
            let aspect = video.width as f32 / video.height as f32;
            let aspect_str = if (aspect - 16.0/9.0).abs() < 0.1 {
                "16:9 Widescreen"
            } else if (aspect - 4.0/3.0).abs() < 0.1 {
                "4:3 Standard"
            } else if (aspect - 21.0/9.0).abs() < 0.1 {
                "21:9 Ultrawide"
            } else if (aspect - 1.0).abs() < 0.1 {
                "1:1 Square"
            } else {
                "Custom"
            };
            
            egui::Frame::none()
                .fill(Color32::from_rgb(60, 70, 90))
                .rounding(4.0)
                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.label(RichText::new(aspect_str).size(12.0));
                });
            
            // Quality badge
            let quality = if video.height >= 2160 {
                "4K UHD"
            } else if video.height >= 1440 {
                "2K QHD"
            } else if video.height >= 1080 {
                "Full HD"
            } else if video.height >= 720 {
                "HD"
            } else if video.height >= 480 {
                "SD"
            } else {
                "Low"
            };
            
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(Color32::from_rgb(70, 90, 60))
                .rounding(4.0)
                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.label(RichText::new(quality).size(12.0));
                });
        } else {
            ui.label(RichText::new("No video metadata available").italics().color(Color32::GRAY));
        }
        
        // File info
        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);
        
        ui.label(RichText::new("File Details").size(14.0));
        ui.add_space(8.0);
        
        egui::Grid::new("file_info_grid")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Size:").color(Color32::GRAY));
                ui.label(format_size(file.size));
                ui.end_row();
                
                if let Some(ref mime) = file.mime_type {
                    ui.label(RichText::new("Type:").color(Color32::GRAY));
                    ui.label(mime);
                    ui.end_row();
                }
            });
        
        // Open externally button
        ui.add_space(16.0);
        if ui.button("ðŸŽ¬ Open in System Player").clicked() {
            let _ = open::that(&file.path);
        }
    }
    
    fn render_controls(&mut self, ui: &mut egui::Ui, file: &OpenFile) {
        let duration = file.video.as_ref().map(|v| v.duration).unwrap_or(0.0);
        
        egui::Frame::none()
            .fill(Color32::from_rgb(25, 28, 35))
            .rounding(8.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                // Progress bar / seek
                ui.horizontal(|ui| {
                    ui.label(format_duration(self.current_time));
                    
                    let mut progress = if duration > 0.0 { 
                        (self.current_time / duration) as f32 
                    } else { 
                        0.0 
                    };
                    
                    let slider = ui.add(
                        egui::Slider::new(&mut progress, 0.0..=1.0)
                            .show_value(false)
                            .clamp_to_range(true)
                    );
                    
                    if slider.changed() {
                        self.current_time = progress as f64 * duration;
                    }
                    
                    ui.label(format_duration(duration));
                });
                
                ui.add_space(8.0);
                
                // Playback buttons
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 100.0);
                    
                    // Loop toggle
                    let loop_icon = if self.loop_enabled { "ðŸ”" } else { "âž¡ï¸" };
                    if ui.button(loop_icon).on_hover_text("Toggle Loop").clicked() {
                        self.loop_enabled = !self.loop_enabled;
                    }
                    
                    // Previous frame / rewind
                    if ui.button("â®ï¸").on_hover_text("Previous").clicked() {
                        self.current_time = (self.current_time - 10.0).max(0.0);
                    }
                    
                    // Play/Pause
                    let play_btn = match self.state {
                        PlaybackState::Playing => "â¸ï¸",
                        _ => "â–¶ï¸",
                    };
                    
                    if ui.add(egui::Button::new(RichText::new(play_btn).size(24.0))
                        .min_size(Vec2::new(48.0, 36.0)))
                        .on_hover_text("Play/Pause")
                        .clicked() 
                    {
                        self.state = match self.state {
                            PlaybackState::Playing => PlaybackState::Paused,
                            _ => PlaybackState::Playing,
                        };
                    }
                    
                    // Stop
                    if ui.button("â¹ï¸").on_hover_text("Stop").clicked() {
                        self.state = PlaybackState::Stopped;
                        self.current_time = 0.0;
                    }
                    
                        // Next frame / forward
                        if ui.button("⏭").on_hover_text("Next").clicked() {
                        self.current_time = (self.current_time + 10.0).min(duration);
                    }
                    
                    // Fullscreen (placeholder)
                    if ui.button("â›¶").on_hover_text("Fullscreen").clicked() {
                        self.fullscreen = !self.fullscreen;
                    }
                    
                    // Spacer
                    ui.add_space(20.0);
                    
                    // Volume
                    let vol_icon = if self.muted { "ðŸ”‡" } else { "ðŸ”Š" };
                    if ui.button(vol_icon).on_hover_text("Mute/Unmute").clicked() {
                        self.muted = !self.muted;
                    }
                    
                    ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0)
                        .show_value(false)
                        .clamp_to_range(true));
                });
                
                // Playback info
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let state_str = match self.state {
                        PlaybackState::Stopped => "Stopped",
                        PlaybackState::Playing => "Playing",
                        PlaybackState::Paused => "Paused",
                    };
                    ui.label(RichText::new(state_str).size(11.0).color(Color32::GRAY));
                    
                    if self.loop_enabled {
                        ui.label(RichText::new("â€¢ Loop").size(11.0).color(Color32::from_rgb(100, 150, 200)));
                    }
                });
            });
    }
}

fn format_duration(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
