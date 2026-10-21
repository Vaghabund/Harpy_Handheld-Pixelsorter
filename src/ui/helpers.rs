use eframe::egui;

/// Fit image inside rect while maintaining aspect ratio (contain mode)
pub fn fit_image_in_rect(image_size: egui::Vec2, container_size: egui::Vec2) -> egui::Vec2 {
    let scale = (container_size.x / image_size.x).min(container_size.y / image_size.y);
    image_size * scale
}

/// Cover mode: scale to fill entire container (crops overflow)
pub fn cover_image_in_rect(image_size: egui::Vec2, container_size: egui::Vec2) -> egui::Vec2 {
    let scale = (container_size.x / image_size.x).max(container_size.y / image_size.y);
    image_size * scale
}

/// Center a rect within another rect
pub fn center_rect_in_rect(content_size: egui::Vec2, container: egui::Rect) -> egui::Rect {
    let offset = (container.size() - content_size) * 0.5;
    egui::Rect::from_min_size(container.min + offset, content_size)
}

/// Path of the Harpy logo, used for the window icon and splash/sleep screens
pub const LOGO_PATH: &str = "assets/Harpy_ICON.png";

/// Load the Harpy logo as RGBA, or None if the file is missing/unreadable
pub fn load_logo_image() -> Option<image::RgbaImage> {
    match image::open(LOGO_PATH) {
        Ok(img) => Some(img.to_rgba8()),
        Err(e) => {
            log::warn!("Could not load logo from {}: {}", LOGO_PATH, e);
            None
        }
    }
}
