/// Stub AR provider (no-op, used when `ar-openxr` feature is disabled)
///
/// This implementation:
/// - Provides the same API as the real OpenXR-backed provider
/// - Records calls for testing
/// - Returns canned data instead of real OpenXR queries
/// - Allows tests to run without a real OpenXR runtime

use super::types::{ArError, ArSessionState, ArSystemInfo};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;

/// Stub AR provider - records operations but does no real OpenXR
pub struct ArProvider {
    /// Whether to claim a runtime is available (default: false in stub)
    pub runtime_available: bool,
    /// Canned system info to return from `system_info()` - uses std Mutex
    /// because `system_info()` is sync (matches the real OpenXR provider)
    pub canned_system_info: Arc<StdMutex<Option<ArSystemInfo>>>,
    /// Current session state
    pub session_state: Arc<Mutex<ArSessionState>>,
    /// Records of operations performed (for test inspection)
    pub call_log: Arc<Mutex<Vec<StubCall>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StubCall {
    Detect,
    SystemInfo,
    BeginSession,
    EndSession,
    PollEvents,
}

impl ArProvider {
    /// "Detect" an OpenXR runtime
    ///
    /// In stub mode, returns a provider that reports `is_runtime_available() == false`
    /// by default. Tests can override this.
    pub async fn detect() -> Result<Self, ArError> {
        let provider = Self {
            runtime_available: false,
            canned_system_info: Arc::new(StdMutex::new(None)),
            session_state: Arc::new(Mutex::new(ArSessionState::Idle)),
            call_log: Arc::new(Mutex::new(vec![StubCall::Detect])),
        };
        Ok(provider)
    }

    /// Test helper: configure the stub to report a runtime as available
    pub async fn set_runtime_available(&mut self, available: bool) {
        self.runtime_available = available;
    }

    /// Test helper: set canned system info
    pub fn set_canned_system_info(&self, info: ArSystemInfo) {
        *self.canned_system_info.lock().expect("std mutex poisoned") = Some(info);
    }

    /// Returns true if an OpenXR runtime is available
    pub fn is_runtime_available(&self) -> bool {
        self.runtime_available
    }

    /// Get information about the connected AR system
    pub fn system_info(&self) -> Result<ArSystemInfo, ArError> {
        if !self.runtime_available {
            return Err(ArError::NoRuntime);
        }

        let canned = self.canned_system_info.lock().expect("std mutex poisoned");
        if let Some(info) = canned.as_ref() {
            Ok(info.clone())
        } else {
            Err(ArError::NoHmd)
        }
    }

    /// Begin an AR session
    pub async fn begin_session(&self) -> Result<(), ArError> {
        self.call_log.lock().await.push(StubCall::BeginSession);
        if !self.runtime_available {
            return Err(ArError::NoRuntime);
        }
        *self.session_state.lock().await = ArSessionState::Running;
        Ok(())
    }

    /// End the current AR session
    pub async fn end_session(&self) -> Result<(), ArError> {
        self.call_log.lock().await.push(StubCall::EndSession);
        *self.session_state.lock().await = ArSessionState::Exited;
        Ok(())
    }

    /// Get the current session state
    pub async fn session_state(&self) -> ArSessionState {
        *self.session_state.lock().await
    }

    /// Poll for OpenXR events; returns the new session state if it changed
    pub async fn poll_events(&self) -> Result<Option<ArSessionState>, ArError> {
        self.call_log.lock().await.push(StubCall::PollEvents);
        Ok(None)
    }

    #[cfg(test)]
    pub async fn calls(&self) -> Vec<StubCall> {
        self.call_log.lock().await.clone()
    }

    /// Shut down the provider gracefully
    pub async fn shutdown(self) -> Result<(), ArError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quest_info() -> ArSystemInfo {
        ArSystemInfo {
            runtime_name: "Stub Runtime".to_string(),
            runtime_version: "1.0".to_string(),
            system_name: "Meta Quest 3".to_string(),
            vendor_id: 0x2833,
            form_factor: FormFactor::HeadMountedDisplay,
            view_configuration: ViewConfiguration::Stereo,
            tracks_position: true,
            supports_passthrough: true,
        }
    }

    #[tokio::test]
    async fn test_stub_detect() {
        let provider = ArProvider::detect().await.unwrap();
        assert!(!provider.is_runtime_available()); // Default
    }

    #[tokio::test]
    async fn test_stub_runtime_available_after_set() {
        let mut provider = ArProvider::detect().await.unwrap();
        provider.set_runtime_available(true).await;
        assert!(provider.is_runtime_available());
    }

    #[tokio::test]
    async fn test_stub_system_info_no_runtime() {
        let provider = ArProvider::detect().await.unwrap();
        let result = provider.system_info();
        assert!(matches!(result, Err(ArError::NoRuntime)));
    }

    #[tokio::test]
    async fn test_stub_system_info_no_hmd() {
        let mut provider = ArProvider::detect().await.unwrap();
        provider.set_runtime_available(true).await;
        let result = provider.system_info();
        assert!(matches!(result, Err(ArError::NoHmd)));
    }

    #[tokio::test]
    async fn test_stub_system_info_returns_canned() {
        let mut provider = ArProvider::detect().await.unwrap();
        provider.set_runtime_available(true).await;
        provider.set_canned_system_info(quest_info());

        let info = provider.system_info().unwrap();
        assert_eq!(info.system_name, "Meta Quest 3");
        assert_eq!(info.classify_spatial_device(), Some("MetaQuest3"));
    }

    #[tokio::test]
    async fn test_stub_begin_session_no_runtime() {
        let provider = ArProvider::detect().await.unwrap();
        let result = provider.begin_session().await;
        assert!(matches!(result, Err(ArError::NoRuntime)));
    }

    #[tokio::test]
    async fn test_stub_session_lifecycle() {
        let mut provider = ArProvider::detect().await.unwrap();
        provider.set_runtime_available(true).await;

        assert_eq!(provider.session_state().await, ArSessionState::Idle);

        provider.begin_session().await.unwrap();
        assert_eq!(provider.session_state().await, ArSessionState::Running);

        provider.end_session().await.unwrap();
        assert_eq!(provider.session_state().await, ArSessionState::Exited);
    }

    #[tokio::test]
    async fn test_stub_poll_events_returns_none() {
        let provider = ArProvider::detect().await.unwrap();
        let result = provider.poll_events().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_stub_records_calls() {
        let mut provider = ArProvider::detect().await.unwrap();
        provider.set_runtime_available(true).await;
        provider.begin_session().await.unwrap();
        provider.poll_events().await.unwrap();
        provider.end_session().await.unwrap();

        let calls = provider.calls().await;
        assert!(calls.contains(&StubCall::Detect));
        assert!(calls.contains(&StubCall::BeginSession));
        assert!(calls.contains(&StubCall::PollEvents));
        assert!(calls.contains(&StubCall::EndSession));
    }
}
