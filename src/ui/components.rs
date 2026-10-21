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
const SLIDER_RAIL_SHRINK_RATIO: f32 = 0.25; // Rail width relative to slider width (0.25 = 25% on each side)
const SLIDER_KNOB_BORDER_RATIO: f32 = 0.13;  // Knob border width relative to radius (0.13 = 13%)

// Value bubble
const BUBBLE_FONT_SIZE: f32 = 18.0;          // Font size for value display
const BUBBLE_PADDING_X: f32 = 20.0;          // Horizontal padding inside bubble
const BUBBLE_PADDING_Y: f32 = 12.0;          // Vertical padding inside bubble
const BUBBLE_OFFSET_X: f32 = 12.0;           // Distance from slider (pixels)
const BUBBLE_CORNER_RADIUS: f32 = 6.0;       // Bubble corner rounding

// Slider label
const LABEL_FONT_SIZE: f32 = 18.0;           // Font size for slider label
const LABEL_OFFSET_Y: f32 = 40.0;            // Distance below slider (pixels)
const LABEL_BG_PADDING_X: f32 = 4.0;         // Label background padding X
const LABEL_BG_PADDING_Y: f32 = 2.0;         // Label background padding Y
const LABEL_BG_CORNER_RADIUS: f32 = 3.0;     // Label background corner rounding

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

/// Circular button with default normal fill color (white text for dark backgrounds)
pub fn circular_button_default(ui: &mut egui::Ui, radius: f32, text: &str) -> bool {
    circular_button(ui, radius, text, button_fill_normal())
}

/// Circular button with light background and dark text
pub fn circular_button_light(
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

        // Draw text in center with DARK color
        let font_id = egui::FontId::proportional(radius * BUTTON_TEXT_SIZE_RATIO);
        let galley = painter.layout_no_wrap(text.to_string(), font_id, egui::Color32::from_rgb(20, 20, 20));
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

/// Helper: calculate knob radius for a given slider width
pub fn slider_knob_radius(slider_width: f32) -> f32 {
    slider_width * SLIDER_KNOB_RATIO
}

/// Vertical slider widget with touch-friendly styling
pub fn vertical_slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    width: f32,
    height: f32,
    label: &str
) -> bool {
    let desired_size = egui::vec2(width, height);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

    let mut changed = false;

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Background rail with glassmorphism
        let rail_rect = rect.shrink2(egui::vec2(width * SLIDER_RAIL_SHRINK_RATIO, 0.0));
        painter.rect(
            rail_rect,
            rail_rect.width() / 2.0,  // Fully rounded
            slider_rail_fill(),
            egui::Stroke::NONE,
        );

        // Calculate normalized position (inverted for vertical)
        let min = *range.start();
        let max = *range.end();
        let normalized = (*value - min) / (max - min);

        // Compute knob geometry
        let knob_radius = slider_knob_radius(width);
        let knob_diameter = knob_radius * 2.0;
        let travel = rect.height() - knob_diameter;

        // Handle dragging
        if response.dragged() || response.clicked() {
            if let Some(mouse_pos) = ui.ctx().pointer_interact_pos() {
                let pos_in_travel = (mouse_pos.y - (rect.top() + knob_radius)).clamp(0.0, travel);
                let new_normalized = 1.0 - (pos_in_travel / travel).clamp(0.0, 1.0);

                let new_value = min + new_normalized * (max - min);
                // Only report a change when the value actually moved; a finger held
                // still would otherwise trigger a full re-sort every frame
                if new_value != *value {
                    *value = new_value;
                    changed = true;
                    response.mark_changed();
                }
            }
        }

        // Compute knob center
        let knob_y = rect.top() + knob_radius + travel * (1.0 - normalized);
        let knob_center = egui::pos2(rect.center().x, knob_y);

        // Filled portion (from bottom up)
        let filled_top = (knob_center.y - knob_radius).max(rail_rect.min.y);
        let filled_rect = egui::Rect::from_min_max(
            egui::pos2(rail_rect.min.x, filled_top),
            rail_rect.max,
        );
        
        if filled_rect.height() > 0.0 {
            painter.rect(
                filled_rect,
                rail_rect.width() / 2.0,
                slider_fill(),
                egui::Stroke::NONE,
            );
        }

        // Knob shadow
        painter.circle(
            knob_center + egui::vec2(0.0, 2.0),
            knob_radius,
            egui::Color32::from_black_alpha(BUTTON_SHADOW_ALPHA),
            egui::Stroke::NONE,
        );

        // Knob with border
        let stroke_width = (knob_radius * SLIDER_KNOB_BORDER_RATIO).max(1.0);
        painter.circle(
            knob_center,
            knob_radius,
            slider_knob_fill(),
            egui::Stroke::new(stroke_width, button_border()),
        );

        // Value bubble when dragging
        if response.dragged() {
            render_value_bubble(ui, knob_y, rect.left(), *value);
        }

        // Label below slider
        render_slider_label(painter, rect, label);
    }

    changed
}

/// Render value bubble during slider drag
fn render_value_bubble(ui: &egui::Ui, knob_y: f32, slider_left: f32, value: f32) {
    let text = format!("{:.0}", value);
    let font_id = egui::FontId::proportional(BUBBLE_FONT_SIZE);

    let layer_id = egui::LayerId::new(egui::Order::Tooltip, ui.id().with("value_bubble"));
    let layer_painter = ui.ctx().layer_painter(layer_id);

    let galley = layer_painter.layout_no_wrap(text, font_id, egui::Color32::WHITE);
    let bubble_size = galley.size() + egui::vec2(BUBBLE_PADDING_X, BUBBLE_PADDING_Y);
    let bubble_pos = egui::pos2(slider_left - bubble_size.x - BUBBLE_OFFSET_X, knob_y - bubble_size.y / 2.0);
    let bubble_rect = egui::Rect::from_min_size(bubble_pos, bubble_size);

    // Glassmorphism bubble
    layer_painter.rect(
        bubble_rect,
        BUBBLE_CORNER_RADIUS,
        bubble_background(),
        egui::Stroke::new(1.0, bubble_border()),
    );

    let text_pos = bubble_rect.center() - galley.size() / 2.0;
    layer_painter.galley(text_pos, galley);
}

/// Render label below slider
fn render_slider_label(painter: &egui::Painter, rect: egui::Rect, label: &str) {
    let label_font = egui::FontId::proportional(LABEL_FONT_SIZE);
    let label_galley = painter.layout_no_wrap(
        label.to_string(), 
        label_font, 
        label_text()
    );
    
    let label_pos = egui::pos2(
        rect.center().x - label_galley.size().x / 2.0,
        rect.bottom() + LABEL_OFFSET_Y,
    );

    // Label background for readability
    let label_bg_rect = egui::Rect::from_min_size(
        label_pos - egui::vec2(LABEL_BG_PADDING_X, LABEL_BG_PADDING_Y),
        label_galley.size() + egui::vec2(LABEL_BG_PADDING_X * 2.0, LABEL_BG_PADDING_Y * 2.0),
    );
    
    painter.rect(
        label_bg_rect,
        LABEL_BG_CORNER_RADIUS,
        label_background(),
        egui::Stroke::NONE,
    );
    
    painter.galley(label_pos, label_galley);
}
