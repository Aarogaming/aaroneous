use crate::native_ingestion::{IngestionDataChunk, IngestionSourceType, ScreenCoordinate};
use crate::native_ingestion::fractional_normalizer::FractionalNormalizer;
use crate::native_ingestion::simd_xor_delta::{XorDeltaScreen, XorDeltaConfig, DeltaReport};

/// Configuration for video stream ingestion.
pub struct VideoStreamConfig {
    /// Path to the MP4/AVI video file.
    pub file_path: String,
    /// Frame width in pixels (after decode).
    pub width: u32,
    /// Frame height in pixels (after decode).
    pub height: u32,
    /// Fractional grid size for normalized frame output.
    pub grid_size: u32,
}

impl Default for VideoStreamConfig {
    fn default() -> Self {
        Self {
            file_path: String::new(),
            width: 640,
            height: 480,
            grid_size: 32,
        }
    }
}

/// Result of processing a single video frame through the XOR-delta pipeline.
pub struct VideoFrameDelta {
    pub frame_index: u64,
    pub timestamp_ns: u64,
    pub delta_report: DeltaReport,
    pub normalized_pixels: Vec<f32>,
}

/// Processes MP4 video recordings of desktop sessions as sequential raw pixel
/// chunks, applying the SIMD XOR-delta loop directly to frame rows and tracking
/// coordinate boundaries and timestamp offsets.
pub struct VideoStreamIngestor {
    config: VideoStreamConfig,
    delta_screen: XorDeltaScreen,
    normalizer: FractionalNormalizer,
    frame_counter: u64,
}

impl VideoStreamIngestor {
    pub fn new(config: VideoStreamConfig) -> Self {
        let total_bytes = (config.width * config.height * 4) as usize;
        Self {
            delta_screen: XorDeltaScreen::new(XorDeltaConfig {
                width: config.width,
                height: config.height,
                change_threshold: 64,
            }),
            normalizer: FractionalNormalizer::new(config.grid_size),
            config,
            frame_counter: 0,
        }
    }

    /// Ingest a single raw BGRA frame (as if decoded from a video stream).
    ///
    /// Returns the frame delta report and a normalized pixel array ready for
    /// VSA vectorization.
    pub fn ingest_frame(&mut self, raw_bgra: &[u8]) -> VideoFrameDelta {
        self.frame_counter += 1;

        // Run SIMD XOR-delta screening
        let delta = self.delta_screen.screen_delta(raw_bgra);

        // Normalize to fractional grid
        let normalized = self.normalizer.normalize(
            raw_bgra,
            self.config.width,
            self.config.height,
        );

        VideoFrameDelta {
            frame_index: self.frame_counter,
            timestamp_ns: now_ns(),
            delta_report: delta,
            normalized_pixels: normalized.pixels,
        }
    }

    /// Convert a frame delta into an IngestionDataChunk suitable for HDF5 storage.
    pub fn to_chunk(&self, delta: &VideoFrameDelta) -> IngestionDataChunk {
        // Compute VSA signature from the normalized pixel data
        let mut vsa = [0u64; 128];
        for (idx, &pixel) in delta.normalized_pixels.iter().enumerate() {
            let bits = (pixel * u64::MAX as f32) as u64;
            vsa[idx % 128] ^= bits;
        }

        IngestionDataChunk {
            source_type: IngestionSourceType::DesktopVideoRecord,
            source_identifier: 0,
            byte_offset: delta.frame_index,
            coordinate_bounds: [0.0, 0.0, 1.0, 1.0],
            spatial_signature: vsa,
        }
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_counter
    }

    /// Reset the delta baseline (useful when switching video segments).
    pub fn reset(&mut self) {
        self.delta_screen.reset();
    }
}

fn now_ns() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_stream_ingest_frame() {
        let cfg = VideoStreamConfig {
            file_path: String::new(),
            width: 16,
            height: 16,
            grid_size: 8,
        };
        let mut ingestor = VideoStreamIngestor::new(cfg);

        let frame = vec![0u8; 16 * 16 * 4];
        let delta = ingestor.ingest_frame(&frame);
        assert_eq!(delta.frame_index, 1);
        assert_eq!(delta.normalized_pixels.len(), 8 * 8);
    }

    #[test]
    fn test_video_stream_to_chunk() {
        let cfg = VideoStreamConfig {
            file_path: String::new(),
            width: 8,
            height: 8,
            grid_size: 4,
        };
        let mut ingestor = VideoStreamIngestor::new(cfg);
        let frame = vec![128u8; 8 * 8 * 4];
        let delta = ingestor.ingest_frame(&frame);
        let chunk = ingestor.to_chunk(&delta);
        assert_eq!(chunk.source_type as u8, IngestionSourceType::DesktopVideoRecord as u8);
    }

    #[test]
    fn test_reset_and_continue() {
        let cfg = VideoStreamConfig {
            file_path: String::new(),
            width: 8,
            height: 8,
            grid_size: 4,
        };
        let mut ingestor = VideoStreamIngestor::new(cfg);
        let frame_a = vec![0u8; 8 * 8 * 4];
        let frame_b = vec![0xFFu8; 8 * 8 * 4];
        ingestor.ingest_frame(&frame_a);
        ingestor.reset();
        let delta = ingestor.ingest_frame(&frame_b);
        // After reset, previous frame is zeros, so frame_b should show changes
        assert!(delta.delta_report.changed_bytes > 0);
    }
}
