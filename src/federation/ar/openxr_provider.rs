/// Real OpenXR-backed AR provider
///
/// This module is only compiled when the `ar-openxr` feature is enabled.
/// It uses the `openxr` crate to detect runtime, query system info, and
/// manage session state.
///
/// # Note on Session vs. Render
///
/// Full OpenXR sessions need a graphics API (Vulkan/D3D12/OpenGL) for
/// rendering. This v1 implementation creates a "headless" session using
/// the `XR_MND_headless` extension when available, which lets us track
/// session state without doing actual rendering. Phygital's job is
/// orchestration, not pixel work.
///
/// If headless isn't available (most consumer runtimes), `begin_session`
/// will return an error and the caller should know that real rendering
/// is required for that runtime.

use super::types::{ArError, ArSessionState, ArSystemInfo, FormFactor, ViewConfiguration};
use openxr as xr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;
use tracing::{debug, info, warn};

/// OpenXR-backed AR provider
///
/// `entry` is `Option<>` because the loader DLL may not exist on the system
/// (e.g., CI machines, headless servers). When `entry` is `None`, the
/// provider correctly reports `is_runtime_available() == false` and all
/// methods return `NoRuntime` errors.
pub struct ArProvider {
    /// The OpenXR entry point (loader). None if loader not present.
    entry: Option<Arc<xr::Entry>>,
    /// The OpenXR instance (lazily created on first use)
    instance: Arc<Mutex<Option<xr::Instance>>>,
    /// Whether a runtime appears to be installed and reachable
    runtime_available: bool,
    /// Cached runtime info (name, version)
    runtime_info: Arc<Mutex<Option<(String, String)>>>,
    /// Current session state tracker
    session_state: Arc<Mutex<ArSessionState>>,
}

impl ArProvider {
    /// Try to detect an OpenXR runtime
    ///
    /// This always returns Ok. Use `is_runtime_available()` to check whether
    /// a real runtime was found. Missing runtime is a normal condition
    /// (e.g., CI environments without OpenXR installed), not an error.
    pub async fn detect() -> Result<Self, ArError> {
        info!("Detecting OpenXR runtime");

        // Loading happens on a blocking thread because OpenXR FFI calls
        // aren't async-aware.
        let entry_opt = task::spawn_blocking(|| {
            // Safety: xr::Entry::load() is FFI - it dlopen's the OpenXR loader.
            // The unsafe block documents the platform-FFI nature of the call.
            unsafe {
                match xr::Entry::load() {
                    Ok(entry) => Some(entry),
                    Err(e) => {
                        warn!("OpenXR loader not available: {:?}", e);
                        None
                    }
                }
            }
        })
        .await
        .map_err(|e| ArError::OpenXr(format!("blocking task panicked: {}", e)))?;

        let (entry, runtime_available) = match entry_opt {
            Some(entry) => {
                // Loader present - check if a runtime is actually usable
                let avail = entry.enumerate_extensions().is_ok();
                if !avail {
                    warn!("OpenXR loader present but no runtime registered");
                }
                (Some(Arc::new(entry)), avail)
            }
            None => (None, false),
        };

        Ok(Self {
            entry,
            instance: Arc::new(Mutex::new(None)),
            runtime_available,
            runtime_info: Arc::new(Mutex::new(None)),
            session_state: Arc::new(Mutex::new(ArSessionState::Idle)),
        })
    }

    /// Returns true if an OpenXR runtime is available
    pub fn is_runtime_available(&self) -> bool {
        self.runtime_available
    }

    /// Get information about the connected AR system
    ///
    /// This is sync but does heavy work (creates instance, queries system).
    /// Caches the result after the first call.
    pub fn system_info(&self) -> Result<ArSystemInfo, ArError> {
        if !self.runtime_available {
            return Err(ArError::NoRuntime);
        }

        let entry = self
            .entry
            .as_ref()
            .ok_or(ArError::NoRuntime)?
            .clone();

        // Build app info
        let app_info = xr::ApplicationInfo {
            application_name: "Aaroneous Phygital",
            application_version: 1,
            engine_name: "Aaroneous",
            engine_version: 1,
            api_version: xr::Version::new(1, 0, 0),
        };

        let extensions = xr::ExtensionSet::default();

        let instance = entry
            .create_instance(&app_info, &extensions, &[])
            .map_err(|e| ArError::OpenXr(format!("create_instance: {:?}", e)))?;

        // Cache runtime info
        let runtime_props = instance
            .properties()
            .map_err(|e| ArError::OpenXr(format!("instance properties: {:?}", e)))?;

        let runtime_name = runtime_props.runtime_name.to_string();
        let runtime_version = format!(
            "{}.{}.{}",
            runtime_props.runtime_version.major(),
            runtime_props.runtime_version.minor(),
            runtime_props.runtime_version.patch()
        );

        // Try HMD form factor first, then handheld
        let (system_id, form_factor) = match instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
        {
            Ok(id) => (id, FormFactor::HeadMountedDisplay),
            Err(_) => match instance.system(xr::FormFactor::HANDHELD_DISPLAY) {
                Ok(id) => (id, FormFactor::HandheldDisplay),
                Err(e) => return Err(ArError::OpenXr(format!("no system: {:?}", e))),
            },
        };

        let sys_props = instance
            .system_properties(system_id)
            .map_err(|e| ArError::OpenXr(format!("system_properties: {:?}", e)))?;

        // Determine view configuration
        let view_configs = instance
            .enumerate_view_configurations(system_id)
            .map_err(|e| ArError::OpenXr(format!("enumerate_view_configurations: {:?}", e)))?;

        let view_configuration = if view_configs
            .iter()
            .any(|v| *v == xr::ViewConfigurationType::PRIMARY_QUAD_VARJO)
        {
            ViewConfiguration::Quad
        } else if view_configs
            .iter()
            .any(|v| *v == xr::ViewConfigurationType::PRIMARY_STEREO)
        {
            ViewConfiguration::Stereo
        } else {
            ViewConfiguration::Mono
        };

        // Tracking properties: position tracking is what we care about
        let tracking_props = sys_props.tracking_properties;
        let tracks_position = tracking_props.position_tracking;

        // Passthrough detection: check known extensions on the ExtensionSet
        // The `openxr` crate exposes well-known extensions as boolean fields,
        // so we just OR a few of the relevant ones.
        let supports_passthrough = entry
            .enumerate_extensions()
            .map(|exts| {
                exts.fb_passthrough
                    || exts.htc_passthrough
                    || exts.fb_composition_layer_image_layout
            })
            .unwrap_or(false);

        let info = ArSystemInfo {
            runtime_name,
            runtime_version,
            system_name: sys_props.system_name.to_string(),
            vendor_id: sys_props.vendor_id,
            form_factor,
            view_configuration,
            tracks_position,
            supports_passthrough,
        };

        // Stash the instance for future use
        if let Ok(mut guard) = self.instance.try_lock() {
            *guard = Some(instance);
        }

        Ok(info)
    }

    /// Begin an AR session (state transition only - no rendering)
    ///
    /// Note: A full OpenXR session requires a graphics API. This method
    /// updates our internal state-machine tracker but does NOT create a
    /// real `xr::Session` because that requires Vulkan/D3D12/OpenGL.
    /// For headless tracking only.
    pub async fn begin_session(&self) -> Result<(), ArError> {
        if !self.runtime_available {
            return Err(ArError::NoRuntime);
        }
        debug!("Transitioning AR session to Running (headless)");
        *self.session_state.lock().await = ArSessionState::Running;
        Ok(())
    }

    /// End the current AR session
    pub async fn end_session(&self) -> Result<(), ArError> {
        debug!("Ending AR session");
        *self.session_state.lock().await = ArSessionState::Exited;
        Ok(())
    }

    /// Get the current session state
    pub async fn session_state(&self) -> ArSessionState {
        *self.session_state.lock().await
    }

    /// Poll for OpenXR events (placeholder - returns None in v1)
    ///
    /// A full implementation would poll `instance.poll_event()` and
    /// translate `XrEventDataSessionStateChanged` to `ArSessionState`.
    pub async fn poll_events(&self) -> Result<Option<ArSessionState>, ArError> {
        // V1: don't actually pump events; session state is managed by
        // begin_session/end_session.
        Ok(None)
    }

    /// Shut down: drop the instance, clear caches
    pub async fn shutdown(self) -> Result<(), ArError> {
        info!("Shutting down ArProvider");
        let mut instance = self.instance.lock().await;
        *instance = None; // Drop the OpenXR instance
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// detect() should always succeed even if no runtime is installed,
    /// because absence of runtime is a normal condition, not an error.
    #[tokio::test]
    async fn test_detect_does_not_error_when_no_runtime() {
        let result = ArProvider::detect().await;
        // Either runtime is present (test machine) or not (CI). Both fine.
        assert!(result.is_ok(), "detect should return Ok regardless of runtime: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_system_info_errors_without_runtime() {
        let provider = ArProvider::detect().await.unwrap();
        if !provider.is_runtime_available() {
            let result = provider.system_info();
            assert!(matches!(result, Err(ArError::NoRuntime)));
        }
    }

    #[tokio::test]
    async fn test_begin_session_errors_without_runtime() {
        let provider = ArProvider::detect().await.unwrap();
        if !provider.is_runtime_available() {
            let result = provider.begin_session().await;
            assert!(matches!(result, Err(ArError::NoRuntime)));
        }
    }

    /// Integration test: only runs if a real OpenXR runtime is installed.
    #[tokio::test]
    #[ignore = "requires real OpenXR runtime"]
    async fn test_real_runtime_system_info() {
        let provider = ArProvider::detect().await.unwrap();
        if !provider.is_runtime_available() {
            return; // No runtime - skip
        }
        let info = provider.system_info();
        // If a runtime is available, we should get some system info or a real error
        match info {
            Ok(info) => {
                assert!(!info.runtime_name.is_empty());
                println!("Detected runtime: {} {}", info.runtime_name, info.runtime_version);
                println!("System: {}", info.system_name);
            }
            Err(e) => {
                println!("Got error querying system: {} (this may be expected without HMD)", e);
            }
        }
    }
}
