use crate::PixelSorterApp;
use crate::ui::background::{take_finished, BackgroundTask};
use eframe::egui;

impl PixelSorterApp {
    /// Take a high-quality snapshot in the background (it takes over a second);
    /// the live preview freezes on its last frame meanwhile
    pub fn capture_and_sort(&mut self, ctx: &egui::Context) {
        if self.capture_task.is_some() {
            return; // Already capturing
        }
        let Some(camera) = self.camera_controller.clone() else {
            return;
        };

        self.capture_task = Some(BackgroundTask::spawn(ctx, "camera-capture", move || {
            let mut camera = camera.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            // Stop streaming to free camera for high-quality capture
            camera.stop_streaming();
            camera.capture_snapshot()
        }));
    }

    /// Switch to Edit phase with the captured photo once the capture finished
    pub fn poll_capture_task(&mut self, ctx: &egui::Context) {
        match take_finished(&mut self.capture_task) {
            Some(Some(Ok(frame))) => {
                self.cancel_sort();
                self.has_unsaved_changes = true;
                self.original_image = Some(frame.clone());
                self.processed_image = Some(frame.clone());
                self.create_processed_texture(ctx, frame);
                self.current_phase = crate::ui::Phase::Edit; // Switch to edit phase
            }
            Some(Some(Err(e))) => {
                // Preview stream restarts automatically on the next frame
                log::error!("Capture failed: {}", e);
                self.show_message(format!("❌ Capture failed: {}", e));
            }
            Some(None) => log::error!("Capture thread ended without a result"),
            None => {}
        }
    }
}
