use crate::PixelSorterApp;
use crate::ui::background::{take_finished, BackgroundTask};
use eframe::egui;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use crate::processing::SortingAlgorithm;

/// Outcome of a USB export, shown to the user as a toast
pub struct UsbExportResult {
    message: String,
    /// Local images were deleted, so the session must be reset
    deleted_local: bool,
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
        
        // Return the path for potential loading in next iteration
        Ok(save_path)
    }

    /// Copy all sorted images to the USB drive in the background
    pub fn start_usb_export(&mut self, ctx: &egui::Context, delete_after: bool) {
        if self.usb_export_task.is_some() {
            return; // Export already running
        }
        log::info!("USB export started (delete_after: {})", delete_after);
        self.show_message("⏳ Copying to USB...");
        self.usb_export_task = Some(BackgroundTask::spawn(ctx, "usb-export", move || {
            Self::export_to_usb(delete_after)
        }));
    }

    /// Show the outcome of a finished USB export
    pub fn poll_usb_export(&mut self) {
        match take_finished(&mut self.usb_export_task) {
            Some(Some(result)) => {
                if result.deleted_local {
                    // Reset session since we deleted everything
                    self.current_session_folder = None;
                    self.iteration_counter = 0;
                }
                self.show_message(result.message);
            }
            Some(None) => self.show_message("✗ Export failed unexpectedly"),
            None => {}
        }
    }

    /// Copy sorted images to USB and optionally delete the local copies.
    /// Runs on a background thread.
    fn export_to_usb(delete_after: bool) -> UsbExportResult {
        if let Err(e) = Self::copy_to_usb() {
            return UsbExportResult { message: format!("✗ Export failed: {}", e), deleted_local: false };
        }
        if !delete_after {
            return UsbExportResult { message: "✓ Exported to USB!".to_string(), deleted_local: false };
        }
        match std::fs::remove_dir_all(crate::system::paths::sorted_images_dir()) {
            Ok(()) => {
                log::info!("Deleted local sorted_images folder after export");
                UsbExportResult { message: "✓ Exported and deleted local copies!".to_string(), deleted_local: true }
            }
            Err(e) => {
                log::error!("Failed to delete images: {}", e);
                UsbExportResult { message: format!("⚠ Exported but deletion failed: {}", e), deleted_local: false }
            }
        }
    }

    fn copy_to_usb() -> Result<(), String> {
        let usb_path = crate::system::usb::find_writable_usb_mount()
            .ok_or("No writable USB drive found")?;

        // Copy sorted_images folder to USB
        let dest_path = usb_path.join("pixelsort_export");
        Self::copy_directory(crate::system::paths::sorted_images_dir(), dest_path.clone())
            .map_err(|e| format!("Copy failed: {}", e))?;

        log::info!("Successfully copied to USB: {}", dest_path.display());
        Ok(())
    }

    fn copy_directory<P: AsRef<std::path::Path>>(src: P, dst: P) -> Result<(), Box<dyn std::error::Error>> {
        let src = src.as_ref();
        let dst = dst.as_ref();
        
        if !src.exists() {
            return Err("Source directory does not exist".into());
        }

        std::fs::create_dir_all(dst)?;

        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                // Recursively copy subdirectories (session folders)
                Self::copy_directory(&src_path, &dst_path)?;
            } else if src_path.is_file() {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    pub fn start_new_photo_session(&mut self) {
        // Reset session state
        self.cancel_sort();
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

    pub fn load_last_iteration_as_source(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref session_folder) = self.current_session_folder {
            if self.iteration_counter > 0 {
                // Load the last saved iteration as the new source
                let session_dir = crate::system::paths::sorted_images_dir().join(session_folder);
                
                // Find the last saved file
                let iteration_prefix = format!("edit_{:03}_", self.iteration_counter);
                for entry in std::fs::read_dir(&session_dir)? {
                    let entry = entry?;
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if filename.starts_with(&iteration_prefix) {
                        // Load this image as the new original
                        let image_path = entry.path();
                        {
                            let img = image::open(&image_path)?;
                            let rgb_image = img.to_rgb8();
                            self.original_image = Some(rgb_image);
                            return Ok(());
                        }
                    }
                }
            }
        }
        Err("No previous iteration found to load".into())
    }

    pub fn save_and_continue_iteration(&mut self, ctx: &egui::Context) {
        if let Some(ref processed) = self.processed_image.clone() {
            // Extract algorithm to avoid borrow conflict
            let algorithm = self.current_algorithm;
            // Save the current iteration using the existing auto-save system
            if let Ok(_saved_path) = self.auto_save_image(processed, &algorithm) {
                // Load the saved image as the new source for next iteration
                if let Ok(()) = self.load_last_iteration_as_source() {
                    // Process the loaded image immediately for preview; a sort of the
                    // previous source still in flight is obsolete
                    self.cancel_sort();
                    self.apply_pixel_sort(ctx);
                }
            }
        }
    }
}