//! File operations for the in-app gallery, USB browser and USB export

use std::path::{Path, PathBuf};

/// File extensions the app can open
const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "tiff"];

/// A session folder of saved images
#[derive(Debug, Clone, PartialEq)]
pub struct SessionFolder {
    pub name: String,
    pub path: PathBuf,
    /// Images in save order (oldest first)
    pub images: Vec<PathBuf>,
}

/// An entry in a browsed directory
#[derive(Debug, Clone, PartialEq)]
pub struct DirEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// List session folders (newest first) with their images; empty sessions are skipped
pub fn list_sessions(root: &Path) -> Vec<SessionFolder> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut sessions: Vec<SessionFolder> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_hidden(&name) {
                return None;
            }
            let path = entry.path();
            let mut images: Vec<PathBuf> = std::fs::read_dir(&path)
                .ok()?
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && is_image(p))
                .collect();
            // edit_001_..., edit_002_... sort in save order
            images.sort();
            (!images.is_empty()).then_some(SessionFolder { name, path, images })
        })
        .collect();
    // session_YYYYMMDD_HHMMSS names sort chronologically; newest first
    sessions.sort_by(|a, b| b.name.cmp(&a.name));
    sessions
}

/// List sub-folders and images of a directory: folders first, then images, each by name
pub fn list_dir(dir: &Path) -> std::io::Result<Vec<DirEntry>> {
    let mut entries: Vec<DirEntry> = std::fs::read_dir(dir)?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let is_dir = path.is_dir();
            (!is_hidden(&name) && (is_dir || is_image(&path))).then_some(DirEntry { path, name, is_dir })
        })
        .collect();
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(entries)
}

/// Delete one saved image; removes its session folder if it is now empty
pub fn delete_image(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path)?;
    if let Some(parent) = path.parent() {
        // Only succeeds if the folder is empty
        let _ = std::fs::remove_dir(parent);
    }
    Ok(())
}

pub fn delete_session(path: &Path) -> std::io::Result<()> {
    std::fs::remove_dir_all(path)
}

/// Copy the given session folders into `dest` (one sub-folder per session).
/// Returns the number of files copied.
pub fn copy_sessions(sessions: &[PathBuf], dest: &Path) -> std::io::Result<usize> {
    let mut copied = 0;
    for session in sessions {
        let Some(name) = session.file_name() else {
            continue;
        };
        let target = dest.join(name);
        std::fs::create_dir_all(&target)?;
        for entry in std::fs::read_dir(session)?.flatten() {
            let path = entry.path();
            if path.is_file() {
                std::fs::copy(&path, target.join(entry.file_name()))?;
                copied += 1;
            }
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh empty directory under the system temp dir
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("harpy_test_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"x").unwrap();
    }

    #[test]
    fn lists_sessions_newest_first_with_images_in_order() {
        let root = temp_dir("sessions");
        touch(&root.join("session_20260101_120000/edit_002_vertical.png"));
        touch(&root.join("session_20260101_120000/edit_001_horizontal.png"));
        touch(&root.join("session_20260102_090000/edit_001_diagonal.png"));
        touch(&root.join("session_20260103_090000/notes.txt")); // no images: skipped
        touch(&root.join(".hidden/edit_001_x.png"));

        let sessions = list_sessions(&root);
        let names: Vec<_> = sessions.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["session_20260102_090000", "session_20260101_120000"]);
        let files: Vec<_> = sessions[1].images.iter().map(|p| p.file_name().unwrap().to_owned()).collect();
        assert_eq!(files, ["edit_001_horizontal.png", "edit_002_vertical.png"]);

        assert!(list_sessions(&root.join("missing")).is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lists_folders_then_images() {
        let root = temp_dir("browse");
        touch(&root.join("b.JPG"));
        touch(&root.join("a.png"));
        touch(&root.join("readme.txt"));
        touch(&root.join(".secret.png"));
        touch(&root.join("Photos/x.png"));

        let names: Vec<_> = list_dir(&root).unwrap().into_iter().map(|e| (e.name, e.is_dir)).collect();
        assert_eq!(
            names,
            [("Photos".to_string(), true), ("a.png".to_string(), false), ("b.JPG".to_string(), false)]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deleting_last_image_removes_session_folder() {
        let root = temp_dir("delete");
        let first = root.join("session_1/edit_001_a.png");
        let second = root.join("session_1/edit_002_a.png");
        touch(&first);
        touch(&second);

        delete_image(&first).unwrap();
        assert!(root.join("session_1").is_dir());
        delete_image(&second).unwrap();
        assert!(!root.join("session_1").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn copies_only_selected_sessions() {
        let root = temp_dir("export");
        touch(&root.join("src/session_1/edit_001_a.png"));
        touch(&root.join("src/session_1/edit_002_a.png"));
        touch(&root.join("src/session_2/edit_001_b.png"));

        let copied = copy_sessions(&[root.join("src/session_1")], &root.join("usb")).unwrap();
        assert_eq!(copied, 2);
        assert!(root.join("usb/session_1/edit_002_a.png").is_file());
        assert!(!root.join("usb/session_2").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
