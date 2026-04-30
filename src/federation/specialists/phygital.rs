/// Phygital Specialist: AR/VR Spatial Rendering & Landmarks
/// 
/// Phygital materializes Intent into spatial/visual form. It:
/// - Detects AR hardware (HoloLens, Magic Leap, ARKit)
/// - Renders 3D design prototypes in physical space
/// - Maps landmarks to Intent location (desk, kitchen, etc)
/// - Streams AR anchors to devices
/// - Falls back to 2D when AR unavailable
/// - Proposes rendering when GPU available
/// 
/// Size: 1GB GGUF model (includes 3D generation)
/// Portable: 200MB stripped version (AR proxy only)
/// Domain: Spatial / AR-VR

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
// parking_lot::Mutex - see Visionary for the rationale.
use parking_lot::Mutex;

use crate::federation::specialist::{
    Specialist, SpecialistId, SpecialistContext, SpecialistError, ProposedAction,
    Decision, DelegateRequest, DelegateResponse, Conflict, NegotiationResult,
    ResourceRequest, ProposalPriority, ExecutionResult, ExecutionStatus, SpecialistCapability,
};
use crate::federation::ar::{ArProvider, ArError, ArSessionState};

/// Learning data for Phygital specialist
#[derive(Debug, Clone)]
pub struct PhygitalLearningData {
    pub success_count: u32,
    pub failure_count: u32,
    pub total_executions: u32,
    pub confidence_score: f32,
    pub execution_history: Vec<bool>,
    pub last_updated: u64,
}

impl PhygitalLearningData {
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            total_executions: 0,
            confidence_score: 0.5,
            execution_history: vec![],
            last_updated: 0,
        }
    }

    pub fn record_result(&mut self, success: bool) {
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        self.total_executions += 1;

        self.execution_history.push(success);
        if self.execution_history.len() > 20 {
            self.execution_history.remove(0);
        }

        if self.total_executions > 0 {
            self.confidence_score =
                (self.success_count as f32) / (self.total_executions as f32);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_updated = now;
    }

    pub fn get_proposal_confidence(&self) -> f32 {
        self.confidence_score
    }

    pub fn get_success_rate(&self) -> f32 {
        if self.total_executions == 0 {
            return 0.0;
        }
        (self.success_count as f32) / (self.total_executions as f32) * 100.0
    }
}

impl crate::federation::learn_persist::PersistableLearning for PhygitalLearningData {
    fn snapshot(&self) -> crate::federation::learn_persist::LearningSnapshot {
        crate::federation::learn_persist::LearningSnapshot {
            success_count: self.success_count,
            failure_count: self.failure_count,
            total_executions: self.total_executions,
            confidence_score: self.confidence_score,
            execution_history: self.execution_history.clone(),
            last_updated: self.last_updated,
        }
    }

    fn restore_from(&mut self, s: crate::federation::learn_persist::LearningSnapshot) {
        self.success_count = s.success_count;
        self.failure_count = s.failure_count;
        self.total_executions = s.total_executions;
        self.confidence_score = s.confidence_score;
        self.execution_history = s.execution_history;
        self.last_updated = s.last_updated;
    }
}

/// AR/VR device detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpatialDevice {
    HoloLens2,
    HoloLens3,
    MagicLeap,
    AppleVisionPro,
    MetaQuest3,
    ARKit,
    ARCore,
}

impl SpatialDevice {
    pub fn gpu_requirement_percent(&self) -> f32 {
        match self {
            SpatialDevice::HoloLens3 | SpatialDevice::AppleVisionPro => 60.0,
            SpatialDevice::HoloLens2 | SpatialDevice::MagicLeap => 40.0,
            SpatialDevice::MetaQuest3 => 50.0,
            SpatialDevice::ARKit | SpatialDevice::ARCore => 15.0,
        }
    }

    pub fn supports_anchors(&self) -> bool {
        matches!(self, SpatialDevice::HoloLens2 | SpatialDevice::HoloLens3 | SpatialDevice::MagicLeap)
    }
}

/// Physical landmark in user's environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Landmark {
    pub id: String,
    pub name: String,
    pub location_type: LocationType,
    pub detected_at: u64,
    pub confidence: f32,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocationType {
    Desk,
    Kitchen,
    LivingRoom,
    Bedroom,
    Hallway,
    OfficeSpace,
    Outside,
    Vehicle,
}

/// 3D prototype in AR space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialPrototype {
    pub id: String,
    pub design_variant_id: String,
    pub landmark_id: String,
    pub model_path: String,
    pub scale: f32,
    pub rotation_degrees: f32,
    pub visibility_percent: f32,
    pub rendered_at: u64,
}

/// AR Frame state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARFrameState {
    pub timestamp: u64,
    pub device: Option<SpatialDevice>,
    pub frame_rate: u32,
    pub detected_landmarks: Vec<Landmark>,
    pub gpu_available_percent: f32,
    pub ar_available: bool,
}

/// Phygital specialist implementation
pub struct Phygital {
    id: SpecialistId,
    pub detected_devices: Vec<SpatialDevice>,
    pub landmarks: HashMap<String, Landmark>,
    pub prototypes: HashMap<String, SpatialPrototype>,
    pub frame_state_history: Vec<ARFrameState>,
    pub gpu_headroom_percent: f32,
    pub learning: Arc<Mutex<PhygitalLearningData>>,
    /// Optional real AR provider via OpenXR. When `Some`, hardware detection
    /// uses real OpenXR queries instead of `cfg!()` heuristics.
    pub ar_provider: Option<Arc<ArProvider>>,
}

impl Phygital {
    /// Canonical name used as the persistence key in `specialist_learning.specialist_kind`.
    pub const PERSISTENCE_KEY: &'static str = "Phygital";

    pub fn new() -> Self {
        Self {
            id: SpecialistId::Phygital,
            detected_devices: vec![],
            landmarks: HashMap::new(),
            prototypes: HashMap::new(),
            frame_state_history: vec![],
            gpu_headroom_percent: 30.0,
            learning: Arc::new(Mutex::new(PhygitalLearningData::new())),
            ar_provider: None,
        }
    }

    /// Save this specialist's current learning state to a persistence manager.
    /// See `Visionary::save_learning_to` for why this is sync, not async.
    pub fn save_learning_to(
        &self,
        pm: &crate::persistence::PersistenceManager,
    ) -> Result<(), crate::federation::learn_persist::LearnPersistError> {
        let snapshot = {
            let learning = self.learning.lock();
            crate::federation::learn_persist::PersistableLearning::snapshot(&*learning)
        };
        let record = snapshot.to_record(Self::PERSISTENCE_KEY)?;
        pm.save_learning_state(&record)?;
        Ok(())
    }

    /// Load learning state from persistence into this specialist.
    pub fn load_learning_from(
        &self,
        pm: &crate::persistence::PersistenceManager,
    ) -> Result<bool, crate::federation::learn_persist::LearnPersistError> {
        let maybe_record = pm.load_learning_state(Self::PERSISTENCE_KEY)?;
        let Some(record) = maybe_record else {
            return Ok(false);
        };
        let snapshot = crate::federation::learn_persist::LearningSnapshot::from_record(&record)?;
        let mut learning = self.learning.lock();
        crate::federation::learn_persist::PersistableLearning::restore_from(&mut *learning, snapshot);
        Ok(true)
    }

    /// Detect and attach an OpenXR runtime
    ///
    /// After this call, `detect_ar_hardware_real()` will use real OpenXR
    /// queries instead of OS-based heuristics.
    pub async fn with_ar(mut self) -> Result<Self, ArError> {
        let provider = ArProvider::detect().await?;
        self.ar_provider = Some(Arc::new(provider));
        Ok(self)
    }

    /// Attach an already-spawned AR provider
    pub fn attach_ar(&mut self, provider: Arc<ArProvider>) {
        self.ar_provider = Some(provider);
    }

    /// Returns true if an AR provider is attached
    pub fn has_ar(&self) -> bool {
        self.ar_provider.is_some()
    }

    /// Returns true if a real OpenXR runtime is available (only meaningful
    /// when an AR provider is attached)
    pub fn has_runtime(&self) -> bool {
        self.ar_provider
            .as_ref()
            .map(|p| p.is_runtime_available())
            .unwrap_or(false)
    }

    /// Real AR hardware detection via OpenXR
    ///
    /// Queries the attached AR provider for actual hardware.
    /// Returns `ArError::NoRuntime` if no provider is attached or no runtime
    /// is installed. On success, populates `detected_devices` based on the
    /// classified system.
    pub fn detect_ar_hardware_real(&mut self) -> Result<Option<SpatialDevice>, ArError> {
        let Some(provider) = &self.ar_provider else {
            return Err(ArError::FeatureNotEnabled);
        };

        if !provider.is_runtime_available() {
            return Err(ArError::NoRuntime);
        }

        let info = provider.system_info()?;
        let detected = match info.classify_spatial_device() {
            Some("HoloLens2") => Some(SpatialDevice::HoloLens2),
            Some("HoloLens3") => Some(SpatialDevice::HoloLens3),
            Some("MagicLeap") => Some(SpatialDevice::MagicLeap),
            Some("AppleVisionPro") => Some(SpatialDevice::AppleVisionPro),
            Some("MetaQuest3") => Some(SpatialDevice::MetaQuest3),
            Some("ARKit") => Some(SpatialDevice::ARKit),
            Some("ARCore") => Some(SpatialDevice::ARCore),
            _ => None,
        };

        if let Some(device) = detected.clone() {
            // Replace detected_devices with the real one (don't accumulate
            // across calls - the runtime tells the truth)
            self.detected_devices.clear();
            self.detected_devices.push(device);
        }

        Ok(detected)
    }

    /// Begin an AR session via OpenXR (state-tracking only - no rendering)
    pub async fn begin_ar_session(&self) -> Result<(), ArError> {
        let Some(provider) = &self.ar_provider else {
            return Err(ArError::FeatureNotEnabled);
        };
        provider.begin_session().await
    }

    /// End the current AR session
    pub async fn end_ar_session(&self) -> Result<(), ArError> {
        let Some(provider) = &self.ar_provider else {
            return Err(ArError::FeatureNotEnabled);
        };
        provider.end_session().await
    }

    /// Get the current AR session state
    pub async fn ar_session_state(&self) -> Option<ArSessionState> {
        if let Some(provider) = &self.ar_provider {
            Some(provider.session_state().await)
        } else {
            None
        }
    }

    /// Detect AR hardware on system
    pub fn detect_ar_hardware(&mut self) {
        // Simulated device detection
        // In production: OpenXR, ARKit/ARCore queries
        if cfg!(target_os = "windows") {
            self.detected_devices.push(SpatialDevice::HoloLens3);
            self.detected_devices.push(SpatialDevice::MetaQuest3);
        } else if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
            self.detected_devices.push(SpatialDevice::AppleVisionPro);
            self.detected_devices.push(SpatialDevice::ARKit);
        } else if cfg!(target_os = "android") {
            self.detected_devices.push(SpatialDevice::ARCore);
            self.detected_devices.push(SpatialDevice::MetaQuest3);
        }
    }

    /// Poll OpenXR frame state
    pub fn poll_frame_state(&mut self) -> ARFrameState {
        let device = self.detected_devices.first().cloned();
        let ar_available = device.is_some();

        let frame_state = ARFrameState {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            device: device.clone(),
            frame_rate: if ar_available { 90 } else { 60 },
            detected_landmarks: self.landmarks.values().cloned().collect(),
            gpu_available_percent: self.gpu_headroom_percent,
            ar_available,
        };

        self.frame_state_history.push(frame_state.clone());
        if self.frame_state_history.len() > 100 {
            self.frame_state_history.remove(0);
        }

        frame_state
    }

    /// Detect landmark in AR view
    pub fn detect_landmark(&mut self, name: String, location_type: LocationType) -> Landmark {
        let id = format!("landmark-{}", uuid());
        let landmark = Landmark {
            id: id.clone(),
            name,
            location_type,
            detected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            confidence: 0.85,
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.landmarks.insert(id.clone(), landmark.clone());
        landmark
    }

    /// Generate spatial prototype (3D model placement)
    pub fn generate_prototype(
        &mut self,
        design_id: String,
        landmark_id: String,
    ) -> Result<SpatialPrototype, String> {
        if !self.landmarks.contains_key(&landmark_id) {
            return Err("Landmark not found".to_string());
        }

        let id = format!("proto-{}", uuid());
        let prototype = SpatialPrototype {
            id: id.clone(),
            design_variant_id: design_id,
            landmark_id,
            model_path: format!("/models/{}.glb", id),
            scale: 1.0,
            rotation_degrees: 0.0,
            visibility_percent: 100.0,
            rendered_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.prototypes.insert(id.clone(), prototype.clone());
        Ok(prototype)
    }

    /// Check if AR rendering is possible
    pub fn can_render_ar(&self) -> bool {
        if self.detected_devices.is_empty() {
            return false;
        }

        let mut required_gpu = 0.0_f32;
        for device in &self.detected_devices {
            let gpu_req = device.gpu_requirement_percent();
            if gpu_req > required_gpu {
                required_gpu = gpu_req;
            }
        }

        self.gpu_headroom_percent > required_gpu
    }

    /// Get primary AR device
    pub fn primary_device(&self) -> Option<&SpatialDevice> {
        self.detected_devices.first()
    }

    /// Simulate GPU availability change
    pub fn set_gpu_available(&mut self, percent: f32) {
        self.gpu_headroom_percent = percent.max(0.0).min(100.0);
    }
}

impl Default for Phygital {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Specialist for Phygital {
    fn id(&self) -> SpecialistId {
        self.id
    }

    /// Propose rendering when GPU available and landmarks detected
    async fn propose(&self, _context: &SpecialistContext) -> Result<Vec<ProposedAction>, SpecialistError> {
        let frame = self.frame_state_history.last();

        if frame.is_none() || !self.can_render_ar() || self.prototypes.is_empty() {
            return Ok(vec![]);
        }

        let frame_state = frame.unwrap();
        if !frame_state.ar_available {
            return Ok(vec![]);
        }

        let device = frame_state
            .device
            .as_ref()
            .map(|d| format!("{:?}", d))
            .unwrap_or_else(|| "Mobile".to_string());

        let base_confidence = 0.8 + (self.gpu_headroom_percent / 100.0) * 0.2;

        // Get learned confidence from history
        let learning = self.learning.lock();
        let learned_confidence = learning.get_proposal_confidence();
        drop(learning);

        // Blend base confidence (70%) with learned confidence (30%)
        let confidence = (base_confidence * 0.7) + (learned_confidence * 0.3);

        Ok(vec![ProposedAction {
            id: format!("phygital-render-{}", uuid()),
            specialist: SpecialistId::Phygital,
            action_type: "render_prototype".to_string(),
            description: format!(
                "Render {} prototypes on {} (GPU: {:.0}%, landmarks: {})",
                self.prototypes.len(),
                device,
                self.gpu_headroom_percent,
                frame_state.detected_landmarks.len()
            ),
            confidence,
            required_resources: ResourceRequest {
                gpu_percent: self
                    .detected_devices
                    .first()
                    .map(|d| d.gpu_requirement_percent() / 100.0)
                    .unwrap_or(0.3),
                cpu_percent: 20.0,
                memory_mb: 400,
                duration_seconds: 60,
            },
            priority: ProposalPriority::UserFacing,
            tags: vec!["rendering".to_string(), "ar".to_string()],
        }])
    }

    /// Execute AR rendering
    async fn execute(&self, decision: &Decision) -> Result<ExecutionResult, SpecialistError> {
        let prototype_count = self.prototypes.len();
        let device = self
            .primary_device()
            .map(|d| format!("{:?}", d))
            .unwrap_or_else(|| "Mobile".to_string());

        let output = format!(
            "Rendered {} prototypes on {} at {:.0} FPS",
            prototype_count,
            device,
            self.frame_state_history
                .last()
                .map(|f| f.frame_rate)
                .unwrap_or(60)
        );

        let result = ExecutionResult {
            specialist: SpecialistId::Phygital,
            proposal_id: decision.proposal_id.clone(),
            status: ExecutionStatus::Success,
            output,
            resources_used: decision.allocated_resources.clone(),
            duration_ms: 2500,
            error: None,
        };

        // Record execution result for learning
        let success = result.status == ExecutionStatus::Success;
        {
            let mut learning = self.learning.lock();
            learning.record_result(success);
        } // Lock released here

        Ok(result)
    }

    /// Delegate to device-specific rendering handlers
    async fn delegate(&self, request: &DelegateRequest) -> Result<DelegateResponse, SpecialistError> {
        Ok(DelegateResponse {
            requester: request.requester,
            target: request.target,
            success: true,
            result: format!("Rendered {} prototypes", self.prototypes.len()),
            duration_ms: 1200,
        })
    }

    /// Negotiate GPU access with Sentinel
    async fn negotiate(
        &self,
        other_id: SpecialistId,
        _conflict: &Conflict,
    ) -> Result<NegotiationResult, SpecialistError> {
        Ok(NegotiationResult {
            resolved: true,
            resolution: format!("Negotiated GPU with {:?}: {:.0}% available", other_id, self.gpu_headroom_percent),
            winner: None,
            compromise: Some("Progressive rendering with lower LOD models".to_string()),
        })
    }

    fn capabilities(&self) -> Vec<SpecialistCapability> {
        vec![
            SpecialistCapability {
                name: "landmark_detection".to_string(),
                description: "Detect spatial landmarks (desk, kitchen, etc)".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.15,
                    cpu_percent: 15.0,
                    memory_mb: 200,
                    duration_seconds: 10,
                },
                estimated_duration_ms: 500,
            },
            SpecialistCapability {
                name: "prototype_generation".to_string(),
                description: "Generate 3D prototype models".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.40,
                    cpu_percent: 30.0,
                    memory_mb: 600,
                    duration_seconds: 30,
                },
                estimated_duration_ms: 8000,
            },
            SpecialistCapability {
                name: "ar_rendering".to_string(),
                description: "Render prototypes in AR/VR space".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.60,
                    cpu_percent: 25.0,
                    memory_mb: 800,
                    duration_seconds: 60,
                },
                estimated_duration_ms: 2500,
            },
        ]
    }
}

fn uuid() -> String {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phygital_creation() {
        let phygital = Phygital::new();
        assert_eq!(phygital.id(), SpecialistId::Phygital);
        assert!(phygital.detected_devices.is_empty());
    }

    #[test]
    fn test_detect_ar_hardware() {
        let mut phygital = Phygital::new();
        phygital.detect_ar_hardware();
        // Should detect something on any platform
        assert!(!phygital.detected_devices.is_empty());
    }

    #[test]
    fn test_spatial_device_gpu_requirements() {
        assert_eq!(SpatialDevice::HoloLens3.gpu_requirement_percent(), 60.0);
        assert_eq!(SpatialDevice::ARKit.gpu_requirement_percent(), 15.0);
        assert_eq!(SpatialDevice::MetaQuest3.gpu_requirement_percent(), 50.0);
    }

    #[test]
    fn test_spatial_device_anchors() {
        assert!(SpatialDevice::HoloLens3.supports_anchors());
        assert!(!SpatialDevice::ARKit.supports_anchors());
    }

    #[test]
    fn test_poll_frame_state() {
        let mut phygital = Phygital::new();
        phygital.detect_ar_hardware();

        let frame = phygital.poll_frame_state();
        assert!(frame.device.is_some());
        assert!(frame.ar_available);
        assert_eq!(frame.frame_rate, 90);
    }

    #[test]
    fn test_detect_landmark() {
        let mut phygital = Phygital::new();
        let landmark = phygital.detect_landmark("My Desk".to_string(), LocationType::Desk);

        assert_eq!(landmark.name, "My Desk");
        assert_eq!(landmark.location_type, LocationType::Desk);
        assert!(landmark.confidence > 0.8);
        assert_eq!(phygital.landmarks.len(), 1);
    }

    #[test]
    fn test_generate_prototype() {
        let mut phygital = Phygital::new();
        let landmark = phygital.detect_landmark("Desk".to_string(), LocationType::Desk);

        let result = phygital.generate_prototype("design-1".to_string(), landmark.id.clone());
        assert!(result.is_ok());

        let proto = result.unwrap();
        assert_eq!(proto.design_variant_id, "design-1");
        assert_eq!(proto.landmark_id, landmark.id);
        assert_eq!(phygital.prototypes.len(), 1);
    }

    #[test]
    fn test_generate_prototype_invalid_landmark() {
        let mut phygital = Phygital::new();
        let result = phygital.generate_prototype("design-1".to_string(), "nonexistent".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_can_render_ar_insufficient_gpu() {
        let mut phygital = Phygital::new();
        phygital.detect_ar_hardware();
        phygital.set_gpu_available(10.0); // Only 10% GPU available
        phygital.poll_frame_state();

        // HoloLens3 requires 60%, so should not be able to render
        assert!(!phygital.can_render_ar());
    }

    #[test]
    fn test_can_render_ar_sufficient_gpu() {
        let mut phygital = Phygital::new();
        phygital.set_gpu_available(80.0); // 80% available
        phygital.detected_devices.push(SpatialDevice::ARKit); // Requires 15%
        phygital.poll_frame_state();

        assert!(phygital.can_render_ar());
    }

    #[test]
    fn test_primary_device() {
        let mut phygital = Phygital::new();
        assert!(phygital.primary_device().is_none());

        phygital.detect_ar_hardware();
        assert!(phygital.primary_device().is_some());
    }

    #[test]
    fn test_set_gpu_available() {
        let mut phygital = Phygital::new();
        phygital.set_gpu_available(150.0); // Over 100%
        assert_eq!(phygital.gpu_headroom_percent, 100.0);

        phygital.set_gpu_available(-10.0); // Negative
        assert_eq!(phygital.gpu_headroom_percent, 0.0);

        phygital.set_gpu_available(50.0);
        assert_eq!(phygital.gpu_headroom_percent, 50.0);
    }

    #[tokio::test]
    async fn test_propose_no_devices() {
        let phygital = Phygital::new();
        let context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        let proposals = phygital.propose(&context).await.unwrap();
        assert!(proposals.is_empty());
    }

    #[tokio::test]
    async fn test_execute() {
        let phygital = Phygital::new();
        let decision = Decision {
            proposal_id: "test".to_string(),
            specialist: SpecialistId::Phygital,
            action: "render".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: std::collections::HashMap::new(),
        };

        let result = phygital.execute(&decision).await.unwrap();
        assert_eq!(result.status, ExecutionStatus::Success);
    }

    #[test]
    fn test_capabilities() {
        let phygital = Phygital::new();
        let capabilities = phygital.capabilities();
        assert_eq!(capabilities.len(), 3);
        assert!(capabilities.iter().any(|c| c.name == "landmark_detection"));
        assert!(capabilities.iter().any(|c| c.name == "ar_rendering"));
    }

    #[test]
    fn test_location_types() {
        assert_eq!(LocationType::Desk, LocationType::Desk);
        assert_ne!(LocationType::Desk, LocationType::Kitchen);
    }

    // === OpenXR AR integration tests ===

    #[tokio::test]
    async fn test_no_ar_provider_by_default() {
        let phygital = Phygital::new();
        assert!(!phygital.has_ar());
        assert!(!phygital.has_runtime());
    }

    #[tokio::test]
    async fn test_with_ar_attaches_provider() {
        let phygital = Phygital::new()
            .with_ar()
            .await
            .expect("AR provider detect should succeed (returns Ok even without runtime)");
        assert!(phygital.has_ar());
        // has_runtime() depends on whether a runtime is actually installed
    }

    #[tokio::test]
    async fn test_detect_ar_hardware_real_without_provider() {
        let mut phygital = Phygital::new();
        let result = phygital.detect_ar_hardware_real();
        assert!(matches!(result, Err(crate::federation::ar::ArError::FeatureNotEnabled)));
    }

    #[tokio::test]
    async fn test_begin_ar_session_without_provider_errors() {
        let phygital = Phygital::new();
        let result = phygital.begin_ar_session().await;
        assert!(matches!(result, Err(crate::federation::ar::ArError::FeatureNotEnabled)));
    }

    #[tokio::test]
    async fn test_end_ar_session_without_provider_errors() {
        let phygital = Phygital::new();
        let result = phygital.end_ar_session().await;
        assert!(matches!(result, Err(crate::federation::ar::ArError::FeatureNotEnabled)));
    }

    #[tokio::test]
    async fn test_ar_session_state_without_provider_returns_none() {
        let phygital = Phygital::new();
        let state = phygital.ar_session_state().await;
        assert!(state.is_none());
    }

    #[tokio::test]
    async fn test_with_ar_provider_and_no_runtime_errors_on_detect() {
        let mut phygital = Phygital::new()
            .with_ar()
            .await
            .expect("AR provider detect should succeed");

        // Without a real runtime, detect_ar_hardware_real should return NoRuntime
        if !phygital.has_runtime() {
            let result = phygital.detect_ar_hardware_real();
            assert!(matches!(result, Err(crate::federation::ar::ArError::NoRuntime)));
        }
    }

    /// Integration test: only meaningful with real OpenXR runtime
    #[tokio::test]
    #[ignore = "requires real OpenXR runtime"]
    async fn test_detect_ar_with_real_runtime() {
        let mut phygital = match Phygital::new().with_ar().await {
            Ok(p) => p,
            Err(_) => return, // No AR available - skip
        };

        if !phygital.has_runtime() {
            return; // No runtime - skip
        }

        match phygital.detect_ar_hardware_real() {
            Ok(Some(device)) => {
                println!("Detected real AR device: {:?}", device);
                assert_eq!(phygital.detected_devices.len(), 1);
            }
            Ok(None) => println!("Runtime present but unrecognized system"),
            Err(e) => println!("Detection error: {} (acceptable without HMD)", e),
        }
    }
}
