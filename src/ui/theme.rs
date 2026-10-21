//! Harpy v2 look: Inter type, black/white with one red, frosted pills on the
//! photo and halftone dots for indicators. Sizes are designed for a 1024 px
//! wide screen and scaled to the actual screen with `scale()`.

use eframe::egui;
use egui::{Color32, FontFamily, FontId, Pos2, Rect, Vec2};

pub const RED: Color32 = Color32::from_rgb(0xFF, 0x3B, 0x30);
/// Secondary text: labels, subtitles, inactive answers
pub const GREY: Color32 = Color32::from_rgb(0x8C, 0x8C, 0x8C);
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

/// Plain text button with an optional red dot after it, placed at `pos` by
/// `anchor`. Dims while pressed; `shadow` keeps it readable on a photo.
pub fn text_button(
    ui: &mut egui::Ui,
    id_source: impl std::hash::Hash,
    pos: Pos2,
    anchor: egui::Align2,
    galley: std::sync::Arc<egui::Galley>,
    dot: bool,
    shadow: bool,
) -> egui::Response {
    let s = scale(ui.ctx());
    let dot_r = (galley.size().y * 0.14).max(3.0 * s);
    let dot_w = if dot { dot_r * 2.0 + galley.size().y * 0.35 } else { 0.0 };
    let size = Vec2::new(galley.size().x + dot_w, galley.size().y);
    let rect = anchor.anchor_rect(Rect::from_min_size(pos, size));
    let response = ui.interact(rect.expand(12.0 * s), ui.id().with(id_source), egui::Sense::click());
    let painter = ui.painter();
    let mut text = egui::epaint::TextShape::new(rect.min, galley.clone());
    if response.is_pointer_button_down_on() {
        text.override_text_color = Some(GREY);
    }
    if shadow {
        paint_shadowed(painter, rect.min, galley.clone(), 1.0);
        if text.override_text_color.is_some() {
            painter.add(text);
        }
    } else {
        painter.add(text);
    }
    if dot {
        painter.circle_filled(Pos2::new(rect.max.x - dot_r, rect.center().y + galley.size().y * 0.06), dot_r, RED);
    }
    response
}

/// Screen-sized texture of a black halftone dot screen (8 px pitch) used to
/// veil what lies outside a crop. Built once per screen size and cached.
pub fn dot_screen(ctx: &egui::Context) -> egui::TextureHandle {
    let screen = ctx.screen_rect().size();
    let ppp = ctx.pixels_per_point();
    let size = [(screen.x * ppp).ceil().max(1.0) as usize, (screen.y * ppp).ceil().max(1.0) as usize];
    let id = egui::Id::new("theme_dot_screen");
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        if texture.size() == size {
            return texture;
        }
    }
    let pitch = 8.0 * ppp;
    let radius = 3.6 * ppp;
    let mut pixels = vec![Color32::TRANSPARENT; size[0] * size[1]];
    for y in 0..size[1] {
        let dy = (y as f32 + 0.5) % pitch - pitch / 2.0;
        for x in 0..size[0] {
            let dx = (x as f32 + 0.5) % pitch - pitch / 2.0;
            let coverage = (radius + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            if coverage > 0.0 {
                pixels[y * size[0] + x] = Color32::from_black_alpha((235.0 * coverage) as u8);
            }
        }
    }
    let image = egui::ColorImage { size, pixels };
    let texture = ctx.load_texture("dot_screen", image, egui::TextureOptions::NEAREST);
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    texture
}

/// Like `galley`, but cut to one line of at most `max_width` with an ellipsis
pub fn galley_clipped(
    ctx: &egui::Context,
    text: &str,
    font: FontId,
    color: Color32,
    tracking_em: f32,
    max_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    let spacing = font.size * tracking_em;
    job.append(text, 0.0, egui::TextFormat { font_id: font, color, extra_letter_spacing: spacing, ..Default::default() });
    job.wrap = egui::text::TextWrapping { max_width, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    ctx.fonts(|f| f.layout_job(job))
}

/// Draw a texture filling `rect`, cropping whatever overflows (CSS object-fit: cover)
pub fn paint_cover(painter: &egui::Painter, texture: &egui::TextureHandle, rect: Rect) {
    let size = texture.size_vec2();
    let scale = (rect.width() / size.x).max(rect.height() / size.y);
    let visible = rect.size() / (size * scale);
    let min = ((Vec2::splat(1.0) - visible) / 2.0).to_pos2();
    let uv = Rect::from_min_size(min, visible);
    painter.image(texture.id(), rect, uv, Color32::WHITE);
}
