#![allow(deprecated)]
//! 3D Model Viewer - OBJ, STL, GLTF/GLB, PLY visualization

use crate::file_handler::{BoundingBox, Face3D, FileContent, Model3DContent, Model3DFormat, OpenFile, Vertex3D};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

pub struct Model3DViewer {
    rotation_x: f32,
    rotation_y: f32,
    zoom: f32,
    pan_offset: Vec2,
    render_mode: RenderMode,
    show_wireframe: bool,
    show_normals: bool,
    show_axes: bool,
    show_grid: bool,
    light_position: [f32; 3],
    background_color: Color32,
    model_color: Color32,
    wireframe_color: Color32,
    auto_rotate: bool,
    auto_rotate_speed: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum RenderMode {
    Wireframe,
    Solid,
    SolidWireframe,
    Points,
}

impl Model3DViewer {
    pub fn new() -> Self {
        Self {
            rotation_x: -30.0,
            rotation_y: 45.0,
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            render_mode: RenderMode::Solid,
            show_wireframe: false,
            show_normals: false,
            show_axes: true,
            show_grid: true,
            light_position: [1.0, 1.0, 1.0],
            background_color: Color32::from_rgb(30, 35, 45),
            model_color: Color32::from_rgb(180, 180, 200),
            wireframe_color: Color32::from_rgb(100, 150, 200),
            auto_rotate: false,
            auto_rotate_speed: 0.5,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, global_zoom: f32, icons: &crate::icons::Icons) {
        if let FileContent::Model3D(model) = &file.content {
            self.render_toolbar(ui, model);
            ui.separator();
            self.render_3d_viewport(ui, model, global_zoom);
            self.render_info_panel(ui, model);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Not a 3D model file");
            });
        }
    }
    
    fn render_toolbar(&mut self, ui: &mut egui::Ui, model: &Model3DContent) {
        ui.horizontal(|ui| {
            // Render mode
            ui.label("Render:");
            if ui.selectable_label(self.render_mode == RenderMode::Wireframe, "Wire").clicked() {
                self.render_mode = RenderMode::Wireframe;
            }
            if ui.selectable_label(self.render_mode == RenderMode::Solid, "Solid").clicked() {
                self.render_mode = RenderMode::Solid;
            }
            if ui.selectable_label(self.render_mode == RenderMode::SolidWireframe, "Both").clicked() {
                self.render_mode = RenderMode::SolidWireframe;
            }
            if ui.selectable_label(self.render_mode == RenderMode::Points, "* Points").clicked() {
                self.render_mode = RenderMode::Points;
            }
            
            ui.separator();
            
            // Display options
            ui.checkbox(&mut self.show_axes, "Axes");
            ui.checkbox(&mut self.show_grid, "Grid");
            
            ui.separator();
            
            // Auto-rotate
            ui.checkbox(&mut self.auto_rotate, "Auto-rotate");
            if self.auto_rotate {
                ui.add(egui::Slider::new(&mut self.auto_rotate_speed, 0.1..=2.0).text("Speed"));
            }
            
            ui.separator();
            
            // Reset view
            if ui.button(" Reset View").clicked() {
                self.rotation_x = -30.0;
                self.rotation_y = 45.0;
                self.zoom = 1.0;
                self.pan_offset = Vec2::ZERO;
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let format_name = match model.format {
                    Model3DFormat::Obj => "OBJ",
                    Model3DFormat::Stl => "STL",
                    Model3DFormat::Gltf => "GLTF",
                    Model3DFormat::Glb => "GLB",
                    Model3DFormat::Ply => "PLY",
                };
                ui.label(format!("{} | {} verts | {} faces", 
                    format_name, model.vertices.len(), model.faces.len()));
            });
        });
    }
    
    fn render_3d_viewport(&mut self, ui: &mut egui::Ui, model: &Model3DContent, global_zoom: f32) {
        let available = ui.available_size();
        let viewport_size = Vec2::new(available.x - 200.0, available.y);
        
        let (response, painter) = ui.allocate_painter(viewport_size, Sense::click_and_drag());
        
        // Background
        painter.rect_filled(response.rect, 0.0, self.background_color);
        
        // Handle interaction
        if response.dragged_by(egui::PointerButton::Primary) {
            self.rotation_y += response.drag_delta().x * 0.5;
            self.rotation_x += response.drag_delta().y * 0.5;
        }
        
        if response.dragged_by(egui::PointerButton::Secondary) {
            self.pan_offset += response.drag_delta();
        }
        
        if response.hovered() {
            ui.input(|i| {
                self.zoom *= 1.0 + i.raw_scroll_delta.y * 0.001;
                self.zoom = self.zoom.clamp(0.1, 10.0);
            });
        }
        
        // Auto-rotate
        if self.auto_rotate {
            self.rotation_y += self.auto_rotate_speed;
            ui.ctx().request_repaint();
        }
        
        let center = response.rect.center() + self.pan_offset;
        
        // Calculate scale based on bounding box
        let bounds = &model.bounds;
        let size = [
            bounds.max[0] - bounds.min[0],
            bounds.max[1] - bounds.min[1],
            bounds.max[2] - bounds.min[2],
        ];
        let max_size = size[0].max(size[1]).max(size[2]).max(0.001);
        let scale = (viewport_size.x.min(viewport_size.y) * 0.4) / max_size * self.zoom * global_zoom;
        
        let model_center = [
            (bounds.min[0] + bounds.max[0]) / 2.0,
            (bounds.min[1] + bounds.max[1]) / 2.0,
            (bounds.min[2] + bounds.max[2]) / 2.0,
        ];
        
        // Draw grid
        if self.show_grid {
            self.draw_grid(&painter, center, scale, &model_center);
        }
        
        // Draw axes
        if self.show_axes {
            self.draw_axes(&painter, center, scale * max_size * 0.6);
        }
        
        // Project and render model
        if model.vertices.is_empty() {
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                "No geometry to display",
                FontId::proportional(16.0),
                Color32::GRAY,
            );
            return;
        }
        
        // Project all vertices
        let projected: Vec<(Pos2, f32)> = model.vertices.iter()
            .map(|v| self.project_vertex(v, &model_center, center, scale))
            .collect();
        
        // Render based on mode
        match self.render_mode {
            RenderMode::Wireframe => {
                self.draw_wireframe(&painter, model, &projected);
            }
            RenderMode::Solid => {
                self.draw_solid(&painter, model, &projected);
            }
            RenderMode::SolidWireframe => {
                self.draw_solid(&painter, model, &projected);
                self.draw_wireframe(&painter, model, &projected);
            }
            RenderMode::Points => {
                self.draw_points(&painter, model, &projected);
            }
        }
    }
    
    fn project_vertex(&self, vertex: &Vertex3D, model_center: &[f32; 3], screen_center: Pos2, scale: f32) -> (Pos2, f32) {
        let x = vertex.position[0] - model_center[0];
        let y = vertex.position[1] - model_center[1];
        let z = vertex.position[2] - model_center[2];
        
        let rot_y = self.rotation_y.to_radians();
        let rot_x = self.rotation_x.to_radians();
        
        // Rotate around Y
        let x1 = x * rot_y.cos() - z * rot_y.sin();
        let z1 = x * rot_y.sin() + z * rot_y.cos();
        
        // Rotate around X
        let y2 = y * rot_x.cos() - z1 * rot_x.sin();
        let z2 = y * rot_x.sin() + z1 * rot_x.cos();
        
        let screen_x = screen_center.x + x1 * scale;
        let screen_y = screen_center.y - y2 * scale;
        
        (Pos2::new(screen_x, screen_y), z2)
    }
    
    fn draw_wireframe(&self, painter: &egui::Painter, model: &Model3DContent, projected: &[(Pos2, f32)]) {
        for face in &model.faces {
            let n = face.vertices.len();
            for i in 0..n {
                let v1 = face.vertices[i];
                let v2 = face.vertices[(i + 1) % n];
                
                if v1 < projected.len() && v2 < projected.len() {
                    let (p1, _) = projected[v1];
                    let (p2, _) = projected[v2];
                    painter.line_segment([p1, p2], Stroke::new(1.0, self.wireframe_color));
                }
            }
        }
    }
    
    fn draw_solid(&self, painter: &egui::Painter, model: &Model3DContent, projected: &[(Pos2, f32)]) {
        // Sort faces by depth (painter's algorithm)
        let mut sorted_faces: Vec<(&Face3D, f32)> = model.faces.iter()
            .map(|face| {
                let avg_z: f32 = face.vertices.iter()
                    .filter_map(|&idx| projected.get(idx).map(|(_, z)| *z))
                    .sum::<f32>() / face.vertices.len() as f32;
                (face, avg_z)
            })
            .collect();
        
        sorted_faces.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        for (face, depth) in sorted_faces {
            if face.vertices.len() >= 3 {
                let points: Vec<Pos2> = face.vertices.iter()
                    .filter_map(|&idx| projected.get(idx).map(|(p, _)| *p))
                    .collect();
                
                if points.len() >= 3 {
                    // Simple shading based on depth
                    let shade = ((depth + 1.0) * 0.5).clamp(0.3, 1.0);
                    let color = Color32::from_rgb(
                        (self.model_color.r() as f32 * shade) as u8,
                        (self.model_color.g() as f32 * shade) as u8,
                        (self.model_color.b() as f32 * shade) as u8,
                    );
                    
                    // Draw triangulated face
                    for i in 1..points.len() - 1 {
                        let triangle = [points[0], points[i], points[i + 1]];
                        painter.add(egui::Shape::convex_polygon(
                            triangle.to_vec(),
                            color,
                            Stroke::NONE,
                        ));
                    }
                }
            }
        }
    }
    
    fn draw_points(&self, painter: &egui::Painter, _model: &Model3DContent, projected: &[(Pos2, f32)]) {
        for (pos, depth) in projected {
            let shade = ((*depth + 1.0) * 0.5).clamp(0.3, 1.0);
            let color = Color32::from_rgb(
                (200.0 * shade) as u8,
                (200.0 * shade) as u8,
                (255.0 * shade) as u8,
            );
            painter.circle_filled(*pos, 2.0, color);
        }
    }
    
    fn draw_grid(&self, painter: &egui::Painter, center: Pos2, scale: f32, _model_center: &[f32; 3]) {
        let grid_color = Color32::from_rgba_unmultiplied(100, 100, 100, 50);
        let grid_size = 10;
        let grid_step = scale * 0.1;
        
        for i in -grid_size..=grid_size {
            let offset = i as f32 * grid_step;
            
            // Lines parallel to X
            let y_offset = center.y + offset * self.rotation_x.to_radians().cos();
            painter.line_segment(
                [Pos2::new(center.x - grid_size as f32 * grid_step, y_offset),
                 Pos2::new(center.x + grid_size as f32 * grid_step, y_offset)],
                Stroke::new(0.5, grid_color),
            );
        }
    }
    
    fn draw_axes(&self, painter: &egui::Painter, center: Pos2, length: f32) {
        let rot_y = self.rotation_y.to_radians();
        let rot_x = self.rotation_x.to_radians();
        
        // X axis (red)
        let x_end = Pos2::new(
            center.x + length * rot_y.cos(),
            center.y + length * rot_y.sin() * rot_x.sin(),
        );
        painter.line_segment([center, x_end], Stroke::new(2.0, Color32::RED));
        painter.text(x_end, egui::Align2::LEFT_CENTER, "X", FontId::proportional(12.0), Color32::RED);
        
        // Y axis (green)
        let y_end = Pos2::new(
            center.x,
            center.y - length * rot_x.cos(),
        );
        painter.line_segment([center, y_end], Stroke::new(2.0, Color32::GREEN));
        painter.text(y_end, egui::Align2::CENTER_BOTTOM, "Y", FontId::proportional(12.0), Color32::GREEN);
        
        // Z axis (blue)
        let z_end = Pos2::new(
            center.x - length * rot_y.sin(),
            center.y - length * rot_y.cos() * rot_x.sin(),
        );
        painter.line_segment([center, z_end], Stroke::new(2.0, Color32::from_rgb(100, 100, 255)));
        painter.text(z_end, egui::Align2::RIGHT_CENTER, "Z", FontId::proportional(12.0), Color32::from_rgb(100, 100, 255));
    }
    
    fn render_info_panel(&self, ui: &mut egui::Ui, model: &Model3DContent) {
        egui::SidePanel::right("model_info")
            .exact_width(190.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Model Info");
                    ui.separator();
                    
                    ui.label(format!("Vertices: {}", model.vertices.len()));
                    ui.label(format!("Faces: {}", model.faces.len()));
                    
                    if !model.normals.is_empty() {
                        ui.label(format!("Normals: {}", model.normals.len()));
                    }
                    
                    if !model.texcoords.is_empty() {
                        ui.label(format!("UV coords: {}", model.texcoords.len()));
                    }
                    
                    if !model.materials.is_empty() {
                        ui.label(format!("Materials: {}", model.materials.len()));
                    }
                    
                    ui.separator();
                    ui.heading("Bounds");
                    
                    let bounds = &model.bounds;
                    let size = [
                        bounds.max[0] - bounds.min[0],
                        bounds.max[1] - bounds.min[1],
                        bounds.max[2] - bounds.min[2],
                    ];
                    
                    ui.label(format!("Size: {:.2} x {:.2} x {:.2}", size[0], size[1], size[2]));
                    ui.label(format!("Min: ({:.2}, {:.2}, {:.2})", 
                        bounds.min[0], bounds.min[1], bounds.min[2]));
                    ui.label(format!("Max: ({:.2}, {:.2}, {:.2})", 
                        bounds.max[0], bounds.max[1], bounds.max[2]));
                    
                    ui.separator();
                    ui.heading("Colors");
                    
                    ui.horizontal(|ui| {
                        ui.label("Model:");
                        ui.color_edit_button_srgba_unmultiplied(&mut [
                            self.model_color.r(),
                            self.model_color.g(),
                            self.model_color.b(),
                            255,
                        ]);
                    });
                    
                    ui.separator();
                    ui.heading("Controls");
                    ui.label("Left drag: Rotate");
                    ui.label("Right drag: Pan");
                    ui.label("Scroll: Zoom");
                });
            });
    }
}
