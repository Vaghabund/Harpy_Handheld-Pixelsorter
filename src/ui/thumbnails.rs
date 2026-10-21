use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

/// Longest side of a generated thumbnail, in pixels
const THUMBNAIL_SIZE: u32 = 320;

enum Thumbnail {
    Loading,
    Ready(egui::TextureHandle),
    Failed,
}

/// Lazily decoded, downscaled image thumbnails for the gallery and USB browser.
/// Decoding runs on the rayon thread pool; results are turned into textures
/// on the UI thread in `poll`.
pub struct ThumbnailCache {
    thumbnails: HashMap<PathBuf, Thumbnail>,
    sender: Sender<(PathBuf, Option<egui::ColorImage>)>,
    receiver: Receiver<(PathBuf, Option<egui::ColorImage>)>,
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { thumbnails: HashMap::new(), sender, receiver }
    }
}

impl ThumbnailCache {
    /// Thumbnail texture for `path`, starting to load it if needed.
    /// Returns None while loading or if the file can't be decoded.
    pub fn get(&mut self, ctx: &egui::Context, path: &Path) -> Option<&egui::TextureHandle> {
        if !self.thumbnails.contains_key(path) {
            self.thumbnails.insert(path.to_path_buf(), Thumbnail::Loading);
            let (sender, ctx, path) = (self.sender.clone(), ctx.clone(), path.to_path_buf());
            rayon::spawn(move || {
                let thumbnail = load_thumbnail(&path);
                let _ = sender.send((path, thumbnail));
                ctx.request_repaint();
            });
        }
        match self.thumbnails.get(path) {
            Some(Thumbnail::Ready(texture)) => Some(texture),
            _ => None,
        }
    }

    pub fn is_failed(&self, path: &Path) -> bool {
        matches!(self.thumbnails.get(path), Some(Thumbnail::Failed))
    }

    /// Upload finished thumbnails as textures
    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok((path, image)) = self.receiver.try_recv() {
            // Ignore results for entries dropped by clear()/forget() meanwhile
            if let Some(slot) = self.thumbnails.get_mut(&path) {
                *slot = match image {
                    Some(image) => {
                        let name = format!("thumb:{}", path.display());
                        Thumbnail::Ready(ctx.load_texture(name, image, egui::TextureOptions::LINEAR))
                    }
                    None => Thumbnail::Failed,
                };
            }
        }
    }

    pub fn forget(&mut self, path: &Path) {
        self.thumbnails.remove(path);
    }

    /// Free all thumbnail textures (when leaving the file screens)
    pub fn clear(&mut self) {
        self.thumbnails.clear();
    }
}

fn load_thumbnail(path: &Path) -> Option<egui::ColorImage> {
    let image = match image::open(path) {
        Ok(image) => image,
        Err(e) => {
            log::warn!("Could not load thumbnail for {}: {}", path.display(), e);
            return None;
        }
    };
    let thumbnail = image.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE).to_rgb8();
    let size = [thumbnail.width() as usize, thumbnail.height() as usize];
    Some(egui::ColorImage::from_rgb(size, thumbnail.as_raw()))
}
