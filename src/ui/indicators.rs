use crate::PixelSorterApp;
use crate::ui::theme;
use eframe::egui;
use egui::Color32;

impl PixelSorterApp {
    /// Show a toast message for a few seconds (✔ prefix = success styling)
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

        if let Some(message) = self.export_message.clone() {
            // Errors and warnings are red, everything else a white pill
            let is_error = message.starts_with(['❌', '⚠']);
            let (fill, color) = if is_error { (theme::RED, Color32::WHITE) } else { (Color32::WHITE, Color32::BLACK) };
            let s = theme::scale(ctx);
            let text = toast_text(&message);

            egui::Area::new("export_message")
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 16.0 * s))
                .order(egui::Order::Tooltip)
                .interactable(false)
                .show(ctx, |ui| {
                    let g = theme::caps(ctx, &text, 13.0 * s, color);
                    let size = g.size() + egui::vec2(40.0, 22.0) * s;
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    ui.painter().rect_filled(rect.translate(egui::vec2(0.0, 2.0 * s)), rect.height() / 2.0, Color32::from_black_alpha(60));
                    ui.painter().rect_filled(rect, rect.height() / 2.0, fill);
                    ui.painter().galley(rect.center() - g.size() / 2.0, g);
                });
        }
    }
}

/// Drop the emoji status prefix ("✔ Saved" -> "Saved"); the pill colour carries it
fn toast_text(message: &str) -> String {
    message.trim_start_matches(|c: char| !c.is_alphanumeric()).to_string()
}

#[cfg(test)]
mod tests {
    use super::toast_text;

    #[test]
    fn toast_text_strips_status_prefix() {
        assert_eq!(toast_text("✔ Session deleted"), "Session deleted");
        assert_eq!(toast_text("❌ Delete failed: x"), "Delete failed: x");
        assert_eq!(toast_text("⚠️ Low battery"), "Low battery");
        assert_eq!(toast_text("Copying 3 images"), "Copying 3 images");
    }
}
