use std::sync::Arc;
use std::time::Instant;
use eframe::egui;
use std::sync::Mutex;

use crate::system::UpdateManager;
use crate::processing::{PixelSorter, SortingAlgorithm, SortingParameters};
use crate::hardware::CameraController;
use crate::processing::image_ops::SortJob;
use crate::session::manager::UsbExportResult;
use background::{take_finished, BackgroundTask};

// Module declarations
pub mod background;
mod files;
mod flight;
mod state;
mod thumbnails;
pub mod helpers;
pub mod controls;
pub mod theme;   // v2 look: fonts, colours, shared widgets
mod layouts;     // Phase-specific positioning
mod screens;
mod indicators;
mod viewport;
mod camera;
mod sheets;

// Re-export public types
pub use state::{Phase, DragState};

// ============================================================================
// MAIN APP STRUCT
// ============================================================================

pub struct PixelSorterApp {
    // Phase management
    pub current_phase: Phase,
    
    // Image data
    pub original_image: Option<image::RgbImage>,
    pub processed_image: Option<image::RgbImage>,
    pub camera_texture: Option<egui::TextureHandle>,
    pub processed_texture: Option<egui::TextureHandle>,
    
    // Processing
    pub pixel_sorter: Arc<PixelSorter>,
    pub current_algorithm: SortingAlgorithm,
    pub sorting_params: SortingParameters,
    pub is_processing: bool,
    pub sort_job: Option<SortJob>,
    /// Settings changed while a sort was running: sort again when it finishes
    pub sort_pending: bool,
    
    // Camera
    pub camera_controller: Option<Arc<Mutex<CameraController>>>,
    pub last_camera_update: Option<Instant>,
    pub capture_task: Option<BackgroundTask<anyhow::Result<image::RgbImage>>>,
    
    // Crop state
    pub crop_rect: Option<egui::Rect>, // In image coordinates
    pub drag_state: DragState,
    
    // Session management
    pub iteration_counter: u32,
    pub current_session_folder: Option<String>,
    /// The current image has edits that haven't been saved (Iterate) yet
    pub has_unsaved_changes: bool,
    pub show_new_session_dialog: bool,
    
    // Export status
    pub export_message: Option<String>,
    pub export_message_time: Option<Instant>,
    
    // Harpy logo (sleep and wake screens)
    pub logo: Option<egui::TextureHandle>,

    // Splash screen
    pub show_splash: bool,
    pub splash_start_time: Option<Instant>,
    
    // Exit mechanism for kiosk mode
    pub exit_tap_count: u32,
    pub exit_tap_last_time: Option<Instant>,
    
    // Sleep mode (power saving after idle)
    pub is_sleeping: bool,
    pub is_waking: bool,
    pub wake_start_time: Option<Instant>,
    pub last_interaction_time: Instant,
    
    // Update checking
    pub update_manager: UpdateManager,
    pub update_check_time: Option<Instant>,
    pub startup_check_done: bool,
    pub update_check_task: Option<BackgroundTask<anyhow::Result<bool>>>,
    /// Show the result of the running update check as a toast
    pub update_check_manual: bool,
    
    // Shutdown menu
    pub show_shutdown_menu: bool,
    // Track previous USB presence to detect new mounts
    pub prev_usb_present: bool,
    pub last_usb_check: Option<Instant>,
    pub usb_check_task: Option<BackgroundTask<bool>>,

    // USB export dialog
    pub show_usb_export_dialog: bool,
    pub usb_export_delete_after: bool,
    /// Sessions offered in the export dialog, with their selection
    pub usb_export_choices: Vec<(crate::session::library::SessionFolder, bool)>,
    pub usb_export_task: Option<BackgroundTask<UsbExportResult>>,
    
    // Developer menu
    pub show_developer_menu: bool,
    
    // In-app gallery and USB browser
    pub files: files::FileManager,

    // v2 camera/edit controls: edge panels and drag dial
    pub controls: controls::ControlsState,

    // Other
    pub tint_enabled: bool,
}

// ============================================================================
// INITIALIZATION
// ============================================================================

impl PixelSorterApp {
    pub fn new(
        pixel_sorter: Arc<PixelSorter>,
        camera_controller: Option<Arc<Mutex<CameraController>>>,
    ) -> Self {
        // Start camera streaming if available
        if let Some(ref camera) = camera_controller {
            if let Ok(mut camera_lock) = camera.try_lock() {
                let _ = camera_lock.start_streaming();
            }
        }

        Self {
            current_phase: Phase::Input,
            original_image: None,
            processed_image: None,
            camera_texture: None,
            processed_texture: None,
            pixel_sorter,
            current_algorithm: SortingAlgorithm::Horizontal,
            sorting_params: SortingParameters::default(),
            is_processing: false,
            sort_job: None,
            sort_pending: false,
            camera_controller,
            last_camera_update: None,
            capture_task: None,
            crop_rect: None,
            drag_state: DragState::None,
            iteration_counter: 0,
            current_session_folder: None,
            has_unsaved_changes: false,
            show_new_session_dialog: false,
            export_message: None,
            export_message_time: None,
            logo: None,
            show_splash: true,
            splash_start_time: Some(Instant::now()),
            exit_tap_count: 0,
            exit_tap_last_time: None,
            is_sleeping: false,
            is_waking: false,
            wake_start_time: None,
            last_interaction_time: Instant::now(),
            update_manager: UpdateManager::new(crate::system::paths::app_dir().to_string_lossy().into_owned()),
            update_check_time: None,
            startup_check_done: false,
            update_check_task: None,
            update_check_manual: false,
            show_shutdown_menu: false,
            prev_usb_present: false,
            last_usb_check: None,
            usb_check_task: None,
            show_usb_export_dialog: false,
            usb_export_delete_after: false,
            usb_export_choices: Vec::new(),
            usb_export_task: None,
            show_developer_menu: false,
            tint_enabled: false,
            controls: controls::ControlsState::default(),
            files: files::FileManager::default(),
        }
    }
}

// ============================================================================
// MAIN UPDATE LOOP
// ============================================================================

impl eframe::App for PixelSorterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if UPS requested shutdown
        if crate::hardware::is_shutdown_requested() {
            log::warn!("UPS shutdown requested - closing application");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        
        // Collect results of background work (sorting, capture, update check, USB)
        self.poll_background_tasks(ctx);

        // Hide cursor in kiosk mode - force it every frame
        ctx.set_cursor_icon(egui::CursorIcon::None);
        ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::None);
        
        // ESC key to open developer menu (for debugging with keyboard)
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_developer_menu = !self.show_developer_menu;
        }
        
        // Hidden developer menu trigger (bottom-right corner, tap 5 times within 3 seconds)
        let screen_size = ctx.screen_rect().size();
        egui::Area::new("dev_menu_trigger")
            .fixed_pos(egui::pos2(screen_size.x - 50.0, screen_size.y - 50.0))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                let trigger_size = egui::vec2(50.0, 50.0);
                let (_rect, response) = ui.allocate_exact_size(trigger_size, egui::Sense::click());
                
                if response.clicked() {
                    let now = Instant::now();
                    
                    if let Some(last_time) = self.exit_tap_last_time {
                        if now.duration_since(last_time).as_secs() > 3 {
                            self.exit_tap_count = 0;
                        }
                    }
                    
                    self.exit_tap_count += 1;
                    self.exit_tap_last_time = Some(now);
                    
                    if self.exit_tap_count >= 5 {
                        self.show_developer_menu = true;
                        self.exit_tap_count = 0;
                    }
                }
            });
        
        // Show the splash animation for one wingbeat
        if self.show_splash {
            if let Some(start_time) = self.splash_start_time {
                let elapsed = start_time.elapsed().as_secs_f32();
                if elapsed > Self::splash_duration() {
                    self.show_splash = false;
                } else {
                    self.render_splash_screen(ctx, elapsed);
                    // The animation only changes once per frame
                    ctx.request_repaint_after(flight::FlightAnimation::frame_interval());
                    return;
                }
            }
        }
        
        // Background update check
        if !self.startup_check_done {
            if self.update_check_time.is_none() {
                self.update_check_time = Some(Instant::now());
            } else if let Some(start_time) = self.update_check_time {
                if start_time.elapsed().as_secs() >= 30 {
                    self.start_update_check(ctx, false);
                    self.startup_check_done = true;
                }
            }
        }
        
        // Check for user interaction
        let has_interaction = ctx.input(|i| {
            i.pointer.any_pressed() || 
            i.pointer.any_down() || 
            i.pointer.is_moving() ||
            !i.events.is_empty() ||
            i.key_pressed(egui::Key::Space) ||
            i.key_pressed(egui::Key::Escape) ||
            i.key_pressed(egui::Key::Enter)
        });
        
        if has_interaction {
            if self.is_sleeping {
                self.is_sleeping = false;
                self.is_waking = true;
                self.wake_start_time = Some(Instant::now());
                log::info!("Waking from sleep mode...");
            }
            self.last_interaction_time = Instant::now();
        }
        
        // Wake-up animation
        if self.is_waking {
            if let Some(wake_start) = self.wake_start_time {
                if wake_start.elapsed().as_secs() >= 1 {
                    self.is_waking = false;
                    self.wake_start_time = None;
                    log::info!("Wake-up complete");
                }
            }
        }
        
        // Sleep mode check (5 minutes)
        let idle_duration = self.last_interaction_time.elapsed().as_secs();
        if !self.is_sleeping && idle_duration >= 300 {
            self.is_sleeping = true;
        }
        
        // If sleeping, show sleep screen
        if self.is_sleeping {
            self.render_sleep_screen(ctx);
            ctx.request_repaint();
            return;
        }
        
        // If waking, show wake-up screen
        if self.is_waking {
            self.render_waking_screen(ctx);
            ctx.request_repaint();
            return;
        }
        
        // Update camera preview at 30 FPS if in Input phase
        if self.current_phase == Phase::Input && !self.is_processing {
            self.update_camera_preview(ctx);
            ctx.request_repaint();
        }

        // Render UI
        self.render_ui(ctx);
        
        // Render overlays AFTER CentralPanel so they appear on top
        let full_rect = ctx.screen_rect();
        self.render_sheets(ctx);
        self.render_export_message(ctx, full_rect);
    }
}

impl PixelSorterApp {
    fn poll_background_tasks(&mut self, ctx: &egui::Context) {
        self.poll_sort_job(ctx);
        self.poll_capture_task(ctx);
        self.poll_update_check();
        self.poll_usb_presence(ctx);
        self.poll_usb_export();
        self.poll_file_tasks(ctx);
    }

    /// Check git for updates in the background; `manual` shows the result as a toast
    pub fn start_update_check(&mut self, ctx: &egui::Context, manual: bool) {
        if self.update_check_task.is_some() {
            return;
        }
        let project_path = self.update_manager.project_path().to_string();
        self.update_check_manual = manual;
        self.update_check_task = Some(BackgroundTask::spawn(ctx, "update-check", move || {
            UpdateManager::check_for_updates(&project_path)
        }));
    }

    fn poll_update_check(&mut self) {
        let Some(outcome) = take_finished(&mut self.update_check_task) else {
            return;
        };
        self.update_check_time = Some(Instant::now());
        match outcome {
            Some(Ok(update_found)) => {
                self.update_manager.update_available = update_found;
                if self.update_check_manual {
                    if update_found {
                        self.show_message("🆕 Update found! Restart to apply.");
                    } else {
                        self.show_message("✅ Already up to date");
                    }
                }
            }
            Some(Err(e)) => {
                log::error!("Update check failed: {}", e);
                if self.update_check_manual {
                    self.show_message(format!("❌ Update check failed: {}", e));
                }
            }
            None => log::error!("Update check thread ended without a result"),
        }
    }

    /// Detect newly mounted USB drives (probed every 2 s on a background thread,
    /// since the probe writes a test file to the drive)
    fn poll_usb_presence(&mut self, ctx: &egui::Context) {
        if let Some(outcome) = take_finished(&mut self.usb_check_task) {
            let usb_now = outcome.unwrap_or(false);
            if usb_now && !self.prev_usb_present && !self.show_usb_export_dialog {
                log::info!("USB mounted - opening export dialog");
                self.open_usb_export_dialog();
            }
            self.prev_usb_present = usb_now;
        }

        let now = Instant::now();
        let should_check_usb = match self.last_usb_check {
            None => true,
            Some(last) => now.duration_since(last) >= std::time::Duration::from_secs(2),
        };
        if should_check_usb && self.usb_check_task.is_none() {
            self.last_usb_check = Some(now);
            self.usb_check_task = Some(BackgroundTask::spawn(ctx, "usb-check", || {
                crate::system::usb::find_writable_usb_mount().is_some()
            }));
        }
    }

    fn update_camera_preview(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let should_update = match self.last_camera_update {
            None => true,
            Some(last) => now.duration_since(last) >= std::time::Duration::from_millis(33),
        };

        if should_update {
            if let Some(camera) = self.camera_controller.clone() {
                if let Ok(mut camera_lock) = camera.try_lock() {
                    if let Ok(Some(preview_image)) = camera_lock.get_fast_preview_image() {
                        self.update_camera_texture(ctx, &preview_image);
                        self.last_camera_update = Some(now);
                    }
                }
            }
        }
    }

    fn render_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let full_rect = ui.max_rect();

                self.render_viewport(ui, full_rect, ctx);
                self.render_button_overlay(ui, ctx, full_rect);
            });
    }
}
