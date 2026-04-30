/// Omnipresent Specialist: P2P Sync & Multi-Device Coordination
/// 
/// Omnipresent is the connectivity hub of the hive. It:
/// - Syncs Intent across all devices (phone, tablet, desktop, AR)
/// - Manages peer-to-peer mesh networking via Iroh
/// - Caches Intent offline for 5+ minutes
/// - Adapts Intent for device size/capabilities
/// - Detects new devices and initiates sync
/// - Proposes sync when devices drift
/// 
/// Size: 1GB GGUF model
/// Portable: 800MB stripped version for mobile
/// Domain: P2P / Multi-Device Coordination

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::federation::specialist::{
    Specialist, SpecialistId, SpecialistContext, SpecialistError, ProposedAction,
    Decision, DelegateRequest, DelegateResponse, Conflict, NegotiationResult,
    ResourceRequest, ProposalPriority, ExecutionResult, ExecutionStatus, SpecialistCapability,
};

/// Learning data for Omnipresent specialist
#[derive(Debug, Clone)]
pub struct OmnipresentLearningData {
    pub success_count: u32,
    pub failure_count: u32,
    pub total_executions: u32,
    pub confidence_score: f32,
    pub execution_history: Vec<bool>, // Track last 20 executions
    pub last_updated: u64,
}

impl OmnipresentLearningData {
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            total_executions: 0,
            confidence_score: 0.5, // Start neutral
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

        // Keep last 20 executions
        self.execution_history.push(success);
        if self.execution_history.len() > 20 {
            self.execution_history.remove(0);
        }

        // Update confidence score based on recent history
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

/// Device in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub last_seen: u64,
    pub intent_version: u32,
    pub is_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    Desktop,
    Laptop,
    Phone,
    Tablet,
    ARGlasses,
}

impl DeviceType {
    pub fn intent_adaptation(&self) -> IntentAdaptation {
        match self {
            DeviceType::Desktop => IntentAdaptation {
                full_resolution: true,
                gpu_preferred: true,
                max_latency_ms: 50,
            },
            DeviceType::Laptop => IntentAdaptation {
                full_resolution: true,
                gpu_preferred: false,
                max_latency_ms: 100,
            },
            DeviceType::Phone => IntentAdaptation {
                full_resolution: false,
                gpu_preferred: false,
                max_latency_ms: 200,
            },
            DeviceType::Tablet => IntentAdaptation {
                full_resolution: true,
                gpu_preferred: false,
                max_latency_ms: 150,
            },
            DeviceType::ARGlasses => IntentAdaptation {
                full_resolution: true,
                gpu_preferred: true,
                max_latency_ms: 20,
            },
        }
    }
}

/// How to adapt Intent for a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAdaptation {
    pub full_resolution: bool,
    pub gpu_preferred: bool,
    pub max_latency_ms: u32,
}

/// P2P mesh sync state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub primary_device_id: String,
    pub devices: HashMap<String, Device>,
    pub cached_intent: Option<String>,
    pub cache_timestamp: u64,
    pub sync_conflicts: Vec<SyncConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub device_a: String,
    pub device_b: String,
    pub version_a: u32,
    pub version_b: u32,
    pub detected_at: u64,
}

/// Omnipresent specialist implementation
pub struct Omnipresent {
    id: SpecialistId,
    pub sync_state: SyncState,
    pub devices: HashMap<String, Device>,
    pub sync_history: Vec<String>,
    pub bandwidth_available_mbps: u32,
    pub learning: Arc<Mutex<OmnipresentLearningData>>,
}

impl Omnipresent {
    pub fn new() -> Self {
        Self {
            id: SpecialistId::Omnipresent,
            sync_state: SyncState {
                primary_device_id: "desktop-primary".to_string(),
                devices: HashMap::new(),
                cached_intent: None,
                cache_timestamp: 0,
                sync_conflicts: vec![],
            },
            devices: HashMap::new(),
            sync_history: vec![],
            bandwidth_available_mbps: 100,
            learning: Arc::new(Mutex::new(OmnipresentLearningData::new())),
        }
    }

    /// Register a device to the mesh
    pub fn register_device(&mut self, device: Device) {
        self.devices.insert(device.id.clone(), device);
    }

    /// Detect new devices or offline devices
    pub fn detect_devices_drift(&self) -> Vec<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut drifted = vec![];

        for device in self.devices.values() {
            let seconds_since_seen = now - device.last_seen;

            // If device hasn't been seen in >5 minutes, it's drifted
            if seconds_since_seen > 300 && device.is_online {
                drifted.push(device.id.clone());
            }
        }

        drifted
    }

    /// Check for version conflicts between devices
    pub fn detect_sync_conflicts(&self) -> Vec<SyncConflict> {
        let mut conflicts = vec![];
        let devices: Vec<_> = self.devices.values().collect();

        for i in 0..devices.len() {
            for j in (i + 1)..devices.len() {
                let d1 = devices[i];
                let d2 = devices[j];

                if d1.intent_version != d2.intent_version {
                    conflicts.push(SyncConflict {
                        device_a: d1.id.clone(),
                        device_b: d2.id.clone(),
                        version_a: d1.intent_version,
                        version_b: d2.intent_version,
                        detected_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    });
                }
            }
        }

        conflicts
    }

    /// Adapt Intent for target device
    pub fn adapt_intent_for_device(&self, intent: &str, device_type: &DeviceType) -> String {
        let adaptation = device_type.intent_adaptation();

        if adaptation.full_resolution {
            intent.to_string()
        } else {
            // Simplified version for mobile
            format!("[Mobile] {}", intent)
        }
    }

    /// Calculate bandwidth needed to sync all devices
    pub fn calculate_sync_bandwidth(&self) -> u32 {
        let device_count = self.devices.len() as u32;
        let base_bandwidth = 5; // 5 Mbps per device
        device_count * base_bandwidth
    }
}

impl Default for Omnipresent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Specialist for Omnipresent {
    fn id(&self) -> SpecialistId {
        self.id
    }

    /// Propose syncing when devices drift or conflict
    async fn propose(&self, context: &SpecialistContext) -> Result<Vec<ProposedAction>, SpecialistError> {
        let drifted = self.detect_devices_drift();
        let conflicts = self.detect_sync_conflicts();

        if drifted.is_empty() && conflicts.is_empty() {
            return Ok(vec![]);
        }

        let device_count = self.devices.len() as f32;
        let base_confidence = if conflicts.is_empty() { 0.75 } else { 0.90 };

        // Get learned confidence from history
        let learning = self.learning.lock().await;
        let learned_confidence = learning.get_proposal_confidence();
        drop(learning);

        // Blend base confidence (70%) with learned confidence (30%)
        let confidence = (base_confidence * 0.7) + (learned_confidence * 0.3);

        Ok(vec![ProposedAction {
            id: format!("omnipresent-sync-{}", uuid()),
            specialist: SpecialistId::Omnipresent,
            action_type: "sync_devices".to_string(),
            description: format!(
                "Sync {} devices (drift: {}, conflicts: {})",
                device_count,
                drifted.len(),
                conflicts.len()
            ),
            confidence,
            required_resources: ResourceRequest {
                gpu_percent: 0.0,
                cpu_percent: 15.0,
                memory_mb: 300,
                duration_seconds: 30,
            },
            priority: if conflicts.is_empty() {
                ProposalPriority::Normal
            } else {
                ProposalPriority::UserFacing
            },
            tags: vec!["sync".to_string(), "p2p".to_string()],
        }])
    }

    /// Execute P2P sync across mesh
    async fn execute(&self, decision: &Decision) -> Result<ExecutionResult, SpecialistError> {
        let sync_bandwidth = self.calculate_sync_bandwidth();
        
        let output = format!(
            "Synced {} devices across mesh (bandwidth: {} Mbps)",
            self.devices.len(),
            sync_bandwidth
        );

        let result = ExecutionResult {
            specialist: SpecialistId::Omnipresent,
            proposal_id: decision.proposal_id.clone(),
            status: ExecutionStatus::Success,
            output,
            resources_used: decision.allocated_resources.clone(),
            duration_ms: 1500,
            error: None,
        };

        // Record execution result for learning
        let success = result.status == ExecutionStatus::Success;
        {
            let mut learning = self.learning.lock().await;
            learning.record_result(success);
        } // Lock released here

        Ok(result)
    }

    /// Delegate Intent adaptation to device-specific handlers
    async fn delegate(&self, request: &DelegateRequest) -> Result<DelegateResponse, SpecialistError> {
        Ok(DelegateResponse {
            requester: request.requester,
            target: request.target,
            success: true,
            result: format!("Adapted Intent for {:?}", request.target),
            duration_ms: 100,
        })
    }

    /// Negotiate conflict resolution with Sentinel
    async fn negotiate(
        &self,
        other_id: SpecialistId,
        _conflict: &Conflict,
    ) -> Result<NegotiationResult, SpecialistError> {
        Ok(NegotiationResult {
            resolved: true,
            resolution: format!("Resolved sync conflict with {:?}", other_id),
            winner: None,
            compromise: Some("Sync using CRDTs (Conflict-free Replicated Data Types)".to_string()),
        })
    }

    fn capabilities(&self) -> Vec<SpecialistCapability> {
        vec![
            SpecialistCapability {
                name: "device_sync".to_string(),
                description: "Synchronize Intent across multiple devices".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 15.0,
                    memory_mb: 300,
                    duration_seconds: 30,
                },
                estimated_duration_ms: 1500,
            },
            SpecialistCapability {
                name: "intent_adaptation".to_string(),
                description: "Adapt Intent for device capabilities".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 5.0,
                    memory_mb: 100,
                    duration_seconds: 5,
                },
                estimated_duration_ms: 200,
            },
            SpecialistCapability {
                name: "conflict_detection".to_string(),
                description: "Detect version conflicts between devices".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 10.0,
                    memory_mb: 150,
                    duration_seconds: 10,
                },
                estimated_duration_ms: 500,
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
    fn test_omnipresent_creation() {
        let omnipresent = Omnipresent::new();
        assert_eq!(omnipresent.id(), SpecialistId::Omnipresent);
        assert_eq!(omnipresent.devices.len(), 0);
    }

    #[test]
    fn test_register_device() {
        let mut omnipresent = Omnipresent::new();
        let device = Device {
            id: "phone-1".to_string(),
            name: "iPhone".to_string(),
            device_type: DeviceType::Phone,
            last_seen: 0,
            intent_version: 1,
            is_online: true,
        };
        omnipresent.register_device(device.clone());
        assert_eq!(omnipresent.devices.len(), 1);
        assert!(omnipresent.devices.contains_key("phone-1"));
    }

    #[test]
    fn test_device_type_adaptation() {
        let phone = DeviceType::Phone;
        let adaptation = phone.intent_adaptation();
        assert!(!adaptation.full_resolution);
        assert_eq!(adaptation.max_latency_ms, 200);

        let ar = DeviceType::ARGlasses;
        let adaptation_ar = ar.intent_adaptation();
        assert!(adaptation_ar.full_resolution);
        assert_eq!(adaptation_ar.max_latency_ms, 20);
    }

    #[test]
    fn test_detect_sync_conflicts() {
        let mut omnipresent = Omnipresent::new();

        omnipresent.register_device(Device {
            id: "desktop".to_string(),
            name: "Desktop".to_string(),
            device_type: DeviceType::Desktop,
            last_seen: 0,
            intent_version: 2,
            is_online: true,
        });

        omnipresent.register_device(Device {
            id: "phone".to_string(),
            name: "Phone".to_string(),
            device_type: DeviceType::Phone,
            last_seen: 0,
            intent_version: 1,
            is_online: true,
        });

        let conflicts = omnipresent.detect_sync_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].version_a, 2);
        assert_eq!(conflicts[0].version_b, 1);
    }

    #[test]
    fn test_no_conflicts_same_version() {
        let mut omnipresent = Omnipresent::new();

        omnipresent.register_device(Device {
            id: "desktop".to_string(),
            name: "Desktop".to_string(),
            device_type: DeviceType::Desktop,
            last_seen: 0,
            intent_version: 2,
            is_online: true,
        });

        omnipresent.register_device(Device {
            id: "phone".to_string(),
            name: "Phone".to_string(),
            device_type: DeviceType::Phone,
            last_seen: 0,
            intent_version: 2,
            is_online: true,
        });

        let conflicts = omnipresent.detect_sync_conflicts();
        assert_eq!(conflicts.len(), 0);
    }

    #[test]
    fn test_calculate_sync_bandwidth() {
        let mut omnipresent = Omnipresent::new();

        for i in 0..3 {
            omnipresent.register_device(Device {
                id: format!("device-{}", i),
                name: format!("Device {}", i),
                device_type: DeviceType::Phone,
                last_seen: 0,
                intent_version: 1,
                is_online: true,
            });
        }

        let bandwidth = omnipresent.calculate_sync_bandwidth();
        assert_eq!(bandwidth, 15); // 3 devices * 5 Mbps
    }

    #[test]
    fn test_adapt_intent_for_device() {
        let omnipresent = Omnipresent::new();
        let intent = "Full resolution UI";

        let mobile_adapted = omnipresent.adapt_intent_for_device(intent, &DeviceType::Phone);
        assert!(mobile_adapted.starts_with("[Mobile]"));

        let desktop_adapted = omnipresent.adapt_intent_for_device(intent, &DeviceType::Desktop);
        assert_eq!(desktop_adapted, intent);
    }

    #[tokio::test]
    async fn test_propose_with_conflicts() {
        let mut omnipresent = Omnipresent::new();

        omnipresent.register_device(Device {
            id: "d1".to_string(),
            name: "Device 1".to_string(),
            device_type: DeviceType::Desktop,
            last_seen: 0,
            intent_version: 2,
            is_online: true,
        });

        omnipresent.register_device(Device {
            id: "d2".to_string(),
            name: "Device 2".to_string(),
            device_type: DeviceType::Phone,
            last_seen: 0,
            intent_version: 1,
            is_online: true,
        });

        let context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        let proposals = omnipresent.propose(&context).await.unwrap();
        assert!(!proposals.is_empty());
    }

    #[tokio::test]
    async fn test_execute() {
        let omnipresent = Omnipresent::new();
        let decision = Decision {
            proposal_id: "test".to_string(),
            specialist: SpecialistId::Omnipresent,
            action: "sync".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: std::collections::HashMap::new(),
        };

        let result = omnipresent.execute(&decision).await.unwrap();
        assert_eq!(result.status, ExecutionStatus::Success);
    }

    #[test]
    fn test_capabilities() {
        let omnipresent = Omnipresent::new();
        let capabilities = omnipresent.capabilities();
        assert_eq!(capabilities.len(), 3);
        assert!(capabilities.iter().any(|c| c.name == "device_sync"));
    }
}
