/// Styling for the developer menu (the v2 look lives in theme.rs)
use eframe::egui;

// ============================================================================
// MENU STYLING
// ============================================================================

/// Menu styling configuration
pub struct MenuStyle {
    pub width: f32,
    pub button_width: f32,
    pub button_height: f32,
    pub cancel_button_height: f32,
    pub spacing: f32,
    pub heading_size: f32,
    pub label_size: f32,
}

impl MenuStyle {
    /// Standard developer menu style (large, detailed)
    pub fn developer() -> Self {
        Self {
            width: 933.0,              // 700.0 * 1.33
            button_width: 800.0,       // 600.0 * 1.33
            button_height: 107.0,      // 80.0 * 1.33
            cancel_button_height: 80.0, // 60.0 * 1.33
            spacing: 20.0,
            heading_size: 32.0,
            label_size: 20.0,
        }
    }
}

/// Button text styling helper
pub fn button_text(text: &str, size: f32) -> egui::RichText {
    egui::RichText::new(text).size(size)
}

/// Fixed-size menu button; returns true when clicked
pub fn menu_button(ui: &mut egui::Ui, size: [f32; 2], text: &str, text_size: f32) -> bool {
    ui.add_sized(size, egui::Button::new(button_text(text, text_size))).clicked()
}
