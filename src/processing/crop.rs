use std::sync::Arc;
use crate::PixelSorterApp;
use crate::processing::image_ops::SortTarget;

impl PixelSorterApp {
    /// Crop the image to the crop rectangle and sort it in the background;
    /// the result becomes the new source image (see poll_sort_job)
    pub fn apply_crop_and_sort(&mut self, ctx: &egui::Context) {
        if matches!(&self.sort_job, Some(job) if job.target == SortTarget::Crop) {
            return; // Already applying a crop
        }
        let (Some(original), Some(crop_rect)) = (&self.original_image, self.crop_rect) else {
            return;
        };

        // The crop_rect is already in image pixel coordinates (set in render_crop_overlay)
        // No need for coordinate conversion - just clamp to image bounds
        let (image_width, image_height) = original.dimensions();
        let crop_min_x = crop_rect.min.x.clamp(0.0, image_width as f32) as u32;
        let crop_min_y = crop_rect.min.y.clamp(0.0, image_height as f32) as u32;
        let crop_max_x = crop_rect.max.x.clamp(0.0, image_width as f32) as u32;
        let crop_max_y = crop_rect.max.y.clamp(0.0, image_height as f32) as u32;

        // Ensure valid crop dimensions
        let crop_width = crop_max_x.saturating_sub(crop_min_x);
        let crop_height = crop_max_y.saturating_sub(crop_min_y);
        if crop_width == 0 || crop_height == 0 {
            return;
        }

        let cropped = image::imageops::crop_imm(original, crop_min_x, crop_min_y, crop_width, crop_height).to_image();

        // Apply pixel sorting to the cropped region; any preview sort of the
        // uncropped image is now obsolete
        let algorithm = self.current_algorithm;
        let params = self.sorting_params.clone();
        let pixel_sorter = Arc::clone(&self.pixel_sorter);
        self.cancel_sort();
        self.start_sort_job(ctx, SortTarget::Crop, move || {
            pixel_sorter.sort_pixels(&cropped, algorithm, &params)
        });
    }
}
