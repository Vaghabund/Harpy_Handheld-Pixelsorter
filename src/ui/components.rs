/// UI component rendering functions
/// 
/// EDIT THIS FILE TO CHANGE: How components are drawn (animations, effects)
/// (Colors are in styles.rs, positions are in layouts.rs)
use eframe::egui;
use crate::ui::styles::*;

// ============================================================================
// 🎬 QUICK EDIT: RENDERING PARAMETERS
// Change these values to adjust component behavior
// ============================================================================

// Button animations
const BUTTON_PRESS_SCALE: f32 = 0.95;      // Button shrinks to 95% when pressed
const BUTTON_TEXT_SIZE_RATIO: f32 = 0.33;  // Text size relative to button radius (0.33 = 33%)
const BUTTON_SHADOW_OFFSET_X: f32 = 2.0;   // Shadow horizontal offset (pixels)
const BUTTON_SHADOW_OFFSET_Y: f32 = 4.0;   // Shadow vertical offset (pixels)
const BUTTON_BORDER_WIDTH: f32 = 1.0;      // Border thickness (pixels)

// Slider behavior

// Value bubble

// Slider label

// ============================================================================
// CIRCULAR BUTTON COMPONENT
// ============================================================================

/// Render a circular button with custom fill color
pub fn circular_button(
    ui: &mut egui::Ui, 
    radius: f32, 
    text: &str, 
    base_fill: egui::Color32
) -> bool {
    let size = egui::vec2(radius * 2.0, radius * 2.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = rect.center();

        // Apply scale transform on press
        let scale = if response.is_pointer_button_down_on() { BUTTON_PRESS_SCALE } else { 1.0 };
        let scaled_radius = radius * scale;

        // Determine colors based on interaction state
        let fill_color = if response.is_pointer_button_down_on() {
            button_fill_active()
        } else if response.hovered() {
            button_fill_hover()
        } else {
            base_fill
        };

        // Draw shadow for depth
        painter.circle(
            center + egui::vec2(BUTTON_SHADOW_OFFSET_X, BUTTON_SHADOW_OFFSET_Y),
            scaled_radius,
            egui::Color32::from_black_alpha(BUTTON_SHADOW_ALPHA),
            egui::Stroke::NONE,
        );

        // Draw main circle with subtle border
        painter.circle(
            center,
            scaled_radius,
            fill_color,
            egui::Stroke::new(BUTTON_BORDER_WIDTH, button_border()),
        );

        // Draw text in center
        let font_id = egui::FontId::proportional(radius * BUTTON_TEXT_SIZE_RATIO);
        let galley = painter.layout_no_wrap(text.to_string(), font_id, egui::Color32::WHITE);
        let text_pos = center - galley.size() / 2.0;
        painter.galley(text_pos, galley);

        // Change cursor on hover
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    response.clicked()
}

// ============================================================================
// VERTICAL SLIDER COMPONENT
// ============================================================================

