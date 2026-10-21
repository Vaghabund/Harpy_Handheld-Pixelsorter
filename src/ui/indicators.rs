use crate::PixelSorterApp;
use eframe::egui;

const UI_PADDING: f32 = 20.0;

impl PixelSorterApp {
    /// Show a toast message for a few seconds (✓ prefix = success styling)
    pub fn show_message(&mut self, message: impl Into<String>) {
        self.export_message = Some(message.into());
        self.export_message_time = Some(std::time::Instant::now());
    }

    pub fn render_export_message(&mut self, ctx: &egui::Context, _screen_rect: egui::Rect) {
        // Auto-hide message after 3 seconds
        if let Some(message_time) = self.export_message_time {
            if message_time.elapsed().as_secs() > 3 {
                self.export_message = None;
                self.export_message_time = None;
            }
        }

        if let Some(ref message) = self.export_message {
            let is_success = message.starts_with('✓');

            egui::Area::new("export_message")
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, UI_PADDING * 3.0))
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(if is_success {
                            egui::Color32::from_rgb(40, 120, 40) // Green for success
                        } else {
                            egui::Color32::from_rgb(180, 40, 40) // Red for error
                        })
                        .rounding(16.0) // Doubled from 8.0
                        .inner_margin(egui::Margin::symmetric(40.0, 30.0)) // Doubled from (20.0, 15.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(message)
                                    .color(egui::Color32::WHITE)
                                    .size(40.0) // Doubled from 20.0
                            );
                        });
                });
        }
    }

    pub fn render_battery_indicator(&mut self, ctx: &egui::Context, _screen_rect: egui::Rect) {
        let battery_status = crate::hardware::get_battery_status();

        // Only show if battery is available
        if !battery_status.is_available {
            return;
        }

        egui::Area::new("battery_indicator")
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-UI_PADDING, UI_PADDING))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                    .rounding(egui::Rounding::same(16.0)) // Doubled from 8.0
                    .inner_margin(egui::Margin::symmetric(24.0, 16.0)) // Doubled from (12.0, 8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Battery icon (simple rectangle representation)
                            let icon_size = egui::vec2(60.0, 32.0); // Doubled from (30.0, 16.0)
                            let (icon_rect, _) = ui.allocate_exact_size(icon_size, egui::Sense::hover());

                            // Determine battery color based on percentage
                            let battery_color = if battery_status.is_charging {
                                egui::Color32::from_rgb(100, 200, 100) // Green when charging
                            } else if battery_status.percentage < 20.0 {
                                egui::Color32::from_rgb(220, 50, 50) // Red when low
                            } else if battery_status.percentage < 40.0 {
                                egui::Color32::from_rgb(220, 180, 50) // Yellow when medium
                            } else {
                                egui::Color32::from_rgb(150, 150, 150) // Grey when good
                            };

                            // Draw battery outline
                            ui.painter().rect_stroke(
                                icon_rect,
                                4.0, // Doubled from 2.0
                                egui::Stroke::new(4.0_f32, egui::Color32::WHITE), // Doubled from 2.0
                            );

                            // Draw battery fill
                            let fill_width = (icon_rect.width() - 8.0) * (battery_status.percentage / 100.0); // Doubled padding from 4.0
                            let fill_rect = egui::Rect::from_min_size(
                                egui::pos2(icon_rect.min.x + 4.0, icon_rect.min.y + 4.0), // Doubled from 2.0
                                egui::vec2(fill_width, icon_rect.height() - 8.0), // Doubled from 4.0
                            );
                            ui.painter().rect_filled(fill_rect, 2.0, battery_color); // Doubled from 1.0

                            // Draw battery terminal (small nub on right)
                            let terminal_rect = egui::Rect::from_min_size(
                                egui::pos2(icon_rect.max.x, icon_rect.min.y + 8.0), // Doubled from 4.0
                                egui::vec2(6.0, icon_rect.height() - 16.0), // Doubled from (3.0, 8.0)
                            );
                            ui.painter().rect_filled(terminal_rect, 2.0, egui::Color32::WHITE); // Doubled from 1.0

                            ui.add_space(8.0); // Doubled from 4.0

                            // Text with percentage and voltage
                            let text = if battery_status.is_charging {
                                format!("⚡ {:.0}%", battery_status.percentage)
                            } else {
                                format!("{:.0}%", battery_status.percentage)
                            };

                            ui.label(
                                egui::RichText::new(text)
                                    .color(egui::Color32::WHITE)
                                    .size(32.0) // Doubled from 16.0
                            );

                            // Show voltage in smaller text
                            ui.label(
                                egui::RichText::new(format!("{:.1}V", battery_status.voltage))
                                    .color(egui::Color32::from_rgb(180, 180, 180))
                                    .size(24.0) // Doubled from 12.0
                            );
                        });
                    });
            });
    }
}
