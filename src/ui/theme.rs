//! Harpy v2 look: Inter type, black/white with one red, frosted pills on the
//! photo and halftone dots for indicators. Sizes are designed for a 1024 px
//! wide screen and scaled to the actual screen with `scale()`.

use eframe::egui;
use egui::{Color32, FontFamily, FontId, Pos2, Rect, Vec2};

pub const RED: Color32 = Color32::from_rgb(0xFF, 0x3B, 0x30);
/// Fill of an inactive pill on top of the photo
pub const PILL: Color32 = Color32::from_rgba_premultiplied(9, 9, 9, 140);

const SEMIBOLD: &str = "Inter SemiBold";

/// Register Inter (Medium as the default proportional font, SemiBold for labels)
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Inter-Medium".into(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/Inter-Medium.ttf")),
    );
    fonts.font_data.insert(
        "Inter-SemiBold".into(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/Inter-SemiBold.ttf")),
    );
    // Inter first; egui's default fonts stay as fallback for symbols (✔ ❌ 🔄 …)
    let fallback = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "Inter-Medium".into());
    let mut semibold = vec!["Inter-SemiBold".to_string()];
    semibold.extend(fallback);
    fonts.families.insert(FontFamily::Name(SEMIBOLD.into()), semibold);
    ctx.set_fonts(fonts);
}

/// Design scale: the layout is drawn for a 1024 px wide screen
pub fn scale(ctx: &egui::Context) -> f32 {
    (ctx.screen_rect().width() / 1024.0).max(0.5)
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

/// Lay out a single line with letter spacing (in em)
pub fn galley(
    ctx: &egui::Context,
    text: &str,
    font: FontId,
    color: Color32,
    tracking_em: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    let spacing = font.size * tracking_em;
    job.append(text, 0.0, egui::TextFormat { font_id: font, color, extra_letter_spacing: spacing, ..Default::default() });
    ctx.fonts(|f| f.layout_job(job))
}

/// Small uppercase label with wide tracking, e.g. "HORIZONTAL · BRIGHTNESS"
pub fn caps(ctx: &egui::Context, text: &str, size: f32, color: Color32) -> std::sync::Arc<egui::Galley> {
    galley(ctx, &text.to_uppercase(), semibold(size), color, 0.14)
}

/// Paint a galley with a soft dark shadow so it reads on any photo
pub fn paint_shadowed(painter: &egui::Painter, pos: Pos2, galley: std::sync::Arc<egui::Galley>, alpha: f32) {
    let shadow = Color32::from_black_alpha((110.0 * alpha) as u8);
    for offset in [Vec2::new(0.0, 1.5), Vec2::new(0.0, -0.5), Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)] {
        let mut text = egui::epaint::TextShape::new(pos + offset, galley.clone());
        text.override_text_color = Some(shadow);
        painter.add(text);
    }
    painter.galley(pos, galley);
}

/// Tapering column of halftone dots marking a screen edge that opens a panel
pub fn edge_handle(painter: &egui::Painter, center: Pos2, s: f32, alpha: f32) {
    let sizes = [3.0, 5.0, 7.0, 5.0, 3.0];
    let gap = 6.0 * s;
    let total: f32 = sizes.iter().map(|d| d * s).sum::<f32>() + gap * 4.0;
    let mut y = center.y - total / 2.0;
    for (i, d) in sizes.iter().enumerate() {
        let r = d * s / 2.0;
        let opacity = [0.6, 0.8, 1.0, 0.8, 0.6][i] * alpha;
        painter.circle_filled(Pos2::new(center.x, y + r), r + 1.5 * s, Color32::from_black_alpha((70.0 * alpha) as u8));
        painter.circle_filled(Pos2::new(center.x, y + r), r, Color32::from_white_alpha((255.0 * opacity) as u8));
        y += d * s + gap;
    }
}

/// A frosted pill button: white with black text when active, translucent dark otherwise.
/// Returns true when tapped.
pub fn pill(ui: &mut egui::Ui, text: &str, active: bool, dot: bool, s: f32) -> bool {
    let ctx = ui.ctx().clone();
    let text_color = if active { Color32::BLACK } else { Color32::WHITE };
    let g = galley(&ctx, text, medium(20.0 * s), text_color, -0.02);
    let dot_w = if dot { 16.0 * s } else { 0.0 };
    let size = Vec2::new(g.size().x + 36.0 * s + dot_w, 44.0 * s);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let pressed = response.is_pointer_button_down_on();
    let fill = match (active, pressed) {
        (true, _) => Color32::WHITE,
        (false, true) => Color32::from_rgba_premultiplied(40, 40, 40, 190),
        (false, false) => PILL,
    };
    let painter = ui.painter();
    painter.rect_filled(rect, rect.height() / 2.0, fill);
    let mut x = rect.min.x + 18.0 * s;
    if dot {
        painter.circle_filled(Pos2::new(x + 4.0 * s, rect.center().y), 4.0 * s, RED);
        x += dot_w;
    }
    painter.galley(Pos2::new(x, rect.center().y - g.size().y / 2.0), g);
    response.clicked()
}

/// A small caps label pill used as a group heading inside a panel
pub fn pill_label(ui: &mut egui::Ui, text: &str, s: f32) {
    let ctx = ui.ctx().clone();
    let g = caps(&ctx, text, 11.0 * s, Color32::WHITE);
    let size = Vec2::new(g.size().x + 20.0 * s, g.size().y + 8.0 * s);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(rect, rect.height() / 2.0, PILL);
    ui.painter().galley(rect.center() - g.size() / 2.0, g);
}

/// Shutter button drawn as a ring of halftone dots around a solid centre.
/// Returns true when tapped.
pub fn dot_shutter(ui: &mut egui::Ui, center: Pos2, s: f32) -> bool {
    let rect = Rect::from_center_size(center, Vec2::splat(96.0 * s));
    let response = ui.interact(rect, ui.id().with("dot_shutter"), egui::Sense::click());
    let press = if response.is_pointer_button_down_on() { 0.94 } else { 1.0 };
    let painter = ui.painter();
    let n = 32;
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let p = center + Vec2::angled(a) * 44.0 * s * press;
        painter.circle_filled(p, 3.5 * s, Color32::from_black_alpha(60));
        painter.circle_filled(p, 2.5 * s, Color32::WHITE);
    }
    painter.circle_filled(center, 34.0 * s * press, Color32::WHITE);
    response.clicked()
}
