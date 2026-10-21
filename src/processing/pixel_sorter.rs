use anyhow::Result;
use image::RgbImage;
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortingAlgorithm {
    Horizontal,
    Vertical,
    Diagonal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortMode {
    Brightness,
    Black,
    White,
}

impl SortMode {
    pub fn all() -> &'static [SortMode] {
        &[SortMode::Brightness, SortMode::Black, SortMode::White]
    }
    pub fn name(&self) -> &'static str {
        match self {
            SortMode::Brightness => "Brightness",
            SortMode::Black => "Black",
            SortMode::White => "White",
        }
    }
    pub fn next(&self) -> SortMode {
        let all = Self::all();
        let idx = all.iter().position(|x| x == self).unwrap();
        all[(idx + 1) % all.len()]
    }
}

impl std::fmt::Display for SortingAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl SortingAlgorithm {
    pub fn all() -> &'static [SortingAlgorithm] {
        &[
            SortingAlgorithm::Horizontal,
            SortingAlgorithm::Vertical,
            SortingAlgorithm::Diagonal,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SortingAlgorithm::Horizontal => "Horizontal",
            SortingAlgorithm::Vertical => "Vertical",
            SortingAlgorithm::Diagonal => "Diagonal",
        }
    }

    pub fn next(&self) -> SortingAlgorithm {
        let all = Self::all();
        let current_index = all.iter().position(|&x| std::mem::discriminant(&x) == std::mem::discriminant(self)).unwrap();
        all[(current_index + 1) % all.len()]
    }
}

#[derive(Debug, Clone)]
pub struct SortingParameters {
    pub threshold: f32,
    pub color_tint: f32,
    pub sort_mode: SortMode,
}

impl Default for SortingParameters {
    fn default() -> Self {
        Self {
            threshold: 0.0,  // Start with no threshold (sort all pixels)
            color_tint: 0.0,
            sort_mode: SortMode::Brightness,
        }
    }
}

pub struct PixelSorter;

impl PixelSorter {
    pub fn new() -> Self {
        Self
    }

    pub fn sort_pixels(
        &self,
        image: &RgbImage,
        algorithm: SortingAlgorithm,
        params: &SortingParameters,
    ) -> Result<RgbImage> {
        let mut result = image.clone();
        let width = image.width() as usize;
        let height = image.height() as usize;
        if width == 0 || height == 0 {
            return Ok(result);
        }

        match algorithm {
            // Rows are contiguous in the buffer: sort each one in place, in parallel
            SortingAlgorithm::Horizontal => {
                result.par_chunks_mut(width * 3).for_each_init(LineBuffers::default, |buffers, row| {
                    buffers.load(row.chunks_exact(3));
                    buffers.sort(params);
                    for (dst, px) in row.chunks_exact_mut(3).zip(&buffers.pixels) {
                        dst.copy_from_slice(px);
                    }
                });
            }
            // Columns and diagonals are strided: sort them in parallel from the
            // source image, then write the results back
            SortingAlgorithm::Vertical => {
                let lines: Vec<Line> = (0..width).map(|x| Line { start: x, step: width, len: height }).collect();
                Self::sort_strided_lines(image, &mut result, &lines, params);
            }
            SortingAlgorithm::Diagonal => {
                // Top-left to bottom-right diagonals, starting along the top row
                // (offset >= 0) and down the left column (offset < 0)
                let lines: Vec<Line> = (0..width)
                    .map(|x| Line { start: x, step: width + 1, len: height.min(width - x) })
                    .chain((1..height).map(|y| Line { start: y * width, step: width + 1, len: width.min(height - y) }))
                    .collect();
                Self::sort_strided_lines(image, &mut result, &lines, params);
            }
        }

        Ok(result)
    }

    fn sort_strided_lines(src: &RgbImage, dst: &mut RgbImage, lines: &[Line], params: &SortingParameters) {
        let src: &[u8] = src.as_raw();
        let sorted: Vec<Vec<[u8; 3]>> = lines
            .par_iter()
            .map_init(LineBuffers::default, |buffers, line| {
                buffers.load(line.indices().map(|i| &src[i * 3..i * 3 + 3]));
                buffers.sort(params);
                buffers.pixels.clone()
            })
            .collect();

        let dst: &mut [u8] = dst;
        for (line, pixels) in lines.iter().zip(sorted) {
            for (i, px) in line.indices().zip(pixels) {
                dst[i * 3..i * 3 + 3].copy_from_slice(&px);
            }
        }
    }


}

/// A line of pixels (row, column or diagonal) as indices into the pixel grid
struct Line {
    start: usize,
    step: usize,
    len: usize,
}

impl Line {
    fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.len).map(move |i| self.start + i * self.step)
    }
}

/// Per-thread scratch buffers reused across lines to avoid allocations
#[derive(Default)]
struct LineBuffers {
    pixels: Vec<[u8; 3]>,
    brightness: Vec<f32>,
    segment: Vec<(u8, [u8; 3])>,
}

impl LineBuffers {
    fn load<'a>(&mut self, pixels: impl Iterator<Item = &'a [u8]>) {
        self.pixels.clear();
        self.pixels.extend(pixels.map(|p| [p[0], p[1], p[2]]));
        self.brightness.clear();
        self.brightness.extend(self.pixels.iter().map(|&p| brightness(p)));
    }

    /// Sort every interval of the line whose neighbouring brightness
    /// differences stay within the threshold
    fn sort(&mut self, params: &SortingParameters) {
        let len = self.pixels.len();
        let mut start = 0;
        for i in 1..=len {
            let at_boundary = i == len || (self.brightness[i] - self.brightness[i - 1]).abs() > params.threshold;
            if at_boundary {
                if i - start > 1 {
                    self.sort_segment(start, i, params.sort_mode);
                }
                start = i;
            }
        }
    }

    fn sort_segment(&mut self, start: usize, end: usize, mode: SortMode) {
        self.segment.clear();
        self.segment.extend((start..end).map(|i| {
            let px = self.pixels[i];
            let key = match mode {
                SortMode::Brightness => self.brightness[i].round() as u8,
                SortMode::Black => px[0],
                SortMode::White => 255 - px[0],
            };
            (key, px)
        }));
        // Stable sort keeps equal-key pixels in their original order
        self.segment.sort_by_key(|&(key, _)| key);
        for (dst, &(_, px)) in self.pixels[start..end].iter_mut().zip(&self.segment) {
            *dst = px;
        }
    }
}

/// Luminance (0-255) using the standard RGB to grayscale weights
fn brightness(p: [u8; 3]) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

/// Convert a hue value (0-360) to RGB (u8, u8, u8)
pub fn hue_to_rgb(hue: f32) -> (u8, u8, u8) {
    let h = ((hue % 360.0) + 360.0) % 360.0; // Normalize to 0-360
    let s = 1.0; // Full saturation
    let v = 1.0; // Full brightness

    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r_prime, g_prime, b_prime) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let r = ((r_prime + m) * 255.0).round() as u8;
    let g = ((g_prime + m) * 255.0).round() as u8;
    let b = ((b_prime + m) * 255.0).round() as u8;

    (r, g, b)
}

/// Convert a hue value (0-360) to RGB image::Rgb<u8>
pub fn hue_to_rgb_pixel(hue: f32) -> image::Rgb<u8> {
    let (r, g, b) = hue_to_rgb(hue);
    image::Rgb([r, g, b])
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn test_pixel_brightness() {
        let white = [255, 255, 255];
        let black = [0, 0, 0];
        let red = [255, 0, 0];

        assert!(brightness(white) > brightness(black));
        assert!(brightness(white) > brightness(red));
    }

    #[test]
    fn test_algorithm_cycling() {
        let algorithm = SortingAlgorithm::Horizontal;
        let next = algorithm.next();
        assert_eq!(next.name(), "Vertical");
    }

    #[test]
    fn test_sorting_parameters_default() {
        let params = SortingParameters::default();
        assert_eq!(params.threshold, 0.0);
        assert_eq!(params.sort_mode, SortMode::Brightness);
    }

    /// The original (pre-optimisation) implementation, kept to verify the
    /// optimised sorter produces identical output
    struct Reference;

    impl Reference {
        pub fn sort_pixels(
            &self,
            image: &RgbImage,
            algorithm: SortingAlgorithm,
            params: &SortingParameters,
        ) -> Result<RgbImage> {
            let mut result = image.clone();

            match algorithm {
                SortingAlgorithm::Horizontal => self.sort_horizontal(&mut result, params),
                SortingAlgorithm::Vertical => self.sort_vertical(&mut result, params),
                SortingAlgorithm::Diagonal => self.sort_diagonal(&mut result, params),
            }

            Ok(result)
        }

        fn sort_horizontal(&self, image: &mut RgbImage, params: &SortingParameters) {
            let (width, height) = image.dimensions();
            for y in 0..height {
                let row_pixels: Vec<(usize, Rgb<u8>)> = (0..width)
                    .map(|x| (x as usize, *image.get_pixel(x, y)))
                    .collect();
                let intervals = self.find_intervals(&row_pixels, params.threshold);
                for (start, end) in intervals {
                    if end - start > 1 {
                        let mut segment: Vec<_> = row_pixels[start..end].iter().map(|(_, pixel)| *pixel).collect();
                        segment.sort_by_key(|a| self.sort_key(a, params.sort_mode));
                        for (i, &pixel) in segment.iter().enumerate() {
                            image.put_pixel((start + i) as u32, y, pixel);
                        }
                    }
                }
            }
        }

        fn sort_vertical(&self, image: &mut RgbImage, params: &SortingParameters) {
            let (width, height) = image.dimensions();
            for x in 0..width {
                let col_pixels: Vec<(usize, Rgb<u8>)> = (0..height)
                    .map(|y| (y as usize, *image.get_pixel(x, y)))
                    .collect();
                let intervals = self.find_intervals(&col_pixels, params.threshold);
                for (start, end) in intervals {
                    if end - start > 1 {
                        let mut segment: Vec<_> = col_pixels[start..end].iter().map(|(_, pixel)| *pixel).collect();
                        segment.sort_by_key(|a| self.sort_key(a, params.sort_mode));
                        for (i, &pixel) in segment.iter().enumerate() {
                            image.put_pixel(x, (start + i) as u32, pixel);
                        }
                    }
                }
            }
        }

        fn sort_diagonal(&self, image: &mut RgbImage, params: &SortingParameters) {
            let (width, height) = image.dimensions();
            let (w, h) = (width as i32, height as i32);
            for offset in -h..w {
                let mut diagonal_pixels = Vec::new();
                if offset >= 0 {
                    for i in 0..std::cmp::min(h, w - offset) {
                        let x = (i + offset) as u32;
                        let y = i as u32;
                        diagonal_pixels.push(((x, y), *image.get_pixel(x, y)));
                    }
                } else {
                    for i in 0..std::cmp::min(w, h + offset) {
                        let x = i as u32;
                        let y = (i - offset) as u32;
                        diagonal_pixels.push(((x, y), *image.get_pixel(x, y)));
                    }
                }
                if diagonal_pixels.len() <= 1 {
                    continue;
                }
                let pixel_values: Vec<_> = diagonal_pixels.iter().map(|(_, pixel)| *pixel).collect();
                let intervals = self.find_intervals_from_pixels(&pixel_values, params.threshold);
                for (start, end) in intervals {
                    if end - start > 1 {
                        let mut segment: Vec<_> = pixel_values[start..end].to_vec();
                        segment.sort_by_key(|a| self.sort_key(a, params.sort_mode));
                        for (i, &pixel) in segment.iter().enumerate() {
                            let ((x, y), _) = diagonal_pixels[start + i];
                            image.put_pixel(x, y, pixel);
                        }
                    }
                }
            }
        }
        fn sort_key(&self, pixel: &Rgb<u8>, mode: SortMode) -> u8 {
            match mode {
                SortMode::Brightness => {
                    let b = self.pixel_brightness(pixel);
                    b.round() as u8
                },
                SortMode::Black => pixel[0],
                SortMode::White => 255 - pixel[0],
            }
        }

        fn find_intervals(&self, pixels: &[(usize, Rgb<u8>)], threshold: f32) -> Vec<(usize, usize)> {
            let pixel_values: Vec<_> = pixels.iter().map(|(_, pixel)| *pixel).collect();
            self.find_intervals_from_pixels(&pixel_values, threshold)
        }

        fn find_intervals_from_pixels(&self, pixels: &[Rgb<u8>], threshold: f32) -> Vec<(usize, usize)> {
            if pixels.len() <= 1 {
                return Vec::new();
            }

            let mut intervals = Vec::new();
            let mut start = 0;

            for i in 1..pixels.len() {
                let brightness_diff = (self.pixel_brightness(&pixels[i]) - self.pixel_brightness(&pixels[i - 1])).abs();
            
                if brightness_diff > threshold {
                    if i - start > 1 {
                        intervals.push((start, i));
                    }
                    start = i;
                }
            }

            // Add final interval
            if pixels.len() - start > 1 {
                intervals.push((start, pixels.len()));
            }

            intervals
        }

        fn pixel_brightness(&self, pixel: &Rgb<u8>) -> f32 {
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;
            0.299 * r + 0.587 * g + 0.114 * b
        }
    }

    /// Deterministic pseudo-random test image (xorshift), with smooth areas so
    /// intervals of varying length occur
    fn test_image(width: u32, height: u32, seed: u32) -> RgbImage {
        let mut state = seed.max(1);
        RgbImage::from_fn(width, height, |x, y| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let noise = (state % 24) as u8;
            let base = ((x * 7 + y * 3) % 200) as u8;
            Rgb([base.wrapping_add(noise), base / 2 + noise, 255 - base])
        })
    }

    #[test]
    fn matches_reference_implementation() {
        let sorter = PixelSorter::new();
        let sizes = [(1, 1), (1, 7), (7, 1), (13, 5), (5, 13), (40, 40), (64, 23)];
        let mut changed_cases = 0;
        for (seed, &(w, h)) in sizes.iter().enumerate() {
            let image = test_image(w, h, seed as u32 + 1);
            for &algorithm in SortingAlgorithm::all() {
                for &sort_mode in SortMode::all() {
                    for threshold in [0.0, 5.0, 30.0, 125.0] {
                        let params = SortingParameters { threshold, sort_mode, ..Default::default() };
                        let expected = Reference.sort_pixels(&image, algorithm, &params).unwrap();
                        let actual = sorter.sort_pixels(&image, algorithm, &params).unwrap();
                        if actual != image {
                            changed_cases += 1;
                        }
                        assert!(
                            expected == actual,
                            "mismatch for {}x{} {:?} {:?} threshold {}",
                            w, h, algorithm, sort_mode, threshold
                        );
                    }
                }
            }
        }
        // Guard against a vacuous test where nothing gets sorted
        assert!(changed_cases > 0);
    }

    /// Timing comparison on a screen-sized image:
    /// cargo test --release -- --ignored --nocapture benchmark
    #[test]
    #[ignore]
    fn benchmark_against_reference() {
        let sorter = PixelSorter::new();
        let image = test_image(1024, 600, 42);
        for &algorithm in SortingAlgorithm::all() {
            let params = SortingParameters { threshold: 30.0, ..Default::default() };
            let start = std::time::Instant::now();
            Reference.sort_pixels(&image, algorithm, &params).unwrap();
            let reference = start.elapsed();
            let start = std::time::Instant::now();
            sorter.sort_pixels(&image, algorithm, &params).unwrap();
            let optimised = start.elapsed();
            println!("{:?}: reference {:?}, optimised {:?}", algorithm, reference, optimised);
        }
    }
}
