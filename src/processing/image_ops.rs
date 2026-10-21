use crate::PixelSorterApp;
use crate::ui::background::BackgroundTask;
use eframe::egui;
use image::RgbImage;
use rayon::prelude::*;
use std::sync::Arc;

/// What the result of a running sort job is used for
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortTarget {
    /// Re-sort of the current image with new settings (Edit phase)
    Preview,
    /// Sorted crop that becomes the new source image
    Crop,
}

/// A pixel sort running on a background thread
pub struct SortJob {
    pub target: SortTarget,
    /// The result counts as an unsaved edit (false for the automatic re-sort after Iterate)
    pub marks_unsaved: bool,
    pub task: BackgroundTask<anyhow::Result<RgbImage>>,
}

impl PixelSorterApp {
    /// Re-sort the current image with the current settings in the background.
    /// If a sort is already running, one more sort with the latest settings
    /// runs as soon as it finishes (intermediate slider positions are skipped).
    pub fn apply_pixel_sort(&mut self, ctx: &egui::Context) {
        if let Some(job) = &self.sort_job {
            if job.target == SortTarget::Preview {
                self.sort_pending = true;
            }
            return;
        }

        let Some(original) = self.original_image.clone() else {
            return;
        };
        let algorithm = self.current_algorithm;
        let params = self.sorting_params.clone();
        let pixel_sorter = Arc::clone(&self.pixel_sorter);
        // Tint is applied AFTER pixel sorting (as a visual effect only)
        let tint_hue = (self.tint_enabled && params.color_tint > 0.0).then_some(params.color_tint);

        self.start_sort_job(ctx, SortTarget::Preview, move || {
            let mut sorted = pixel_sorter.sort_pixels(&original, algorithm, &params)?;
            if let Some(hue) = tint_hue {
                apply_tint_to_image(&mut sorted, hue);
            }
            Ok(sorted)
        });
    }

    pub fn start_sort_job(
        &mut self,
        ctx: &egui::Context,
        target: SortTarget,
        job: impl FnOnce() -> anyhow::Result<RgbImage> + Send + 'static,
    ) {
        self.is_processing = true;
        self.sort_job = Some(SortJob {
            target,
            marks_unsaved: true,
            task: BackgroundTask::spawn(ctx, "pixel-sort", job),
        });
    }

    /// Discard any running or queued sort (e.g. when the source image is replaced)
    pub fn cancel_sort(&mut self) {
        self.sort_job = None;
        self.sort_pending = false;
        self.is_processing = false;
    }

    /// Apply the result of a finished sort job, if any
    pub fn poll_sort_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.sort_job else {
            return;
        };
        let Some(outcome) = job.task.poll() else {
            return;
        };
        let target = job.target;
        let marks_unsaved = job.marks_unsaved;
        self.sort_job = None;
        self.is_processing = false;

        match outcome {
            Some(Ok(sorted)) => {
                if marks_unsaved {
                    self.has_unsaved_changes = true;
                }
                match target {
                    SortTarget::Preview => {
                        self.processed_image = Some(sorted.clone());
                        self.create_processed_texture(ctx, sorted);
                    }
                    SortTarget::Crop => {
                        // Make the sorted cropped region the new full image
                        self.original_image = Some(sorted.clone());
                        self.processed_image = Some(sorted.clone());
                        self.create_processed_texture(ctx, sorted);

                        // Exit crop and return to Edit phase
                        self.crop_rect = None;
                        self.current_phase = crate::ui::Phase::Edit;
                    }
                }
            }
            Some(Err(e)) => {
                log::error!("Pixel sort failed: {}", e);
                self.show_message(format!("✗ Sort failed: {}", e));
            }
            None => log::error!("Pixel sort thread ended without a result"),
        }

        if self.sort_pending {
            self.sort_pending = false;
            self.apply_pixel_sort(ctx);
        }
    }

    pub fn load_image(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image Files", &["png", "jpg", "jpeg", "bmp", "tiff"])
            .pick_file()
        {
            match image::open(&path) {
                Ok(img) => {
                    let rgb_image = img.to_rgb8();
                    self.cancel_sort();
                    self.has_unsaved_changes = true;
                    self.original_image = Some(rgb_image.clone());
                    self.processed_image = Some(rgb_image.clone());
                    self.create_processed_texture(ctx, rgb_image);
                    self.current_phase = crate::ui::Phase::Edit; // Switch to edit phase
                }
                Err(e) => {
                    log::error!("Failed to load {}: {}", path.display(), e);
                    self.show_message(format!("✗ Could not open image: {}", e));
                }
            }
        }
    }
}

fn blend_tint_preserve_luminance(original: &image::Rgb<u8>, tint: &image::Rgb<u8>, strength: f32) -> image::Rgb<u8> {
    let strength = strength.clamp(0.0, 1.0);
    
    let orig_r = original[0] as f32 / 255.0;
    let orig_g = original[1] as f32 / 255.0;
    let orig_b = original[2] as f32 / 255.0;
    
    // Calculate luminance to preserve brightness
    let luminance = 0.299 * orig_r + 0.587 * orig_g + 0.114 * orig_b;
    
    // For very dark or very bright pixels, reduce tint strength
    let adjusted_strength = if !(0.1..=0.9).contains(&luminance) {
        strength * 0.3  // Preserve blacks and whites more
    } else {
        strength
    };
    
    let tint_r = tint[0] as f32 / 255.0;
    let tint_g = tint[1] as f32 / 255.0;
    let tint_b = tint[2] as f32 / 255.0;
    
    // Blend with original
    let final_r = (orig_r * (1.0 - adjusted_strength) + orig_r * tint_r * adjusted_strength).clamp(0.0, 1.0);
    let final_g = (orig_g * (1.0 - adjusted_strength) + orig_g * tint_g * adjusted_strength).clamp(0.0, 1.0);
    let final_b = (orig_b * (1.0 - adjusted_strength) + orig_b * tint_b * adjusted_strength).clamp(0.0, 1.0);
    
    image::Rgb([
        (final_r * 255.0).round() as u8,
        (final_g * 255.0).round() as u8,
        (final_b * 255.0).round() as u8,
    ])
}

fn apply_tint_to_image(image: &mut RgbImage, tint_hue: f32) {
    let tint_color = crate::processing::pixel_sorter::hue_to_rgb_pixel(tint_hue);
    let strength = 0.2; // Strength for tinting

    let pixels: &mut [u8] = image;
    pixels.par_chunks_exact_mut(3).for_each(|px| {
        let tinted = blend_tint_preserve_luminance(&image::Rgb([px[0], px[1], px[2]]), &tint_color, strength);
        px.copy_from_slice(&tinted.0);
    });
}
