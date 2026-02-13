#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! Video Player - MP4, WebM, AVI, MKV playback
//!
//! Displays video metadata, codec information, and provides a rich
//! player UI with controls overlay. Frame rendering is placeholder-based
//! since pure-Rust video decoding is not yet implemented.

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
    show_info_overlay: bool,
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
            show_info_overlay: false,
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

                    if self.show_info_overlay {
                        self.render_info_overlay(ui, video);
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
        let vid_w = video.width.max(1) as f32;
        let vid_h = video.height.max(1) as f32;
        let aspect_ratio = vid_w / vid_h;

        let (width, height) = if aspect_ratio > 0.0 && aspect_ratio.is_finite() {
            if available.x / available.y > aspect_ratio {
                (available.y * aspect_ratio, available.y)
            } else {
                (available.x, available.x / aspect_ratio)
            }
        } else {
            (available.x.min(640.0), available.y.min(360.0))
        };

        let rect = Rect::from_center_size(
            ui.available_rect_before_wrap().center(),
            Vec2::new(width * zoom, height * zoom),
        );

        // Dark video area background
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(15, 15, 20));

        // Film strip border effect
        let border_color = Color32::from_rgb(45, 50, 60);
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(2.0, border_color));

        // Centered play button icon (large)
        let center = rect.center();
        if !self.is_playing {
            // Draw large circular play button
            let btn_radius = 35.0 * zoom;
            ui.painter().circle_filled(center, btn_radius, Color32::from_rgba_unmultiplied(255, 255, 255, 60));
            ui.painter().circle_stroke(center, btn_radius, Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 120)));

            // Play triangle
            let tri_size = 20.0 * zoom;
            let offset = 4.0 * zoom; // slight right offset for visual centering
            let points = vec![
                Pos2::new(center.x - tri_size * 0.4 + offset, center.y - tri_size * 0.6),
                Pos2::new(center.x + tri_size * 0.6 + offset, center.y),
                Pos2::new(center.x - tri_size * 0.4 + offset, center.y + tri_size * 0.6),
            ];
            ui.painter().add(egui::Shape::convex_polygon(
                points,
                Color32::WHITE,
                Stroke::NONE,
            ));
        }

        // Resolution badge (top-right corner)
        if video.width > 0 && video.height > 0 {
            let res_label = if video.height >= 2160 { "4K" }
                else if video.height >= 1440 { "1440p" }
                else if video.height >= 1080 { "1080p" }
                else if video.height >= 720 { "720p" }
                else if video.height >= 480 { "480p" }
                else { "SD" };

            let badge_pos = Pos2::new(rect.right() - 60.0, rect.top() + 10.0);
            let badge_rect = Rect::from_min_size(badge_pos, Vec2::new(50.0, 22.0));
            ui.painter().rect_filled(badge_rect, 4.0, Color32::from_rgba_unmultiplied(0, 0, 0, 180));
            ui.painter().text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                res_label,
                FontId::proportional(12.0),
                Color32::from_rgb(100, 180, 255),
            );
        }

        // Video info text (center)
        let info_text = if video.width > 0 {
            let fps_str = if video.frame_rate > 0.0 {
                format!(" @ {:.1}fps", video.frame_rate)
            } else {
                String::new()
            };
            format!("{}x{}{}", video.width, video.height, fps_str)
        } else {
            "Unknown Resolution".to_string()
        };

        // Sub-text with codec info
        let codec_text = match (&video.video_codec, &video.audio_codec) {
            (Some(vc), Some(ac)) => format!("{} + {}", vc, ac),
            (Some(vc), None) => vc.clone(),
            (None, Some(ac)) => format!("Audio: {}", ac),
            (None, None) => video.format.clone(),
        };

        let text_y = center.y + 50.0 * zoom;
        ui.painter().text(
            Pos2::new(center.x, text_y),
            egui::Align2::CENTER_CENTER,
            &info_text,
            FontId::proportional(14.0),
            Color32::from_rgb(140, 140, 160),
        );
        ui.painter().text(
            Pos2::new(center.x, text_y + 20.0),
            egui::Align2::CENTER_CENTER,
            &codec_text,
            FontId::proportional(12.0),
            Color32::from_rgb(100, 100, 120),
        );

        // Film grain effect (subtle dots pattern)
        let grain_color = Color32::from_rgba_unmultiplied(255, 255, 255, 5);
        let grain_spacing = 12.0;
        let mut gx = rect.left();
        while gx < rect.right() {
            let mut gy = rect.top();
            while gy < rect.bottom() {
                // Pseudo-random visibility based on position
                let hash = ((gx as u32).wrapping_mul(2654435761)).wrapping_add((gy as u32).wrapping_mul(2246822519));
                if hash % 3 == 0 {
                    ui.painter().circle_filled(Pos2::new(gx, gy), 0.5, grain_color);
                }
                gy += grain_spacing;
            }
            gx += grain_spacing;
        }

        // Click to toggle play
        let click_rect = ui.allocate_rect(rect, egui::Sense::click());
        if click_rect.clicked() {
            self.is_playing = !self.is_playing;
        }
    }

    fn render_controls_overlay(&mut self, ui: &mut egui::Ui, video: &VideoContent) {
        let rect = ui.available_rect_before_wrap();
        let controls_height = 70.0;
        let controls_rect = Rect::from_min_size(
            Pos2::new(rect.left(), rect.bottom() - controls_height),
            Vec2::new(rect.width(), controls_height),
        );

        // Semi-transparent gradient background
        ui.painter().rect_filled(
            controls_rect,
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 200),
        );

        // Progress bar (thin line across top of controls)
        let progress = if video.duration > 0.001 {
            (self.current_position / video.duration) as f32
        } else {
            0.0
        };
        let progress_rect = Rect::from_min_size(
            Pos2::new(controls_rect.left(), controls_rect.top()),
            Vec2::new(controls_rect.width(), 3.0),
        );
        ui.painter().rect_filled(progress_rect, 0.0, Color32::from_rgb(50, 55, 65));
        let filled_rect = Rect::from_min_size(
            progress_rect.min,
            Vec2::new(progress_rect.width() * progress, 3.0),
        );
        ui.painter().rect_filled(filled_rect, 0.0, Color32::from_rgb(100, 180, 255));

        // Progress dot
        let dot_x = controls_rect.left() + progress * controls_rect.width();
        ui.painter().circle_filled(
            Pos2::new(dot_x, controls_rect.top() + 1.5),
            5.0,
            Color32::from_rgb(100, 180, 255),
        );

        ui.allocate_ui_at_rect(controls_rect.shrink2(Vec2::new(10.0, 8.0)), |ui| {
            ui.add_space(6.0); // Space below progress bar

            ui.horizontal(|ui| {
                // Play/Pause button
                let play_text = if self.is_playing { "||" } else { ">" };
                if ui.button(RichText::new(play_text).size(20.0)).clicked() {
                    self.is_playing = !self.is_playing;
                }

                ui.add_space(4.0);

                // Time display
                let current = format_time(self.current_position);
                let total = format_time(video.duration);
                ui.label(RichText::new(format!("{} / {}", current, total))
                    .monospace()
                    .color(Color32::from_rgb(200, 200, 210)));

                ui.add_space(8.0);

                // Playback rate
                let rate_text = format!("{}x", self.playback_rate);
                if ui.button(RichText::new(&rate_text).size(12.0).color(Color32::from_rgb(180, 180, 200))).clicked() {
                    self.playback_rate = match self.playback_rate as u32 {
                        0 => 1.0,
                        1 => 1.5,
                        _ => {
                            if self.playback_rate >= 2.0 { 0.5 } else { 2.0 }
                        }
                    };
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Fullscreen toggle
                    let fs_icon = if self.is_fullscreen { "[ ]" } else { "[#]" };
                    if ui.button(RichText::new(fs_icon).size(16.0)).clicked() {
                        self.is_fullscreen = !self.is_fullscreen;
                    }

                    // Info toggle
                    let info_color = if self.show_info_overlay {
                        Color32::from_rgb(100, 180, 255)
                    } else {
                        Color32::from_rgb(180, 180, 200)
                    };
                    if ui.button(RichText::new("i").size(16.0).color(info_color)).clicked() {
                        self.show_info_overlay = !self.show_info_overlay;
                    }

                    // Volume
                    let vol_icon = if self.is_muted { "Mute" } else { "Vol" };
                    if ui.button(vol_icon).clicked() {
                        self.is_muted = !self.is_muted;
                    }
                });
            });
        });
    }

    /// Render a translucent info overlay with detailed video metadata
    fn render_info_overlay(&self, ui: &mut egui::Ui, video: &VideoContent) {
        let rect = ui.available_rect_before_wrap();
        let overlay_width = 280.0;
        let overlay_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 10.0, rect.top() + 10.0),
            Vec2::new(overlay_width, 200.0),
        );

        ui.painter().rect_filled(overlay_rect, 6.0, Color32::from_rgba_unmultiplied(0, 0, 0, 200));
        ui.painter().rect_stroke(overlay_rect, 6.0, Stroke::new(1.0, Color32::from_rgb(60, 65, 75)));

        ui.allocate_ui_at_rect(overlay_rect.shrink(12.0), |ui| {
            ui.label(RichText::new("Video Information").size(14.0).strong().color(Color32::from_rgb(100, 180, 255)));
            ui.add_space(8.0);

            let label_color = Color32::from_rgb(130, 130, 150);
            let value_color = Color32::from_rgb(220, 220, 230);

            egui::Grid::new("video_info_overlay_grid")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Format").small().color(label_color));
                    ui.label(RichText::new(&video.format).small().color(value_color));
                    ui.end_row();

                    if video.width > 0 {
                        ui.label(RichText::new("Resolution").small().color(label_color));
                        ui.label(RichText::new(format!("{}x{}", video.width, video.height)).small().color(value_color));
                        ui.end_row();
                    }

                    if video.frame_rate > 0.0 {
                        ui.label(RichText::new("Frame Rate").small().color(label_color));
                        ui.label(RichText::new(format!("{:.2} fps", video.frame_rate)).small().color(value_color));
                        ui.end_row();
                    }

                    ui.label(RichText::new("Duration").small().color(label_color));
                    ui.label(RichText::new(format_time(video.duration)).small().color(value_color));
                    ui.end_row();

                    if let Some(vc) = &video.video_codec {
                        ui.label(RichText::new("Video Codec").small().color(label_color));
                        ui.label(RichText::new(vc).small().color(value_color));
                        ui.end_row();
                    }

                    if let Some(ac) = &video.audio_codec {
                        ui.label(RichText::new("Audio Codec").small().color(label_color));
                        ui.label(RichText::new(ac).small().color(value_color));
                        ui.end_row();
                    }

                    if let Some(bitrate) = video.bitrate {
                        ui.label(RichText::new("Bitrate").small().color(label_color));
                        if bitrate > 1_000_000 {
                            ui.label(RichText::new(format!("{:.1} Mbps", bitrate as f64 / 1_000_000.0)).small().color(value_color));
                        } else {
                            ui.label(RichText::new(format!("{} kbps", bitrate / 1000)).small().color(value_color));
                        }
                        ui.end_row();
                    }

                    ui.label(RichText::new("Speed").small().color(label_color));
                    ui.label(RichText::new(format!("{}x", self.playback_rate)).small().color(value_color));
                    ui.end_row();
                });
        });
    }
}

fn format_time(seconds: f64) -> String {
    let total = seconds as u64;
    let hours = total / 3600;
    let mins = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}
