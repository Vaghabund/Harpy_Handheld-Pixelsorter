use crate::PixelSorterApp;
use eframe::egui;

impl PixelSorterApp {
    /// Load the Harpy logo texture once (shared by splash, sleep and wake screens)
    fn ensure_logo_loaded(&mut self, ctx: &egui::Context) {
        if self.logo.is_none() {
            if let Some(rgba) = crate::ui::helpers::load_logo_image() {
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_flat_samples().as_slice());
                self.logo = Some(ctx.load_texture("harpy_logo", color_image, Default::default()));
            }
        }
    }

    pub fn render_splash_screen(&mut self, ctx: &egui::Context, elapsed: f32) {
        self.ensure_logo_loaded(ctx);

        // Calculate fade alpha (fade in first 0.3s, stay visible, fade out last 0.5s)
        let alpha = if elapsed < 0.3 {
            // Fade in
            elapsed / 0.3
        } else if elapsed > 1.5 {
            // Fade out
            (2.0 - elapsed) / 0.5
        } else {
            // Fully visible
            1.0
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let screen_rect = ui.max_rect();

                // Black background
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    egui::Color32::BLACK,
                );

                // Center content
                let center = screen_rect.center();

                // Draw logo
                if let Some(logo_texture) = &self.logo {
                    let logo_size = 256.0; // Size of the logo
                    let logo_rect = egui::Rect::from_center_size(
                        egui::pos2(center.x, center.y - 40.0),
                        egui::vec2(logo_size, logo_size),
                    );

                    let tint = egui::Color32::from_white_alpha((alpha * 255.0) as u8);
                    ui.painter().image(
                        logo_texture.id(),
                        logo_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                }

                // Draw "Harpy" text below logo
                let text = "Harpy";
                let font_id = egui::FontId::proportional(48.0);
                let text_color = egui::Color32::from_white_alpha((alpha * 255.0) as u8);
                let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, text_color);

                let text_pos = egui::pos2(
                    center.x - galley.size().x / 2.0,
                    center.y + 120.0,
                );
                ui.painter().galley(text_pos, galley);
            });
    }

    pub fn render_sleep_screen(&mut self, ctx: &egui::Context) {
        self.ensure_logo_loaded(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let screen_rect = ui.max_rect();

                // Add invisible full-screen button to capture all touches
                let _response = ui.allocate_rect(screen_rect, egui::Sense::click());

                // Dark background (very dark grey for OLED power saving)
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    egui::Color32::from_rgb(10, 10, 10),
                );

                // Center content
                let center = screen_rect.center();

                // Draw logo (dim for sleep mode)
                if let Some(logo_texture) = &self.logo {
                    let logo_size = 200.0; // Slightly smaller than splash
                    let logo_rect = egui::Rect::from_center_size(
                        egui::pos2(center.x, center.y - 40.0),
                        egui::vec2(logo_size, logo_size),
                    );

                    // Dim logo (50% opacity)
                    let tint = egui::Color32::from_white_alpha(128);
                    ui.painter().image(
                        logo_texture.id(),
                        logo_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                }

                // Draw "Harpy" text below logo
                let text = "Harpy";
                let font_id = egui::FontId::proportional(42.0);
                let text_color = egui::Color32::from_white_alpha(128); // Dim text
                let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, text_color);

                let text_pos = egui::pos2(
                    center.x - galley.size().x / 2.0,
                    center.y + 100.0,
                );
                ui.painter().galley(text_pos, galley);

                // Draw "Touch to wake" hint
                let hint_text = "Touch to wake";
                let hint_font = egui::FontId::proportional(18.0);
                let hint_color = egui::Color32::from_white_alpha(80); // Very dim
                let hint_galley = ui.painter().layout_no_wrap(hint_text.to_string(), hint_font, hint_color);

                let hint_pos = egui::pos2(
                    center.x - hint_galley.size().x / 2.0,
                    center.y + 160.0,
                );
                ui.painter().galley(hint_pos, hint_galley);
            });
    }

    pub fn render_waking_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let screen_rect = ui.max_rect();

                // Same dark background as sleep
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    egui::Color32::from_rgb(10, 10, 10),
                );

                let center = screen_rect.center();

                // Draw logo (full brightness for waking)
                if let Some(logo_texture) = &self.logo {
                    let logo_size = 200.0;
                    let logo_rect = egui::Rect::from_center_size(
                        egui::pos2(center.x, center.y - 40.0),
                        egui::vec2(logo_size, logo_size),
                    );

                    // Full brightness logo
                    let tint = egui::Color32::WHITE;
                    ui.painter().image(
                        logo_texture.id(),
                        logo_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                }

                // Draw "Waking up..." text
                let text = "Waking up...";
                let font_id = egui::FontId::proportional(48.0); // Larger font
                let text_color = egui::Color32::WHITE; // Full brightness
                let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, text_color);

                let text_pos = egui::pos2(
                    center.x - galley.size().x / 2.0,
                    center.y + 100.0,
                );
                ui.painter().galley(text_pos, galley);
            });
    }
}
