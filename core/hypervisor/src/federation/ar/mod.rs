// AR/VR stub
use async_trait::async_trait;

pub mod types;
pub use types::{FormFactor, ViewConfiguration, ArError, ArSessionState, ArSystemInfo};

#[async_trait]
pub trait ArProvider: Send + Sync {
    fn initialize(&mut self) -> Result<(), ArError>;
    fn get_session_state(&self) -> ArSessionState;
    fn get_system_info(&self) -> ArSystemInfo;

    fn is_runtime_available(&self) -> bool { false }

    fn system_info(&self) -> Result<ArSystemInfo, ArError> { Err(ArError::NoRuntime) }

    async fn begin_session(&self) -> Result<(), ArError> { Err(ArError::FeatureNotEnabled) }

    async fn end_session(&self) -> Result<(), ArError> { Err(ArError::FeatureNotEnabled) }

    async fn session_state(&self) -> ArSessionState {
        ArSessionState::Idle
    }
}

#[cfg(feature = "ar-openxr")]
pub mod openxr_provider;
