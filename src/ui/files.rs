//! In-app file screens: gallery of saved sessions (with full-screen viewer,
//! re-edit and delete) and a USB image browser. Replaces the desktop file dialog.

use crate::PixelSorterApp;
use crate::session::library::{self, DirEntry, SessionFolder};
use crate::ui::background::BackgroundTask;
use crate::ui::helpers::{center_rect_in_rect, fit_image_in_rect};
use crate::ui::state::Phase;
use crate::ui::theme;
use crate::ui::thumbnails::ThumbnailCache;
use eframe::egui;
use egui::{Align2, Color32, Pos2, Rect, Vec2};
use std::path::{Path, PathBuf};

/// White page, black ink, light grey for unselected rows
const PAGE: Color32 = Color32::WHITE;
const INK: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0A);
const FAINT: Color32 = Color32::from_rgb(0xBD, 0xBD, 0xBD);
const DIM: Color32 = Color32::from_rgb(0x6E, 0x6E, 0x6E);
const MARGIN: f32 = 48.0;
/// Top of the lists below the page title
const LIST_TOP: f32 = 112.0;

/// Why an image is being decoded in the background
#[derive(Clone, Copy, PartialEq)]
enum OpenPurpose {
    /// Show it in the gallery viewer
    View,
    /// Load it into Edit; `saved` is false for images from USB (not yet in a session)
    Edit { saved: bool },
}

/// A saved image shown full screen in the gallery, one of a session's images
struct Viewer {
    images: Vec<PathBuf>,
    index: usize,
    texture: Option<egui::TextureHandle>,
    image: Option<image::RgbImage>,
    confirm_delete: bool,
    /// Horizontal distance of the current swipe
    swipe_dx: f32,
}

impl Viewer {
    fn path(&self) -> &Path {
        &self.images[self.index]
    }
}

#[derive(Default)]
pub struct FileManager {
    thumbnails: ThumbnailCache,
    /// Gallery: saved sessions, newest first
    sessions: Vec<SessionFolder>,
    /// Gallery: session whose images are shown
    selected_session: usize,
    /// Gallery: session awaiting a second tap to confirm deletion
    confirm_delete_session: Option<PathBuf>,
    viewer: Option<Viewer>,
    /// USB browser: mount point the browser can't go above
    browse_root: PathBuf,
    browse_dir: PathBuf,
    browse_entries: Vec<DirEntry>,
    open_task: Option<OpenTask>,
}

/// An image being decoded in the background
struct OpenTask {
    purpose: OpenPurpose,
    path: PathBuf,
    task: BackgroundTask<Result<image::RgbImage, String>>,
}

impl PixelSorterApp {
    /// Show the saved sessions
    pub fn open_gallery(&mut self) {
        self.files.sessions = library::list_sessions(&crate::system::paths::sorted_images_dir());
        self.files.confirm_delete_session = None;
        self.files.viewer = None;
        self.enter_file_screen(Phase::Gallery);
    }

    /// Browse a USB drive for an image to sort
    pub fn open_usb_browser(&mut self) {
        let Some(root) = crate::system::usb::find_usb_mounts().into_iter().next() else {
            self.show_message("❌ No USB drive found");
            return;
        };
        self.files.browse_root = root.clone();
        if self.browse_to(root) {
            self.enter_file_screen(Phase::Browse);
        }
    }

    fn enter_file_screen(&mut self, phase: Phase) {
        // The live preview isn't visible here; stop decoding camera frames
        if let Some(camera) = &self.camera_controller {
            if let Ok(mut camera) = camera.try_lock() {
                camera.stop_streaming();
            }
        }
        self.current_phase = phase;
    }

    /// Back to the camera; frees thumbnails (the preview restarts streaming itself)
    pub fn close_file_screens(&mut self) {
        self.files.thumbnails.clear();
        self.files.viewer = None;
        self.files.open_task = None;
        self.files.sessions.clear();
        self.files.browse_entries.clear();
        self.current_phase = Phase::Input;
    }

    fn browse_to(&mut self, dir: PathBuf) -> bool {
        match library::list_dir(&dir) {
            Ok(entries) => {
                self.files.browse_entries = entries;
                self.files.browse_dir = dir;
                true
            }
            Err(e) => {
                log::error!("Cannot open {}: {}", dir.display(), e);
                self.show_message(format!("❌ Cannot open folder: {}", e));
                false
            }
        }
    }

    /// Decode an image in the background for viewing or editing
    fn open_image(&mut self, ctx: &egui::Context, path: &Path, purpose: OpenPurpose) {
        let task_path = path.to_path_buf();
        let task = BackgroundTask::spawn(ctx, "open-image", move || {
            image::open(&task_path).map(|img| img.to_rgb8()).map_err(|e| e.to_string())
        });
        self.files.open_task = Some(OpenTask { purpose, path: path.to_path_buf(), task });
    }

    /// Turn a decoded image into the edit source
    fn edit_image(&mut self, ctx: &egui::Context, image: image::RgbImage, saved: bool) {
        self.close_file_screens();
        self.cancel_sort();
        // Further iterations go to a new session
        self.current_session_folder = None;
        self.iteration_counter = 0;
        self.has_unsaved_changes = !saved;
        self.original_image = Some(image.clone());
        self.processed_image = Some(image.clone());
        self.create_processed_texture(ctx, image);
        self.current_phase = Phase::Edit;
    }

    /// Collect finished thumbnails and decoded images
    pub fn poll_file_tasks(&mut self, ctx: &egui::Context) {
        self.files.thumbnails.poll(ctx);

        let Some(open) = &self.files.open_task else {
            return;
        };
        let Some(outcome) = open.task.poll() else {
            return;
        };
        let (purpose, path) = (open.purpose, open.path.clone());
        self.files.open_task = None;

        match outcome {
            Some(Ok(image)) => match purpose {
                OpenPurpose::View => {
                    if let Some(viewer) = self.files.viewer.as_mut().filter(|v| v.path() == path) {
                        let size = [image.width() as usize, image.height() as usize];
                        let color_image = egui::ColorImage::from_rgb(size, image.as_raw());
                        viewer.texture = Some(ctx.load_texture("gallery_viewer", color_image, egui::TextureOptions::LINEAR));
                        viewer.image = Some(image);
                    }
                }
                OpenPurpose::Edit { saved } => self.edit_image(ctx, image, saved),
            },
            Some(Err(e)) => {
                log::error!("Failed to open {}: {}", path.display(), e);
                self.show_message(format!("❌ Could not open image: {}", e));
            }
            None => log::error!("Image loading thread ended without a result"),
        }
    }

    // ========================================================================
    // RENDERING: white Swiss pages (Gallery, USB browser) and a black viewer
    // ========================================================================

    pub fn render_file_screen(&mut self, ui: &mut egui::Ui, rect: Rect, ctx: &egui::Context) {
        let s = theme::scale(ctx);
        match self.current_phase {
            Phase::Gallery if self.files.viewer.is_some() => self.render_viewer(ui, rect, ctx, s),
            Phase::Gallery => {
                ui.painter().rect_filled(rect, 0.0, PAGE);
                self.render_gallery(ui, rect, ctx, s);
            }
            Phase::Browse => {
                ui.painter().rect_filled(rect, 0.0, PAGE);
                self.render_browser(ui, rect, ctx, s);
            }
            _ => {}
        }
    }

    fn render_gallery(&mut self, ui: &mut egui::Ui, screen: Rect, ctx: &egui::Context, s: f32) {
        let m = MARGIN * s;
        let count = self.files.sessions.len();
        page_title(ui, screen, s, "Gallery", &format!(" {} {}", count, if count == 1 { "session" } else { "sessions" }));
        if top_link(ui, screen, s, 0, "Close", INK).clicked() {
            self.close_file_screens();
            return;
        }
        let total: usize = self.files.sessions.iter().map(|sess| sess.images.len()).sum();
        big_count(ui, screen, s, "All", total);

        if self.files.sessions.is_empty() {
            let g = theme::galley(ctx, "No saved images yet", theme::semibold(40.0 * s), FAINT, -0.03);
            ui.painter().galley(Pos2::new(screen.min.x + m, screen.min.y + LIST_TOP * s), g);
            return;
        }
        self.files.selected_session = self.files.selected_session.min(count - 1);

        // Left: sessions as big type, the selected one black with a red dot
        let column_split = screen.min.x + 500.0 * s;
        let list = Rect::from_min_max(
            Pos2::new(screen.min.x + m, screen.min.y + LIST_TOP * s),
            Pos2::new(column_split - 24.0 * s, screen.max.y - 150.0 * s),
        );
        let mut select = None;
        ui.allocate_ui_at_rect(list, |ui| {
            egui::ScrollArea::vertical().id_source("gallery_sessions").auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0 * s;
                for (i, session) in self.files.sessions.iter().enumerate() {
                    let selected = i == self.files.selected_session;
                    let title = session_title(&session.name);
                    if big_row(ui, s, &title, session.images.len(), selected, selected, 52.0).clicked() {
                        select = Some(i);
                    }
                }
            });
        });
        if let Some(i) = select {
            self.files.selected_session = i;
            self.files.confirm_delete_session = None;
        }

        // Right: the selected session's images, two per row
        let session = &self.files.sessions[self.files.selected_session];
        let grid = Rect::from_min_max(
            Pos2::new(column_split, screen.min.y + LIST_TOP * s),
            Pos2::new(screen.max.x - m, screen.max.y - 84.0 * s),
        );
        let mut open = None;
        ui.allocate_ui_at_rect(grid, |ui| {
            egui::ScrollArea::vertical().id_source("gallery_images").auto_shrink([false, false]).show(ui, |ui| {
                let columns = 2;
                let gap = 8.0 * s;
                let width = (grid.width() - gap * (columns as f32 - 1.0)) / columns as f32;
                ui.spacing_mut().item_spacing = Vec2::splat(gap);
                for (row, paths) in session.images.chunks(columns).enumerate() {
                    ui.horizontal(|ui| {
                        for (col, path) in paths.iter().enumerate() {
                            let size = Vec2::new(width, 136.0 * s);
                            if image_tile(ui, ctx, &mut self.files.thumbnails, path, size, None, s).clicked() {
                                open = Some(row * columns + col);
                            }
                        }
                    });
                }
            });
        });

        // Bottom of the right column: delete (tap twice) and export
        let session_path = session.path.clone();
        let confirming = self.files.confirm_delete_session.as_ref() == Some(&session_path);
        let y = screen.max.y - 36.0 * s;
        let (label, color) = if confirming { ("Tap again to delete", theme::RED) } else { ("Delete session", theme::GREY) };
        let g = theme::caps(ctx, label, 13.0 * s, color);
        let delete = theme::text_button(ui, "delete_session", Pos2::new(column_split, y), Align2::LEFT_BOTTOM, g, false, false);
        let g = theme::caps(ctx, "Export to USB →", 13.0 * s, INK);
        let export = theme::text_button(ui, "gallery_export", Pos2::new(screen.max.x - m, y), Align2::RIGHT_BOTTOM, g, false, false);

        if delete.clicked() {
            if confirming {
                match library::delete_session(&session_path) {
                    Ok(()) => self.show_message("✔ Session deleted"),
                    Err(e) => self.show_message(format!("❌ Delete failed: {}", e)),
                }
                self.open_gallery();
            } else {
                self.files.confirm_delete_session = Some(session_path);
            }
        } else if export.clicked() {
            if self.prev_usb_present {
                // Preselect just this session
                self.open_usb_export_dialog();
                for (sess, on) in &mut self.usb_export_choices {
                    *on = sess.path == session_path;
                }
            } else {
                self.show_message("❌ No USB drive found");
            }
        } else if let Some(index) = open {
            self.files.confirm_delete_session = None;
            let images = self.files.sessions[self.files.selected_session].images.clone();
            self.show_in_viewer(ctx, images, index);
        }
    }

    /// Open `images[index]` full screen
    fn show_in_viewer(&mut self, ctx: &egui::Context, images: Vec<PathBuf>, index: usize) {
        let Some(path) = images.get(index).cloned() else {
            return;
        };
        self.files.viewer = Some(Viewer { images, index, texture: None, image: None, confirm_delete: false, swipe_dx: 0.0 });
        self.open_image(ctx, &path, OpenPurpose::View);
    }

    fn render_viewer(&mut self, ui: &mut egui::Ui, screen: Rect, ctx: &egui::Context, s: f32) {
        let Some(viewer) = &self.files.viewer else {
            return;
        };
        let path = viewer.path().to_path_buf();
        let (index, count, confirm_delete) = (viewer.index, viewer.images.len(), viewer.confirm_delete);
        let painter = ui.painter().clone();
        painter.rect_filled(screen, 0.0, Color32::BLACK);

        // Swipe left/right for the next/previous image
        let swipe = ui.interact(screen, ui.id().with("viewer_swipe"), egui::Sense::drag());
        let mut step: Option<isize> = None;
        let dx = viewer.swipe_dx + swipe.drag_delta().x;
        if swipe.drag_released() && dx.abs() > 60.0 * s {
            step = Some(if dx < 0.0 { 1 } else { -1 });
        }
        let swipe_dx = if swipe.dragged() { dx } else { 0.0 };

        // The full image, or its thumbnail while it decodes
        let texture = viewer.texture.clone().or_else(|| self.files.thumbnails.get(ctx, &path).cloned());
        if let Some(texture) = &texture {
            let display = fit_image_in_rect(texture.size_vec2(), screen.size());
            painter.image(texture.id(), center_rect_in_rect(display, screen), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        bottom_shade(&painter, screen, 200.0 * s);

        // Top: back link and position dots
        let g = theme::caps(ctx, "← Gallery", 13.0 * s, Color32::WHITE);
        let back = theme::text_button(ui, "viewer_back", screen.min + Vec2::new(24.0, 20.0) * s, Align2::LEFT_TOP, g, false, true);
        position_dots(&painter, ctx, Pos2::new(screen.max.x - 24.0 * s, screen.min.y + 28.0 * s), index, count, s);

        // Bottom left: session and "Edit 002"
        let x = screen.min.x + 40.0 * s;
        let mut y = screen.max.y - 32.0 * s;
        let number = format!("{:03}", index + 1);
        let lead = number.len() - number.trim_start_matches('0').len().max(1);
        let mut job = egui::text::LayoutJob::default();
        let font = theme::semibold(48.0 * s);
        let tracking = -0.04 * 48.0 * s;
        for (text, color) in [("Edit ", Color32::WHITE), (&number[..lead], Color32::from_white_alpha(115)), (&number[lead..], Color32::WHITE)] {
            job.append(text, 0.0, egui::TextFormat { font_id: font.clone(), color, extra_letter_spacing: tracking, ..Default::default() });
        }
        let title = ctx.fonts(|f| f.layout_job(job));
        y -= title.size().y;
        theme::paint_shadowed(&painter, Pos2::new(x, y), title, 1.0);
        let session = path.parent().and_then(|p| p.file_name()).map(|n| session_title(&n.to_string_lossy())).unwrap_or_default();
        let g = theme::caps(ctx, &session, 11.0 * s, Color32::from_white_alpha(190));
        theme::paint_shadowed(&painter, Pos2::new(x, y - g.size().y - 2.0 * s), g, 1.0);

        // Bottom right: Delete (tap twice) and Edit
        let font = theme::medium(28.0 * s);
        let right = screen.max.x - 40.0 * s;
        let base = screen.max.y - 36.0 * s;
        let g = theme::galley(ctx, "Edit", font.clone(), Color32::WHITE, -0.02);
        let edit_w = g.size().x + g.size().y * 0.63;
        let edit = theme::text_button(ui, "viewer_edit", Pos2::new(right, base), Align2::RIGHT_BOTTOM, g, true, true);
        let (label, color) = if confirm_delete { ("Sure?", theme::RED) } else { ("Delete", Color32::from_white_alpha(150)) };
        let g = theme::galley(ctx, label, font, color, -0.02);
        let delete = theme::text_button(ui, "viewer_delete", Pos2::new(right - edit_w - 36.0 * s, base), Align2::RIGHT_BOTTOM, g, false, true);

        if let Some(viewer) = &mut self.files.viewer {
            viewer.swipe_dx = swipe_dx;
        }
        if back.clicked() {
            self.files.viewer = None;
        } else if edit.clicked() {
            if let Some(image) = self.files.viewer.as_mut().and_then(|v| v.image.take()) {
                self.edit_image(ctx, image, true);
            }
        } else if delete.clicked() {
            if !confirm_delete {
                if let Some(viewer) = &mut self.files.viewer {
                    viewer.confirm_delete = true;
                }
                return;
            }
            match library::delete_image(&path) {
                Ok(()) => self.show_message("✔ Image deleted"),
                Err(e) => self.show_message(format!("❌ Delete failed: {}", e)),
            }
            self.files.thumbnails.forget(&path);
            let mut images = self.files.viewer.take().map(|v| v.images).unwrap_or_default();
            images.retain(|p| p != &path);
            let selected = self.files.selected_session;
            self.open_gallery();
            self.files.selected_session = selected;
            if !images.is_empty() {
                self.show_in_viewer(ctx, images, index.min(count.saturating_sub(2)));
            }
        } else if let Some(step) = step {
            let next = index as isize + step;
            if next >= 0 && (next as usize) < count {
                let images = self.files.viewer.take().map(|v| v.images).unwrap_or_default();
                self.show_in_viewer(ctx, images, next as usize);
            }
        }
    }

    fn render_browser(&mut self, ui: &mut egui::Ui, screen: Rect, ctx: &egui::Context, s: f32) {
        let m = MARGIN * s;
        let drive = self.files.browse_root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let relative = self.files.browse_dir.strip_prefix(&self.files.browse_root).unwrap_or(Path::new(""));
        let path_text: String = relative.iter().map(|part| format!(" / {}", part.to_string_lossy())).collect();
        page_title(ui, screen, s, &drive, &path_text);

        if top_link(ui, screen, s, 0, "Close", INK).clicked() {
            self.close_file_screens();
            return;
        }
        let at_root = self.files.browse_dir == self.files.browse_root;
        if !at_root && top_link(ui, screen, s, 1, "↑ Up", INK).clicked() {
            if let Some(parent) = self.files.browse_dir.parent().map(Path::to_path_buf) {
                self.browse_to(parent);
            }
            return;
        }

        let images: Vec<&DirEntry> = self.files.browse_entries.iter().filter(|e| !e.is_dir).collect();
        let label = if self.files.open_task.is_some() { "Opening…" } else { "Images here" };
        big_count(ui, screen, s, label, images.len());

        let column_split = screen.min.x + 500.0 * s;
        let top = screen.min.y + LIST_TOP * s;
        let mut clicked: Option<DirEntry> = None;

        // Left: folders as big type
        let folders = Rect::from_min_max(Pos2::new(screen.min.x + m, top), Pos2::new(column_split - 24.0 * s, screen.max.y - 150.0 * s));
        section_label(ui, folders.min, s, "Folders");
        let list = folders.with_min_y(folders.min.y + 24.0 * s);
        ui.allocate_ui_at_rect(list, |ui| {
            egui::ScrollArea::vertical().id_source("browse_folders").auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0 * s;
                let mut any = false;
                for entry in self.files.browse_entries.iter().filter(|e| e.is_dir) {
                    any = true;
                    if big_row(ui, s, &entry.name, 0, true, false, 40.0).clicked() {
                        clicked = Some(entry.clone());
                    }
                }
                if !any {
                    let g = theme::galley(ctx, "None", theme::semibold(40.0 * s), FAINT, -0.04);
                    ui.label(egui::WidgetText::from(g));
                }
            });
        });

        // Right: images, three per row with their names
        let grid = Rect::from_min_max(Pos2::new(column_split, top), Pos2::new(screen.max.x - m, screen.max.y - 36.0 * s));
        section_label(ui, grid.min, s, "Images · tap to sort");
        let grid = grid.with_min_y(grid.min.y + 24.0 * s);
        ui.allocate_ui_at_rect(grid, |ui| {
            egui::ScrollArea::vertical().id_source("browse_images").auto_shrink([false, false]).show(ui, |ui| {
                let columns = 3;
                let gap = 8.0 * s;
                let width = (grid.width() - gap * (columns as f32 - 1.0)) / columns as f32;
                ui.spacing_mut().item_spacing = Vec2::splat(gap);
                for row in images.chunks(columns) {
                    ui.horizontal(|ui| {
                        for entry in row {
                            let size = Vec2::new(width, 100.0 * s);
                            if image_tile(ui, ctx, &mut self.files.thumbnails, &entry.path, size, Some(&entry.name), s).clicked() {
                                clicked = Some((*entry).clone());
                            }
                        }
                    });
                }
            });
        });

        if let Some(entry) = clicked {
            if entry.is_dir {
                self.browse_to(entry.path);
            } else if self.files.open_task.is_none() {
                self.open_image(ctx, &entry.path, OpenPurpose::Edit { saved: false });
            }
        }
    }
}

/// "session_20261021_084818" -> "21.10 08:48"
pub(super) fn session_title(name: &str) -> String {
    let digits: String = name.chars().filter(char::is_ascii_digit).collect();
    if digits.len() >= 12 {
        format!("{}.{} {}:{}", &digits[6..8], &digits[4..6], &digits[8..10], &digits[10..12])
    } else {
        name.to_string()
    }
}

/// "Gallery" in black followed by a grey tail, top left
fn page_title(ui: &mut egui::Ui, screen: Rect, s: f32, title: &str, tail: &str) {
    let font = theme::semibold(26.0 * s);
    let tracking = -0.03 * 26.0 * s;
    let mut job = egui::text::LayoutJob::default();
    for (text, color) in [(title, INK), (tail, theme::GREY)] {
        job.append(text, 0.0, egui::TextFormat { font_id: font.clone(), color, extra_letter_spacing: tracking, ..Default::default() });
    }
    job.wrap = egui::text::TextWrapping { max_width: screen.width() - 300.0 * s, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    let g = ui.ctx().fonts(|f| f.layout_job(job));
    ui.painter().galley(screen.min + Vec2::new(MARGIN, 40.0) * s, g);
}

/// Caps link in the top right; `slot` 0 is the rightmost
fn top_link(ui: &mut egui::Ui, screen: Rect, s: f32, slot: usize, text: &str, color: Color32) -> egui::Response {
    let g = theme::caps(ui.ctx(), text, 13.0 * s, color);
    let x = screen.max.x - MARGIN * s - slot as f32 * 110.0 * s;
    theme::text_button(ui, ("top_link", text), Pos2::new(x, screen.min.y + 44.0 * s), Align2::RIGHT_TOP, g, false, false)
}

fn section_label(ui: &egui::Ui, pos: Pos2, s: f32, text: &str) {
    let g = theme::caps(ui.ctx(), text, 11.0 * s, theme::GREY);
    ui.painter().galley(pos, g);
}

/// "ALL" over a big number, bottom left
fn big_count(ui: &egui::Ui, screen: Rect, s: f32, label: &str, count: usize) {
    let ctx = ui.ctx();
    let number = theme::galley(ctx, &count.to_string(), theme::semibold(64.0 * s), INK, -0.05);
    let y = screen.max.y - 30.0 * s - number.size().y;
    let x = screen.min.x + MARGIN * s;
    let l = theme::caps(ctx, label, 11.0 * s, theme::GREY);
    ui.painter().galley(Pos2::new(x, y - l.size().y), l);
    ui.painter().galley(Pos2::new(x, y), number);
}

/// One line of big type in a list, with an optional "(n)" count and red dot
fn big_row(ui: &mut egui::Ui, s: f32, text: &str, count: usize, strong: bool, dot: bool, size: f32) -> egui::Response {
    let ctx = ui.ctx().clone();
    let width = ui.available_width();
    let color = if strong { INK } else { FAINT };
    let count_text = if count > 0 { format!("({})", count) } else { String::new() };
    let small = theme::galley(&ctx, &count_text, theme::medium(13.0 * s), theme::GREY, 0.0);
    let reserve = small.size().x + 40.0 * s;
    let g = theme::galley_clipped(&ctx, text, theme::semibold(size * s), color, -0.04, width - reserve);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, g.size().y * 1.02), egui::Sense::click());
    let color = if response.is_pointer_button_down_on() { theme::GREY } else { color };
    let p = ui.painter();
    p.galley_with_color(rect.min, g.clone(), color);
    let mut x = rect.min.x + g.size().x + 6.0 * s;
    p.galley(Pos2::new(x, rect.min.y + 6.0 * s), small.clone());
    x += small.size().x;
    if dot {
        p.circle_filled(Pos2::new(x + 13.0 * s, rect.min.y + size * 0.55 * s), 5.0 * s, theme::RED);
    }
    response
}

/// Thumbnail filling `size` (cropped to fit), optionally with a file name below
fn image_tile(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    thumbnails: &mut ThumbnailCache,
    path: &Path,
    size: Vec2,
    label: Option<&str>,
    s: f32,
) -> egui::Response {
    let label_h = if label.is_some() { 22.0 * s } else { 0.0 };
    let (rect, response) = ui.allocate_exact_size(size + Vec2::new(0.0, label_h), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let image_area = Rect::from_min_size(rect.min, size);
    let painter = ui.painter();
    painter.rect_filled(image_area, 0.0, Color32::from_gray(0xF0));
    if let Some(texture) = thumbnails.get(ctx, path) {
        theme::paint_cover(painter, texture, image_area);
    } else if thumbnails.is_failed(path) {
        let g = theme::caps(ctx, "?", 13.0 * s, theme::GREY);
        painter.galley(image_area.center() - g.size() / 2.0, g);
    }
    if response.is_pointer_button_down_on() {
        painter.rect_filled(image_area, 0.0, Color32::from_white_alpha(90));
    }
    if let Some(label) = label {
        let g = theme::galley_clipped(ctx, label, theme::medium(12.0 * s), DIM, 0.0, rect.width());
        painter.galley(Pos2::new(rect.min.x, image_area.max.y + 6.0 * s), g);
    }
    response
}

/// Dark gradient behind the viewer's bottom captions
fn bottom_shade(painter: &egui::Painter, screen: Rect, height: f32) {
    let top = screen.max.y - height;
    let mut mesh = egui::Mesh::default();
    let clear = Color32::TRANSPARENT;
    let dark = Color32::from_black_alpha(180);
    mesh.colored_vertex(Pos2::new(screen.min.x, top), clear);
    mesh.colored_vertex(Pos2::new(screen.max.x, top), clear);
    mesh.colored_vertex(screen.right_bottom(), dark);
    mesh.colored_vertex(screen.left_bottom(), dark);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(mesh);
}

/// Halftone page dots, current one large; right-aligned at `right_center`
fn position_dots(painter: &egui::Painter, ctx: &egui::Context, right_center: Pos2, index: usize, count: usize, s: f32) {
    if count <= 1 {
        return;
    }
    if count > 15 {
        let g = theme::caps(ctx, &format!("{} / {}", index + 1, count), 11.0 * s, Color32::WHITE);
        theme::paint_shadowed(painter, Pos2::new(right_center.x - g.size().x, right_center.y - g.size().y / 2.0), g, 1.0);
        return;
    }
    let gap = 8.0 * s;
    let mut x = right_center.x;
    for i in (0..count).rev() {
        let r = if i == index { 4.5 * s } else { 2.0 * s };
        x -= r;
        let color = if i == index { Color32::WHITE } else { Color32::from_white_alpha(150) };
        painter.circle_filled(Pos2::new(x, right_center.y), r + 1.5 * s, Color32::from_black_alpha(60));
        painter.circle_filled(Pos2::new(x, right_center.y), r, color);
        x -= r + gap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_session_titles() {
        assert_eq!(session_title("session_20261021_084818"), "21.10 08:48");
        assert_eq!(session_title("imported"), "imported");
    }
}
