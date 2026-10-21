// UPS battery monitor for safe shutdown
// Monitors I2C-based UPS HAT for low battery warnings

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

// Shared flag to signal shutdown request
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

// Shared battery status
lazy_static::lazy_static! {
    static ref BATTERY_STATUS: Arc<Mutex<BatteryStatus>> = Arc::new(Mutex::new(BatteryStatus::default()));
}

/// Battery status information
#[derive(Clone, Debug)]
pub struct BatteryStatus {
    pub voltage: f32,
    pub percentage: f32,
    pub is_charging: bool,
    pub is_available: bool,
}

impl Default for BatteryStatus {
    fn default() -> Self {
        Self {
            voltage: 0.0,
            percentage: 0.0,
            is_charging: false,
            is_available: false,
        }
    }
}

/// Get current battery status
pub fn get_battery_status() -> BatteryStatus {
    BATTERY_STATUS.lock().unwrap().clone()
}

/// Check if shutdown has been requested by UPS
pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::Relaxed)
}

/// UPS Monitor configuration
#[derive(Clone)]
pub struct UpsConfig {
    pub enabled: bool,
    pub i2c_bus: u8,
    pub i2c_address: u8,
    pub voltage_threshold: f32,  // Minimum voltage before shutdown
    pub check_interval_secs: u64,
}

impl Default for UpsConfig {
    fn default() -> Self {
        Self {
            enabled: false,  // Disabled by default, enable in config
            i2c_bus: 1,      // Usually /dev/i2c-1 on Pi
            i2c_address: 0x36, // Common address for fuel gauge ICs
            voltage_threshold: 3.2, // 3.2V per cell (for 2S = 6.4V total)
            check_interval_secs: 5,
        }
    }
}

/// Start UPS monitoring in background
pub fn start_monitoring(config: UpsConfig) -> Arc<AtomicBool> {
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = shutdown_flag.clone();

    if !config.enabled {
        log::info!("UPS monitoring is disabled");
        return shutdown_flag;
    }

    tokio::spawn(async move {
        log::info!("UPS monitoring started (I2C bus {}, address 0x{:02X})", 
                   config.i2c_bus, config.i2c_address);
        
        // Do an immediate check first to populate battery status
        if let Ok((voltage, is_charging)) = check_battery_voltage(&config) {
            let percentage = voltage_to_percentage(voltage, config.voltage_threshold);
            if let Ok(mut status) = BATTERY_STATUS.lock() {
                status.voltage = voltage;
                status.percentage = percentage;
                status.is_charging = is_charging;
                status.is_available = true;
            }
            log::info!("Initial battery status: {:.2}V ({:.0}%) {}", 
                      voltage, percentage, 
                      if is_charging { "charging" } else { "discharging" });
        }
        
        loop {
            sleep(Duration::from_secs(config.check_interval_secs)).await;
            
            // Check battery status
            match check_battery_voltage(&config) {
                Ok((voltage, is_charging)) => {
                    // Calculate percentage (rough estimate based on voltage)
                    let percentage = voltage_to_percentage(voltage, config.voltage_threshold);
                    
                    // Update shared battery status
                    if let Ok(mut status) = BATTERY_STATUS.lock() {
                        status.voltage = voltage;
                        status.percentage = percentage;
                        status.is_charging = is_charging;
                        status.is_available = true;
                    }
                    
                    log::debug!("Battery: {:.2}V ({:.0}%) {}", 
                              voltage, percentage, 
                              if is_charging { "charging" } else { "discharging" });
                    
                    if voltage < config.voltage_threshold && !is_charging {
                        log::warn!("LOW BATTERY WARNING: {:.2}V (threshold: {:.2}V)", 
                                  voltage, config.voltage_threshold);
                        
                        // Give user 30 seconds warning
                        for i in (1..=6).rev() {
                            log::warn!("Shutting down in {} seconds...", i * 5);
                            sleep(Duration::from_secs(5)).await;
                        }
                        
                        log::error!("Battery critical! Initiating safe shutdown...");
                        SHUTDOWN_REQUESTED.store(true, Ordering::Relaxed);
                        flag_clone.store(true, Ordering::Relaxed);
                        
                        // Trigger system shutdown
                        if let Err(e) = initiate_system_shutdown() {
                            log::error!("Failed to initiate system shutdown: {}", e);
                        }
                        break;
                    }
                }
                Err(e) => {
                    // Don't spam errors, just log once per minute
                    if config.check_interval_secs <= 60 {
                        log::debug!("Could not read battery voltage: {}", e);
                    }
                }
            }
        }
    });

    shutdown_flag
}

/// Convert voltage to percentage (rough estimation)
fn voltage_to_percentage(voltage: f32, min_voltage: f32) -> f32 {
    // For 2S Li-ion (7.4V nominal):
    // 8.4V = 100%, 6.4V = 0%
    let max_voltage = 8.4;
    
    ((voltage - min_voltage) / (max_voltage - min_voltage) * 100.0).clamp(0.0, 100.0)
}

/// Read battery voltage from I2C UPS HAT
/// Returns (voltage, is_charging)
#[cfg(target_os = "linux")]
fn check_battery_voltage(config: &UpsConfig) -> Result<(f32, bool), String> {
    use std::fs::File;
    use std::io::Read;
    
    let mut is_charging = false;
    
    // Check if charging from sysfs
    let status_paths = [
        "/sys/class/power_supply/battery/status",
        "/sys/class/power_supply/BAT0/status",
    ];
    
    for path in &status_paths {
        if let Ok(mut file) = File::open(path) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                // Exact match: "Discharging" contains "charging" as a substring
                is_charging = contents.trim().eq_ignore_ascii_case("charging");
                break;
            }
        }
    }
    
    // Try to read from sysfs first (some UPS HATs expose this)
    let sysfs_paths = [
        "/sys/class/power_supply/battery/voltage_now",
        "/sys/class/power_supply/BAT0/voltage_now",
        "/sys/class/power_supply/ups/voltage_now",
    ];
    
    for path in &sysfs_paths {
        if let Ok(mut file) = File::open(path) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                if let Ok(microvolts) = contents.trim().parse::<u64>() {
                    let volts = microvolts as f32 / 1_000_000.0;
                    return Ok((volts, is_charging));
                }
            }
        }
    }
    
    // Read directly from I2C using i2cdev
    use i2cdev::core::I2CDevice;
    use i2cdev::linux::LinuxI2CDevice;
    
    let i2c_path = format!("/dev/i2c-{}", config.i2c_bus);
    
    if let Ok(mut dev) = LinuxI2CDevice::new(&i2c_path, config.i2c_address as u16) {
        // INA219-style voltage reading (common for 0x42 UPS HATs)
        // Register 0x02: Bus Voltage Register
        // Read 2 bytes from register 0x02
        if let Ok(buf) = dev.smbus_read_i2c_block_data(0x02, 2) {
            if buf.len() >= 2 {
                // Combine bytes (big-endian)
                let raw_value = ((buf[0] as u16) << 8) | (buf[1] as u16);
                
                // INA219: Voltage = (raw >> 3) * 4mV
                let voltage = ((raw_value >> 3) as f32) * 0.004;
                
                if voltage > 0.0 && voltage < 20.0 {  // Sanity check
                    // Estimate charging based on voltage
                    // 2S Li-ion: > 8.0V usually means charging or full
                    let estimated_charging = voltage > 8.0;
                    return Ok((voltage, is_charging || estimated_charging));
                }
            }
        }
    }
    
    Err("Battery voltage unavailable (sysfs and I2C failed)".to_string())
}

#[cfg(not(target_os = "linux"))]
fn check_battery_voltage(_config: &UpsConfig) -> Result<(f32, bool), String> {
    // On non-Linux (development machines), return safe voltage
    Ok((7.8, false)) // ~75% battery, not charging
}

/// Initiate safe system shutdown
#[cfg(target_os = "linux")]
fn initiate_system_shutdown() -> Result<(), String> {
    use std::process::Command;
    
    log::info!("Executing system shutdown command...");
    
    Command::new("sudo")
        .args(&["shutdown", "-h", "now"])
        .spawn()
        .map_err(|e| format!("Failed to execute shutdown: {}", e))?;
    
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn initiate_system_shutdown() -> Result<(), String> {
    log::info!("Shutdown requested (simulated on non-Linux)");
    Ok(())
}

/// Check if running on battery power (vs AC)
#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub fn is_on_battery() -> bool {
    use std::fs;
    
    // Check common power supply status files
    let status_paths = [
        "/sys/class/power_supply/AC/online",
        "/sys/class/power_supply/ADP0/online",
    ];
    
    for path in &status_paths {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(online) = contents.trim().parse::<u8>() {
                return online == 0; // 0 = on battery, 1 = AC connected
            }
        }
    }
    
    false // Assume on AC if unknown
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
pub fn is_on_battery() -> bool {
    false // Development machine
}
