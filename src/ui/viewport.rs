use crate::PixelSorterApp;
use crate::ui::state::{Phase, DragState, HandlePosition};
use crate::ui::helpers::{fit_image_in_rect, cover_image_in_rect, center_rect_in_rect};
use eframe::egui;

const HANDLE_SIZE: f32 = 28.0;

impl PixelSorterApp {
    pub fn render_viewport(&mut self, ui: &mut egui::Ui, rect: egui::Rect, ctx: &egui::Context) {
        match self.current_phase {
            Phase::Input => self.render_input_viewport(ui, rect),
            Phase::Edit => self.render_edit_viewport(ui, rect),
            Phase::Crop => self.render_crop_viewport(ui, rect, ctx),
        }
    }

    fn render_input_viewport(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        // Draw black background
        ui.painter().rect_filled(
            rect,
            0.0,
            egui::Color32::BLACK,
        );

        if let Some(texture) = &self.camera_texture {
            // Cover mode: fill entire screen, crop overflow
            let image_size = texture.size_vec2();
            let display_size = cover_image_in_rect(image_size, rect.size());
            let centered_rect = center_rect_in_rect(display_size, rect);

            ui.allocate_ui_at_rect(centered_rect, |ui| {
                ui.add(egui::Image::new(texture).fit_to_exact_size(display_size));
            });
        } else {
            ui.allocate_ui_at_rect(rect, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("No camera available");
                });
            });
        }
    }

    fn render_edit_viewport(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        // Draw black background
        ui.painter().rect_filled(
            rect,
            0.0,
            egui::Color32::BLACK,
        );

        if let Some(texture) = &self.processed_texture {
            let image_size = texture.size_vec2();
            let display_size = fit_image_in_rect(image_size, rect.size());
            let centered_rect = center_rect_in_rect(display_size, rect);

            ui.allocate_ui_at_rect(centered_rect, |ui| {
                ui.add(egui::Image::new(texture).fit_to_exact_size(display_size));
            });
        } else {
            ui.allocate_ui_at_rect(rect, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("No image");
                });
            });
        }
    }

    fn render_crop_viewport(&mut self, ui: &mut egui::Ui, rect: egui::Rect, ctx: &egui::Context) {
        // Draw black background
        ui.painter().rect_filled(
            rect,
            0.0,
            egui::Color32::BLACK,
        );

        if let Some(texture) = &self.processed_texture {
            let image_size = texture.size_vec2();
            let display_size = fit_image_in_rect(image_size, rect.size());
            let centered_rect = center_rect_in_rect(display_size, rect);

            ui.allocate_ui_at_rect(centered_rect, |ui| {
                ui.add(egui::Image::new(texture).fit_to_exact_size(display_size));
            });

            // Draw overlay and crop handles
            self.render_crop_overlay(ui, centered_rect, image_size, ctx);
        }
    }

    fn render_crop_overlay(
        &mut self,
        ui: &mut egui::Ui,
        display_rect: egui::Rect,
        image_size: egui::Vec2,
        _ctx: &egui::Context,
    ) {
        // Scale factor from image to display coordinates
        let scale_x = display_rect.width() / image_size.x;
        let scale_y = display_rect.height() / image_size.y;
        let scale = scale_x.min(scale_y);

        // Initialize crop rect if needed
        if self.crop_rect.is_none() {
            let margin = 50.0;
            self.crop_rect = Some(egui::Rect::from_min_max(
                egui::pos2(margin, margin),
                egui::pos2(image_size.x - margin, image_size.y - margin),
            ));
        }

        let crop_rect = self.crop_rect.unwrap();

        // Convert crop rect to display coordinates
        let crop_display = egui::Rect::from_min_max(
            display_rect.min + egui::vec2(crop_rect.min.x * scale, crop_rect.min.y * scale),
            display_rect.min + egui::vec2(crop_rect.max.x * scale, crop_rect.max.y * scale),
        );

        // Handle interactions first (before borrowing painter)
        self.handle_crop_interactions(ui, crop_display, display_rect, image_size, scale);

        // Now borrow painter for drawing
        let painter = ui.painter();

        // Draw grey overlay outside crop area
        let grey = egui::Color32::from_black_alpha(180);

        // Top
        painter.rect_filled(
            egui::Rect::from_min_max(display_rect.min, egui::pos2(display_rect.max.x, crop_display.min.y)),
            0.0,
            grey,
        );
        // Bottom
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(display_rect.min.x, crop_display.max.y), display_rect.max),
            0.0,
            grey,
        );
        // Left
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(display_rect.min.x, crop_display.min.y),
                egui::pos2(crop_display.min.x, crop_display.max.y),
            ),
            0.0,
            grey,
        );
        // Right
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(crop_display.max.x, crop_display.min.y),
                egui::pos2(display_rect.max.x, crop_display.max.y),
            ),
            0.0,
            grey,
        );

        // Draw crop border
        painter.rect_stroke(crop_display, 0.0, egui::Stroke::new(3.0_f32, egui::Color32::WHITE));

        // Draw handles
        self.draw_crop_handles(painter, crop_display);
    }

    fn handle_crop_interactions(
        &mut self,
        ui: &mut egui::Ui,
        crop_display: egui::Rect,
        display_rect: egui::Rect,
        image_size: egui::Vec2,
        scale: f32,
    ) {
        let handles = [
            (HandlePosition::TopLeft, crop_display.left_top()),
            (HandlePosition::TopRight, crop_display.right_top()),
            (HandlePosition::BottomLeft, crop_display.left_bottom()),
            (HandlePosition::BottomRight, crop_display.right_bottom()),
        ];

        // Check handle interactions
        for (handle_pos, handle_center) in handles {
            let handle_rect = egui::Rect::from_center_size(handle_center, egui::vec2(HANDLE_SIZE, HANDLE_SIZE));
            let response = ui.interact(handle_rect, ui.id().with(format!("{:?}", handle_pos)), egui::Sense::drag());

            if response.drag_started() {
                self.drag_state = DragState::DraggingHandle(handle_pos);
            }

            if response.dragged() && self.drag_state == DragState::DraggingHandle(handle_pos) {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.update_crop_rect_from_handle(handle_pos, pos, display_rect, image_size, scale);
                }
            }
        }

        // Move crop area by dragging inside
        let crop_response = ui.interact(crop_display, ui.id().with("crop_move"), egui::Sense::drag());

        if crop_response.drag_started() && self.drag_state == DragState::None {
            self.drag_state = DragState::MovingCrop;
        }

        if crop_response.dragged() && self.drag_state == DragState::MovingCrop {
            let delta = crop_response.drag_delta() / scale;
            if let Some(rect) = self.crop_rect {
                // Clamp the movement (not the corners) so the box stops at the
                // image edge instead of shrinking
                let dx = delta.x.max(-rect.min.x).min(image_size.x - rect.max.x);
                let dy = delta.y.max(-rect.min.y).min(image_size.y - rect.max.y);
                self.crop_rect = Some(rect.translate(egui::vec2(dx, dy)));
            }
        }

        // Reset drag state on release
        if ui.input(|i| i.pointer.any_released()) {
            self.drag_state = DragState::None;
        }
    }

    fn update_crop_rect_from_handle(
        &mut self,
        handle: HandlePosition,
        screen_pos: egui::Pos2,
        display_rect: egui::Rect,
        image_size: egui::Vec2,
        scale: f32,
    ) {
        if let Some(mut rect) = self.crop_rect {
            // Convert screen position to image coordinates
            let image_pos = (screen_pos - display_rect.min) / scale;

            // Update rect based on which handle
            match handle {
                HandlePosition::TopLeft => {
                    rect.min = egui::pos2(
                        image_pos.x.max(0.0).min(rect.max.x - 10.0),
                        image_pos.y.max(0.0).min(rect.max.y - 10.0),
                    );
                }
                HandlePosition::TopRight => {
                    rect.min.y = image_pos.y.max(0.0).min(rect.max.y - 10.0);
                    rect.max.x = image_pos.x.min(image_size.x).max(rect.min.x + 10.0);
                }
                HandlePosition::BottomLeft => {
                    rect.min.x = image_pos.x.max(0.0).min(rect.max.x - 10.0);
                    rect.max.y = image_pos.y.min(image_size.y).max(rect.min.y + 10.0);
                }
                HandlePosition::BottomRight => {
                    rect.max = egui::pos2(
                        image_pos.x.min(image_size.x).max(rect.min.x + 10.0),
                        image_pos.y.min(image_size.y).max(rect.min.y + 10.0),
                    );
                }
            }

            self.crop_rect = Some(rect);
        }
    }

    fn draw_crop_handles(&self, painter: &egui::Painter, crop_display: egui::Rect) {
        let handle_color = egui::Color32::WHITE;

        // Corner handles
        let handles = [
            crop_display.left_top(),
            crop_display.right_top(),
            crop_display.left_bottom(),
            crop_display.right_bottom(),
        ];

        for center in handles {
            painter.circle_filled(center, HANDLE_SIZE / 2.0, handle_color);
            painter.circle_stroke(center, HANDLE_SIZE / 2.0, egui::Stroke::new(2.0_f32, egui::Color32::BLACK));
        }
    }
}
