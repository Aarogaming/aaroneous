/// AR/VR Module: Real OpenXR Integration for Phygital Specialist
///
/// This module bridges the Phygital specialist's abstract spatial model to
/// real OpenXR runtimes (Monado, SteamVR, Meta Link, Windows Mixed Reality,
/// etc.) via the `openxr` crate.
///
/// # Feature Gating
///
/// Real OpenXR support is gated behind the `ar-openxr` feature. Without it,
/// a stub provider is used so tests and development continue to work.
///
/// # Scope of v1
///
/// This integration provides:
/// - OpenXR runtime detection (is one installed?)
/// - System enumeration (what HMD is available?)
/// - Session lifecycle (begin / end session for state tracking)
/// - Reference space creation (local, stage)
///
/// It does NOT provide actual frame rendering, which would require a full
/// graphics pipeline (Vulkan/D3D12/OpenGL). That's deliberately kept separate
/// because:
/// 1. The pixel work belongs in a renderer, not a specialist
/// 2. Phygital orchestrates *what* to render, not *how*
/// 3. Tests can verify orchestration logic without a real GPU
///
/// # Usage
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use a_run::federation::ar::ArProvider;
///
/// let provider = ArProvider::detect().await?;
/// if provider.is_runtime_available() {
///     let info = provider.system_info()?;
///     println!("HMD: {} from {}", info.system_name, info.runtime_name);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Platform Support
///
/// The `openxr` crate supports any platform with an OpenXR 1.0+ runtime:
/// - Windows: Meta Link, SteamVR, WMR, Varjo Base, Pico Connect
/// - Linux: Monado, SteamVR, Envision
/// - Android: Meta Quest, Pico, ByteDance, Magic Leap

pub mod types;

#[cfg(feature = "ar-openxr")]
pub mod openxr_provider;

#[cfg(not(feature = "ar-openxr"))]
pub mod stub_provider;

pub use types::{ArError, ArSystemInfo, ArSessionState, FormFactor, ViewConfiguration};

#[cfg(feature = "ar-openxr")]
pub use openxr_provider::ArProvider;

#[cfg(not(feature = "ar-openxr"))]
pub use stub_provider::ArProvider;
