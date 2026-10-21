/// Centralized UI styling constants for consistent look across the app
/// 
/// EDIT THIS FILE TO CHANGE: Colors, sizes, visual appearance
use eframe::egui;

// ============================================================================
// 🎨 QUICK EDIT: COLOR PALETTE
// Change these values to customize the app's appearance
// Format: (Red, Green, Blue, Alpha) where each value is 0-255
// 
// 💡 HOW TO CHANGE COLORS:
//    1. Click the colored square next to the rgba() value
//    2. VS Code's color picker will open - adjust visually
//    3. The rgba values will update automatically in the comment
//    4. Copy the new values into the RGBA tuple
// ============================================================================

// Button glassmorphism colors (white with varying transparency)
const BUTTON_HOVER_RGBA: (u8, u8, u8, u8) = (255, 255, 255, 50);    // rgba(255, 255, 255, 0.2) - Hover state
const BUTTON_ACTIVE_RGBA: (u8, u8, u8, u8) = (255, 255, 255, 64);   // rgba(255, 255, 255, 0.25) - Pressed state
const BUTTON_BORDER_RGBA: (u8, u8, u8, u8) = (255, 255, 255, 30);   // rgba(255, 255, 255, 0.12) - Border color
const BUTTON_SHADOW_RGBA: (u8, u8, u8, u8) = (0, 0, 0, 60);         // rgba(0, 0, 0, 0.24) - Shadow color

// Specific button colors
const BUTTON_DARK_RGBA: (u8, u8, u8, u8) = (60, 60, 70, 120);              // rgba(60, 60, 70, 0.49) - Dark buttons (Crop/Iterate/New)
const BUTTON_GREEN_RGBA: (u8, u8, u8, u8) = (40, 80, 40, 180);   // rgba(40, 80, 40, 0.71) - Green buttons (USB/Apply)
const BUTTON_RED_RGBA: (u8, u8, u8, u8) = (80, 40, 40, 180);     // rgba(80, 40, 40, 0.71) - Red buttons (Cancel)

// Slider colors

// Value bubble colors (shown when dragging slider)

// Slider label colors


// ============================================================================
// 🔢 QUICK EDIT: SIZE CONSTANTS
// Change these values to adjust button/slider sizes
// ============================================================================

const BUTTON_NORMAL_RADIUS: f32 = 100.0;  // Standard buttons (Edit phase)
const BUTTON_SPACING: f32 = 20.0;         // Space between buttons


// ============================================================================
// Internal functions (uses constants above)
// ============================================================================

pub fn button_fill_hover() -> egui::Color32 { 
    let (r, g, b, a) = BUTTON_HOVER_RGBA;
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn button_fill_active() -> egui::Color32 { 
    let (r, g, b, a) = BUTTON_ACTIVE_RGBA;
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn button_border() -> egui::Color32 { 
    let (r, g, b, a) = BUTTON_BORDER_RGBA;
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub const BUTTON_SHADOW_ALPHA: u8 = BUTTON_SHADOW_RGBA.3;

/// Specific button colors
pub fn button_dark() -> egui::Color32 {
    let (r, g, b, a) = BUTTON_DARK_RGBA;
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn button_green() -> egui::Color32 {
    let (r, g, b, a) = BUTTON_GREEN_RGBA;
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn button_red() -> egui::Color32 {
    let (r, g, b, a) = BUTTON_RED_RGBA;
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}


// ============================================================================
// Button/Slider size configuration structs
// ============================================================================

/// Button sizes for different phases
pub struct ButtonSizes {
    pub normal_radius: f32,  // Standard buttons (Edit phase)
    pub spacing: f32,        // Standard spacing between elements
}

impl ButtonSizes {
    pub fn standard() -> Self {
        Self {
            normal_radius: BUTTON_NORMAL_RADIUS,
            spacing: BUTTON_SPACING,
        }
    }
}

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

    /// Power/shutdown menu style (matches developer menu sizing)
    pub fn power() -> Self {
        Self {
            width: 933.0,              // Match developer menu
            button_width: 800.0,       // Match developer menu
            button_height: 107.0,      // Match developer menu
            cancel_button_height: 80.0, // Match developer menu
            spacing: 20.0,
            heading_size: 32.0,
            label_size: 24.0,
        }
    }

    /// USB export dialog style (smaller, focused)
    pub fn usb_export() -> Self {
        Self {
            width: 450.0,
            button_width: 400.0,
            button_height: 70.0,
            cancel_button_height: 60.0,
            spacing: 15.0,
            heading_size: 28.0,
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
