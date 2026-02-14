import re

# Read the file
with open(r'V:\sassy-browser-FIXED\src\app.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# The code to insert before render_status_bar
new_code = r'''
    // =========================================================================
    // DEVELOPER CONSOLE - Full-featured DevTools panel (F12)
    // =========================================================================

    fn render_dev_console(&mut self, ctx: &egui::Context) {
        if !self.show_dev_tools { return; }
        let mut open = self.show_dev_tools;
        egui::Window::new("Developer Tools")
            .open(&mut open)
            .resizable(true)
            .default_size(Vec2::new(800.0, 400.0))
            .min_width(400.0)
            .min_height(200.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for panel in ConsolePanel::all() {
                        let label = panel.label();
                        let selected = self.dev_console.active_panel == *panel;
                        if ui.selectable_label(selected, label).clicked() {
                            self.dev_console.active_panel = *panel;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Toggle").clicked() {
                            self.dev_console.toggle();
                        }
                        ui.label(RichText::new(format!("h:{}", self.dev_console.height)).small().color(Color32::GRAY));
                    });
                });
                ui.separator();
                match self.dev_console.active_panel {
                    ConsolePanel::Console => self.render_dev_console_tab(ui),
                    ConsolePanel::Network => self.render_dev_network_tab(ui),
                    ConsolePanel::Elements => self.render_dev_elements_tab(ui),
                    ConsolePanel::Sources => self.render_dev_sources_tab(ui),
                    ConsolePanel::Application => self.render_dev_application_tab(ui),
                }
            });
        if !open { self.show_dev_tools = false; }
    }

    fn render_dev_console_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Levels:").small());
            if ui.selectable_label(self.dev_console.show_log, "Log").clicked() { self.dev_console.show_log = !self.dev_console.show_log; }
            if ui.selectable_label(self.dev_console.show_info, "Info").clicked() { self.dev_console.show_info = !self.dev_console.show_info; }
            if ui.selectable_label(self.dev_console.show_warn, "Warn").clicked() { self.dev_console.show_warn = !self.dev_console.show_warn; }
            if ui.selectable_label(self.dev_console.show_error, "Error").clicked() { self.dev_console.show_error = !self.dev_console.show_error; }
            ui.separator();
            if ui.small_button("Clear").clicked() { self.dev_console.clear(); }
            ui.separator();
            ui.label(RichText::new("Filter:").small());
            ui.text_edit_singleline(&mut self.dev_console.console_filter);
        });
        ui.separator();
        let filtered: Vec<ConsoleEntry> = self.dev_console.filtered_console_entries().into_iter().cloned().collect();
        let entry_count = filtered.len();
        let total_count = self.dev_console.console_entries.len();
        egui::ScrollArea::vertical().max_height(ui.available_height() - 40.0).stick_to_bottom(true).show(ui, |ui| {
            if filtered.is_empty() {
                ui.label(RichText::new(format!("No console output ({} total, all filtered)", total_count)).italics().color(Color32::GRAY));
            } else {
                ui.label(RichText::new(format!("Showing {} of {} entries", entry_count, total_count)).small().color(Color32::GRAY));
                for entry in &filtered {
                    let c = entry.level.color();
                    let color = egui::Color32::from_rgba_premultiplied(c.r, c.g, c.b, c.a);
                    let prefix = entry.level.prefix();
                    let _level_desc = entry.level.describe();
                    let ts = entry.timestamp.format("%H:%M:%S%.3f").to_string();
                    let source_str = entry.source.as_deref().unwrap_or("");
                    let stack_str = entry.stack_trace.as_deref().unwrap_or("");
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&ts).monospace().size(10.0).color(Color32::GRAY));
                        ui.label(RichText::new(format!("{}{}", prefix, &entry.message)).monospace().size(11.0).color(color));
                        if !source_str.is_empty() {
                            ui.label(RichText::new(format!("@ {}", source_str)).monospace().size(10.0).color(Color32::GRAY));
                        }
                    });
                    let full_desc = entry.describe();
                    if !stack_str.is_empty() {
                        ui.label(RichText::new(stack_str).monospace().size(10.0).color(Color32::from_rgb(180, 180, 180)));
                    }
                    if ui.small_button("...").on_hover_text(&full_desc).clicked() {
                        self.status_message = full_desc;
                    }
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new(">").monospace().color(Color32::from_rgb(100, 180, 255)));
            let response = ui.text_edit_singleline(&mut self.dev_console.input_buffer);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.dev_console.handle_key("Enter", false);
            }
            ui.label(RichText::new(format!("cur:{} hist:{}", self.dev_console.input_cursor, self.dev_console.command_history.len())).small().color(Color32::GRAY));
            if let Some(idx) = self.dev_console.history_index {
                ui.label(RichText::new(format!("[{}]", idx)).small().color(Color32::YELLOW));
            }
        });
    }

    fn render_dev_network_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Filter:").small());
            ui.text_edit_singleline(&mut self.dev_console.network_filter);
            ui.separator();
            if ui.small_button("Clear Network").clicked() { self.dev_console.network_entries.clear(); self.dev_console.selected_network_entry = None; }
            ui.label(RichText::new(format!("next_id:{}", self.dev_console.next_request_id)).small().color(Color32::GRAY));
        });
        ui.separator();
        let filtered: Vec<NetworkEntry> = self.dev_console.filtered_network_entries().into_iter().cloned().collect();
        let total = self.dev_console.network_entries.len();
        let max = self.dev_console.max_network_entries;
        egui::ScrollArea::vertical().max_height(ui.available_height() - 10.0).show(ui, |ui| {
            if filtered.is_empty() {
                ui.label(RichText::new(format!("No network requests ({}/{})", total, max)).italics().color(Color32::GRAY));
            } else {
                ui.label(RichText::new(format!("Showing {} of {}/{} requests", filtered.len(), total, max)).small().color(Color32::GRAY));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ID").monospace().size(10.0).strong());
                    ui.add_space(10.0);
                    ui.label(RichText::new("Method").monospace().size(10.0).strong());
                    ui.add_space(10.0);
                    ui.label(RichText::new("Status").monospace().size(10.0).strong());
                    ui.add_space(10.0);
                    ui.label(RichText::new("URL").monospace().size(10.0).strong());
                    ui.add_space(10.0);
                    ui.label(RichText::new("Duration").monospace().size(10.0).strong());
                    ui.add_space(10.0);
                    ui.label(RichText::new("Type").monospace().size(10.0).strong());
                });
                ui.separator();
                for entry in &filtered {
                    let sc = entry.status_color();
                    let status_color = Color32::from_rgb(sc.r, sc.g, sc.b);
                    let is_selected = self.dev_console.selected_network_entry == Some(entry.id);
                    let row = ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{}", entry.id)).monospace().size(10.0));
                        ui.add_space(10.0);
                        ui.label(RichText::new(&entry.method).monospace().size(10.0).color(Color32::from_rgb(100, 180, 255)));
                        ui.add_space(10.0);
                        let status_str = match (&entry.status, &entry.status_text) {
                            (Some(s), Some(t)) => format!("{} {}", s, t),
                            (Some(s), None) => format!("{}", s),
                            _ => "pending".to_string(),
                        };
                        ui.label(RichText::new(&status_str).monospace().size(10.0).color(status_color));
                        ui.add_space(10.0);
                        let url_display = if entry.url.len() > 60 { format!("{}...", &entry.url[..57]) } else { entry.url.clone() };
                        ui.label(RichText::new(&url_display).monospace().size(10.0));
                        ui.add_space(10.0);
                        let dur = entry.duration_ms.map_or("--".to_string(), |d| format!("{}ms", d));
                        ui.label(RichText::new(&dur).monospace().size(10.0));
                        ui.add_space(10.0);
                        let ct = entry.content_type.as_deref().unwrap_or("--");
                        ui.label(RichText::new(ct).monospace().size(10.0).color(Color32::GRAY));
                    });
                    if row.response.interact(egui::Sense::click()).clicked() {
                        if is_selected { self.dev_console.selected_network_entry = None; } else { self.dev_console.selected_network_entry = Some(entry.id); }
                    }
                    if is_selected {
                        ui.indent("net_detail", |ui| {
                            ui.add_space(4.0);
                            let desc = entry.describe();
                            ui.label(RichText::new(&desc).monospace().size(10.0).color(Color32::from_rgb(200, 200, 200)));
                            ui.add_space(4.0);
                            ui.label(RichText::new("Waterfall:").small().strong());
                            let wf_desc = entry.waterfall.describe();
                            ui.label(RichText::new(&wf_desc).monospace().size(10.0).color(Color32::from_rgb(180, 180, 220)));
                            let total_wf = entry.waterfall.total_ms();
                            if total_wf > 0.0 {
                                let segments = entry.waterfall.segments();
                                ui.horizontal(|ui| {
                                    for (name, _start, dur, wf_color) in &segments {
                                        let frac = (*dur / total_wf).clamp(0.0, 1.0) as f32;
                                        let bar_width = (frac * 200.0).max(4.0);
                                        let bar_color = Color32::from_rgb(wf_color.r, wf_color.g, wf_color.b);
                                        let (rect, _) = ui.allocate_exact_size(Vec2::new(bar_width, 12.0), egui::Sense::hover());
                                        ui.painter().rect_filled(rect, 0.0, bar_color);
                                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, name, FontId::monospace(8.0), Color32::WHITE);
                                    }
                                });
                            }
                            if !entry.request_headers.is_empty() {
                                ui.collapsing("Request Headers", |ui| { for (k, v) in &entry.request_headers { ui.label(RichText::new(format!("{}: {}", k, v)).monospace().size(10.0)); } });
                            }
                            if !entry.response_headers.is_empty() {
                                ui.collapsing("Response Headers", |ui| { for (k, v) in &entry.response_headers { ui.label(RichText::new(format!("{}: {}", k, v)).monospace().size(10.0)); } });
                            }
                            if let Some(body) = &entry.request_body {
                                ui.collapsing("Request Body", |ui| { ui.label(RichText::new(body).monospace().size(10.0)); });
                            }
                            if let Some(body) = &entry.response_body {
                                ui.collapsing("Response Body", |ui| {
                                    let is_json = entry.content_type.as_ref().map_or(false, |ct| ct.contains("json"));
                                    if is_json {
                                        let tokens = self.dev_console.highlight_js(body);
                                        for line_tokens in &tokens {
                                            ui.horizontal(|ui| {
                                                for tok in line_tokens {
                                                    ui.label(RichText::new(&tok.text).monospace().size(10.0).color(Color32::from_rgb(tok.color.r, tok.color.g, tok.color.b)));
                                                }
                                            });
                                        }
                                    } else {
                                        ui.label(RichText::new(body).monospace().size(10.0));
                                    }
                                });
                            }
                            if let Some(cl) = entry.content_length { ui.label(RichText::new(format!("Content-Length: {} bytes", cl)).monospace().size(10.0).color(Color32::GRAY)); }
                            if let Some(err) = &entry.error { ui.label(RichText::new(format!("Error: {}", err)).monospace().size(10.0).color(Color32::from_rgb(255, 100, 100))); }
                            ui.label(RichText::new(format!("Started: {}", entry.start_time.format("%H:%M:%S%.3f"))).monospace().size(10.0).color(Color32::GRAY));
                            if let Some(end) = &entry.end_time { ui.label(RichText::new(format!("Ended: {}", end.format("%H:%M:%S%.3f"))).monospace().size(10.0).color(Color32::GRAY)); }
                            ui.add_space(4.0);
                        });
                    }
                }
            }
        });
    }

    fn render_dev_elements_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let pick_label = if self.dev_console.inspector.pick_mode { "Picking (click element)" } else { "Pick Element" };
            if ui.selectable_label(self.dev_console.inspector.pick_mode, pick_label).clicked() { self.dev_console.inspector.toggle_pick_mode(); }
            if ui.small_button("Clear Inspector").clicked() { self.dev_console.inspector.clear(); }
            if ui.small_button("Select Root").clicked() { self.dev_console.inspector.select_element(vec![0]); }
            if ui.small_button("Reset Styles").clicked() { let ds = crate::style::ComputedStyle::default(); self.dev_console.inspector.update_from_computed(&ds); }
            if ui.small_button("Set Content 100x100").clicked() { self.dev_console.inspector.set_content_size(100.0, 100.0); }
        });
        ui.separator();
        let inspector_desc = self.dev_console.inspector.describe();
        ui.label(RichText::new(&inspector_desc).monospace().size(10.0).color(Color32::from_rgb(180, 200, 220)));
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            if !self.dev_console.inspector.selected_path.is_empty() {
                let path_str: Vec<String> = self.dev_console.inspector.selected_path.iter().map(|i| i.to_string()).collect();
                ui.label(RichText::new(format!("Selected: /{}", path_str.join("/"))).monospace().size(11.0).color(Color32::from_rgb(100, 180, 255)));
                if !self.dev_console.inspector.hovered_path.is_empty() {
                    let hover_str: Vec<String> = self.dev_console.inspector.hovered_path.iter().map(|i| i.to_string()).collect();
                    ui.label(RichText::new(format!("Hovered: /{}", hover_str.join("/"))).monospace().size(10.0).color(Color32::GRAY));
                }
                ui.add_space(8.0);
                if !self.dev_console.inspector.computed_styles.is_empty() {
                    ui.collapsing("Computed Styles", |ui| {
                        for (prop, val) in &self.dev_console.inspector.computed_styles {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{}:", prop)).monospace().size(10.0).color(Color32::from_rgb(200, 150, 255)));
                                ui.label(RichText::new(val).monospace().size(10.0));
                            });
                        }
                    });
                }
                if !self.dev_console.inspector.matched_rules.is_empty() {
                    ui.collapsing("Matched Rules", |ui| {
                        for rule in &self.dev_console.inspector.matched_rules {
                            let rule_desc = rule.describe();
                            ui.label(RichText::new(&rule_desc).monospace().size(10.0).color(Color32::from_rgb(180, 200, 160)));
                            for (name, val, overridden) in &rule.properties {
                                let color = if *overridden { Color32::from_rgb(128, 128, 128) } else { Color32::from_rgb(220, 220, 220) };
                                ui.label(RichText::new(format!("  {}: {}{}", name, val, if *overridden { " (overridden)" } else { "" })).monospace().size(10.0).color(color));
                            }
                        }
                    });
                }
                let bm = &self.dev_console.inspector.box_model;
                ui.collapsing("Box Model", |ui| {
                    let bm_desc = bm.describe();
                    ui.label(RichText::new(&bm_desc).monospace().size(10.0));
                    ui.add_space(4.0);
                    ui.label(RichText::new(format!("Margin:  {}", bm.margin.describe())).monospace().size(10.0));
                    ui.label(RichText::new(format!("Border:  {}", bm.border.describe())).monospace().size(10.0));
                    ui.label(RichText::new(format!("Padding: {}", bm.padding.describe())).monospace().size(10.0));
                    ui.label(RichText::new(format!("Content: {}", bm.content.describe())).monospace().size(10.0));
                });
            } else {
                ui.label(RichText::new("No element selected. Click 'Pick Element' then click on the page.").italics().color(Color32::GRAY));
            }
        });
    }

    fn render_dev_sources_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Sources").strong());
        ui.separator();
        let code = if self.dev_console.input_buffer.is_empty() { "// Enter JavaScript in the Console tab\nvar x = 1;\nconsole.log(x);" } else { &self.dev_console.input_buffer };
        let tokens = self.dev_console.highlight_js(code);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (line_num, line_tokens) in tokens.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{:4}", line_num + 1)).monospace().size(10.0).color(Color32::from_rgb(100, 100, 100)));
                    for tok in line_tokens { ui.label(RichText::new(&tok.text).monospace().size(11.0).color(Color32::from_rgb(tok.color.r, tok.color.g, tok.color.b))); }
                });
            }
        });
    }

    fn render_dev_application_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Application State").strong());
        ui.separator();
        ui.horizontal(|ui| {
            if ui.small_button("Log Test Message").clicked() {
                self.dev_console.log(LogLevel::Info, "Test info message".to_string());
                self.dev_console.log(LogLevel::Warn, "Test warning".to_string());
                self.dev_console.log(LogLevel::Error, "Test error".to_string());
                self.dev_console.log(LogLevel::Debug, "Test debug".to_string());
                self.dev_console.log_with_source(LogLevel::Log, "Message with source".to_string(), "app.rs:42".to_string());
            }
            if ui.small_button("Start Test Request").clicked() {
                let id = self.dev_console.start_request("GET", "https://example.com/api/test");
                self.dev_console.complete_request(id, 200, "OK");
            }
            if ui.small_button("Start Failed Request").clicked() {
                let id = self.dev_console.start_request("POST", "https://example.com/api/fail");
                self.dev_console.fail_request(id, "Connection refused");
            }
            if ui.small_button("Clear All").clicked() { self.dev_console.clear(); self.dev_console.inspector.clear(); }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.dev_console.render(ui);
            ui.add_space(8.0);
            let status = self.dev_console.status();
            for line in status.lines() { ui.label(RichText::new(line).monospace().size(10.0)); }
        });
    }

'''

# Find the exact insertion point - before "fn render_status_bar"
# We need to insert our new methods right before that function
target = '    fn render_status_bar(&self, ctx: &egui::Context) {'
if target not in content:
    print("ERROR: Could not find render_status_bar")
    exit(1)

content = content.replace(target, new_code + target)

# Also update the import line
old_import = 'use crate::console::DevConsole;'
new_import = 'use crate::console::{DevConsole, ConsolePanel, LogLevel, ConsoleEntry, NetworkEntry};'
if old_import in content:
    content = content.replace(old_import, new_import)
else:
    print("WARN: import line already changed or not found")

# Write back
with open(r'V:\sassy-browser-FIXED\src\app.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("SUCCESS: Inserted render_dev_console and related methods")
