#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! Chemical/Biological Viewer - PDB, MOL, SDF molecular structure viewer

use crate::file_handler::{Atom, Bond, ChemicalContent, FileContent, OpenFile};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashMap;

pub struct ChemicalViewer {
    rotation_x: f32,
    rotation_y: f32,
    zoom: f32,
    pan_offset: Vec2,
    show_atoms: bool,
    show_bonds: bool,
    show_labels: bool,
    show_backbone: bool,
    color_mode: ColorMode,
    render_mode: RenderMode,
    selected_atom: Option<usize>,
    element_colors: HashMap<String, Color32>,
}

#[derive(Clone, Copy, PartialEq)]
enum ColorMode {
    ByElement,
    ByChain,
    ByResidue,
    BFactor,
}

#[derive(Clone, Copy, PartialEq)]
enum RenderMode {
    BallAndStick,
    Wireframe,
    Spacefill,
    Cartoon,
}

impl ChemicalViewer {
    pub fn new() -> Self {
        let mut element_colors = HashMap::new();
        
        // CPK coloring scheme
        element_colors.insert("C".into(), Color32::from_rgb(144, 144, 144));
        element_colors.insert("N".into(), Color32::from_rgb(48, 80, 248));
        element_colors.insert("O".into(), Color32::from_rgb(255, 13, 13));
        element_colors.insert("H".into(), Color32::from_rgb(255, 255, 255));
        element_colors.insert("S".into(), Color32::from_rgb(255, 255, 48));
        element_colors.insert("P".into(), Color32::from_rgb(255, 128, 0));
        element_colors.insert("Fe".into(), Color32::from_rgb(224, 102, 51));
        element_colors.insert("Zn".into(), Color32::from_rgb(125, 128, 176));
        element_colors.insert("Ca".into(), Color32::from_rgb(61, 255, 0));
        element_colors.insert("Mg".into(), Color32::from_rgb(138, 255, 0));
        element_colors.insert("Cl".into(), Color32::from_rgb(31, 240, 31));
        element_colors.insert("Na".into(), Color32::from_rgb(171, 92, 242));
        element_colors.insert("K".into(), Color32::from_rgb(143, 64, 212));
        
        Self {
            rotation_x: 0.0,
            rotation_y: 0.0,
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            show_atoms: true,
            show_bonds: true,
            show_labels: false,
            show_backbone: false,
            color_mode: ColorMode::ByElement,
            render_mode: RenderMode::BallAndStick,
            selected_atom: None,
            element_colors,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, global_zoom: f32) {
        if let FileContent::Chemical(chem) = &file.content {
            // Toolbar
            self.render_toolbar(ui, chem, global_zoom);
            
            ui.separator();
            
            // Split view: 3D viewer + info panel
            ui.horizontal(|ui| {
                // Main 3D view
                self.render_3d_view(ui, chem, global_zoom);
                
                // Info panel
                self.render_info_panel(ui, chem);
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Not a chemical/molecular file");
            });
        }
    }
    
    fn render_toolbar(&mut self, ui: &mut egui::Ui, chem: &ChemicalContent, global_zoom: f32) {
        ui.horizontal(|ui| {
            // Render mode
            ui.label("Render:");
            if ui.selectable_label(self.render_mode == RenderMode::BallAndStick, "âš› Ball & Stick").clicked() {
                self.render_mode = RenderMode::BallAndStick;
            }
            if ui.selectable_label(self.render_mode == RenderMode::Wireframe, "ðŸ“ Wireframe").clicked() {
                self.render_mode = RenderMode::Wireframe;
            }
            if ui.selectable_label(self.render_mode == RenderMode::Spacefill, "ðŸ”´ Spacefill").clicked() {
                self.render_mode = RenderMode::Spacefill;
            }
            
            ui.separator();
            
            // Color mode
            ui.label("Color:");
            egui::ComboBox::from_id_salt("color_mode")
                .selected_text(match self.color_mode {
                    ColorMode::ByElement => "Element",
                    ColorMode::ByChain => "Chain",
                    ColorMode::ByResidue => "Residue",
                    ColorMode::BFactor => "B-Factor",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.color_mode, ColorMode::ByElement, "Element");
                    ui.selectable_value(&mut self.color_mode, ColorMode::ByChain, "Chain");
                    ui.selectable_value(&mut self.color_mode, ColorMode::ByResidue, "Residue");
                    ui.selectable_value(&mut self.color_mode, ColorMode::BFactor, "B-Factor");
                });
            
            ui.separator();
            
            // Display options
            ui.checkbox(&mut self.show_atoms, "Atoms");
            ui.checkbox(&mut self.show_bonds, "Bonds");
            ui.checkbox(&mut self.show_labels, "Labels");
            
            ui.separator();
            
            // Reset view
            if ui.button("ðŸ”„ Reset View").clicked() {
                self.rotation_x = 0.0;
                self.rotation_y = 0.0;
                self.zoom = 1.0;
                self.pan_offset = Vec2::ZERO;
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Atoms: {} | Bonds: {}", chem.atoms.len(), chem.bonds.len()));
            });
        });
    }
    
    fn render_3d_view(&mut self, ui: &mut egui::Ui, chem: &ChemicalContent, global_zoom: f32) {
        let available = ui.available_size();
        let view_size = Vec2::new(available.x - 250.0, available.y);
        
        let (response, painter) = ui.allocate_painter(view_size, Sense::click_and_drag());
        
        // Background
        painter.rect_filled(response.rect, 0.0, Color32::from_rgb(20, 25, 35));
        
        if chem.atoms.is_empty() {
            painter.text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                "No atoms to display",
                FontId::proportional(16.0),
                Color32::GRAY,
            );
            return;
        }
        
        // Handle rotation
        if response.dragged_by(egui::PointerButton::Primary) {
            self.rotation_y += response.drag_delta().x * 0.5;
            self.rotation_x += response.drag_delta().y * 0.5;
        }
        
        // Handle zoom with scroll
        if response.hovered() {
            ui.input(|i| {
                self.zoom *= 1.0 + i.raw_scroll_delta.y * 0.001;
                self.zoom = self.zoom.clamp(0.1, 10.0);
            });
        }
        
        // Calculate molecule bounds
        let (min_x, max_x, min_y, max_y, min_z, max_z) = self.calculate_bounds(chem);
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;
        let center_z = (min_z + max_z) / 2.0;
        
        let scale = (view_size.x.min(view_size.y) / 2.0) / 
            ((max_x - min_x).max(max_y - min_y).max(max_z - min_z).max(1.0)) *
            self.zoom * global_zoom;
        
        let view_center = response.rect.center();
        
        // Project atoms to 2D
        let mut projected_atoms: Vec<(usize, Pos2, f32, Color32)> = Vec::new();
        
        for (idx, atom) in chem.atoms.iter().enumerate() {
            // Center coordinates
            let x = atom.x - center_x;
            let y = atom.y - center_y;
            let z = atom.z - center_z;
            
            // Apply rotation (simplified)
            let rot_y_rad = self.rotation_y.to_radians();
            let rot_x_rad = self.rotation_x.to_radians();
            
            let x1 = x * rot_y_rad.cos() - z * rot_y_rad.sin();
            let z1 = x * rot_y_rad.sin() + z * rot_y_rad.cos();
            
            let y2 = y * rot_x_rad.cos() - z1 * rot_x_rad.sin();
            let z2 = y * rot_x_rad.sin() + z1 * rot_x_rad.cos();
            
            // Project to 2D
            let screen_x = view_center.x + x1 * scale;
            let screen_y = view_center.y - y2 * scale;
            
            let color = self.get_atom_color(atom);
            
            projected_atoms.push((idx, Pos2::new(screen_x, screen_y), z2, color));
        }
        
        // Sort by depth (painter's algorithm)
        projected_atoms.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        
        // Draw bonds first
        if self.show_bonds {
            for bond in &chem.bonds {
                if bond.atom1 < projected_atoms.len() && bond.atom2 < projected_atoms.len() {
                    let atom1 = projected_atoms.iter().find(|(i, _, _, _)| *i == bond.atom1);
                    let atom2 = projected_atoms.iter().find(|(i, _, _, _)| *i == bond.atom2);
                    
                    if let (Some((_, pos1, _, color1)), Some((_, pos2, _, color2))) = (atom1, atom2) {
                        let thickness = match self.render_mode {
                            RenderMode::Wireframe => 1.0,
                            _ => 2.0 * global_zoom,
                        };
                        
                        // Gradient bond
                        let mid = Pos2::new((pos1.x + pos2.x) / 2.0, (pos1.y + pos2.y) / 2.0);
                        painter.line_segment([*pos1, mid], Stroke::new(thickness, *color1));
                        painter.line_segment([mid, *pos2], Stroke::new(thickness, *color2));
                    }
                }
            }
        }
        
        // Draw atoms
        if self.show_atoms {
            for (idx, pos, _z, color) in &projected_atoms {
                let radius = match self.render_mode {
                    RenderMode::BallAndStick => 6.0 * global_zoom * self.zoom,
                    RenderMode::Spacefill => 15.0 * global_zoom * self.zoom,
                    RenderMode::Wireframe => 3.0 * global_zoom,
                    RenderMode::Cartoon => 4.0 * global_zoom,
                };
                
                // Highlight selected atom
                let is_selected = self.selected_atom == Some(*idx);
                
                if is_selected {
                    painter.circle_stroke(*pos, radius + 3.0, Stroke::new(2.0, Color32::YELLOW));
                }
                
                painter.circle_filled(*pos, radius, *color);
                
                // Labels
                if self.show_labels {
                    if let Some(atom) = chem.atoms.get(*idx) {
                        painter.text(
                            Pos2::new(pos.x + radius + 2.0, pos.y),
                            egui::Align2::LEFT_CENTER,
                            &atom.element,
                            FontId::proportional(10.0 * global_zoom),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }
        
        // Handle atom selection
        if response.clicked() {
            if let Some(click_pos) = response.interact_pointer_pos() {
                self.selected_atom = projected_atoms.iter()
                    .find(|(_, pos, _, _)| pos.distance(click_pos) < 10.0)
                    .map(|(idx, _, _, _)| *idx);
            }
        }
        
        // Draw axis indicator
        self.draw_axis_indicator(&painter, response.rect);
    }
    
    fn render_info_panel(&mut self, ui: &mut egui::Ui, chem: &ChemicalContent) {
        egui::SidePanel::right("chem_info")
            .exact_width(240.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Title
                    ui.heading("ðŸ“Š Structure Info");
                    ui.separator();
                    
                    if !chem.title.is_empty() {
                        ui.label(format!("Title: {}", chem.title));
                    }
                    
                    ui.label(format!("Atoms: {}", chem.atoms.len()));
                    ui.label(format!("Bonds: {}", chem.bonds.len()));
                    
                    // Element composition
                    ui.separator();
                    ui.heading("ðŸ§ª Composition");
                    
                    let mut element_counts: HashMap<String, usize> = HashMap::new();
                    for atom in &chem.atoms {
                        *element_counts.entry(atom.element.clone()).or_insert(0) += 1;
                    }
                    
                    let mut elements: Vec<_> = element_counts.iter().collect();
                    elements.sort_by(|a, b| b.1.cmp(a.1));
                    
                    for (element, count) in elements.iter().take(10) {
                        let color = self.element_colors.get(*element)
                            .copied()
                            .unwrap_or(Color32::GRAY);
                        
                        ui.horizontal(|ui| {
                            ui.colored_label(color, format!("â— {}: {}", element, count));
                        });
                    }
                    
                    // Selected atom info
                    if let Some(idx) = self.selected_atom {
                        if let Some(atom) = chem.atoms.get(idx) {
                            ui.separator();
                            ui.heading("ðŸŽ¯ Selected Atom");
                            ui.label(format!("Element: {}", atom.element));
                            ui.label(format!("Serial: {}", atom.serial));
                            ui.label(format!("Position: ({:.2}, {:.2}, {:.2})", 
                                atom.x, atom.y, atom.z));
                            if !atom.residue.is_empty() {
                                ui.label(format!("Residue: {}", atom.residue));
                            }
                            ui.label(format!("Chain: {}", atom.chain));
                        }
                    }
                    
                    // Chain info
                    let chains: std::collections::HashSet<_> = chem.atoms.iter()
                        .map(|a| a.chain)
                        .collect();
                    
                    if chains.len() > 1 {
                        ui.separator();
                        ui.heading("ðŸ”— Chains");
                        for chain in chains {
                            let count = chem.atoms.iter().filter(|a| a.chain == chain).count();
                            ui.label(format!("Chain {}: {} atoms", chain, count));
                        }
                    }
                });
            });
    }
    
    fn calculate_bounds(&self, chem: &ChemicalContent) -> (f32, f32, f32, f32, f32, f32) {
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;
        
        for atom in &chem.atoms {
            min_x = min_x.min(atom.x);
            max_x = max_x.max(atom.x);
            min_y = min_y.min(atom.y);
            max_y = max_y.max(atom.y);
            min_z = min_z.min(atom.z);
            max_z = max_z.max(atom.z);
        }
        
        (min_x, max_x, min_y, max_y, min_z, max_z)
    }
    
    fn get_atom_color(&self, atom: &Atom) -> Color32 {
        match self.color_mode {
            ColorMode::ByElement => {
                self.element_colors.get(&atom.element)
                    .copied()
                    .unwrap_or(Color32::from_rgb(200, 200, 200))
            }
            ColorMode::ByChain => {
                let chain_idx = atom.chain as u8;
                let hue = (chain_idx as f32 * 137.5) % 360.0;
                hsv_to_rgb(hue, 0.7, 0.9)
            }
            ColorMode::ByResidue => {
                let hash = atom.residue.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
                let hue = (hash % 360) as f32;
                hsv_to_rgb(hue, 0.6, 0.85)
            }
            ColorMode::BFactor => {
                // Use position as proxy for B-factor visualization
                let intensity = ((atom.x + atom.y + atom.z).abs() % 100.0) / 100.0;
                let r = (255.0 * intensity) as u8;
                let b = (255.0 * (1.0 - intensity)) as u8;
                Color32::from_rgb(r, 50, b)
            }
        }
    }
    
    fn draw_axis_indicator(&self, painter: &egui::Painter, rect: Rect) {
        let origin = Pos2::new(rect.left() + 40.0, rect.bottom() - 40.0);
        let length = 25.0;
        
        let rot_y_rad = self.rotation_y.to_radians();
        let rot_x_rad = self.rotation_x.to_radians();
        
        // X axis (red)
        let x_end = Pos2::new(
            origin.x + length * rot_y_rad.cos(),
            origin.y + length * rot_y_rad.sin() * rot_x_rad.sin(),
        );
        painter.line_segment([origin, x_end], Stroke::new(2.0, Color32::RED));
        painter.text(x_end, egui::Align2::LEFT_CENTER, "X", FontId::proportional(10.0), Color32::RED);
        
        // Y axis (green)
        let y_end = Pos2::new(
            origin.x,
            origin.y - length * rot_x_rad.cos(),
        );
        painter.line_segment([origin, y_end], Stroke::new(2.0, Color32::GREEN));
        painter.text(y_end, egui::Align2::CENTER_BOTTOM, "Y", FontId::proportional(10.0), Color32::GREEN);
        
        // Z axis (blue)
        let z_end = Pos2::new(
            origin.x - length * rot_y_rad.sin(),
            origin.y - length * rot_y_rad.cos() * rot_x_rad.sin(),
        );
        painter.line_segment([origin, z_end], Stroke::new(2.0, Color32::from_rgb(100, 100, 255)));
        painter.text(z_end, egui::Align2::RIGHT_CENTER, "Z", FontId::proportional(10.0), Color32::from_rgb(100, 100, 255));
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color32 {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    
    Color32::from_rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
