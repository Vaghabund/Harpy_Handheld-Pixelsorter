//! In-app file screens: gallery of saved sessions (with full-screen viewer,
//! re-edit and delete) and a USB image browser. Replaces the desktop file dialog.

use crate::PixelSorterApp;
use crate::session::library::{self, DirEntry, SessionFolder};
use crate::ui::background::BackgroundTask;
use crate::ui::components::circular_button;
use crate::ui::helpers::{center_rect_in_rect, fit_image_in_rect};
use crate::ui::state::Phase;
use crate::ui::styles::{button_dark, button_green, button_red};
use crate::ui::thumbnails::ThumbnailCache;
use eframe::egui;
use std::path::{Path, PathBuf};

const BUTTON_RADIUS: f32 = 60.0;
const SPACING: f32 = 20.0;
/// Space reserved at the top-left for the power button
const TOP_RESERVED: f32 = 120.0;
const HEADER_HEIGHT: f32 = 60.0;
const TILE_MIN_SIZE: f32 = 220.0;
const TILE_GAP: f32 = 16.0;
const TILE_LABEL_HEIGHT: f32 = 36.0;

/// Why an image is being decoded in the background
#[derive(Clone, Copy, PartialEq)]
enum OpenPurpose {
    /// Show it in the gallery viewer
    View,
    /// Load it into Edit; `saved` is false for images from USB (not yet in a session)
    Edit { saved: bool },
}

/// A saved image shown full screen in the gallery
struct Viewer {
    path: PathBuf,
    texture: Option<egui::TextureHandle>,
    image: Option<image::RgbImage>,
    confirm_delete: bool,
}

#[derive(Default)]
pub struct FileManager {
    thumbnails: ThumbnailCache,
    /// Gallery: saved sessions, newest first
    sessions: Vec<SessionFolder>,
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
                    if let Some(viewer) = self.files.viewer.as_mut().filter(|v| v.path == path) {
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
    // RENDERING
    // ========================================================================

    pub fn render_file_screen(&mut self, ui: &mut egui::Ui, rect: egui::Rect, ctx: &egui::Context) {
        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);

        let content = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + BUTTON_RADIUS * 2.0 + SPACING * 2.0, rect.min.y + SPACING),
            egui::pos2(rect.max.x - SPACING, rect.max.y),
        );

        match self.current_phase {
            Phase::Gallery if self.files.viewer.is_some() => self.render_viewer(ui, rect, content, ctx),
            Phase::Gallery => self.render_gallery(ui, rect, content, ctx),
            Phase::Browse => self.render_browser(ui, rect, content, ctx),
            _ => {}
        }
    }

    /// Circular button in the left column, `slot` 0 = top
    fn side_button(ui: &mut egui::Ui, screen: egui::Rect, slot: usize, label: &str, fill: egui::Color32) -> bool {
        let min = egui::pos2(
            screen.min.x + SPACING,
            screen.min.y + TOP_RESERVED + slot as f32 * (BUTTON_RADIUS * 2.0 + SPACING),
        );
        let rect = egui::Rect::from_min_size(min, egui::vec2(BUTTON_RADIUS * 2.0, BUTTON_RADIUS * 2.0));
        ui.allocate_ui_at_rect(rect, |ui| circular_button(ui, BUTTON_RADIUS, label, fill)).inner
    }

    fn header(ui: &mut egui::Ui, content: egui::Rect, title: &str, subtitle: &str) {
        let painter = ui.painter();
        painter.text(
            content.min,
            egui::Align2::LEFT_TOP,
            title,
            egui::FontId::proportional(32.0),
            egui::Color32::WHITE,
        );
        painter.text(
            content.min + egui::vec2(0.0, 38.0),
            egui::Align2::LEFT_TOP,
            subtitle,
            egui::FontId::proportional(18.0),
            egui::Color32::GRAY,
        );
    }

    fn grid_layout(width: f32) -> (usize, f32) {
        let columns = (((width + TILE_GAP) / (TILE_MIN_SIZE + TILE_GAP)).floor() as usize).max(1);
        let tile = (width - TILE_GAP * (columns as f32 - 1.0)) / columns as f32;
        (columns, tile)
    }

    fn render_gallery(&mut self, ui: &mut egui::Ui, screen: egui::Rect, content: egui::Rect, ctx: &egui::Context) {
        if Self::side_button(ui, screen, 0, "Back", button_dark()) {
            self.close_file_screens();
            return;
        }

        let count: usize = self.files.sessions.iter().map(|s| s.images.len()).sum();
        Self::header(ui, content, "Gallery", &format!("{} images in {} sessions", count, self.files.sessions.len()));

        let list_rect = egui::Rect::from_min_max(content.min + egui::vec2(0.0, HEADER_HEIGHT + SPACING), content.max);
        if self.files.sessions.is_empty() {
            ui.painter().text(
                list_rect.center_top() + egui::vec2(0.0, 80.0),
                egui::Align2::CENTER_CENTER,
                "No saved images yet. Use Iterate to save your edits.",
                egui::FontId::proportional(24.0),
                egui::Color32::GRAY,
            );
            return;
        }

        let (columns, tile) = Self::grid_layout(list_rect.width());
        let mut open: Option<PathBuf> = None;
        let mut delete_session: Option<PathBuf> = None;

        ui.allocate_ui_at_rect(list_rect, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for session in &self.files.sessions {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{}  ·  {} images", session_title(&session.name), session.images.len()))
                                .size(22.0)
                                .color(egui::Color32::WHITE),
                        );
                        let confirming = self.files.confirm_delete_session.as_ref() == Some(&session.path);
                        let (label, fill) = if confirming {
                            ("🗑 Tap again to delete session", egui::Color32::from_rgb(180, 40, 40))
                        } else {
                            ("🗑 Delete session", egui::Color32::from_rgb(60, 60, 70))
                        };
                        ui.add_space(SPACING);
                        if ui.add_sized([280.0, 44.0], egui::Button::new(egui::RichText::new(label).size(18.0)).fill(fill)).clicked() {
                            if confirming {
                                delete_session = Some(session.path.clone());
                            } else {
                                self.files.confirm_delete_session = Some(session.path.clone());
                            }
                        }
                    });
                    ui.add_space(8.0);
                    for row in session.images.chunks(columns) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = TILE_GAP;
                            for path in row {
                                if image_tile(ui, ctx, &mut self.files.thumbnails, path, tile, None).clicked() {
                                    open = Some(path.clone());
                                }
                            }
                        });
                        ui.add_space(TILE_GAP);
                    }
                    ui.add_space(SPACING);
                }
            });
        });

        if let Some(path) = delete_session {
            match library::delete_session(&path) {
                Ok(()) => self.show_message("✔ Session deleted"),
                Err(e) => self.show_message(format!("❌ Delete failed: {}", e)),
            }
            self.open_gallery();
        } else if let Some(path) = open {
            self.files.confirm_delete_session = None;
            self.files.viewer = Some(Viewer { path: path.clone(), texture: None, image: None, confirm_delete: false });
            self.open_image(ctx, &path, OpenPurpose::View);
        }
    }

    fn render_viewer(&mut self, ui: &mut egui::Ui, screen: egui::Rect, content: egui::Rect, ctx: &egui::Context) {
        let Some(viewer) = &self.files.viewer else {
            return;
        };
        let path = viewer.path.clone();
        let confirm_delete = viewer.confirm_delete;

        if let Some(texture) = &viewer.texture {
            let display = fit_image_in_rect(texture.size_vec2(), content.size());
            let image_rect = center_rect_in_rect(display, content);
            ui.allocate_ui_at_rect(image_rect, |ui| {
                ui.add(egui::Image::new(texture).fit_to_exact_size(display));
            });
        } else {
            ui.painter().text(content.center(), egui::Align2::CENTER_CENTER, "Loading…", egui::FontId::proportional(28.0), egui::Color32::GRAY);
        }

        if Self::side_button(ui, screen, 0, "Back", button_dark()) {
            self.files.viewer = None;
            return;
        }
        if Self::side_button(ui, screen, 1, "Edit", button_green()) {
            if let Some(image) = self.files.viewer.as_mut().and_then(|v| v.image.take()) {
                self.edit_image(ctx, image, true);
            }
            return;
        }
        let delete_label = if confirm_delete { "Sure?" } else { "Delete" };
        if Self::side_button(ui, screen, 2, delete_label, button_red()) {
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
            self.open_gallery();
        }
    }

    fn render_browser(&mut self, ui: &mut egui::Ui, screen: egui::Rect, content: egui::Rect, ctx: &egui::Context) {
        if Self::side_button(ui, screen, 0, "Back", button_dark()) {
            self.close_file_screens();
            return;
        }
        let at_root = self.files.browse_dir == self.files.browse_root;
        if !at_root && Self::side_button(ui, screen, 1, "Up", button_dark()) {
            if let Some(parent) = self.files.browse_dir.parent().map(Path::to_path_buf) {
                self.browse_to(parent);
            }
            return;
        }

        let drive = self.files.browse_root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let relative = self.files.browse_dir.strip_prefix(&self.files.browse_root).unwrap_or(Path::new(""));
        let subtitle = if self.files.open_task.is_some() {
            "Opening image…".to_string()
        } else {
            format!("/{}", relative.display())
        };
        Self::header(ui, content, &format!("💾 {}", drive), &subtitle);

        let list_rect = egui::Rect::from_min_max(content.min + egui::vec2(0.0, HEADER_HEIGHT + SPACING), content.max);
        if self.files.browse_entries.is_empty() {
            ui.painter().text(
                list_rect.center_top() + egui::vec2(0.0, 80.0),
                egui::Align2::CENTER_CENTER,
                "No images or folders here",
                egui::FontId::proportional(24.0),
                egui::Color32::GRAY,
            );
            return;
        }

        let (columns, tile) = Self::grid_layout(list_rect.width());
        let mut clicked: Option<DirEntry> = None;
        ui.allocate_ui_at_rect(list_rect, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for row in self.files.browse_entries.chunks(columns) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = TILE_GAP;
                        for entry in row {
                            let response = if entry.is_dir {
                                folder_tile(ui, &entry.name, tile)
                            } else {
                                image_tile(ui, ctx, &mut self.files.thumbnails, &entry.path, tile, Some(&entry.name))
                            };
                            if response.clicked() {
                                clicked = Some(entry.clone());
                            }
                        }
                    });
                    ui.add_space(TILE_GAP);
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

/// "session_20261021_084818" -> "21.10.2026 08:48"
fn session_title(name: &str) -> String {
    let digits: String = name.chars().filter(char::is_ascii_digit).collect();
    if digits.len() >= 12 {
        format!("{}.{}.{} {}:{}", &digits[6..8], &digits[4..6], &digits[0..4], &digits[8..10], &digits[10..12])
    } else {
        name.to_string()
    }
}

/// Square tile with an image thumbnail (loaded lazily when visible)
fn image_tile(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    thumbnails: &mut ThumbnailCache,
    path: &Path,
    size: f32,
    label: Option<&str>,
) -> egui::Response {
    let height = size + if label.is_some() { TILE_LABEL_HEIGHT } else { 0.0 };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, height), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let image_area = egui::Rect::from_min_size(rect.min, egui::vec2(size, size));
    let pressed = response.is_pointer_button_down_on();
    ui.painter().rect_filled(image_area, 8.0, egui::Color32::from_gray(if pressed { 60 } else { 30 }));

    if let Some(texture) = thumbnails.get(ctx, path) {
        let display = fit_image_in_rect(texture.size_vec2(), image_area.size() - egui::vec2(8.0, 8.0));
        let image_rect = center_rect_in_rect(display, image_area);
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        ui.painter().image(texture.id(), image_rect, uv, egui::Color32::WHITE);
    } else {
        let text = if thumbnails.is_failed(path) { "?" } else { "…" };
        ui.painter().text(image_area.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(32.0), egui::Color32::GRAY);
    }

    if let Some(label) = label {
        tile_label(ui, rect, label);
    }
    response
}

fn folder_tile(ui: &mut egui::Ui, name: &str, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size + TILE_LABEL_HEIGHT), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let image_area = egui::Rect::from_min_size(rect.min, egui::vec2(size, size));
        let pressed = response.is_pointer_button_down_on();
        ui.painter().rect_filled(image_area, 8.0, egui::Color32::from_gray(if pressed { 60 } else { 30 }));
        ui.painter().text(image_area.center(), egui::Align2::CENTER_CENTER, "📁", egui::FontId::proportional(size * 0.35), egui::Color32::from_rgb(220, 190, 90));
        tile_label(ui, rect, name);
    }
    response
}

fn tile_label(ui: &egui::Ui, rect: egui::Rect, label: &str) {
    let label_rect = egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - TILE_LABEL_HEIGHT), rect.max);
    // Truncate long names to fit the tile
    let max_chars = ((rect.width() / 10.0) as usize).max(4);
    let text = if label.chars().count() > max_chars {
        format!("{}…", label.chars().take(max_chars - 1).collect::<String>())
    } else {
        label.to_string()
    };
    ui.painter().text(label_rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(18.0), egui::Color32::LIGHT_GRAY);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_session_titles() {
        assert_eq!(session_title("session_20261021_084818"), "21.10.2026 08:48");
        assert_eq!(session_title("imported"), "imported");
    }

    #[test]
    fn grid_fits_at_least_one_column() {
        assert_eq!(PixelSorterApp::grid_layout(100.0).0, 1);
        let (columns, tile) = PixelSorterApp::grid_layout(1700.0);
        assert!(columns >= 6);
        assert!(tile >= TILE_MIN_SIZE);
    }
}
