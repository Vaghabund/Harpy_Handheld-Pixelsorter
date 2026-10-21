/// Phase-specific layout and positioning logic
/// 
/// EDIT THIS FILE TO CHANGE: Positions, padding, spacing, alignment
/// (Colors/appearance are in styles.rs)
use crate::PixelSorterApp;
use crate::ui::state::Phase;
use crate::ui::components::{circular_button, circular_button_default, circular_button_light, vertical_slider, slider_knob_radius};
use crate::ui::styles::{ButtonSizes, SliderSizes, button_dark, button_green, button_red, button_fill_normal};
use eframe::egui;

// ============================================================================
// 📐 QUICK EDIT: LAYOUT PARAMETERS
// Change these values to adjust spacing and positioning
// ============================================================================

// Edit Phase - Button row positioning
const EDIT_ROW1_OFFSET: f32 = 4.0;  // Row 1 distance from bottom (in button heights + spacing)
const EDIT_ROW2_OFFSET: f32 = 2.0;  // Row 2 distance from bottom (in button heights + spacing)

// Edit Phase - Button horizontal positioning
const EDIT_BUTTON_COLUMNS: f32 = 3.0;  // Number of button columns (for New button position)

// Edit Phase - Slider positioning
const SLIDER_TOP_PADDING_MULTIPLIER: f32 = 3.0;    // Top padding (spacing * this value + knob radius)
const SLIDER_BOTTOM_PADDING_MULTIPLIER: f32 = 5.0; // Bottom padding (spacing * this value)
const SLIDER_SPACING_BETWEEN: f32 = 40.0;          // Horizontal space between Threshold and Hue sliders

// Slider value ranges
const THRESHOLD_MIN: f32 = 0.0;    // Minimum threshold value
const THRESHOLD_MAX: f32 = 125.0;  // Maximum threshold value
const HUE_MIN: f32 = 0.0;          // Minimum hue value (degrees)
const HUE_MAX: f32 = 360.0;        // Maximum hue value (degrees)

// USB Export button
const USB_BUTTON_SCALE: f32 = 0.7;           // USB button size relative to normal buttons (0.7 = 70%)
const USB_BUTTON_Y_OFFSET_DIVISOR: f32 = 2.0; // Y position offset (spacing divided by this value)

// Crop Phase - Button positioning
const CROP_BUTTON_VERTICAL_SPACING_MULTIPLIER: f32 = 2.0; // Spacing between Cancel/Apply buttons
const CROP_BUTTON_TOTAL_HEIGHT_MULTIPLIER: f32 = 4.0;     // Total height calculation (radius * this value)
const CROP_APPLY_BUTTON_Y_MULTIPLIER: f32 = 3.0;          // Apply button Y position (radius * this value)

impl PixelSorterApp {
    /// Main entry point for rendering phase-specific button overlays
    pub fn render_button_overlay(&mut self, _ui: &mut egui::Ui, ctx: &egui::Context, screen_rect: egui::Rect) {
        match self.current_phase {
            Phase::Input => self.render_input_layout(ctx, screen_rect),
            Phase::Edit => self.render_edit_layout(ctx, screen_rect),
            Phase::Crop => self.render_crop_layout(ctx, screen_rect),
            // File screens draw their own buttons
            Phase::Gallery | Phase::Browse => {}
        }
    }

    // ============================================================================
    // PHASE 1: INPUT LAYOUT - Two circles in right bottom corner
    // ============================================================================
    fn render_input_layout(&mut self, ctx: &egui::Context, screen_rect: egui::Rect) {
        let sizes = ButtonSizes::standard();

        // Calculate positions - right bottom corner alignment
        let large_center = egui::pos2(
            screen_rect.max.x - sizes.large_radius - sizes.spacing,
            screen_rect.max.y - sizes.large_radius - sizes.spacing,
        );

        let small_center = egui::pos2(
            screen_rect.max.x - sizes.small_radius - sizes.spacing,
            large_center.y - sizes.large_radius - sizes.small_radius - sizes.spacing,
        );

        // Take Picture button (large primary action)
        egui::Area::new("take_picture_btn")
            .fixed_pos(large_center - egui::vec2(sizes.large_radius, sizes.large_radius))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button_default(ui, sizes.large_radius, "") {
                    self.capture_and_sort(ctx);
                }
            });

        // Upload Image button (small secondary action)
        egui::Area::new("upload_btn")
            .fixed_pos(small_center - egui::vec2(sizes.small_radius, sizes.small_radius))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button_default(ui, sizes.small_radius, "USB") {
                    self.open_usb_browser();
                }
            });

        // Gallery button (small, left of the Take Picture button)
        let gallery_center = egui::pos2(
            large_center.x - sizes.large_radius - sizes.spacing - sizes.small_radius,
            screen_rect.max.y - sizes.small_radius - sizes.spacing,
        );
        egui::Area::new("gallery_btn")
            .fixed_pos(gallery_center - egui::vec2(sizes.small_radius, sizes.small_radius))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button_default(ui, sizes.small_radius, "Gallery") {
                    self.open_gallery();
                }
            });
    }

    // ============================================================================
    // PHASE 2: EDIT LAYOUT - Sliders on right, buttons on left in two rows
    // ============================================================================
    fn render_edit_layout(&mut self, ctx: &egui::Context, screen_rect: egui::Rect) {
        let btn_sizes = ButtonSizes::standard();
        let slider_sizes = SliderSizes::standard();

        // Right side: Vertical sliders
        self.render_sliders(ctx, screen_rect, &slider_sizes, btn_sizes.spacing);

        // Left side: Buttons in two rows (using offset constants)
        
        // Row 1: Algorithm and Sort Mode (top row)
        let row1_y = screen_rect.max.y - btn_sizes.normal_radius * EDIT_ROW1_OFFSET 
            - btn_sizes.spacing * (EDIT_ROW1_OFFSET - 1.0);

        // Algorithm button
        egui::Area::new("algo_btn")
            .fixed_pos(egui::pos2(btn_sizes.spacing, row1_y))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button_light(ui, btn_sizes.normal_radius, self.current_algorithm.name(), button_fill_normal()) {
                    self.current_algorithm = self.current_algorithm.next();
                    self.apply_pixel_sort(ctx);
                }
            });

        // Sort Mode button
        egui::Area::new("mode_btn")
            .fixed_pos(egui::pos2(
                btn_sizes.spacing + btn_sizes.normal_radius * 2.0 + btn_sizes.spacing, 
                row1_y
            ))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button_light(ui, btn_sizes.normal_radius, self.sorting_params.sort_mode.name(), button_fill_normal()) {
                    self.sorting_params.sort_mode = self.sorting_params.sort_mode.next();
                    self.apply_pixel_sort(ctx);
                }
            });

        // Row 2: Action buttons (bottom row) - Crop, Iterate, New
        let row2_y = screen_rect.max.y - btn_sizes.normal_radius * EDIT_ROW2_OFFSET 
            - btn_sizes.spacing * (EDIT_ROW2_OFFSET - 1.0);

        // Crop button
        egui::Area::new("crop_btn")
            .fixed_pos(egui::pos2(btn_sizes.spacing, row2_y))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button(ui, btn_sizes.normal_radius, "Crop", button_dark()) {
                    self.current_phase = Phase::Crop;
                    self.crop_rect = None;
                }
            });

        // Iterate button
        egui::Area::new("iterate_btn")
            .fixed_pos(egui::pos2(
                btn_sizes.spacing + btn_sizes.normal_radius * 2.0 + btn_sizes.spacing, 
                row2_y
            ))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button(ui, btn_sizes.normal_radius, "Iterate", button_dark()) {
                    self.save_and_continue_iteration(ctx);
                }
            });

        // New Image button
        egui::Area::new("new_btn")
            .fixed_pos(egui::pos2(
                btn_sizes.spacing + (btn_sizes.normal_radius * 2.0 + btn_sizes.spacing) * (EDIT_BUTTON_COLUMNS - 1.0), 
                row2_y
            ))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button(ui, btn_sizes.normal_radius, "New", button_dark()) {
                    self.request_new_photo_session();
                }
            });

        // Optional: USB export button if USB present (uses the throttled check
        // from render_ui; probing here would spawn `mount` and write to the stick every frame)
        if self.prev_usb_present {
            let export_y = screen_rect.max.y - btn_sizes.normal_radius - btn_sizes.spacing / USB_BUTTON_Y_OFFSET_DIVISOR;
            egui::Area::new("export_btn")
                .fixed_pos(egui::pos2(btn_sizes.spacing, export_y))
                .order(egui::Order::Background)
                .show(ctx, |ui| {
                    if circular_button(ui, btn_sizes.normal_radius * USB_BUTTON_SCALE, "USB", button_green()) {
                        self.open_usb_export_dialog();
                    }
                });
        }
    }

    // ============================================================================
    // PHASE 3: CROP LAYOUT - Cancel/Apply buttons on left
    // ============================================================================
    fn render_crop_layout(&mut self, ctx: &egui::Context, screen_rect: egui::Rect) {
        let sizes = ButtonSizes::standard();

        let left_x = sizes.spacing + sizes.normal_radius;
        let button_vertical_spacing = sizes.spacing * CROP_BUTTON_VERTICAL_SPACING_MULTIPLIER;

        // Center buttons vertically
        let total_height = sizes.normal_radius * CROP_BUTTON_TOTAL_HEIGHT_MULTIPLIER + button_vertical_spacing;
        let start_y = (screen_rect.height() - total_height) / 2.0 + screen_rect.min.y;

        // Cancel button (top)
        egui::Area::new("cancel_crop_btn")
            .fixed_pos(egui::pos2(left_x, start_y + sizes.normal_radius) 
                - egui::vec2(sizes.normal_radius, sizes.normal_radius))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button(ui, sizes.normal_radius, "Cancel", button_red()) {
                    // Discard a crop that is still being applied
                    if matches!(&self.sort_job, Some(job) if job.target == crate::processing::image_ops::SortTarget::Crop) {
                        self.cancel_sort();
                    }
                    self.current_phase = Phase::Edit;
                    self.crop_rect = None;
                }
            });

        // Apply Crop button (bottom)
        egui::Area::new("apply_crop_btn")
            .fixed_pos(egui::pos2(
                left_x,
                start_y + sizes.normal_radius * CROP_APPLY_BUTTON_Y_MULTIPLIER + button_vertical_spacing
            ) - egui::vec2(sizes.normal_radius, sizes.normal_radius))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                if circular_button(ui, sizes.normal_radius, "Apply", button_green()) {
                    self.apply_crop_and_sort(ctx);
                }
            });
    }

    // ============================================================================
    // SLIDER RENDERING (used in Edit and Crop phases)
    // ============================================================================
    fn render_sliders(
        &mut self, 
        ctx: &egui::Context, 
        screen_rect: egui::Rect,
        slider_sizes: &SliderSizes,
        spacing: f32
    ) {
        // Calculate padding to prevent handle cutoff (using constants)
        let knob_radius = slider_knob_radius(slider_sizes.width);
        let top_padding = spacing * SLIDER_TOP_PADDING_MULTIPLIER + knob_radius;
        let bottom_padding = spacing * SLIDER_BOTTOM_PADDING_MULTIPLIER;

        let full_slider_height = screen_rect.height() - top_padding - bottom_padding;

        // Position sliders side by side on right edge
        let slider2_x = screen_rect.max.x - slider_sizes.width - spacing;
        let slider1_x = slider2_x - slider_sizes.width - SLIDER_SPACING_BETWEEN;
        let start_y = screen_rect.min.y + top_padding;

        // Threshold slider (left)
        let mut threshold = self.sorting_params.threshold;
        let threshold_changed = egui::Area::new("threshold_slider")
            .fixed_pos(egui::pos2(slider1_x, start_y))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    vertical_slider(ui, &mut threshold, THRESHOLD_MIN..=THRESHOLD_MAX, 
                        slider_sizes.width, full_slider_height, "Threshold")
                }).inner
            }).inner;

        if threshold_changed {
            self.sorting_params.threshold = threshold;
            self.apply_pixel_sort(ctx);
        }

        // Hue slider (right)
        let mut color_tint = self.sorting_params.color_tint;
        let hue_changed = egui::Area::new("hue_slider")
            .fixed_pos(egui::pos2(slider2_x, start_y))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    vertical_slider(ui, &mut color_tint, HUE_MIN..=HUE_MAX, 
                        slider_sizes.width, full_slider_height, "Hue")
                }).inner
            }).inner;

        if hue_changed {
            if !self.tint_enabled && color_tint > 0.0 {
                self.tint_enabled = true;
            }
            self.sorting_params.color_tint = color_tint;
            self.apply_pixel_sort(ctx);
        }
    }
}
