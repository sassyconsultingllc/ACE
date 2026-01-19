#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! Audio Player - MP3, WAV, FLAC, OGG playback with visualization

use crate::file_handler::{AudioContent, FileContent, OpenFile};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
pub struct AudioViewer {
    is_playing: bool,
    current_position: f64,
    volume: f32,
    is_muted: bool,
    show_waveform: bool,
    repeat_mode: RepeatMode,
}

#[derive(Clone, Copy, PartialEq)]
enum RepeatMode {
    None,
    One,
    All,
}

impl AudioViewer {
    pub fn new() -> Self {
        Self {
            is_playing: false,
            current_position: 0.0,
            volume: 0.8,
            is_muted: false,
            show_waveform: true,
            repeat_mode: RepeatMode::None,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        if let FileContent::Audio(audio) = &file.content {
            egui::Frame::none()
                .fill(Color32::from_rgb(25, 28, 35))
                .inner_margin(20.0)
                .show(ui, |ui| {
                    self.render_album_art(ui, audio, zoom);
                    ui.add_space(20.0);
                    self.render_track_info(ui, audio);
                    ui.add_space(20.0);
                    
                    if self.show_waveform {
                        self.render_waveform(ui, audio, zoom);
                        ui.add_space(10.0);
                    }
                    
                    self.render_progress(ui, audio);
                    ui.add_space(15.0);
                    self.render_controls(ui);
                    ui.add_space(15.0);
                    self.render_volume(ui);
                    ui.add_space(20.0);
                    self.render_audio_info(ui, audio);
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Not an audio file");
            });
        }
    }
    
    fn render_album_art(&self, ui: &mut egui::Ui, audio: &AudioContent, zoom: f32) {
        let art_size = 200.0 * zoom;
        
        ui.vertical_centered(|ui| {
            if let Some(_cover_data) = &audio.cover_art {
                // TODO: Render cover art from data
                egui::Frame::none()
                    .fill(Color32::from_rgb(40, 45, 55))
                    .rounding(8.0)
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::splat(art_size));
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("🎵").size(80.0 * zoom));
                        });
                    });
            } else {
                // Placeholder album art
                egui::Frame::none()
                    .fill(Color32::from_rgb(40, 45, 55))
                    .rounding(8.0)
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::splat(art_size));
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("🎵").size(80.0 * zoom));
                        });
                    });
            }
        });
    }
    
    fn render_track_info(&self, ui: &mut egui::Ui, audio: &AudioContent) {
        ui.vertical_centered(|ui| {
            let title = audio.title.as_deref()
                .unwrap_or("Unknown Track");
            
            let artist = audio.artist.as_deref()
                .unwrap_or("Unknown Artist");
            
            let album = audio.album.as_deref()
                .unwrap_or("");
            
            ui.label(RichText::new(title).size(24.0).strong());
            ui.label(RichText::new(artist).size(16.0).color(Color32::GRAY));
            
            if !album.is_empty() {
                ui.label(RichText::new(album).size(14.0).color(Color32::from_rgb(100, 100, 120)));
            }
        });
    }
    
    fn render_waveform(&mut self, ui: &mut egui::Ui, audio: &AudioContent, zoom: f32) {
        let width = ui.available_width();
        let height = 60.0 * zoom;
        
        let (response, painter) = ui.allocate_painter(Vec2::new(width, height), egui::Sense::click());
        
        // Background
        painter.rect_filled(response.rect, 4.0, Color32::from_rgb(35, 40, 50));
        
        // Generate simple waveform visualization
        let bar_count = 100;
        let bar_width = width / bar_count as f32;
        let progress_ratio = self.current_position / audio.duration_secs.max(1.0);
        
        for i in 0..bar_count {
            let x = response.rect.left() + i as f32 * bar_width;
            
            // Fake waveform using noise pattern
            let amplitude = ((i as f32 * 0.1).sin().abs() * 0.5 + 
                            (i as f32 * 0.23).cos().abs() * 0.3 + 
                            (i as f32 * 0.07).sin().abs() * 0.2) * height * 0.8;
            
            let is_played = (i as f32 / bar_count as f32) < progress_ratio as f32;
            let color = if is_played {
                Color32::from_rgb(100, 180, 255)
            } else {
                Color32::from_rgb(80, 85, 100)
            };
            
            let center_y = response.rect.center().y;
            let rect = Rect::from_min_max(
                Pos2::new(x, center_y - amplitude / 2.0),
                Pos2::new(x + bar_width - 1.0, center_y + amplitude / 2.0),
            );
            
            painter.rect_filled(rect, 1.0, color);
        }
        
        // Click to seek
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let ratio = (pos.x - response.rect.left()) / response.rect.width();
                self.current_position = ratio as f64 * audio.duration_secs;
            }
        }
    }
    
    fn render_progress(&mut self, ui: &mut egui::Ui, audio: &AudioContent) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format_duration(self.current_position)).monospace());
            
            let mut progress = (self.current_position / audio.duration_secs.max(1.0)) as f32;
            
            if ui.add(egui::Slider::new(&mut progress, 0.0..=1.0)
                .show_value(false)
                .trailing_fill(true)
            ).changed() {
                self.current_position = progress as f64 * audio.duration_secs;
            }
            
            ui.label(RichText::new(format_duration(audio.duration_secs)).monospace());
        });
    }
    
    fn render_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 200.0) / 2.0);
            
            // Repeat button
            let repeat_icon = match self.repeat_mode {
                RepeatMode::None => "🔁",
                RepeatMode::One => "🔂",
                RepeatMode::All => "🔁",
            };
            let repeat_color = if self.repeat_mode != RepeatMode::None {
                Color32::from_rgb(100, 180, 255)
            } else {
                Color32::GRAY
            };
            
            if ui.add(egui::Button::new(RichText::new(repeat_icon).color(repeat_color))).clicked() {
                self.repeat_mode = match self.repeat_mode {
                    RepeatMode::None => RepeatMode::One,
                    RepeatMode::One => RepeatMode::All,
                    RepeatMode::All => RepeatMode::None,
                };
            }
            
            // Previous
            if ui.button(RichText::new("⏮").size(20.0)).clicked() {
                self.current_position = 0.0;
            }
            
            // Play/Pause
            let play_icon = if self.is_playing { "⏸" } else { "▶" };
            if ui.add(egui::Button::new(RichText::new(play_icon).size(32.0))
                .min_size(Vec2::splat(50.0))
            ).clicked() {
                self.is_playing = !self.is_playing;
            }
            
            // Next
            if ui.button(RichText::new("⏭").size(20.0)).clicked() {
                // Next track
            }
            
            // Shuffle
            if ui.button(RichText::new("🔀").color(Color32::GRAY)).clicked() {
                // Toggle shuffle
            }
        });
    }
    
    fn render_volume(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 200.0) / 2.0);
            
            // Mute button
            let volume_icon = if self.is_muted || self.volume == 0.0 {
                "🔇"
            } else if self.volume < 0.5 {
                "🔉"
            } else {
                "🔊"
            };
            
            if ui.button(volume_icon).clicked() {
                self.is_muted = !self.is_muted;
            }
            
            // Volume slider
            ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0)
                .show_value(false));
            
            ui.label(format!("{}%", (self.volume * 100.0) as u32));
        });
    }
    
    fn render_audio_info(&self, ui: &mut egui::Ui, audio: &AudioContent) {
        ui.separator();
        
        egui::Grid::new("audio_info_grid")
            .num_columns(4)
            .spacing([20.0, 5.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Format").small().color(Color32::GRAY));
                ui.label(&audio.format);
                
                ui.label(RichText::new("Sample Rate").small().color(Color32::GRAY));
                ui.label(format!("{} Hz", audio.sample_rate));
                ui.end_row();
                
                ui.label(RichText::new("Channels").small().color(Color32::GRAY));
                ui.label(format!("{}", audio.channels));
                
                ui.label(RichText::new("Bit Depth").small().color(Color32::GRAY));
                ui.label(format!("{} bit", audio.bit_depth));
                ui.end_row();
                
                if let Some(bitrate) = audio.bitrate {
                    ui.label(RichText::new("Bitrate").small().color(Color32::GRAY));
                    ui.label(format!("{} kbps", bitrate / 1000));
                }
                
                if let Some(year) = audio.year {
                    ui.label(RichText::new("Year").small().color(Color32::GRAY));
                    ui.label(format!("{}", year));
                }
                ui.end_row();
                
                if let Some(genre) = &audio.genre {
                    ui.label(RichText::new("Genre").small().color(Color32::GRAY));
                    ui.label(genre);
                }
            });
    }
}

fn format_duration(secs: f64) -> String {
    let total_secs = secs as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}
