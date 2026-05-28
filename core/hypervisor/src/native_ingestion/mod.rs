pub mod shmem_capture;
pub mod simd_xor_delta;
pub mod svd_feature_select;
pub mod fractional_normalizer;
pub mod hardware_governor;
pub mod fs_crawl;
pub mod chebyshev_trajectory;
pub mod video_stream;

pub use shmem_capture::{ShmemCapture, FrameCaptureConfig};
pub use simd_xor_delta::{XorDeltaScreen, XorDeltaConfig, DeltaReport};
pub use svd_feature_select::{SvdReducer, SvdConfig, ReducedFeatures};
pub use fractional_normalizer::{FractionalNormalizer, NormalizedFrame};
pub use hardware_governor::{HardwareGovernor, HardwareProfile, HwInferenceAction};
pub use fs_crawl::FsCrawlIngestor;
pub use chebyshev_trajectory::{fit_chebyshev, evaluate_chebyshev};
pub use video_stream::{VideoStreamIngestor, VideoStreamConfig, VideoFrameDelta};

// Re-export from substrate for convenience
pub use crate::substrate::{IngestionDataChunk, IngestionSourceType, ScreenCoordinate};
