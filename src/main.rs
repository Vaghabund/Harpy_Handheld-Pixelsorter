use anyhow::Result;
use eframe::egui;
use log::info;
use std::sync::Arc;
use tokio::sync::RwLock;

// Domain modules
mod hardware;
mod processing;
mod system;
mod session;
mod ui;

use crate::processing::PixelSorter;
use crate::ui::PixelSorterApp;
use crate::hardware::{CameraController, UpsConfig};

#[tokio::main]
#[allow(clippy::arc_with_non_send_sync)]
async fn main() -> Result<()> {
    // Set up logging (Info level for production)
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    
    info!("Starting Raspberry Pi Pixel Sorter (Rust Edition)");

    // Load UPS configuration
    let ups_config = load_ups_config();
    
    // Start UPS monitoring (runs in background)
    hardware::start_monitoring(ups_config);
    
    // Initialize components
    let pixel_sorter = Arc::new(PixelSorter::new());

    // Initialize Camera controller  
    let camera_controller = match CameraController::new() {
        Ok(controller) => {
            Some(Arc::new(RwLock::new(controller)))
        }
        Err(e) => {
            log::error!("Camera initialization failed: {}. Camera features disabled.", e);
            None
        }
    };

    // Load application icon
    let icon_data = load_icon();
    
    // KIOSK MODE: Fullscreen borderless window (press ESC to exit for debugging)
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1920.0, 1080.0])    // Force full resolution
            .with_fullscreen(true)                // Start in fullscreen
            .with_maximized(true)                 // Maximize if fullscreen fails
            .with_decorations(false)              // No title bar or borders
            .with_resizable(false)                // Cannot be resized
            .with_icon(icon_data),                // Set window icon
        ..Default::default()
    };

    info!("Launching GUI application...");

    // Run the application
    eframe::run_native(
        "Raspberry Pi Pixel Sorter",
        options,
        Box::new(|cc| {
            // Setup egui style for touch interface
            setup_touch_style(&cc.egui_ctx);
            
            Box::new(PixelSorterApp::new(
                pixel_sorter,
                camera_controller,
            ))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run application: {}", e))?;

    info!("Application shut down gracefully");
    Ok(())
}

fn setup_touch_style(ctx: &egui::Context) {
    // Force 1.0 zoom to use full resolution (disable DPI scaling)
    ctx.set_zoom_factor(1.0);
    
    let mut style = (*ctx.style()).clone();
    
    // Kiosk-style UI with minimal margins
    style.spacing.button_padding = egui::vec2(16.0, 12.0);
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(4.0);  // Minimal margins
    style.spacing.menu_margin = egui::Margin::same(4.0);
    
    // Larger text for better readability
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(18.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(24.0, egui::FontFamily::Proportional),
    );
    
    // Touch-friendly slider and other controls
    style.spacing.slider_width = 300.0;
    style.spacing.combo_width = 200.0;
    
    ctx.set_style(style);
}

fn load_icon() -> egui::IconData {
    // Try to load icon from file, fallback to embedded default
    let icon_path = "assets/Harpy_ICON.png";
    
    match image::open(icon_path) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            }
        }
        Err(_) => {
            // Fallback: Create a simple colored square as default icon
            log::warn!("Could not load icon from {}, using default", icon_path);
            let width = 64;
            let height = 64;
            let mut rgba = vec![0u8; (width * height * 4) as usize];
            
            // Create a simple gradient icon
            for y in 0..height {
                for x in 0..width {
                    let idx = ((y * width + x) * 4) as usize;
                    rgba[idx] = (x * 255 / width) as u8;      // R
                    rgba[idx + 1] = (y * 255 / height) as u8; // G
                    rgba[idx + 2] = 150;                       // B
                    rgba[idx + 3] = 255;                       // A
                }
            }
            
            egui::IconData {
                rgba,
                width,
                height,
            }
        }
    }
}

fn load_ups_config() -> UpsConfig {
    use std::fs;
    
    let config_path = "ups_config.toml";
    
    // Try to load from file
    if let Ok(contents) = fs::read_to_string(config_path) {
        // Simple TOML parsing for our needs
        let mut config = UpsConfig::default();
        
        for line in contents.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() || line.starts_with('[') {
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
                
                match key {
                    "enabled" => config.enabled = value == "true",
                    "i2c_bus" => config.i2c_bus = value.parse().unwrap_or(1),
                    "i2c_address" => {
                        // Handle hex format: 0x36
                        if value.starts_with("0x") || value.starts_with("0X") {
                            config.i2c_address = u8::from_str_radix(&value[2..], 16).unwrap_or(0x36);
                        } else {
                            config.i2c_address = value.parse().unwrap_or(0x36);
                        }
                    }
                    "voltage_threshold" => config.voltage_threshold = value.parse().unwrap_or(6.4),
                    "check_interval_secs" => config.check_interval_secs = value.parse().unwrap_or(10),
                    _ => {}
                }
            }
        }
        
        info!("UPS configuration loaded from {}", config_path);
        config
    } else {
        info!("No UPS config found at {}, using defaults (disabled)", config_path);
        UpsConfig::default()
    }
}