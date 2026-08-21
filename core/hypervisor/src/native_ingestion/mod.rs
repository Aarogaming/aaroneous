pub mod chebyshev_trajectory;
pub mod fractional_normalizer;
pub mod fs_crawl;
pub mod hardware_governor;
pub mod shmem_capture;
pub mod simd_xor_delta;
pub mod svd_feature_select;
pub mod video_stream;

pub use chebyshev_trajectory::{evaluate_chebyshev, fit_chebyshev};
pub use fractional_normalizer::{FractionalNormalizer, NormalizedFrame};
pub use fs_crawl::FsCrawlIngestor;
pub use hardware_governor::{HardwareGovernor, HardwareProfile, HwInferenceAction};
pub use shmem_capture::{FrameCaptureConfig, ShmemCapture};
pub use simd_xor_delta::{DeltaReport, XorDeltaConfig, XorDeltaScreen};
pub use svd_feature_select::{ReducedFeatures, SvdConfig, SvdReducer};
pub use video_stream::{VideoFrameDelta, VideoStreamConfig, VideoStreamIngestor};

// Re-export from substrate for convenience
pub use crate::substrate::{IngestionDataChunk, IngestionSourceType, ScreenCoordinate};
