//! Halftone harpy flight loop shown full screen on the splash.
//!
//! `assets/harpy_flight.png` is a grayscale strip: `COLS` wide and
//! `ROWS * frames` tall, one pixel per grid cell holding that cell's dot
//! radius (0..255 = 0..half a cell). The dots are drawn as circles at
//! runtime, so the animation stays crisp at any screen resolution.

use eframe::egui;
use std::sync::OnceLock;

const FLIGHT_PNG: &[u8] = include_bytes!("../../assets/harpy_flight.png");
const COLS: usize = 120;
const ROWS: usize = 67;
const FPS: f32 = 10.0;

pub struct FlightAnimation {
    frames: usize,
    radii: Vec<u8>,
}

impl FlightAnimation {
    /// Decoded once on first use
    pub fn get() -> Option<&'static FlightAnimation> {
        static FLIGHT: OnceLock<Option<FlightAnimation>> = OnceLock::new();
        FLIGHT.get_or_init(Self::decode).as_ref()
    }

    fn decode() -> Option<FlightAnimation> {
        let strip = match image::load_from_memory(FLIGHT_PNG) {
            Ok(img) => img.to_luma8(),
            Err(e) => {
                log::warn!("Could not decode flight animation: {}", e);
                return None;
            }
        };
        let (w, h) = strip.dimensions();
        if w as usize != COLS || !(h as usize).is_multiple_of(ROWS) {
            log::warn!("Flight animation has unexpected size {}x{}", w, h);
            return None;
        }
        Some(FlightAnimation { frames: h as usize / ROWS, radii: strip.into_raw() })
    }

    /// Time between frames; the splash only repaints this often
    pub fn frame_interval() -> std::time::Duration {
        std::time::Duration::from_secs_f32(1.0 / FPS)
    }

    /// `elapsed` snapped back to the start of its frame, so everything
    /// tied to the animation (like the fade) steps at the same rate
    pub fn snap(elapsed: f32) -> f32 {
        (elapsed * FPS).floor() / FPS
    }

    /// Length of one wingbeat loop in seconds
    pub fn duration(&self) -> f32 {
        self.frames as f32 / FPS
    }

    /// Draw the frame for `elapsed` seconds, centered and covering `rect`
    /// (the grid overflows and is clipped on the longer side, so the dots
    /// reach every edge of the screen)
    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, elapsed: f32, color: egui::Color32) {
        let frame = (elapsed * FPS) as usize % self.frames;
        let cell = (rect.width() / COLS as f32).max(rect.height() / ROWS as f32);
        let painter = painter.with_clip_rect(rect);
        let origin = rect.center() - egui::vec2(cell * COLS as f32, cell * ROWS as f32) / 2.0;
        let cells = &self.radii[frame * COLS * ROWS..(frame + 1) * COLS * ROWS];
        for (i, &r) in cells.iter().enumerate() {
            if r == 0 {
                continue;
            }
            let (x, y) = ((i % COLS) as f32, (i / COLS) as f32);
            let center = origin + egui::vec2((x + 0.5) * cell, (y + 0.5) * cell);
            painter.circle_filled(center, r as f32 / 255.0 * cell / 2.0, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_animation_decodes() {
        let flight = FlightAnimation::get().expect("flight animation should decode");
        assert!(flight.frames > 1);
        assert!(flight.duration() > 1.0);
        // Every frame shows the bird
        for f in 0..flight.frames {
            let cells = &flight.radii[f * COLS * ROWS..(f + 1) * COLS * ROWS];
            assert!(cells.iter().filter(|&&r| r > 0).count() > 100, "frame {} is empty", f);
        }
    }
}
