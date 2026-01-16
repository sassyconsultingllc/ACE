//! Archive Viewer - ZIP, RAR, 7z, TAR viewing with extraction support

use crate::file_handler::{ArchiveContent, ArchiveEntry, ArchiveFormat, FileContent, OpenFile};
use eframe::egui::{self, Color32, FontId, RichText, Vec2};
use std::path::PathBuf;

pub struct ArchiveViewer {
    selected_entry: Option<usize>,
    sort_column: SortColumn,
    sort_ascending: bool,
    filter_query: String,
    show_hidden: bool,
    tree_view: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum SortColumn {
    Name,
    Size,
    CompressedSize,
    Modified,
}

impl ArchiveViewer {
    pub fn new() -> Self {
        Self {
            selected_entry: None,
            sort_column: SortColumn::Name,
            sort_ascending: true,
            filter_query: String::new(),
            show_hidden: false,
            tree_view: true,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        if let FileContent::Archive(archive) = &file.content {
            self.render_toolbar(ui, archive);
            ui.separator();
            self.render_info_bar(ui, archive);
            ui.separator();
            self.render_entries(ui, archive, zoom);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Not an archive file");
            });
        }
    }
    
    fn render_toolbar(&mut self, ui: &mut egui::Ui, archive: &ArchiveContent) {
        ui.horizontal(|ui| {
            // View mode
            if ui.selectable_label(self.tree_view, "🗂️ Tree").clicked() {
                self.tree_view = true;
            }
            if ui.selectable_label(!self.tree_view, "📋 List").clicked() {
                self.tree_view = false;
            }
            
            ui.separator();
            
            // Filter
            ui.label("🔍");
            ui.add(egui::TextEdit::singleline(&mut self.filter_query)
                .desired_width(150.0)
                .hint_text("Filter..."));
            
            ui.separator();
            
            ui.checkbox(&mut self.show_hidden, "Show hidden");
            
            ui.separator();
            
            // Sort options
            ui.label("Sort:");
            if ui.selectable_label(self.sort_column == SortColumn::Name, "Name").clicked() {
                if self.sort_column == SortColumn::Name {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = SortColumn::Name;
                    self.sort_ascending = true;
                }
            }
            if ui.selectable_label(self.sort_column == SortColumn::Size, "Size").clicked() {
                if self.sort_column == SortColumn::Size {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = SortColumn::Size;
                    self.sort_ascending = false;
                }
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📤 Extract All...").clicked() {
                    // TODO: Extract dialog
                }
            });
        });
    }
    
    fn render_info_bar(&mut self, ui: &mut egui::Ui, archive: &ArchiveContent) {
        ui.horizontal(|ui| {
            let format_name = match archive.format {
                ArchiveFormat::Zip => "ZIP",
                ArchiveFormat::Rar => "RAR",
                ArchiveFormat::SevenZ => "7z",
                ArchiveFormat::Tar => "TAR",
                ArchiveFormat::TarGz => "TAR.GZ",
                ArchiveFormat::TarXz => "TAR.XZ",
                ArchiveFormat::TarBz2 => "TAR.BZ2",
                ArchiveFormat::TarZstd => "TAR.ZSTD",
            };
            
            ui.label(RichText::new(format!("📦 {}", format_name)).strong());
            ui.separator();
            
            let file_count = archive.entries.iter().filter(|e| !e.is_dir).count();
            let dir_count = archive.entries.iter().filter(|e| e.is_dir).count();
            
            ui.label(format!("{} files, {} folders", file_count, dir_count));
            ui.separator();
            
            ui.label(format!("Total: {}", format_size(archive.total_size)));
            
            if archive.compressed_size > 0 && archive.compressed_size < archive.total_size {
                let ratio = (archive.compressed_size as f64 / archive.total_size as f64) * 100.0;
                ui.label(format!("→ {} ({:.1}%)", format_size(archive.compressed_size), ratio));
            }
            
            if let Some(comment) = &archive.comment {
                if !comment.is_empty() {
                    ui.separator();
                    ui.label(format!("💬 {}", comment));
                }
            }
        });
    }
    
    fn render_entries(&mut self, ui: &mut egui::Ui, archive: &ArchiveContent, zoom: f32) {
        let filtered_entries: Vec<(usize, &ArchiveEntry)> = archive.entries.iter()
            .enumerate()
            .filter(|(_, e)| {
                if !self.filter_query.is_empty() {
                    e.path.to_lowercase().contains(&self.filter_query.to_lowercase())
                } else {
                    true
                }
            })
            .filter(|(_, e)| {
                if !self.show_hidden {
                    !e.path.split('/').last().unwrap_or("").starts_with('.')
                } else {
                    true
                }
            })
            .collect();
        
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Header row
                ui.horizontal(|ui| {
                    ui.add_space(30.0);
                    ui.label(RichText::new("Name").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new("Modified").strong());
                        ui.add_space(20.0);
                        ui.label(RichText::new("Compressed").strong());
                        ui.add_space(20.0);
                        ui.label(RichText::new("Size").strong());
                    });
                });
                
                ui.separator();
                
                if self.tree_view {
                    self.render_tree_view(ui, &filtered_entries, zoom);
                } else {
                    self.render_list_view(ui, &filtered_entries, zoom);
                }
            });
    }
    
    fn render_tree_view(&mut self, ui: &mut egui::Ui, entries: &[(usize, &ArchiveEntry)], zoom: f32) {
        // Build tree structure
        let mut root_items: Vec<&(usize, &ArchiveEntry)> = Vec::new();
        
        for entry in entries {
            let depth = entry.1.path.matches('/').count();
            if depth == 0 || (depth == 1 && entry.1.path.ends_with('/')) {
                root_items.push(entry);
            }
        }
        
        for (idx, entry) in root_items {
            self.render_entry_row(ui, *idx, entry, 0, zoom);
            
            // Render children
            let prefix = &entry.path;
            for child in entries.iter().filter(|(_, e)| {
                e.path.starts_with(prefix) && e.path != *prefix
            }) {
                let child_depth = child.1.path[prefix.len()..].matches('/').count();
                if child_depth <= 1 {
                    self.render_entry_row(ui, child.0, child.1, 1, zoom);
                }
            }
        }
    }
    
    fn render_list_view(&mut self, ui: &mut egui::Ui, entries: &[(usize, &ArchiveEntry)], zoom: f32) {
        for (idx, entry) in entries {
            self.render_entry_row(ui, *idx, entry, 0, zoom);
        }
    }
    
    fn render_entry_row(&mut self, ui: &mut egui::Ui, idx: usize, entry: &ArchiveEntry, indent: usize, zoom: f32) {
        let is_selected = self.selected_entry == Some(idx);
        
        let bg_color = if is_selected {
            Color32::from_rgb(50, 80, 120)
        } else {
            Color32::TRANSPARENT
        };
        
        egui::Frame::none()
            .fill(bg_color)
            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Indent
                    ui.add_space(indent as f32 * 20.0);
                    
                    // Icon
                    let icon = if entry.is_dir {
                        "📁"
                    } else {
                        get_file_icon(&entry.path)
                    };
                    ui.label(icon);
                    
                    // Name
                    let name = entry.path.split('/').last().unwrap_or(&entry.path);
                    let name_display = if entry.is_encrypted {
                        format!("🔒 {}", name)
                    } else {
                        name.to_string()
                    };
                    
                    let response = ui.selectable_label(is_selected, &name_display);
                    if response.clicked() {
                        self.selected_entry = Some(idx);
                    }
                    
                    // Context menu
                    response.context_menu(|ui| {
                        if ui.button("📤 Extract...").clicked() {
                            // TODO: Extract single file
                            ui.close_menu();
                        }
                        if ui.button("👁 Preview").clicked() {
                            // TODO: Preview file
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("📋 Copy path").clicked() {
                            ui.output_mut(|o| o.copied_text = entry.path.clone());
                            ui.close_menu();
                        }
                    });
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        
                        // Modified date
                        if let Some(modified) = &entry.modified {
                            ui.label(RichText::new(modified).small().color(Color32::GRAY));
                        } else {
                            ui.label(RichText::new("-").small().color(Color32::GRAY));
                        }
                        
                        ui.add_space(20.0);
                        
                        // Compressed size
                        if !entry.is_dir {
                            ui.label(RichText::new(format_size(entry.compressed_size)).small());
                        } else {
                            ui.label(RichText::new("-").small());
                        }
                        
                        ui.add_space(20.0);
                        
                        // Size
                        if !entry.is_dir {
                            ui.label(format_size(entry.size));
                        } else {
                            ui.label("-");
                        }
                    });
                });
            });
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

fn get_file_icon(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" => "🖼️",
        "pdf" => "📕",
        "doc" | "docx" | "odt" | "rtf" => "📄",
        "xls" | "xlsx" | "ods" | "csv" => "📊",
        "mp3" | "wav" | "flac" | "ogg" | "aac" => "🎵",
        "mp4" | "mkv" | "avi" | "mov" | "webm" => "🎬",
        "zip" | "rar" | "7z" | "tar" | "gz" => "📦",
        "exe" | "msi" | "app" | "dmg" => "⚙️",
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "java" => "📝",
        "html" | "htm" | "css" => "🌐",
        "json" | "xml" | "yaml" | "toml" => "📋",
        "txt" | "md" | "log" => "📃",
        "ttf" | "otf" | "woff" => "🔤",
        "pdb" | "mol" | "sdf" => "🧬",
        "obj" | "stl" | "gltf" | "glb" => "🎲",
        _ => "📄",
    }
}
