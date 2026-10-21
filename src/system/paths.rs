use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Environment variable that overrides the application directory
const APP_DIR_ENV: &str = "HARPY_APP_DIR";

/// Root directory of the app checkout (holds Cargo.toml, ups_config.toml,
/// sorted_images/ and deployment/). Resolved once:
/// 1. $HARPY_APP_DIR if set
/// 2. the nearest ancestor of the executable containing Cargo.toml
///    (e.g. <checkout>/target/release/pixelsort-pi -> <checkout>)
/// 3. the current working directory
pub fn app_dir() -> &'static Path {
    static APP_DIR: OnceLock<PathBuf> = OnceLock::new();
    APP_DIR.get_or_init(|| {
        let dir = resolve_app_dir();
        log::info!("Using app directory: {}", dir.display());
        dir
    })
}

fn resolve_app_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(APP_DIR_ENV) {
        return PathBuf::from(dir);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.ancestors().skip(1).find(|dir| dir.join("Cargo.toml").is_file()) {
            return dir.to_path_buf();
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Folder where sorted images are saved, one sub-folder per session
pub fn sorted_images_dir() -> PathBuf {
    app_dir().join("sorted_images")
}

/// Optional UPS configuration file
pub fn ups_config_path() -> PathBuf {
    app_dir().join("ups_config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_checkout_from_executable() {
        // The test binary lives in <checkout>/target/<profile>/deps/
        if std::env::var_os(APP_DIR_ENV).is_none() {
            assert!(resolve_app_dir().join("Cargo.toml").is_file());
        }
    }
}
