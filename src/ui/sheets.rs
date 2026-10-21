//! Full-screen sheets in the v2 style (New photo, Power, Export to USB):
//! a dark page with a Swiss heading block top left, a big figure top right
//! and large type answers bottom left, instead of floating dialog windows.

use crate::PixelSorterApp;
use crate::system::SystemControl;
use crate::ui::files::session_title;
use crate::ui::theme::{self, caps};
use eframe::egui;
use egui::{Color32, Pos2, Rect, Vec2};

const SHEET_BG: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0A);
const DIM: Color32 = Color32::from_rgb(0x6E, 0x6E, 0x6E);
const LINE: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);

/// Shared drawing for one sheet frame
struct Sheet<'a> {
    ui: &'a mut egui::Ui,
    rect: Rect,
    s: f32,
}

impl Sheet<'_> {
    /// "CAPS LABEL" / "Title" / "grey subtitle" block in the top left
    fn heading(&mut self, label: &str, title: &str, subtitle: &str) {
        let ctx = self.ui.ctx().clone();
        let s = self.s;
        let p = self.ui.painter();
        let mut y = self.rect.min.y + 44.0 * s;
        let x = self.rect.min.x + 48.0 * s;
        let g = caps(&ctx, label, 11.0 * s, theme::GREY);
        let h = g.size().y;
        p.galley(Pos2::new(x, y), g);
        y += h + 4.0 * s;
        for (text, color) in [(title, Color32::WHITE), (subtitle, theme::GREY)] {
            let g = theme::galley(&ctx, text, theme::semibold(40.0 * s), color, -0.03);
            let h = g.size().y;
            p.galley(Pos2::new(x, y), g);
            y += h;
        }
    }

    /// Big number in the top right, with an optional dimmed part (leading zeros, "%")
    fn figure(&mut self, label: &str, dim_prefix: &str, value: &str, dim_suffix: &str) {
        let ctx = self.ui.ctx().clone();
        let s = self.s;
        let font = theme::medium(120.0 * s);
        let tracking = -0.06 * 120.0 * s;
        let mut job = egui::text::LayoutJob::default();
        for (text, color) in [(dim_prefix, DIM), (value, Color32::WHITE), (dim_suffix, DIM)] {
            job.append(text, 0.0, egui::TextFormat { font_id: font.clone(), color, extra_letter_spacing: tracking, ..Default::default() });
        }
        let g = ctx.fonts(|f| f.layout_job(job));
        let right = self.rect.max.x - 48.0 * s;
        let p = self.ui.painter();
        let mut y = self.rect.min.y + 36.0 * s;
        if !label.is_empty() {
            let l = caps(&ctx, label, 11.0 * s, theme::GREY);
            let h = l.size().y;
            p.galley(Pos2::new(right - l.size().x, y), l);
            y += h;
        }
        p.galley(Pos2::new(right - g.size().x, y), g);
    }

    /// Large type answer; returns true when tapped
    fn answer(&mut self, pos: Pos2, text: &str, size: f32, color: Color32, dot: bool) -> (bool, Rect) {
        let ctx = self.ui.ctx().clone();
        let s = self.s;
        let g = theme::galley(&ctx, text, theme::medium(size * s), color, -0.045);
        let dot_r = 7.0 * s * size / 72.0;
        let width = g.size().x + if dot { 14.0 * s + dot_r * 2.0 } else { 0.0 };
        let rect = Rect::from_min_size(pos, Vec2::new(width, g.size().y));
        let response = self.ui.interact(rect.expand(8.0 * s), self.ui.id().with(("answer", text)), egui::Sense::click());
        let color = if response.is_pointer_button_down_on() { color.gamma_multiply(0.6) } else { color };
        let p = self.ui.painter();
        p.galley_with_color(pos, g.clone(), color);
        if dot {
            p.circle_filled(Pos2::new(pos.x + g.size().x + 14.0 * s + dot_r, rect.center().y + 4.0 * s), dot_r, theme::RED);
        }
        (response.clicked(), rect)
    }

    /// Stack of answers anchored to the bottom left; returns the index tapped
    fn answers(&mut self, items: &[(&str, Color32, bool)]) -> Option<usize> {
        let ctx = self.ui.ctx().clone();
        let s = self.s;
        let line = ctx.fonts(|f| f.row_height(&theme::medium(72.0 * s))) * 1.02;
        let mut y = self.rect.max.y - 36.0 * s - line * items.len() as f32;
        let mut tapped = None;
        for (i, (text, color, dot)) in items.iter().enumerate() {
            if self.answer(Pos2::new(self.rect.min.x + 48.0 * s, y), text, 72.0, *color, *dot).0 {
                tapped = Some(i);
            }
            y += line;
        }
        tapped
    }
}

impl PixelSorterApp {
    /// Draw whichever sheet is open (at most one) over everything else
    pub fn render_sheets(&mut self, ctx: &egui::Context) {
        if !(self.show_new_session_dialog || self.show_shutdown_menu || self.show_usb_export_dialog) {
            return;
        }
        let rect = ctx.screen_rect();
        let s = theme::scale(ctx);
        egui::Area::new("sheet")
            .fixed_pos(rect.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // The sheet covers the screen and swallows taps meant for what's below
                let _ = ui.allocate_rect(rect, egui::Sense::click_and_drag());
                ui.painter().rect_filled(rect, 0.0, SHEET_BG);
                let mut sheet = Sheet { ui, rect, s };
                if self.show_new_session_dialog {
                    self.new_photo_sheet(&mut sheet);
                } else if self.show_shutdown_menu {
                    self.power_sheet(&mut sheet);
                } else {
                    self.export_sheet(&mut sheet);
                }
            });
    }

    fn new_photo_sheet(&mut self, sheet: &mut Sheet) {
        sheet.heading("Unsaved edit", "New photo", "Keep this one?");
        if let Some(texture) = &self.processed_texture {
            let s = sheet.s;
            let size = crate::ui::helpers::fit_image_in_rect(texture.size_vec2(), Vec2::new(300.0, 176.0) * s);
            let r = Rect::from_min_size(Pos2::new(sheet.rect.max.x - 48.0 * s - size.x, sheet.rect.min.y + 48.0 * s), size);
            sheet.ui.painter().image(texture.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        match sheet.answers(&[("Save", Color32::WHITE, true), ("Discard", Color32::WHITE, false), ("Cancel", DIM, false)]) {
            Some(0) => self.save_and_start_new_photo_session(),
            Some(1) => {
                log::info!("Discarding unsaved image for new photo");
                self.start_new_photo_session();
            }
            Some(_) => self.show_new_session_dialog = false,
            None => {}
        }
    }

    fn power_sheet(&mut self, sheet: &mut Sheet) {
        let s = sheet.s;
        let battery = crate::hardware::get_battery_status();
        if battery.is_available {
            sheet.heading("Power", "Battery", &format!("{:.1} V", battery.voltage));
            sheet.figure("", "", &format!("{:.0}", battery.percentage), "%");
            // Battery level as a row of halftone dots that taper at the level
            let n = 50;
            let level = battery.percentage / 100.0;
            let left = sheet.rect.min.x + 48.0 * s;
            let width = sheet.rect.width() - 96.0 * s;
            let y = sheet.rect.min.y + 240.0 * s;
            for i in 0..n {
                let t = (i as f32 + 0.5) / n as f32;
                let filled = t <= level;
                let d = if filled { 4.0 + 6.0 * ((level - t) * 12.0).min(1.0) } else { 4.0 } * s;
                let x = left + width * i as f32 / (n - 1) as f32;
                sheet.ui.painter().circle_filled(Pos2::new(x, y), d / 2.0, if filled { Color32::WHITE } else { Color32::from_gray(0x33) });
            }
        } else {
            sheet.heading("Power", "Harpy", "Mains power");
        }
        match sheet.answers(&[("Power off", Color32::WHITE, true), ("Restart", Color32::WHITE, false), ("Cancel", DIM, false)]) {
            Some(0) => {
                log::info!("Shutdown requested by user");
                if let Err(e) = SystemControl::shutdown() {
                    log::error!("Failed to shutdown: {}", e);
                    self.show_message(format!("❌ Shutdown failed: {}", e));
                }
                self.show_shutdown_menu = false;
            }
            Some(1) => {
                log::info!("Reboot requested by user");
                if let Err(e) = SystemControl::reboot() {
                    log::error!("Failed to reboot: {}", e);
                    self.show_message(format!("❌ Reboot failed: {}", e));
                }
                self.show_shutdown_menu = false;
            }
            Some(_) => self.show_shutdown_menu = false,
            None => {}
        }
    }

    fn export_sheet(&mut self, sheet: &mut Sheet) {
        let ctx = sheet.ui.ctx().clone();
        let s = sheet.s;
        let drive = crate::system::usb::find_writable_usb_mount()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "USB drive".to_string());
        sheet.heading(&format!("USB drive found · {}", drive), "Export", "to USB");

        if self.usb_export_choices.is_empty() {
            let g = theme::galley(&ctx, "No saved images yet", theme::medium(26.0 * s), theme::GREY, -0.02);
            sheet.ui.painter().galley(Pos2::new(sheet.rect.min.x + 48.0 * s, sheet.rect.min.y + 240.0 * s), g);
            if sheet.answers(&[("Close", Color32::WHITE, false)]).is_some() {
                self.show_usb_export_dialog = false;
            }
            return;
        }

        let selected: usize = self.usb_export_choices.iter().filter(|(_, on)| *on).map(|(sess, _)| sess.images.len()).sum();
        let digits = format!("{:02}", selected);
        let lead = if digits.len() > 1 && digits.starts_with('0') { 1 } else { 0 };
        sheet.figure("Selected", &digits[..lead], &digits[lead..], "");

        // Session rows with a dot toggle: red = selected, ring = not
        let left = sheet.rect.min.x + 48.0 * s;
        let right = sheet.rect.max.x - 48.0 * s;
        let row_h = 58.0 * s;
        let top = sheet.rect.min.y + 230.0 * s;
        let max_rows = 3;
        for (i, (session, on)) in self.usb_export_choices.iter_mut().take(max_rows).enumerate() {
            let y = top + i as f32 * row_h;
            let row = Rect::from_min_max(Pos2::new(left, y), Pos2::new(right, y + row_h));
            let response = sheet.ui.interact(row, sheet.ui.id().with(("export_row", i)), egui::Sense::click());
            if response.clicked() {
                *on = !*on;
            }
            let p = sheet.ui.painter();
            p.line_segment([row.left_top(), row.right_top()], egui::Stroke::new(1.0_f32, LINE));
            let color = if *on { Color32::WHITE } else { DIM };
            let mut job = egui::text::LayoutJob::default();
            job.append(&session_title(&session.name), 0.0, egui::TextFormat { font_id: theme::medium(26.0 * s), color, extra_letter_spacing: -0.5 * s, ..Default::default() });
            job.append(&format!("  ({})", session.images.len()), 0.0, egui::TextFormat { font_id: theme::medium(13.0 * s), color: theme::GREY, ..Default::default() });
            let g = ctx.fonts(|f| f.layout_job(job));
            p.galley(Pos2::new(left, row.center().y - g.size().y / 2.0), g);
            let dot = Pos2::new(right - 6.0 * s, row.center().y);
            if *on {
                p.circle_filled(dot, 6.0 * s, theme::RED);
            } else {
                p.circle_stroke(dot, 5.5 * s, egui::Stroke::new(1.0 * s, DIM));
            }
        }
        let shown = self.usb_export_choices.len().min(max_rows);
        let bottom = top + shown as f32 * row_h;
        sheet.ui.painter().line_segment([Pos2::new(left, bottom), Pos2::new(right, bottom)], egui::Stroke::new(1.0_f32, LINE));
        if self.usb_export_choices.len() > max_rows {
            let more = caps(&ctx, &format!("+ {} older sessions (selected)", self.usb_export_choices.len() - max_rows), 11.0 * s, theme::GREY);
            sheet.ui.painter().galley(Pos2::new(left, bottom + 10.0 * s), more);
        }

        // Delete-after-copying toggle, bottom right
        let label = format!("{} Delete after copying", if self.usb_export_delete_after { "●" } else { "○" });
        let g = caps(&ctx, &label, 13.0 * s, if self.usb_export_delete_after { Color32::WHITE } else { theme::GREY });
        let pos = Pos2::new(right - g.size().x, sheet.rect.max.y - 50.0 * s - g.size().y);
        let r = Rect::from_min_size(pos, g.size()).expand(12.0 * s);
        if sheet.ui.interact(r, sheet.ui.id().with("delete_after"), egui::Sense::click()).clicked() {
            self.usb_export_delete_after = !self.usb_export_delete_after;
        }
        sheet.ui.painter().galley(pos, g);

        // Copy (big) and Cancel (smaller) on one baseline
        let line = ctx.fonts(|f| f.row_height(&theme::medium(72.0 * s)));
        let y = sheet.rect.max.y - 30.0 * s - line;
        let copy_color = if selected > 0 { Color32::WHITE } else { DIM };
        let (copy, copy_rect) = sheet.answer(Pos2::new(left, y), "Copy", 72.0, copy_color, false);
        let cancel_h = ctx.fonts(|f| f.row_height(&theme::medium(40.0 * s)));
        let (cancel, _) = sheet.answer(Pos2::new(copy_rect.max.x + 36.0 * s, copy_rect.max.y - cancel_h - 8.0 * s), "Cancel", 40.0, DIM, false);
        if copy && selected > 0 {
            self.start_usb_export(&ctx);
            self.show_usb_export_dialog = false;
        } else if cancel {
            self.show_usb_export_dialog = false;
            self.usb_export_delete_after = false;
        }
    }
}
