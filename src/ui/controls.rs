//! Camera and Edit screens in the v2 style: no permanent buttons on the photo.
//!
//! - Dragging left/right anywhere on the image changes the current value
//!   (threshold or hue); a dial with a halftone dot ruler appears while dragging.
//! - Tapping a screen edge, or swiping in from it, slides in that edge's panel
//!   of frosted pills; tapping elsewhere or swiping back closes it.

use crate::PixelSorterApp;
use crate::processing::pixel_sorter::SortMode;
use crate::processing::SortingAlgorithm;
use crate::ui::state::Phase;
use crate::ui::theme::{self, caps, paint_shadowed, pill, pill_label};
use eframe::egui;
use egui::{Align, Color32, Layout, Pos2, Rect, Vec2};
use std::time::{Duration, Instant};

/// Width of the strip at each screen edge that opens a panel (design px)
const EDGE_ZONE: f32 = 96.0;
/// Movement before a press becomes a dial drag (design px)
const DIAL_SLOP: f32 = 8.0;
/// Movement for a swipe from an edge, or back toward it (design px)
const SWIPE_DISTANCE: f32 = 24.0;
/// Drag distance for the full value range (design px)
const FULL_RANGE_DRAG: f32 = 700.0;
/// How long the dial stays after the finger lifts
const DIAL_HOLD: Duration = Duration::from_millis(900);
const PANEL_WIDTH: f32 = 360.0;

const THRESHOLD_MAX: f32 = 125.0;
const HUE_MAX: f32 = 360.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Right,
}

/// What a horizontal drag on the image changes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragTarget {
    Threshold,
    Hue,
}

impl DragTarget {
    fn max(self) -> f32 {
        match self {
            DragTarget::Threshold => THRESHOLD_MAX,
            DragTarget::Hue => HUE_MAX,
        }
    }

    fn label(self) -> &'static str {
        match self {
            DragTarget::Threshold => "Threshold",
            DragTarget::Hue => "Hue",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum PressMode {
    /// Not yet clear whether this is a dial drag or a swipe
    Undecided,
    Dial,
    /// Opened or closed a panel; the rest of the press is ignored
    Swiped,
}

#[derive(Clone, Copy)]
struct Press {
    origin: Pos2,
    edge: Option<Side>,
    start_value: f32,
    mode: PressMode,
}

pub struct ControlsState {
    /// Screen the controls were last drawn for; panels close when it changes
    phase: Option<Phase>,
    pub panel: Option<Side>,
    pub target: DragTarget,
    press: Option<Press>,
    dial_until: Option<Instant>,
}

impl Default for ControlsState {
    fn default() -> Self {
        Self { phase: None, panel: None, target: DragTarget::Threshold, press: None, dial_until: None }
    }
}

impl ControlsState {
    fn dial_visible(&self) -> bool {
        matches!(self.press, Some(p) if p.mode == PressMode::Dial)
            || self.dial_until.is_some_and(|t| Instant::now() < t)
    }
}

impl PixelSorterApp {
    /// Camera (Input) and Edit overlays; called after the image is drawn
    pub fn render_controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect) {
        let s = theme::scale(ctx);
        let editing = self.current_phase == Phase::Edit;
        if self.controls.phase != Some(self.current_phase) {
            self.controls.phase = Some(self.current_phase);
            self.controls.panel = None;
            self.controls.press = None;
        }

        // Taps on these controls must not also open an edge panel
        let exclude: Vec<Rect> = if editing {
            Vec::new()
        } else {
            vec![
                Rect::from_center_size(Pos2::new(rect.max.x - 84.0 * s, rect.center().y), Vec2::splat(110.0 * s)),
                Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - 64.0 * s), Pos2::new(rect.min.x + 220.0 * s, rect.max.y)),
            ]
        };
        self.handle_gestures(ui, ctx, rect, s, editing, &exclude);

        let painter = ui.painter();
        let panel = self.controls.panel;

        // Top bar: status left, battery right
        let status = if editing {
            format!("{} · {}", self.current_algorithm.name(), self.sorting_params.sort_mode.name())
        } else {
            "Live".to_string()
        };
        let mut x = rect.min.x + 24.0 * s;
        if !editing {
            painter.circle_filled(Pos2::new(x + 3.5 * s, rect.min.y + 26.5 * s), 3.5 * s, theme::RED);
            x += 15.0 * s;
        }
        paint_shadowed(painter, Pos2::new(x, rect.min.y + 20.0 * s), caps(ctx, &status, 11.0 * s, Color32::WHITE), 1.0);
        let battery = crate::hardware::get_battery_status();
        if battery.is_available {
            let g = caps(ctx, &format!("{:.0}%", battery.percentage), 11.0 * s, Color32::WHITE);
            paint_shadowed(painter, Pos2::new(rect.max.x - 24.0 * s - g.size().x, rect.min.y + 20.0 * s), g, 1.0);
        }

        // Edge handles fade out while a panel is open
        let handles = ctx.animate_bool_with_time(ui.id().with("handles"), panel.is_none(), 0.2);
        if handles > 0.0 {
            theme::edge_handle(painter, Pos2::new(rect.min.x + 10.0 * s, rect.center().y), s, handles);
            if editing {
                theme::edge_handle(painter, Pos2::new(rect.max.x - 10.0 * s, rect.center().y), s, handles);
            }
        }

        if editing {
            self.render_dial(ctx, rect, s);
        } else {
            // Camera: shutter on the right, quick links bottom left
            if theme::dot_shutter(ui, Pos2::new(rect.max.x - 84.0 * s, rect.center().y), s) {
                self.capture_and_sort(ctx);
            }
            self.render_camera_links(ui, ctx, rect, s);
        }

        self.render_left_panel(ctx, rect, s, editing);
        if editing {
            self.render_right_panel(ctx, rect, s);
        }
    }

    // ------------------------------------------------------------------------
    // Gestures
    // ------------------------------------------------------------------------

    fn handle_gestures(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect, s: f32, editing: bool, exclude: &[Rect]) {
        let response = ui.interact(rect, ui.id().with("surface_gestures"), egui::Sense::click_and_drag());
        let edge_at = |p: Pos2| -> Option<Side> {
            if p.x < rect.min.x + EDGE_ZONE * s {
                Some(Side::Left)
            } else if editing && p.x > rect.max.x - EDGE_ZONE * s {
                Some(Side::Right)
            } else {
                None
            }
        };

        if response.drag_started() {
            let origin = ctx.input(|i| i.pointer.press_origin()).or(response.interact_pointer_pos()).unwrap_or_default();
            self.controls.press = Some(Press {
                origin,
                edge: edge_at(origin),
                start_value: self.drag_value(),
                mode: PressMode::Undecided,
            });
        }

        if response.dragged() {
            if let (Some(mut press), Some(pos)) = (self.controls.press, response.interact_pointer_pos()) {
                let dx = (pos.x - press.origin.x) / s;
                if press.mode == PressMode::Undecided {
                    let panel = self.controls.panel;
                    let inward = match press.edge {
                        Some(Side::Left) => dx > SWIPE_DISTANCE,
                        Some(Side::Right) => dx < -SWIPE_DISTANCE,
                        None => false,
                    };
                    let back_out = match panel {
                        Some(Side::Left) => dx < -SWIPE_DISTANCE,
                        Some(Side::Right) => dx > SWIPE_DISTANCE,
                        None => false,
                    };
                    if inward && panel.is_none() {
                        self.controls.panel = press.edge;
                        press.mode = PressMode::Swiped;
                    } else if back_out {
                        self.controls.panel = None;
                        press.mode = PressMode::Swiped;
                    } else if editing && dx.abs() > DIAL_SLOP && press.edge.is_none() && panel.is_none()
                        || editing && dx.abs() > SWIPE_DISTANCE && (press.edge.is_some() || panel.is_some())
                    {
                        // From an edge or with a panel open, wait for a clear drag
                        // so a swipe never nudges the value by accident
                        self.controls.panel = None;
                        press.mode = PressMode::Dial;
                        self.controls.dial_until = None;
                    }
                }
                if press.mode == PressMode::Dial {
                    let max = self.controls.target.max();
                    let value = (press.start_value + dx * max / FULL_RANGE_DRAG).clamp(0.0, max);
                    self.set_drag_value(ctx, value);
                }
                self.controls.press = Some(press);
            }
        }

        if response.drag_released() {
            if matches!(self.controls.press, Some(p) if p.mode == PressMode::Dial) {
                self.controls.dial_until = Some(Instant::now() + DIAL_HOLD);
                ctx.request_repaint_after(DIAL_HOLD);
            }
            self.controls.press = None;
        }

        if response.clicked() {
            if self.controls.panel.is_some() {
                self.controls.panel = None;
            } else if let Some(pos) = response.interact_pointer_pos() {
                if !exclude.iter().any(|r| r.contains(pos)) {
                    self.controls.panel = edge_at(pos);
                }
            }
        }
    }

    fn drag_value(&self) -> f32 {
        match self.controls.target {
            DragTarget::Threshold => self.sorting_params.threshold,
            DragTarget::Hue => self.sorting_params.color_tint,
        }
    }

    /// Set the dragged value and re-sort when its whole-number value changes
    fn set_drag_value(&mut self, ctx: &egui::Context, value: f32) {
        let value = value.round();
        if value == self.drag_value().round() {
            return;
        }
        match self.controls.target {
            DragTarget::Threshold => self.sorting_params.threshold = value,
            DragTarget::Hue => {
                self.sorting_params.color_tint = value;
                self.tint_enabled = value > 0.0;
            }
        }
        self.apply_pixel_sort(ctx);
    }

    // ------------------------------------------------------------------------
    // Dial
    // ------------------------------------------------------------------------

    fn render_dial(&self, ctx: &egui::Context, rect: Rect, s: f32) {
        let visible = self.controls.dial_visible();
        let alpha = ctx.animate_bool_with_time(egui::Id::new("dial_alpha"), visible, 0.25);
        if alpha <= 0.0 {
            return;
        }
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("dial")));
        let fade = |c: Color32| c.gamma_multiply(alpha);
        let target = self.controls.target;
        let max = target.max();
        let value = self.drag_value().round().clamp(0.0, max);

        // Ruler: halftone dots that swell toward the red needle
        let ticks = 120;
        let spacing = 18.0 * s;
        let ruler_top = rect.max.y - 18.0 * s - 64.0 * s;
        let center_x = rect.center().x;
        let needle_tick = value / max * ticks as f32;
        for i in 0..=ticks {
            let x = center_x + (i as f32 - needle_tick) * spacing;
            if x < rect.min.x - 20.0 * s || x > rect.max.x + 20.0 * s {
                continue;
            }
            let major = i % 20 == 0;
            let base = if major { 6.0 } else if i % 10 == 0 { 4.0 } else { 3.0 };
            let d = i as f32 - needle_tick;
            let size = (base + 9.0 * (-(d * d) / 40.0).exp()) * s;
            let y = ruler_top + 16.0 * s;
            painter.circle_filled(Pos2::new(x, y), size / 2.0 + 1.0 * s, fade(Color32::from_black_alpha(80)));
            painter.circle_filled(Pos2::new(x, y), size / 2.0, fade(Color32::from_white_alpha(if major { 242 } else { 180 })));
            if major {
                let label = format!("{}", (i as f32 * max / ticks as f32).round());
                let g = theme::galley(ctx, &label, theme::medium(11.0 * s), fade(Color32::from_white_alpha(205)), 0.0);
                paint_shadowed(&painter, Pos2::new(x - g.size().x / 2.0, ruler_top + 34.0 * s), g, alpha);
            }
        }
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(center_x - 1.0 * s, ruler_top), Vec2::new(2.0 * s, 40.0 * s)),
            0.0,
            fade(theme::RED),
        );

        // Big value: leading zeros in grey, like a radio frequency
        let digits = format!("{:03}", value as u32);
        let lead = digits.chars().take_while(|&c| c == '0').count().min(2);
        let mut job = egui::text::LayoutJob::default();
        let font = theme::medium(96.0 * s);
        let tracking = -0.05 * 96.0 * s;
        job.append(&digits[..lead], 0.0, egui::TextFormat { font_id: font.clone(), color: fade(Color32::from_white_alpha(102)), extra_letter_spacing: tracking, ..Default::default() });
        job.append(&digits[lead..], 0.0, egui::TextFormat { font_id: font, color: fade(Color32::WHITE), extra_letter_spacing: tracking, ..Default::default() });
        let number = ctx.fonts(|f| f.layout_job(job));
        let number_pos = Pos2::new(center_x - number.size().x / 2.0, ruler_top - 8.0 * s - number.size().y);
        paint_shadowed(&painter, number_pos, number, alpha);

        let label = caps(ctx, target.label(), 11.0 * s, fade(Color32::WHITE));
        paint_shadowed(&painter, Pos2::new(center_x - label.size().x / 2.0, number_pos.y - label.size().y), label, alpha);

        if visible || alpha < 1.0 {
            ctx.request_repaint();
        }
    }

    // ------------------------------------------------------------------------
    // Panels
    // ------------------------------------------------------------------------

    /// Slide-in amount (0 closed .. 1 open) and the panel's area rect
    fn panel_rect(&self, ctx: &egui::Context, rect: Rect, s: f32, side: Side) -> (f32, Rect) {
        let open = ctx.animate_bool_with_time(egui::Id::new(("panel", side == Side::Left)), self.controls.panel == Some(side), 0.3);
        let w = PANEL_WIDTH * s;
        let hidden_offset = (1.0 - open) * w;
        let x = match side {
            Side::Left => rect.min.x - hidden_offset,
            Side::Right => rect.max.x - w + hidden_offset,
        };
        (open, Rect::from_min_size(Pos2::new(x, rect.min.y), Vec2::new(w, rect.height())))
    }

    fn render_left_panel(&mut self, ctx: &egui::Context, rect: Rect, s: f32, editing: bool) {
        let (open, area) = self.panel_rect(ctx, rect, s, Side::Left);
        if open <= 0.0 {
            return;
        }
        let inner = area.shrink2(Vec2::new(32.0 * s, 0.0));
        egui::Area::new("left_panel")
            .fixed_pos(Pos2::new(inner.min.x, inner.min.y + 56.0 * s))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_width(inner.width());
                ui.spacing_mut().item_spacing = Vec2::splat(8.0 * s);
                if editing {
                    self.render_sort_options(ui, s);
                } else {
                    self.render_camera_menu(ui, s);
                }
            });
    }

    fn render_sort_options(&mut self, ui: &mut egui::Ui, s: f32) {
        let ctx = ui.ctx().clone();
        let mut changed = false;

        pill_label(ui, "Direction", s);
        ui.horizontal_wrapped(|ui| {
            for &algo in SortingAlgorithm::all() {
                if pill(ui, algo.name(), self.current_algorithm == algo, false, s) && self.current_algorithm != algo {
                    self.current_algorithm = algo;
                    changed = true;
                }
            }
        });
        ui.add_space(14.0 * s);

        pill_label(ui, "Sort by", s);
        ui.horizontal_wrapped(|ui| {
            for &mode in SortMode::all() {
                if pill(ui, mode.name(), self.sorting_params.sort_mode == mode, false, s) && self.sorting_params.sort_mode != mode {
                    self.sorting_params.sort_mode = mode;
                    changed = true;
                }
            }
        });
        ui.add_space(14.0 * s);

        pill_label(ui, "Drag sets", s);
        ui.horizontal_wrapped(|ui| {
            for target in [DragTarget::Threshold, DragTarget::Hue] {
                let active = self.controls.target == target;
                if pill(ui, target.label(), active, active, s) {
                    self.controls.target = target;
                }
            }
        });

        if changed {
            self.apply_pixel_sort(&ctx);
        }
    }

    fn render_camera_menu(&mut self, ui: &mut egui::Ui, s: f32) {
        pill_label(ui, "Harpy", s);
        if pill(ui, "Gallery", false, false, s) {
            self.controls.panel = None;
            self.open_gallery();
        }
        if pill(ui, "Open from USB", false, false, s) {
            self.controls.panel = None;
            self.open_usb_browser();
        }
        ui.add_space(14.0 * s);
        let battery = crate::hardware::get_battery_status();
        let power = if battery.is_available { format!("Power · {:.0}%", battery.percentage) } else { "Power".to_string() };
        if pill(ui, &power, false, false, s) {
            self.controls.panel = None;
            self.show_shutdown_menu = true;
        }
    }

    fn render_right_panel(&mut self, ctx: &egui::Context, rect: Rect, s: f32) {
        let (open, area) = self.panel_rect(ctx, rect, s, Side::Right);
        if open <= 0.0 {
            return;
        }
        let inner = area.shrink2(Vec2::new(32.0 * s, 0.0));
        egui::Area::new("right_panel")
            .fixed_pos(Pos2::new(inner.min.x, inner.min.y + 56.0 * s))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_width(inner.width());
                ui.spacing_mut().item_spacing = Vec2::splat(8.0 * s);
                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                    let heading = match &self.current_session_folder {
                        Some(_) if self.iteration_counter > 0 => format!("Edit {:03}", self.iteration_counter),
                        _ => "Not saved yet".to_string(),
                    };
                    pill_label(ui, &heading, s);
                    ui.add_space(6.0 * s);
                    if pill(ui, "Save", self.has_unsaved_changes, self.has_unsaved_changes, s) {
                        self.controls.panel = None;
                        self.save_and_continue_iteration(ctx);
                        self.show_message(format!("✔ Saved · edit {:03}", self.iteration_counter));
                    }
                    if pill(ui, "Crop", false, false, s) {
                        self.controls.panel = None;
                        self.current_phase = Phase::Crop;
                        self.crop_rect = None;
                    }
                    if pill(ui, "New photo", false, false, s) {
                        self.controls.panel = None;
                        self.request_new_photo_session();
                    }
                    ui.add_space(14.0 * s);
                    if pill(ui, "Gallery", false, false, s) {
                        self.controls.panel = None;
                        self.open_gallery();
                    }
                    if pill(ui, "Export to USB", false, self.prev_usb_present, s) {
                        self.controls.panel = None;
                        if self.prev_usb_present {
                            self.open_usb_export_dialog();
                        } else {
                            self.show_message("❌ No USB drive found");
                        }
                    }
                    ui.add_space(14.0 * s);
                    let battery = crate::hardware::get_battery_status();
                    let power = if battery.is_available {
                        format!("Power · {:.0}%", battery.percentage)
                    } else {
                        "Power".to_string()
                    };
                    if pill(ui, &power, false, false, s) {
                        self.controls.panel = None;
                        self.show_shutdown_menu = true;
                    }
                });
            });
    }

    fn render_camera_links(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect, s: f32) {
        let mut x = rect.min.x + 24.0 * s;
        let y = rect.max.y - 22.0 * s;
        for (label, gallery) in [("Gallery", true), ("USB", false)] {
            let g = caps(ctx, label, 13.0 * s, Color32::WHITE);
            let r = Rect::from_min_size(Pos2::new(x, y - g.size().y), g.size()).expand(10.0 * s);
            let response = ui.interact(r, ui.id().with(("camera_link", label)), egui::Sense::click());
            paint_shadowed(ui.painter(), Pos2::new(x, y - g.size().y), g.clone(), 1.0);
            if response.clicked() {
                if gallery {
                    self.open_gallery();
                } else {
                    self.open_usb_browser();
                }
            }
            x += g.size().x + 22.0 * s;
        }
    }
}
