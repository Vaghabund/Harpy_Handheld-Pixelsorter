use crate::PixelSorterApp;
use crate::session::library;
use crate::ui::background::{take_finished, BackgroundTask};
use eframe::egui;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use crate::processing::SortingAlgorithm;

/// Outcome of a USB export, shown to the user as a toast
pub struct UsbExportResult {
    message: String,
    /// Session folders deleted after copying
    deleted_sessions: Vec<PathBuf>,
}

impl PixelSorterApp {
    fn auto_save_image(&mut self, image: &image::RgbImage, algorithm: &SortingAlgorithm) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Create session folder if this is the first save
        if self.current_session_folder.is_none() {
            let now: DateTime<Local> = Local::now();
            let session_folder = format!("session_{}", now.format("%Y%m%d_%H%M%S"));
            self.current_session_folder = Some(session_folder.clone());
            self.iteration_counter = 0;
        }
        
        // Create session directory
        let session_dir = crate::system::paths::sorted_images_dir().join(self.current_session_folder.as_ref().unwrap());
        std::fs::create_dir_all(&session_dir)?;
        
        // Increment iteration counter for this save
        self.iteration_counter += 1;
        
        // Generate iteration-based filename
        let filename = format!("edit_{:03}_{}.png", 
            self.iteration_counter,
            algorithm.name().to_lowercase()
        );
        
        let save_path = session_dir.join(filename);
        image.save(&save_path)?;
        self.has_unsaved_changes = false;
        
        // Return the path for potential loading in next iteration
        Ok(save_path)
    }

    /// Open the export dialog with all saved sessions selected
    pub fn open_usb_export_dialog(&mut self) {
        self.usb_export_choices = library::list_sessions(&crate::system::paths::sorted_images_dir())
            .into_iter()
            .map(|session| (session, true))
            .collect();
        self.usb_export_delete_after = false;
        self.show_usb_export_dialog = true;
    }

    /// Copy the selected sessions to the USB drive in the background
    pub fn start_usb_export(&mut self, ctx: &egui::Context) {
        if self.usb_export_task.is_some() {
            return; // Export already running
        }
        let sessions: Vec<PathBuf> = self
            .usb_export_choices
            .iter()
            .filter(|(_, selected)| *selected)
            .map(|(session, _)| session.path.clone())
            .collect();
        if sessions.is_empty() {
            self.show_message("❌ Nothing selected to export");
            return;
        }
        let delete_after = self.usb_export_delete_after;
        log::info!("USB export of {} sessions started (delete_after: {})", sessions.len(), delete_after);
        self.show_message("⏳ Copying to USB...");
        self.usb_export_task = Some(BackgroundTask::spawn(ctx, "usb-export", move || {
            Self::export_to_usb(&sessions, delete_after)
        }));
    }

    /// Show the outcome of a finished USB export
    pub fn poll_usb_export(&mut self) {
        match take_finished(&mut self.usb_export_task) {
            Some(Some(result)) => {
                let current_deleted = self.current_session_folder.as_ref().is_some_and(|current| {
                    result.deleted_sessions.iter().any(|path| path.file_name().is_some_and(|name| name == current.as_str()))
                });
                if current_deleted {
                    // Reset session since its folder was deleted
                    self.current_session_folder = None;
                    self.iteration_counter = 0;
                }
                self.show_message(result.message);
            }
            Some(None) => self.show_message("❌ Export failed unexpectedly"),
            None => {}
        }
    }

    /// Copy sessions to USB and optionally delete the local copies.
    /// Runs on a background thread.
    fn export_to_usb(sessions: &[PathBuf], delete_after: bool) -> UsbExportResult {
        let failed = |message: String| UsbExportResult { message, deleted_sessions: Vec::new() };

        let Some(usb_path) = crate::system::usb::find_writable_usb_mount() else {
            return failed("❌ Export failed: No writable USB drive found".to_string());
        };
        let dest_path = usb_path.join("pixelsort_export");
        let copied = match library::copy_sessions(sessions, &dest_path) {
            Ok(copied) => copied,
            Err(e) => return failed(format!("❌ Export failed: Copy failed: {}", e)),
        };
        log::info!("Copied {} files to USB: {}", copied, dest_path.display());

        if !delete_after {
            return UsbExportResult { message: format!("✔ Exported {} images to USB!", copied), deleted_sessions: Vec::new() };
        }
        let mut deleted_sessions = Vec::new();
        for session in sessions {
            match library::delete_session(session) {
                Ok(()) => deleted_sessions.push(session.clone()),
                Err(e) => {
                    log::error!("Failed to delete {}: {}", session.display(), e);
                    return UsbExportResult { message: format!("⚠ Exported but deletion failed: {}", e), deleted_sessions };
                }
            }
        }
        log::info!("Deleted {} exported sessions", deleted_sessions.len());
        UsbExportResult { message: format!("✔ Exported {} images and deleted local copies!", copied), deleted_sessions }
    }

    /// "New" button: start over, asking first if the current image has unsaved edits
    pub fn request_new_photo_session(&mut self) {
        if self.has_unsaved_changes && self.processed_image.is_some() {
            self.show_new_session_dialog = true;
        } else {
            self.start_new_photo_session();
        }
    }

    /// Save the current result (as an iteration of this session), then start over
    pub fn save_and_start_new_photo_session(&mut self) {
        let Some(processed) = self.processed_image.clone() else {
            self.start_new_photo_session();
            return;
        };
        let algorithm = self.current_algorithm;
        match self.auto_save_image(&processed, &algorithm) {
            Ok(path) => {
                log::info!("Saved {} before starting new photo", path.display());
                self.show_message("✔ Saved");
                self.start_new_photo_session();
            }
            Err(e) => {
                // Stay on the image so nothing is lost
                log::error!("Failed to save before new photo: {}", e);
                self.show_message(format!("❌ Save failed: {}", e));
                self.show_new_session_dialog = false;
            }
        }
    }

    pub fn start_new_photo_session(&mut self) {
        // Reset session state
        self.cancel_sort();
        self.has_unsaved_changes = false;
        self.show_new_session_dialog = false;
        self.iteration_counter = 0;
        self.current_session_folder = None;
        self.original_image = None;
        self.processed_image = None;
        self.camera_texture = None;
        self.processed_texture = None;
        self.last_camera_update = None; // Reset camera timer to immediately start fresh
        self.current_phase = crate::ui::Phase::Input; // Return to Input phase

        // Restart camera streaming for new session
        if let Some(ref camera) = self.camera_controller {
            if let Ok(mut camera_lock) = camera.try_lock() {
                let _ = camera_lock.start_streaming();
            }
        }
    }

    /// Save the current result and use it as the source for the next round of sorting
    pub fn save_and_continue_iteration(&mut self, ctx: &egui::Context) {
        let Some(processed) = self.processed_image.clone() else {
            return;
        };
        let algorithm = self.current_algorithm;
        if let Err(e) = self.auto_save_image(&processed, &algorithm) {
            log::error!("Failed to save iteration: {}", e);
            self.show_message(format!("❌ Save failed: {}", e));
            return;
        }

        // The saved image (with its tint baked in) becomes the new source. Reset
        // the tint so it isn't applied again on top on the next pass.
        self.original_image = Some(processed);
        self.sorting_params.color_tint = 0.0;
        self.tint_enabled = false;

        // Re-sort the new source for preview; a sort of the previous source
        // still in flight is obsolete. This automatic re-sort isn't a new edit.
        self.cancel_sort();
        self.apply_pixel_sort(ctx);
        if let Some(job) = &mut self.sort_job {
            job.marks_unsaved = false;
        }
    }
}