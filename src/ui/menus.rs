use crate::PixelSorterApp;
use crate::system::SystemControl;
use eframe::egui;
use super::styles::{MenuStyle, button_text, menu_button};

const UI_PADDING: f32 = 20.0;

impl PixelSorterApp {
    pub fn render_shutdown_button(&mut self, ctx: &egui::Context, _screen_rect: egui::Rect) {
        egui::Area::new("shutdown_button")
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(UI_PADDING, UI_PADDING))
            .order(egui::Order::Debug)
            .show(ctx, |ui| {
                let button_size = 80.0; // Doubled from 40.0
                let button_pos = ui.cursor().min;
                let button_rect = egui::Rect::from_min_size(
                    button_pos,
                    egui::vec2(button_size, button_size),
                );

                let response = ui.allocate_rect(button_rect, egui::Sense::click());

                // Draw power icon (circle with vertical line at top)
                let center = button_rect.center();
                let radius = button_size * 0.35;

                // Glassmorphism background
                let bg_color = if response.is_pointer_button_down_on() {
                    egui::Color32::from_rgba_unmultiplied(255, 80, 80, 200) // Bright red on press
                } else if response.hovered() {
                    egui::Color32::from_rgba_unmultiplied(220, 50, 50, 180) // Red on hover
                } else {
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 38) // Glassmorphism rgba(255, 255, 255, 0.15)
                };

                // Apply scale transform on press
                let scale = if response.is_pointer_button_down_on() { 0.95 } else { 1.0 };
                let scaled_radius = button_size * 0.45 * scale;

                ui.painter().circle_filled(center, scaled_radius, bg_color);

                // Power symbol: partial circle (arc) + line
                let stroke = egui::Stroke::new(6.0_f32, egui::Color32::WHITE); // Doubled from 3.0

                // Vertical line (power button line)
                let line_start = egui::pos2(center.x, center.y - radius * 0.8);
                let line_end = egui::pos2(center.x, center.y + radius * 0.3);
                ui.painter().line_segment([line_start, line_end], stroke);

                // Arc (incomplete circle)
                use std::f32::consts::PI;
                let num_points = 20;
                let start_angle = PI * 0.7; // Start at bottom-left
                let end_angle = PI * 2.3;   // End at bottom-right

                let mut points = Vec::new();
                for i in 0..=num_points {
                    let t = i as f32 / num_points as f32;
                    let angle = start_angle + (end_angle - start_angle) * t;
                    let x = center.x + radius * angle.cos();
                    let y = center.y + radius * angle.sin();
                    points.push(egui::pos2(x, y));
                }

                for i in 0..points.len() - 1 {
                    ui.painter().line_segment([points[i], points[i + 1]], stroke);
                }

                // Handle click
                if response.clicked() {
                    log::info!("Shutdown button clicked - opening menu");
                    self.show_shutdown_menu = true;
                }
            });

        // Show shutdown confirmation menu
        if self.show_shutdown_menu {
            log::debug!("Rendering power menu - show_shutdown_menu is true");
            let style = MenuStyle::power();
            
            let response = egui::Window::new("Power Options")
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.set_min_width(style.width);

                    ui.vertical_centered(|ui| {
                        // Shutdown button
                        if menu_button(ui, [style.button_width, style.button_height], "🔌 Shutdown", style.label_size) {
                            log::info!("Shutdown requested by user");
                            if let Err(e) = SystemControl::shutdown() {
                                log::error!("Failed to shutdown: {}", e);
                                self.show_message(format!("❌ Shutdown failed: {}", e));
                            }
                            self.show_shutdown_menu = false;
                        }

                        ui.add_space(style.spacing);

                        // Reboot button
                        if menu_button(ui, [style.button_width, style.button_height], "🔄 Reboot", style.label_size) {
                            log::info!("Reboot requested by user");
                            if let Err(e) = SystemControl::reboot() {
                                log::error!("Failed to reboot: {}", e);
                                self.show_message(format!("❌ Reboot failed: {}", e));
                            }
                            self.show_shutdown_menu = false;
                        }

                        ui.add_space(style.spacing);

                        ui.add_space(style.spacing * 0.5);

                        // Cancel button
                        if menu_button(ui, [style.button_width, style.cancel_button_height], "Cancel", style.label_size) {
                            self.show_shutdown_menu = false;
                        }
                    });
                });
            
            // Force window to top layer
            if let Some(response) = response {
                ctx.move_to_top(response.response.layer_id);
            }
        }
    }

    pub fn render_developer_menu(&mut self, ctx: &egui::Context, _screen_rect: egui::Rect) {
        if !self.show_developer_menu {
            return;
        }

        let style = MenuStyle::developer();

        let response = egui::Window::new("🛠 Developer Menu")
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_min_width(style.width);

                ui.heading(button_text("Developer Tools", style.heading_size));
                ui.add_space(style.spacing);

                ui.vertical_centered(|ui| {
                    // System info section
                    ui.group(|ui| {
                        ui.set_min_width(style.button_width);
                        ui.label(button_text("System Info", style.label_size).strong());
                        ui.separator();

                        // Battery status
                        let battery = crate::hardware::get_battery_status();
                        if battery.is_available {
                            let battery_text = if battery.is_charging {
                                format!("🔋 Battery: {:.0}% ({:.1}V) ⚡ Charging",
                                       battery.percentage, battery.voltage)
                            } else {
                                format!("🔋 Battery: {:.0}% ({:.1}V)",
                                       battery.percentage, battery.voltage)
                            };
                            ui.label(button_text(&battery_text, style.label_size));
                        } else {
                            ui.label(button_text("🔋 Battery: Not detected", style.label_size));
                        }

                        // Current phase
                        ui.label(button_text(&format!("📍 Phase: {:?}", self.current_phase), style.label_size));

                        // Session info
                        if let Some(ref session) = self.current_session_folder {
                            ui.label(button_text(&format!("📁 Session: {}", session), style.label_size));
                            ui.label(button_text(&format!("🔢 Iteration: {}", self.iteration_counter), style.label_size));
                        }
                    });

                    ui.add_space(style.spacing);

                    // Actions section
                    ui.group(|ui| {
                        ui.set_min_width(style.button_width);
                        ui.label(button_text("Actions", style.label_size).strong());
                        ui.separator();

                        // Update status
                        if self.update_manager.update_available {
                            ui.label(egui::RichText::new("🆕 Update Available!").color(egui::Color32::from_rgb(100, 220, 100)).size(style.label_size));
                            ui.add_space(5.0);

                            // Restart & Update button
                            if menu_button(ui, [style.button_width, style.button_height], "🔄 Pull & Restart", style.label_size) {
                                log::info!("Pull & Restart requested");
                                self.show_message("🔄 Pulling updates and restarting...");
                                self.show_developer_menu = false;

                                // Pull updates and restart service using update_manager
                                // (only returns if the update could not be started)
                                if let Err(e) = self.update_manager.pull_and_restart_service("pixelsort-kiosk") {
                                    log::error!("Failed to start update: {}", e);
                                    self.show_message(format!("❌ Update failed: {}", e));
                                }
                            }
                        } else {
                            ui.label(egui::RichText::new("✅ App is up to date").color(egui::Color32::GRAY).size(style.label_size));
                            ui.add_space(5.0);

                            // Manual check button (result is shown as a toast when the check finishes)
                            if self.update_check_task.is_some() {
                                ui.add_enabled_ui(false, |ui| {
                                    menu_button(ui, [style.button_width, style.button_height], "⏳ Checking...", style.label_size);
                                });
                            } else if menu_button(ui, [style.button_width, style.button_height], "🔄 Check Now", style.label_size) {
                                log::info!("Manual update check requested");
                                self.start_update_check(ctx, true);
                                // Don't close menu - let user see the result
                            }
                        }

                        ui.add_space(style.spacing * 0.5);                        // Clear session
                        if menu_button(ui, [style.button_width, style.button_height], "🗑 Clear Session", style.label_size) {
                            self.iteration_counter = 0;
                            self.current_session_folder = None;
                            self.show_message("✔ Session cleared");
                            log::info!("Session manually cleared");
                            self.show_developer_menu = false;
                        }

                        ui.add_space(style.spacing * 0.5);

                        // Restart app
                        if menu_button(ui, [style.button_width, style.button_height], "🔁 Restart App", style.label_size) {
                            log::info!("App restart requested");
                            self.show_message("🔁 Restarting...");
                            // A clean exit doesn't trigger Restart=on-failure, so ask systemd
                            // to restart the service; fall back to closing if not under systemd
                            if let Err(e) = SystemControl::restart_service("pixelsort-kiosk") {
                                log::warn!("Service restart failed ({}), closing app instead", e);
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }

                        ui.add_space(style.spacing * 0.5);

                        // Power off: in the kiosk there is no desktop to exit to
                        if menu_button(ui, [style.button_width, style.button_height], "🔌 Power Off", style.label_size) {
                            log::info!("Power off requested from dev menu");
                            if let Err(e) = SystemControl::shutdown() {
                                // Not on the device (e.g. desktop development): just close the app
                                log::warn!("Shutdown unavailable ({}), closing app instead", e);
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                    });

                    ui.add_space(style.spacing);

                    // Close button
                    if menu_button(ui, [style.button_width, style.cancel_button_height], "Close Menu", style.label_size) {
                        self.show_developer_menu = false;
                    }
                });

                ui.add_space(style.spacing * 0.5);
                ui.label(
                    button_text("Tip: Press ESC or 5-tap bottom-right corner to toggle this menu", 16.0)
                        .size(18.0)
                        .color(egui::Color32::GRAY)
                );
            });
        
        // Force window to top layer
        if let Some(response) = response {
            ctx.move_to_top(response.response.layer_id);
        }
    }

    pub fn render_usb_export_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_usb_export_dialog {
            return;
        }

        let style = MenuStyle::usb_export();

        let response = egui::Window::new("💾 Export to USB")
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_min_width(style.width);

                ui.vertical_centered(|ui| {
                    ui.add_space(style.spacing * 0.67);
                    
                    if self.usb_export_choices.is_empty() {
                        ui.label(button_text("No saved images to export yet", style.heading_size));
                        ui.add_space(style.spacing);
                    } else {
                        ui.label(button_text("📁 Copy sorted images to USB", style.heading_size));
                        ui.add_space(style.spacing);

                        // Finger-sized checkboxes and rows
                        ui.spacing_mut().icon_width = 32.0;
                        ui.spacing_mut().icon_width_inner = 20.0;
                        ui.spacing_mut().icon_spacing = 12.0;
                        ui.spacing_mut().item_spacing.y = 12.0;

                        // Select all / none
                        let all_selected = self.usb_export_choices.iter().all(|(_, selected)| *selected);
                        let mut select_all = all_selected;
                        ui.checkbox(&mut select_all, button_text("All sessions", style.label_size));
                        if select_all != all_selected {
                            for (_, selected) in &mut self.usb_export_choices {
                                *selected = select_all;
                            }
                        }
                        ui.separator();

                        // One checkbox per session, newest first
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            for (session, selected) in &mut self.usb_export_choices {
                                let label = format!("{}  ({} images)", session.name, session.images.len());
                                ui.checkbox(selected, button_text(&label, style.label_size * 0.9));
                            }
                        });
                        ui.separator();
                        ui.add_space(style.spacing * 0.5);

                        ui.checkbox(&mut self.usb_export_delete_after,
                            button_text("Delete selected after copying", style.label_size * 0.9));

                        ui.add_space(style.spacing * 1.33);

                        let selected_count: usize = self.usb_export_choices.iter()
                            .filter(|(_, selected)| *selected)
                            .map(|(session, _)| session.images.len())
                            .sum();
                        let copy_label = format!("✔ Copy {} images to USB", selected_count);
                        let copy_clicked = ui.add_enabled(
                            selected_count > 0,
                            egui::Button::new(button_text(&copy_label, style.label_size))
                                .fill(egui::Color32::from_rgb(40, 120, 40))
                                .min_size(egui::vec2(style.button_width, style.button_height)),
                        ).clicked();
                        if copy_clicked {
                            self.start_usb_export(ctx);
                            self.show_usb_export_dialog = false;
                        }

                        ui.add_space(style.spacing * 0.67);
                    }

                    // Cancel button
                    if menu_button(ui, [style.button_width, style.cancel_button_height], "Cancel", style.label_size) {
                        self.show_usb_export_dialog = false;
                        self.usb_export_delete_after = false; // Reset
                    }
                });
            });
        
        // Force window to top layer
        if let Some(response) = response {
            ctx.move_to_top(response.response.layer_id);
        }
    }

    /// Confirmation shown by "New" when the current image has unsaved edits
    pub fn render_new_session_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_new_session_dialog {
            return;
        }

        let style = MenuStyle::usb_export();

        let response = egui::Window::new("📷 New Photo")
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_min_width(style.width);

                ui.vertical_centered(|ui| {
                    ui.add_space(style.spacing * 0.67);
                    ui.label(button_text("Save before starting a new photo?", style.heading_size));
                    ui.add_space(style.spacing * 1.33);

                    if ui.add_sized(
                        [style.button_width, style.button_height],
                        egui::Button::new(button_text("💾 Save", style.label_size))
                            .fill(egui::Color32::from_rgb(40, 120, 40)))
                        .clicked()
                    {
                        self.save_and_start_new_photo_session();
                    }

                    ui.add_space(style.spacing * 0.67);

                    if menu_button(ui, [style.button_width, style.button_height], "🗑 Discard", style.label_size) {
                        log::info!("Discarding unsaved image for new photo");
                        self.start_new_photo_session();
                    }

                    ui.add_space(style.spacing * 0.67);

                    if menu_button(ui, [style.button_width, style.cancel_button_height], "Cancel", style.label_size) {
                        self.show_new_session_dialog = false;
                    }
                });
            });

        // Force window to top layer
        if let Some(response) = response {
            ctx.move_to_top(response.response.layer_id);
        }
    }
}
