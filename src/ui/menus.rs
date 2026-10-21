use crate::PixelSorterApp;
use crate::system::SystemControl;
use eframe::egui;
use super::styles::{MenuStyle, button_text, menu_button};


impl PixelSorterApp {
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
}
