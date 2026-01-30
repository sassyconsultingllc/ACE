#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! Video Player - MP4, WebM, AVI, MKV playback

use crate::file_handler::{VideoContent, FileContent, OpenFile};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};

pub struct VideoViewer {
    is_playing: bool,
    current_position: f64,
    volume: f32,
    is_muted: bool,
    is_fullscreen: bool,
    show_controls: bool,
    playback_rate: f32,
}

impl VideoViewer {
    pub fn new() -> Self {
        Self {
            is_playing: false,
            current_position: 0.0,
            volume: 0.8,
            is_muted: false,
            is_fullscreen: false,
            show_controls: true,
            playback_rate: 1.0,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        if let FileContent::Video(video) = &file.content {
            egui::Frame::none()
                .fill(Color32::from_rgb(0, 0, 0))
                .inner_margin(0.0)
                .show(ui, |ui| {
                    self.render_video_frame(ui, video, zoom);
                    
                    if self.show_controls {
                        self.render_controls_overlay(ui, video);
                    }
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Not a video file");
            });
        }
    }
    
    fn render_video_frame(&mut self, ui: &mut egui::Ui, video: &VideoContent, zoom: f32) {
        let available = ui.available_size();
        let aspect_ratio = video.width as f32 / video.height.max(1) as f32;
        
        let (width, height) = if available.x / available.y > aspect_ratio {
            (available.y * aspect_ratio, available.y)
        } else {
            (available.x, available.x / aspect_ratio)
        };
        
        let rect = Rect::from_center_size(
            ui.available_rect_before_wrap().center(),
            Vec2::new(width * zoom, height * zoom),
        );
        
        // Placeholder for video frame
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(20, 20, 20));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}x{} @ {}fps", video.width, video.height, video.frame_rate),
            FontId::proportional(14.0),
            Color32::GRAY,
        );
    }
    
    fn render_controls_overlay(&mut self, ui: &mut egui::Ui, video: &VideoContent) {
        let rect = ui.available_rect_before_wrap();
        let controls_rect = Rect::from_min_size(
            Pos2::new(rect.left(), rect.bottom() - 60.0),
            Vec2::new(rect.width(), 60.0),
        );
        
        // Semi-transparent background
        ui.painter().rect_filled(
            controls_rect,
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        );
        
        ui.allocate_ui_at_rect(controls_rect.shrink(10.0), |ui| {
            ui.horizontal(|ui| {
                // Play/Pause button
                let play_text = if self.is_playing { "⏸" } else { ">" };
                if ui.button(RichText::new(play_text).size(20.0)).clicked() {
                    self.is_playing = !self.is_playing;
                }
                
                // Progress bar
                let progress = self.current_position / video.duration.max(0.001);
                ui.add(egui::ProgressBar::new(progress as f32).desired_width(200.0));
                
                // Time display
                let current = format_time(self.current_position);
                let total = format_time(video.duration);
                ui.label(format!("{} / {}", current, total));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Fullscreen toggle
                    if ui.button("").clicked() {
                        self.is_fullscreen = !self.is_fullscreen;
                    }
                    
                    // Volume
                    let vol_icon = if self.is_muted { "" } else { "" };
                    if ui.button(vol_icon).clicked() {
                        self.is_muted = !self.is_muted;
                    }
                });
            });
        });
    }
}

fn format_time(seconds: f64) -> String {
    let mins = (seconds / 60.0) as u32;
    let secs = (seconds % 60.0) as u32;
    format!("{:02}:{:02}", mins, secs)
}
