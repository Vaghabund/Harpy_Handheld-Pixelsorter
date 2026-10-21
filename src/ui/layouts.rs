/// Phase-specific layout and positioning logic
/// 
/// EDIT THIS FILE TO CHANGE: Positions, padding, spacing, alignment
/// (Colors/appearance are in styles.rs)
use crate::PixelSorterApp;
use crate::ui::state::Phase;
use crate::ui::theme;
use eframe::egui;

impl PixelSorterApp {
    /// Main entry point for rendering phase-specific button overlays
    pub fn render_button_overlay(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, screen_rect: egui::Rect) {
        match self.current_phase {
            Phase::Input | Phase::Edit => self.render_controls(ui, ctx, screen_rect),
            Phase::Crop => self.render_crop_layout(ui, ctx, screen_rect),
            // File screens draw their own buttons
            Phase::Gallery | Phase::Browse => {}
        }
    }

    /// Crop: plain-text Cancel / Apply centred at the bottom. Drawn in the
    /// viewport's ui after the crop handles, so the buttons win where they overlap.
    fn render_crop_layout(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, screen_rect: egui::Rect) {
        let s = theme::scale(ctx);
        let font = theme::medium(28.0 * s);
        let cancel = theme::galley(ctx, "Cancel", font.clone(), egui::Color32::from_white_alpha(160), -0.02);
        let apply = theme::galley(ctx, "Apply", font, egui::Color32::WHITE, -0.02);
        let y = screen_rect.max.y - 36.0 * s;
        let gap = 28.0 * s;
        let cx = screen_rect.center().x;
        if theme::text_button(ui, "crop_cancel", egui::pos2(cx - gap, y), egui::Align2::RIGHT_BOTTOM, cancel, false, true).clicked() {
            // Discard a crop that is still being applied
            if matches!(&self.sort_job, Some(job) if job.target == crate::processing::image_ops::SortTarget::Crop) {
                self.cancel_sort();
            }
            self.current_phase = Phase::Edit;
            self.crop_rect = None;
        }
        if theme::text_button(ui, "crop_apply", egui::pos2(cx + gap, y), egui::Align2::LEFT_BOTTOM, apply, true, true).clicked() {
            self.apply_crop_and_sort(ctx);
        }
    }
}
