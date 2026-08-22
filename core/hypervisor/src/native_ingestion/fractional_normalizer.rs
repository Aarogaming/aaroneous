/// Resolution-agnostic fractional normalizer.
///
/// Encodes screen-update regions within 0.0–1.0 float boundaries,
/// making all coordinate spaces resolution-independent. Inputs at
/// any resolution produce normalized feature vectors of fixed size,
/// enabling the upstream SIMD/SVD pipeline to process uniformly.
/// A normalized frame with all coordinates in [0.0, 1.0].
#[repr(C)]
pub struct NormalizedFrame {
    /// Normalized pixel buffer: linearized row-major f32 values in [0.0, 1.0].
    pub pixels: Vec<f32>,
    /// Normalized width in fractional units (always 1.0).
    pub width_norm: f32,
    /// Normalized height in fractional units (always 1.0).
    pub height_norm: f32,
    /// Original source width in pixels.
    pub source_width: u32,
    /// Original source height in pixels.
    pub source_height: u32,
    /// Aspect ratio (width / height) at source resolution.
    pub aspect_ratio: f32,
}

/// Maps pixel coordinates between screen resolutions and fractional space.
pub struct FractionalNormalizer {
    /// Target number of samples per dimension for the normalized grid.
    pub sample_grid: u32,
}

impl FractionalNormalizer {
    /// Create a normalizer with the given sample grid size.
    ///
    /// `sample_grid`: the normalized output will be `sample_grid × sample_grid`
    ///               regardless of input resolution.
    pub fn new(sample_grid: u32) -> Self {
        Self {
            sample_grid: sample_grid.max(4),
        }
    }

    /// Normalize a raw BGRA framebuffer into a fixed-size [0.0, 1.0] float grid.
    ///
    /// `raw_bgra`: raw BGRA 8bpp pixel data, length = `width * height * 4`.
    /// `width`, `height`: source frame dimensions.
    ///
    /// Returns a `NormalizedFrame` with `sample_grid * sample_grid` float pixels.
    pub fn normalize(&self, raw_bgra: &[u8], width: u32, height: u32) -> NormalizedFrame {
        let aspect = width as f32 / height as f32;
        let grid = self.sample_grid as usize;
        let mut pixels = vec![0.0f32; grid * grid];

        let sx = width as f32 / grid as f32;
        let sy = height as f32 / grid as f32;

        for gy in 0..grid {
            for gx in 0..grid {
                // Map normalized grid cell to source pixel center
                let src_x = (gx as f32 + 0.5) * sx;
                let src_y = (gy as f32 + 0.5) * sy;
                let px = src_x as usize;
                let py = src_y as usize;
                let idx = (py * width as usize + px) * 4;

                if idx + 3 < raw_bgra.len() {
                    // BGRA → luminance ITU-R BT.601, normalized to [0.0, 1.0]
                    let b = raw_bgra[idx] as f32;
                    let g = raw_bgra[idx + 1] as f32;
                    let r = raw_bgra[idx + 2] as f32;
                    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
                    pixels[gy * grid + gx] = (lum / 255.0).clamp(0.0, 1.0);
                }
            }
        }

        NormalizedFrame {
            pixels,
            width_norm: 1.0,
            height_norm: 1.0,
            source_width: width,
            source_height: height,
            aspect_ratio: aspect,
        }
    }

    /// Normalize a pre-extracted region of interest (ROI) to the grid.
    ///
    /// `raw_bgra`: full frame in BGRA 8bpp.
    /// `frame_w`, `frame_h`: full frame dimensions.
    /// `roi_x`, `roi_y`, `roi_w`, `roi_h`: pixel-coordinate bounding box.
    #[allow(clippy::too_many_arguments)]
    pub fn normalize_roi(
        &self,
        raw_bgra: &[u8],
        frame_w: u32,
        _frame_h: u32,
        roi_x: u32,
        roi_y: u32,
        roi_w: u32,
        roi_h: u32,
    ) -> NormalizedFrame {
        let grid = self.sample_grid as usize;
        let mut pixels = vec![0.0f32; grid * grid];
        let rw = roi_w.max(1) as f32;
        let rh = roi_h.max(1) as f32;

        for gy in 0..grid {
            for gx in 0..grid {
                let fx = (gx as f32 + 0.5) / grid as f32; // [0,1]
                let fy = (gy as f32 + 0.5) / grid as f32;
                let src_x = (roi_x as f32 + fx * rw) as usize;
                let src_y = (roi_y as f32 + fy * rh) as usize;
                let idx = (src_y * frame_w as usize + src_x) * 4;

                if idx + 3 < raw_bgra.len() {
                    let b = raw_bgra[idx] as f32;
                    let g = raw_bgra[idx + 1] as f32;
                    let r = raw_bgra[idx + 2] as f32;
                    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
                    pixels[gy * grid + gx] = (lum / 255.0).clamp(0.0, 1.0);
                }
            }
        }

        NormalizedFrame {
            pixels,
            width_norm: 1.0,
            height_norm: 1.0,
            source_width: roi_w,
            source_height: roi_h,
            aspect_ratio: rw / rh,
        }
    }

    /// Convert a fractional coordinate back to source pixel coordinate.
    pub fn frac_to_pixel(&self, frac_x: f32, frac_y: f32, src_w: u32, src_h: u32) -> (u32, u32) {
        let px = (frac_x.clamp(0.0, 1.0) * src_w as f32) as u32;
        let py = (frac_y.clamp(0.0, 1.0) * src_h as f32) as u32;
        (px.min(src_w - 1), py.min(src_h - 1))
    }

    /// Denormalize a frame back to BGRA pixel data using bilinear interpolation.
    ///
    /// Reconstructs a full-resolution BGRA framebuffer from a normalized float grid.
    /// Each grid cell is expanded to a block of pixels with interpolated values.
    pub fn denormalize(&self, frame: &NormalizedFrame, target_w: u32, target_h: u32) -> Vec<u8> {
        let grid = self.sample_grid as usize;
        let mut bgra = vec![0u8; (target_w * target_h * 4) as usize];

        let sx = target_w as f32 / grid as f32;
        let sy = target_h as f32 / grid as f32;

        for ty in 0..target_h {
            for tx in 0..target_w {
                // Map target pixel to grid coordinates (floating point)
                let gx = tx as f32 / sx;
                let gy = ty as f32 / sy;

                // Bilinear interpolation from 4 nearest grid cells
                let gx0 = gx as usize;
                let gy0 = gy as usize;
                let gx1 = (gx0 + 1).min(grid - 1);
                let gy1 = (gy0 + 1).min(grid - 1);
                let fx = gx - gx0 as f32;
                let fy = gy - gy0 as f32;

                let v00 = frame.pixels[gy0 * grid + gx0];
                let v10 = frame.pixels[gy0 * grid + gx1];
                let v01 = frame.pixels[gy1 * grid + gx0];
                let v11 = frame.pixels[gy1 * grid + gx1];

                let lum = v00 * (1.0 - fx) * (1.0 - fy)
                    + v10 * fx * (1.0 - fy)
                    + v01 * (1.0 - fx) * fy
                    + v11 * fx * fy;

                let gray = (lum * 255.0).clamp(0.0, 255.0) as u8;
                let idx = ((ty * target_w + tx) * 4) as usize;
                // Write as BGRA (same as source format)
                bgra[idx] = gray; // B
                bgra[idx + 1] = gray; // G
                bgra[idx + 2] = gray; // R
                bgra[idx + 3] = 255; // A
            }
        }

        bgra
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_basic() {
        let norm = FractionalNormalizer::new(16);
        let w = 1920u32;
        let h = 1080u32;
        let raw = vec![128u8; (w * h * 4) as usize];
        let result = norm.normalize(&raw, w, h);
        assert_eq!(result.pixels.len(), 16 * 16);
        assert!((result.aspect_ratio - 1920.0 / 1080.0).abs() < 0.001);
        // All mid-gray should produce ~0.5 luminance
        assert!((result.pixels[0] - 0.5).abs() < 0.02);
    }

    #[test]
    fn test_normalize_different_resolutions_same_output_size() {
        let norm = FractionalNormalizer::new(32);
        // 4K
        let raw_4k = vec![0u8; (3840 * 2160 * 4) as usize];
        let r1 = norm.normalize(&raw_4k, 3840, 2160);
        assert_eq!(r1.pixels.len(), 32 * 32);

        // 1080p
        let raw_1080 = vec![0u8; (1920 * 1080 * 4) as usize];
        let r2 = norm.normalize(&raw_1080, 1920, 1080);
        assert_eq!(r2.pixels.len(), 32 * 32);

        // Both black → identical output
        for i in 0..32 * 32 {
            assert!((r1.pixels[i] - r2.pixels[i]).abs() < 0.001);
        }
    }

    #[test]
    fn test_roi_normalization() {
        let norm = FractionalNormalizer::new(8);
        let w = 100u32;
        let h = 100u32;
        let raw = vec![255u8; (w * h * 4) as usize];
        // White ROI in top-left quadrant
        let result = norm.normalize_roi(&raw, w, h, 0, 0, 50, 50);
        assert_eq!(result.pixels.len(), 64);
        assert!((result.aspect_ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_frac_to_pixel_roundtrip() {
        let norm = FractionalNormalizer::new(16);
        let (px, py) = norm.frac_to_pixel(0.5, 0.5, 1920, 1080);
        assert_eq!(px, 960);
        assert_eq!(py, 540);
    }

    #[test]
    fn test_clamping() {
        let norm = FractionalNormalizer::new(8);
        let (px, py) = norm.frac_to_pixel(2.0, -0.5, 100, 100);
        assert_eq!(px, 99);
        assert_eq!(py, 0);
    }
}
