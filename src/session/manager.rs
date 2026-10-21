use crate::PixelSorterApp;
use eframe::egui;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use crate::processing::SortingAlgorithm;

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
        let session_dir = PathBuf::from("sorted_images").join(self.current_session_folder.as_ref().unwrap());
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

    pub fn copy_to_usb(&self) -> Result<(), Box<dyn std::error::Error>> {
        let usb_path = crate::system::usb::find_writable_usb_mount()
            .ok_or("No writable USB drive found")?;

        // Copy sorted_images folder to USB
        let dest_path = usb_path.join("pixelsort_export");
        Self::copy_directory(PathBuf::from("sorted_images"), dest_path.clone())
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
                let session_dir = PathBuf::from("sorted_images").join(session_folder);
                
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
                    // Process the loaded image immediately for preview
                    self.apply_pixel_sort(ctx);
                }
            }
        }
    }
}