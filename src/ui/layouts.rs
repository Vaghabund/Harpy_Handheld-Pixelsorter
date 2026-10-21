/// Phase-specific layout and positioning logic
/// 
/// EDIT THIS FILE TO CHANGE: Positions, padding, spacing, alignment
/// (Colors/appearance are in styles.rs)
use crate::PixelSorterApp;
use crate::ui::state::Phase;
use crate::ui::components::circular_button;
use crate::ui::styles::{ButtonSizes, button_green, button_red};
use eframe::egui;

// ============================================================================
// 📐 QUICK EDIT: LAYOUT PARAMETERS
// Change these values to adjust spacing and positioning
// ============================================================================

// Edit Phase - Button row positioning

// Edit Phase - Button horizontal positioning

// Edit Phase - Slider positioning

// Slider value ranges

// USB Export button

// Crop Phase - Button positioning
const CROP_BUTTON_VERTICAL_SPACING_MULTIPLIER: f32 = 2.0; // Spacing between Cancel/Apply buttons
const CROP_BUTTON_TOTAL_HEIGHT_MULTIPLIER: f32 = 4.0;     // Total height calculation (radius * this value)
const CROP_APPLY_BUTTON_Y_MULTIPLIER: f32 = 3.0;          // Apply button Y position (radius * this value)

impl PixelSorterApp {
    /// Main entry point for rendering phase-specific button overlays
    pub fn render_button_overlay(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, screen_rect: egui::Rect) {
        match self.current_phase {
            Phase::Input | Phase::Edit => self.render_controls(ui, ctx, screen_rect),
            Phase::Crop => self.render_crop_layout(ctx, screen_rect),
            // File screens draw their own buttons
            Phase::Gallery | Phase::Browse => {}
        }
    }

    // ============================================================================
    // PHASE 3: CROP LAYOUT - Cancel/Apply buttons on left
    // ============================================================================
    fn render_crop_layout(&mut self, ctx: &egui::Context, screen_rect: egui::Rect) {
        let sizes = ButtonSizes::standard();

        let left_x = sizes.spacing + sizes.normal_radius;
        let button_vertical_spacing = sizes.spacing * CROP_BUTTON_VERTICAL_SPACING_MULTIPLIER;

        // Center buttons vertically
        let total_height = sizes.normal_radius * CROP_BUTTON_TOTAL_HEIGHT_MULTIPLIER + button_vertical_spacing;
        let start_y = (screen_rect.height() - total_height) / 2.0 + screen_rect.min.y;

        // Cancel button (top)
        egui::Area::new("cancel_crop_btn")
            .fixed_pos(egui::pos2(left_x, start_y + sizes.normal_radius) 
                - egui::vec2(sizes.normal_radius, sizes.normal_radius))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button(ui, sizes.normal_radius, "Cancel", button_red()) {
                    // Discard a crop that is still being applied
                    if matches!(&self.sort_job, Some(job) if job.target == crate::processing::image_ops::SortTarget::Crop) {
                        self.cancel_sort();
                    }
                    self.current_phase = Phase::Edit;
                    self.crop_rect = None;
                }
            });

        // Apply Crop button (bottom)
        egui::Area::new("apply_crop_btn")
            .fixed_pos(egui::pos2(
                left_x,
                start_y + sizes.normal_radius * CROP_APPLY_BUTTON_Y_MULTIPLIER + button_vertical_spacing
            ) - egui::vec2(sizes.normal_radius, sizes.normal_radius))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button(ui, sizes.normal_radius, "Apply", button_green()) {
                    self.apply_crop_and_sort(ctx);
                }
            });
    }
}
