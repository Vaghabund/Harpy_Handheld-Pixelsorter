use crate::PixelSorterApp;
use eframe::egui;

impl PixelSorterApp {
    pub fn capture_and_sort(&mut self, ctx: &egui::Context) {
        if let Some(camera) = self.camera_controller.clone() {
            if let Ok(mut camera_lock) = camera.try_lock() {
                // Stop streaming to free camera for high-quality capture
                camera_lock.stop_streaming();
                
                match camera_lock.capture_snapshot() {
                    Ok(frame) => {
                        self.original_image = Some(frame.clone());
                        self.processed_image = Some(frame.clone());
                        self.create_processed_texture(ctx, frame);
                        self.current_phase = crate::ui::Phase::Edit; // Switch to edit phase
                    }
                    Err(e) => {
                        // Preview stream restarts automatically on the next frame
                        log::error!("Capture failed: {}", e);
                        self.export_message = Some(format!("✗ Capture failed: {}", e));
                        self.export_message_time = Some(std::time::Instant::now());
                    }
                }
            }
        }
    }
}