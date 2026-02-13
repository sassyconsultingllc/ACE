#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! Audio Player - MP3, WAV, FLAC, OGG playback with visualization
//!
//! Uses rodio for real audio playback and symphonia-decoded waveform data
//! for accurate visualization. Supports play/pause, seek, volume, mute,
//! and repeat modes.

use crate::file_handler::{AudioContent, FileContent, OpenFile};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use image::GenericImageView;
use rodio::Source;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Shared state between the UI thread and the audio playback thread
struct PlaybackState {
    /// rodio output stream — must be kept alive for audio to play
    _stream: rodio::OutputStream,
    /// Stream handle for creating sinks
    _stream_handle: rodio::OutputStreamHandle,
    /// The actual audio sink controlling playback
    sink: rodio::Sink,
    /// When playback started (for tracking position)
    started_at: Option<Instant>,
    /// Position offset when playback was paused/seeked
    position_offset: f64,
    /// Path of the currently loaded file
    loaded_path: PathBuf,
}

pub struct AudioViewer {
    is_playing: bool,
    current_position: f64,
    volume: f32,
    is_muted: bool,
    show_waveform: bool,
    repeat_mode: RepeatMode,
    /// Active playback state (None = no audio loaded)
    playback: Option<PlaybackState>,
    /// Path of the file we last tried to load
    last_loaded_path: Option<PathBuf>,
    /// Error message if playback failed to initialize
    playback_error: Option<String>,
    /// Cached album art texture handle
    album_art_texture: Option<egui::TextureHandle>,
    /// Path the cached texture was loaded from
    album_art_path: Option<PathBuf>,
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
            playback: None,
            last_loaded_path: None,
            playback_error: None,
            album_art_texture: None,
            album_art_path: None,
        }
    }

    /// Initialize rodio playback for the given file
    fn init_playback(&mut self, file_path: &std::path::Path) {
        // Don't re-initialize if already loaded for this file
        if let Some(pb) = &self.playback {
            if pb.loaded_path == file_path {
                return;
            }
        }

        self.playback = None;
        self.playback_error = None;
        self.is_playing = false;
        self.current_position = 0.0;

        // Create output stream
        let (stream, stream_handle) = match rodio::OutputStream::try_default() {
            Ok(s) => s,
            Err(e) => {
                self.playback_error = Some(format!("Audio output error: {}", e));
                return;
            }
        };

        let sink = match rodio::Sink::try_new(&stream_handle) {
            Ok(s) => s,
            Err(e) => {
                self.playback_error = Some(format!("Sink creation error: {}", e));
                return;
            }
        };

        // Set initial volume
        sink.set_volume(if self.is_muted { 0.0 } else { self.volume });
        sink.pause(); // Start paused

        // Try to load the audio file into the sink
        match std::fs::File::open(file_path) {
            Ok(file) => {
                let buf_reader = std::io::BufReader::new(file);
                match rodio::Decoder::new(buf_reader) {
                    Ok(source) => {
                        sink.append(source);
                        sink.pause(); // Keep paused until user hits play
                    }
                    Err(e) => {
                        self.playback_error = Some(format!("Decoder error: {}", e));
                        return;
                    }
                }
            }
            Err(e) => {
                self.playback_error = Some(format!("File open error: {}", e));
                return;
            }
        }

        self.playback = Some(PlaybackState {
            _stream: stream,
            _stream_handle: stream_handle,
            sink,
            started_at: None,
            position_offset: 0.0,
            loaded_path: file_path.to_path_buf(),
        });
        self.last_loaded_path = Some(file_path.to_path_buf());
    }

    /// Toggle play/pause
    fn toggle_play(&mut self) {
        if let Some(pb) = &mut self.playback {
            if self.is_playing {
                // Pause: save current position
                if let Some(started) = pb.started_at {
                    pb.position_offset += started.elapsed().as_secs_f64();
                }
                pb.started_at = None;
                pb.sink.pause();
                self.is_playing = false;
            } else {
                // Play
                pb.started_at = Some(Instant::now());
                pb.sink.play();
                self.is_playing = true;
            }
        }
    }

    /// Get current playback position in seconds
    fn get_position(&self) -> f64 {
        if let Some(pb) = &self.playback {
            let base = pb.position_offset;
            if let Some(started) = pb.started_at {
                base + started.elapsed().as_secs_f64()
            } else {
                base
            }
        } else {
            self.current_position
        }
    }

    /// Seek to a specific position (0.0 to duration)
    fn seek_to(&mut self, position: f64, file_path: &std::path::Path, duration: f64) {
        let pos = position.clamp(0.0, duration);

        // rodio doesn't support native seeking, so we need to reload and skip
        // For now, track position for UI, and reload from start on actual seek
        if let Some(pb) = &mut self.playback {
            let was_playing = self.is_playing;
            pb.sink.stop();

            // Recreate sink
            if let Ok(new_sink) = rodio::Sink::try_new(&pb._stream_handle) {
                new_sink.set_volume(if self.is_muted { 0.0 } else { self.volume });

                if let Ok(file) = std::fs::File::open(file_path) {
                    let buf_reader = std::io::BufReader::new(file);
                    if let Ok(source) = rodio::Decoder::new(buf_reader) {
                        // Skip ahead by consuming samples
                        let skip_samples = (pos * 44100.0) as usize; // approximate
                        let skipped = source.skip_duration(std::time::Duration::from_secs_f64(pos));
                        new_sink.append(skipped);

                        if !was_playing {
                            new_sink.pause();
                        }
                    }
                }

                pb.sink = new_sink;
                pb.position_offset = pos;
                pb.started_at = if was_playing { Some(Instant::now()) } else { None };
                self.is_playing = was_playing;
            }
        }

        self.current_position = pos;
    }

    /// Update volume on the sink
    fn update_volume(&self) {
        if let Some(pb) = &self.playback {
            let vol = if self.is_muted { 0.0 } else { self.volume };
            pb.sink.set_volume(vol);
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        if let FileContent::Audio(audio) = &file.content {
            // Initialize playback if not done yet
            self.init_playback(&file.path);

            // Update current position from playback
            if self.is_playing {
                self.current_position = self.get_position();

                // Check if playback finished
                if let Some(pb) = &self.playback {
                    if pb.sink.empty() && self.current_position > 0.5 {
                        match self.repeat_mode {
                            RepeatMode::One => {
                                let path = file.path.clone();
                                self.seek_to(0.0, &path, audio.duration_secs);
                            }
                            RepeatMode::All | RepeatMode::None => {
                                self.is_playing = false;
                                self.current_position = audio.duration_secs;
                            }
                        }
                    }
                }

                // Clamp position to duration
                if self.current_position > audio.duration_secs {
                    self.current_position = audio.duration_secs;
                }

                // Request repaint while playing to update progress
                ui.ctx().request_repaint();
            }

            egui::Frame::none()
                .fill(Color32::from_rgb(25, 28, 35))
                .inner_margin(20.0)
                .show(ui, |ui| {
                    self.render_album_art(ui, audio, zoom, &file.path);
                    ui.add_space(20.0);
                    self.render_track_info(ui, audio);
                    ui.add_space(20.0);

                    if self.show_waveform {
                        let path = file.path.clone();
                        let dur = audio.duration_secs;
                        self.render_waveform(ui, audio, zoom, &path, dur);
                        ui.add_space(10.0);
                    }

                    let path = file.path.clone();
                    let dur = audio.duration_secs;
                    self.render_progress(ui, audio, &path, dur);
                    ui.add_space(15.0);
                    self.render_controls(ui, &file.path.clone(), audio.duration_secs);
                    ui.add_space(15.0);
                    self.render_volume(ui);

                    // Playback error message
                    if let Some(err) = &self.playback_error {
                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("⚠ {}", err))
                            .color(Color32::from_rgb(255, 180, 80))
                            .small());
                    }

                    ui.add_space(20.0);
                    self.render_audio_info(ui, audio);
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Not an audio file");
            });
        }
    }

    fn render_album_art(&mut self, ui: &mut egui::Ui, audio: &AudioContent, zoom: f32, file_path: &std::path::Path) {
        let art_size = 200.0 * zoom;

        ui.vertical_centered(|ui| {
            if let Some(cover_data) = &audio.cover_art {
                // Cache the texture to avoid re-uploading every frame
                let need_reload = match &self.album_art_path {
                    Some(p) => p != file_path,
                    None => true,
                };

                if need_reload || self.album_art_texture.is_none() {
                    match image::load_from_memory(cover_data) {
                        Ok(img) => {
                            let rgba = img.to_rgba8();
                            let size = [rgba.width() as _, rgba.height() as _];
                            let pixels = rgba.as_flat_samples();

                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                size,
                                pixels.as_slice(),
                            );

                            self.album_art_texture = Some(ui.ctx().load_texture(
                                "album_art",
                                color_image,
                                egui::TextureOptions::LINEAR,
                            ));
                            self.album_art_path = Some(file_path.to_path_buf());
                        }
                        Err(_) => {
                            self.album_art_texture = None;
                            self.album_art_path = Some(file_path.to_path_buf());
                        }
                    }
                }

                if let Some(texture) = &self.album_art_texture {
                    egui::Frame::none()
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.add(egui::Image::new(texture).max_size(Vec2::splat(art_size)));
                        });
                } else {
                    Self::render_placeholder_art(ui, art_size, zoom);
                }
            } else {
                Self::render_placeholder_art(ui, art_size, zoom);
            }
        });
    }

    fn render_placeholder_art(ui: &mut egui::Ui, art_size: f32, zoom: f32) {
        egui::Frame::none()
            .fill(Color32::from_rgb(40, 45, 55))
            .rounding(8.0)
            .show(ui, |ui| {
                ui.set_min_size(Vec2::splat(art_size));
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("♫").size(80.0 * zoom).color(Color32::from_rgb(100, 110, 130)));
                });
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
                let album_text = if let Some(year) = audio.year {
                    format!("{} ({})", album, year)
                } else {
                    album.to_string()
                };
                ui.label(RichText::new(album_text).size(14.0).color(Color32::from_rgb(100, 100, 120)));
            }

            if let Some(track_num) = audio.track {
                ui.label(RichText::new(format!("Track {}", track_num))
                    .size(12.0).color(Color32::from_rgb(80, 80, 100)));
            }
        });
    }

    fn render_waveform(&mut self, ui: &mut egui::Ui, audio: &AudioContent, zoom: f32, file_path: &std::path::Path, duration: f64) {
        let width = ui.available_width();
        let height = 60.0 * zoom;

        let (response, painter) = ui.allocate_painter(Vec2::new(width, height), egui::Sense::click());

        // Background
        painter.rect_filled(response.rect, 4.0, Color32::from_rgb(35, 40, 50));

        let has_real_waveform = !audio.waveform_data.is_empty();
        let bar_count = if has_real_waveform { audio.waveform_data.len() } else { 100 };
        let bar_width = width / bar_count as f32;
        let progress_ratio = self.current_position / duration.max(1.0);

        for i in 0..bar_count {
            let x = response.rect.left() + i as f32 * bar_width;

            // Use real waveform data if available, otherwise generate fake
            let amplitude = if has_real_waveform {
                audio.waveform_data[i].min(1.0) * height * 0.85
            } else {
                ((i as f32 * 0.1).sin().abs() * 0.5 +
                 (i as f32 * 0.23).cos().abs() * 0.3 +
                 (i as f32 * 0.07).sin().abs() * 0.2) * height * 0.8
            };

            let is_played = (i as f32 / bar_count as f32) < progress_ratio as f32;
            let color = if is_played {
                Color32::from_rgb(100, 180, 255)
            } else {
                Color32::from_rgb(80, 85, 100)
            };

            let center_y = response.rect.center().y;
            let half_amp = amplitude.max(1.0) / 2.0;
            let rect = Rect::from_min_max(
                Pos2::new(x, center_y - half_amp),
                Pos2::new(x + bar_width - 1.0, center_y + half_amp),
            );

            painter.rect_filled(rect, 1.0, color);
        }

        // Playhead line
        let playhead_x = response.rect.left() + (progress_ratio as f32) * width;
        painter.line_segment(
            [Pos2::new(playhead_x, response.rect.top()), Pos2::new(playhead_x, response.rect.bottom())],
            Stroke::new(2.0, Color32::from_rgb(255, 255, 255)),
        );

        // Click to seek
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let ratio = (pos.x - response.rect.left()) / response.rect.width();
                let new_pos = ratio as f64 * duration;
                let path = file_path.to_path_buf();
                self.seek_to(new_pos, &path, duration);
            }
        }
    }

    fn render_progress(&mut self, ui: &mut egui::Ui, audio: &AudioContent, file_path: &std::path::Path, duration: f64) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format_duration(self.current_position)).monospace());

            let mut progress = (self.current_position / duration.max(1.0)) as f32;

            if ui.add(egui::Slider::new(&mut progress, 0.0..=1.0)
                .show_value(false)
                .trailing_fill(true)
            ).changed() {
                let new_pos = progress as f64 * duration;
                let path = file_path.to_path_buf();
                self.seek_to(new_pos, &path, duration);
            }

            ui.label(RichText::new(format_duration(duration)).monospace());
        });
    }

    fn render_controls(&mut self, ui: &mut egui::Ui, file_path: &std::path::Path, duration: f64) {
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 250.0).max(0.0) / 2.0);

            // Repeat button
            let repeat_text = match self.repeat_mode {
                RepeatMode::None => "Repeat: Off",
                RepeatMode::One => "Repeat: One",
                RepeatMode::All => "Repeat: All",
            };
            let repeat_color = if self.repeat_mode != RepeatMode::None {
                Color32::from_rgb(100, 180, 255)
            } else {
                Color32::GRAY
            };

            if ui.add(egui::Button::new(RichText::new(repeat_text).size(11.0).color(repeat_color))).clicked() {
                self.repeat_mode = match self.repeat_mode {
                    RepeatMode::None => RepeatMode::One,
                    RepeatMode::One => RepeatMode::All,
                    RepeatMode::All => RepeatMode::None,
                };
            }

            ui.add_space(8.0);

            // Previous (restart)
            if ui.button(RichText::new("⏮").size(20.0)).clicked() {
                let path = file_path.to_path_buf();
                self.seek_to(0.0, &path, duration);
            }

            // Play/Pause
            let play_icon = if self.is_playing { "⏸" } else { "▶" };
            if ui.add(egui::Button::new(RichText::new(play_icon).size(32.0))
                .min_size(Vec2::splat(50.0))
            ).clicked() {
                self.toggle_play();
            }

            // Skip forward 10s
            if ui.button(RichText::new("⏭").size(20.0)).clicked() {
                let new_pos = (self.current_position + 10.0).min(duration);
                let path = file_path.to_path_buf();
                self.seek_to(new_pos, &path, duration);
            }

            ui.add_space(8.0);

            // Waveform toggle
            let wf_color = if self.show_waveform {
                Color32::from_rgb(100, 180, 255)
            } else {
                Color32::GRAY
            };
            if ui.add(egui::Button::new(RichText::new("Wave").size(11.0).color(wf_color))).clicked() {
                self.show_waveform = !self.show_waveform;
            }
        });
    }

    fn render_volume(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 220.0).max(0.0) / 2.0);

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
                self.update_volume();
            }

            // Volume slider
            let old_vol = self.volume;
            ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0)
                .show_value(false));

            if (self.volume - old_vol).abs() > 0.001 {
                self.update_volume();
            }

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
                let ch_label = match audio.channels {
                    1 => "Mono".to_string(),
                    2 => "Stereo".to_string(),
                    n => format!("{} ch", n),
                };
                ui.label(ch_label);

                ui.label(RichText::new("Bit Depth").small().color(Color32::GRAY));
                ui.label(format!("{} bit", audio.bit_depth));
                ui.end_row();

                if let Some(bitrate) = audio.bitrate {
                    ui.label(RichText::new("Bitrate").small().color(Color32::GRAY));
                    if bitrate >= 1000 {
                        ui.label(format!("{} kbps", bitrate / 1000));
                    } else {
                        ui.label(format!("{} bps", bitrate));
                    }
                }

                ui.label(RichText::new("Duration").small().color(Color32::GRAY));
                ui.label(format_duration(audio.duration_secs));
                ui.end_row();

                if let Some(year) = audio.year {
                    ui.label(RichText::new("Year").small().color(Color32::GRAY));
                    ui.label(format!("{}", year));
                }

                if let Some(genre) = &audio.genre {
                    ui.label(RichText::new("Genre").small().color(Color32::GRAY));
                    ui.label(genre);
                }
                ui.end_row();

                // Playback status
                ui.label(RichText::new("Playback").small().color(Color32::GRAY));
                let status = if self.playback.is_some() {
                    if self.is_playing { "▶ Playing" } else { "⏸ Paused" }
                } else if self.playback_error.is_some() {
                    "⚠ Error"
                } else {
                    "⏹ Stopped"
                };
                ui.label(RichText::new(status).color(
                    if self.is_playing { Color32::from_rgb(100, 255, 100) }
                    else { Color32::GRAY }
                ));
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
