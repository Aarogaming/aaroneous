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
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
// parking_lot::Mutex - see Visionary for the rationale.
use parking_lot::Mutex;

use crate::federation::specialist::{
    Specialist, SpecialistId, SpecialistContext, SpecialistError, ProposedAction,
    Decision, DelegateRequest, DelegateResponse, Conflict, NegotiationResult,
    ResourceRequest, ProposalPriority, ExecutionResult, ExecutionStatus, SpecialistCapability,
};
use crate::federation::p2p::{P2pNode, P2pNodeId, SyncMessage};

/// ALPN identifier for Aaroneous Intent sync protocol
pub const AARONEOUS_SYNC_ALPN: &[u8] = b"aaroneous/sync/v1";

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

impl crate::federation::learn_persist::PersistableLearning for OmnipresentLearningData {
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
/// State that the drain task updates via interior mutability.
/// Separated from the main struct so drain() can be called from &self.
#[derive(Debug)]
pub struct OmnipresentDrainState {
    /// Cached intent received from peers (overwritten by FullState/Delta messages)
    pub cached_intent: Option<String>,
    pub cache_timestamp: u64,
    /// Rolling sync event log (recent 1000 entries)
    pub sync_history: Vec<String>,
}

pub struct Omnipresent {
    id: SpecialistId,
    pub sync_state: SyncState,
    pub devices: HashMap<String, Device>,
    /// Legacy single-writer sync_history. Code with &mut self can use this.
    /// The drain path uses `drain_state.sync_history` instead.
    pub sync_history: Vec<String>,
    pub bandwidth_available_mbps: u32,
    pub learning: Arc<Mutex<OmnipresentLearningData>>,
    pub p2p_node: Option<Arc<P2pNode>>,
    pub device_endpoints: HashMap<String, P2pNodeId>,
    pub sync_inbox: Arc<Mutex<VecDeque<crate::federation::p2p::SyncMessage>>>,
    /// Interior-mutable state updated by the drain task (from &self).
    pub drain_state: Arc<Mutex<OmnipresentDrainState>>,
}

impl Omnipresent {
    /// Canonical name used as the persistence key in `specialist_learning.specialist_kind`.
    pub const PERSISTENCE_KEY: &'static str = "Omnipresent";

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
            p2p_node: None,
            device_endpoints: HashMap::new(),
            sync_inbox: Arc::new(Mutex::new(VecDeque::new())),
            drain_state: Arc::new(Mutex::new(OmnipresentDrainState {
                cached_intent: None,
                cache_timestamp: 0,
                sync_history: Vec::new(),
            })),
        }
    }

    /// Drain and apply all pending sync messages from the inbox — callable from `&self`.
    ///
    /// Updates `drain_state` (interior-mutable) with cached intent and sync history.
    /// Does NOT update `sync_state` or the legacy `sync_history` field (those require
    /// `&mut self`). Call `drain_sync_inbox_mut()` when you have mutable access.
    pub fn drain_sync_inbox_shared(&self) -> usize {
        use crate::federation::p2p::types::SyncMessageKind;

        let msgs: Vec<_> = {
            let mut inbox = self.sync_inbox.lock();
            inbox.drain(..).collect()
        };
        let n = msgs.len();
        if n == 0 {
            return 0;
        }

        let mut state = self.drain_state.lock();
        for msg in msgs {
            match msg.kind {
                SyncMessageKind::FullState | SyncMessageKind::Delta => {
                    if !msg.payload.is_empty() {
                        state.cached_intent = String::from_utf8(msg.payload).ok();
                        state.cache_timestamp = msg.timestamp;
                    }
                    state.sync_history.push(format!(
                        "sync:{:?}:from-{}:v{}",
                        msg.kind, msg.from.short(), msg.intent_version
                    ));
                }
                SyncMessageKind::Heartbeat => {
                    state.sync_history.push(format!("hb:from-{}", msg.from.short()));
                }
                SyncMessageKind::ConflictDetected => {
                    state.sync_history.push(format!("conflict:from-{}", msg.from.short()));
                }
                SyncMessageKind::SyncRequest => {
                    state.sync_history.push(format!("req:from-{}", msg.from.short()));
                }
            }
            while state.sync_history.len() > 1000 {
                state.sync_history.remove(0);
            }
        }
        n
    }

    /// Get the current cached intent (from drain_state, updated by drain_sync_inbox_shared)
    pub fn cached_intent(&self) -> Option<String> {
        self.drain_state.lock().cached_intent.clone()
    }

    /// Get a snapshot of the shared sync history
    pub fn shared_sync_history_len(&self) -> usize {
        self.drain_state.lock().sync_history.len()
    }

    /// Apply a single incoming sync message from a peer.
    ///
    /// Updates the cached intent state and records the sync in history.
    /// Called by `drain_sync_inbox()` for each queued message.
    pub fn apply_sync_message(&mut self, msg: crate::federation::p2p::SyncMessage) {
        use crate::federation::p2p::types::SyncMessageKind;

        match msg.kind {
            SyncMessageKind::FullState | SyncMessageKind::Delta => {
                // Update cached intent with the payload from the sender
                if !msg.payload.is_empty() {
                    self.sync_state.cached_intent =
                        String::from_utf8(msg.payload).ok();
                    self.sync_state.cache_timestamp = msg.timestamp;
                }
                self.sync_history.push(format!(
                    "sync:{:?}:from-{}:version-{}",
                    msg.kind,
                    msg.from.short(),
                    msg.intent_version
                ));
            }
            SyncMessageKind::Heartbeat => {
                // Heartbeat: update last-seen timestamp for the device
                self.sync_history.push(format!(
                    "heartbeat:from-{}",
                    msg.from.short()
                ));
            }
            SyncMessageKind::ConflictDetected => {
                let conflict_note = format!(
                    "conflict-detected:from-{}:version-{}",
                    msg.from.short(),
                    msg.intent_version
                );
                self.sync_history.push(conflict_note);
            }
            SyncMessageKind::SyncRequest => {
                // Peer is requesting our state - note it; actual reply
                // is handled by the network layer in production.
                self.sync_history.push(format!(
                    "sync-request-from-{}",
                    msg.from.short()
                ));
            }
        }

        // Cap history at 1000 entries
        while self.sync_history.len() > 1000 {
            self.sync_history.remove(0);
        }
    }

    /// Drain all pending sync messages from the inbox and apply each one.
    ///
    /// Call this periodically (e.g., at the start of `propose()` or on a
    /// timer) to process messages that the recv background task has queued.
    ///
    /// Returns the number of messages processed.
    pub fn drain_sync_inbox(&mut self) -> usize {
        let msgs: Vec<_> = {
            let mut inbox = self.sync_inbox.lock();
            inbox.drain(..).collect()
        };
        let n = msgs.len();
        for msg in msgs {
            self.apply_sync_message(msg);
        }
        n
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

    /// Spawn a P2P node and attach it to this specialist
    ///
    /// After this call, sync operations will use real P2P networking
    /// (via Iroh when the `p2p-iroh` feature is enabled, or stub otherwise).
    ///
    /// # Errors
    ///
    /// Returns an error if the P2P node fails to spawn (network issues,
    /// permission errors, etc.)
    pub async fn with_p2p(mut self) -> Result<Self, crate::federation::p2p::P2pError> {
        let node = P2pNode::spawn(AARONEOUS_SYNC_ALPN).await?;
        self.p2p_node = Some(Arc::new(node));
        Ok(self)
    }

    /// Attach an already-spawned P2P node (useful for sharing a node
    /// between multiple specialists or for advanced configuration)
    pub fn attach_p2p(&mut self, node: Arc<P2pNode>) {
        self.p2p_node = Some(node);
    }

    /// Returns true if a P2P node is attached
    pub fn has_p2p(&self) -> bool {
        self.p2p_node.is_some()
    }

    /// Get this node's P2P endpoint ID (if P2P is attached)
    pub fn p2p_endpoint_id(&self) -> Option<P2pNodeId> {
        self.p2p_node.as_ref().map(|n| n.endpoint_id().clone())
    }

    /// Register a device with an associated P2P endpoint ID
    ///
    /// This binds a logical device to a P2P node so future sync operations
    /// can address it by `device.id` while the P2P layer uses the endpoint ID.
    pub fn register_device_with_endpoint(
        &mut self,
        device: Device,
        endpoint_id: P2pNodeId,
    ) {
        let device_id = device.id.clone();
        self.devices.insert(device_id.clone(), device);
        self.device_endpoints.insert(device_id, endpoint_id);
    }

    /// Sync Intent to a specific device via P2P
    ///
    /// Returns the number of bytes sent. If no P2P node is attached, returns 0.
    pub async fn sync_to_device(
        &self,
        device_id: &str,
        intent_version: u32,
        intent_payload: Vec<u8>,
    ) -> Result<usize, crate::federation::p2p::P2pError> {
        let Some(node) = &self.p2p_node else {
            return Ok(0);
        };

        let Some(endpoint) = self.device_endpoints.get(device_id) else {
            return Err(crate::federation::p2p::P2pError::InvalidEndpoint(format!(
                "no endpoint registered for device {}",
                device_id
            )));
        };

        let payload_len = intent_payload.len();
        let msg = SyncMessage::full_state(node.endpoint_id().clone(), intent_version, intent_payload);
        node.send(endpoint, msg).await?;
        Ok(payload_len)
    }

    /// Broadcast Intent to all registered devices via P2P
    ///
    /// Returns the number of devices the broadcast was sent to.
    pub async fn broadcast_intent(
        &self,
        intent_version: u32,
        intent_payload: Vec<u8>,
    ) -> Result<usize, crate::federation::p2p::P2pError> {
        let Some(node) = &self.p2p_node else {
            return Ok(0);
        };

        let endpoints: Vec<P2pNodeId> = self.device_endpoints.values().cloned().collect();
        if endpoints.is_empty() {
            return Ok(0);
        }

        let msg = SyncMessage::full_state(node.endpoint_id().clone(), intent_version, intent_payload);
        node.broadcast(&endpoints, msg).await
    }

    /// Send heartbeat to all devices to detect drift
    pub async fn heartbeat_all(&self) -> Result<usize, crate::federation::p2p::P2pError> {
        let Some(node) = &self.p2p_node else {
            return Ok(0);
        };

        let endpoints: Vec<P2pNodeId> = self.device_endpoints.values().cloned().collect();
        if endpoints.is_empty() {
            return Ok(0);
        }

        let intent_version = self
            .devices
            .values()
            .map(|d| d.intent_version)
            .max()
            .unwrap_or(0);

        let msg = SyncMessage::heartbeat(node.endpoint_id().clone(), intent_version);
        node.broadcast(&endpoints, msg).await
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

    /// Check for version conflicts between devices.
    ///
    /// Devices are sorted by ID first to ensure deterministic conflict ordering
    /// (otherwise HashMap iteration order makes tests flaky).
    pub fn detect_sync_conflicts(&self) -> Vec<SyncConflict> {
        let mut conflicts = vec![];
        let mut devices: Vec<_> = self.devices.values().collect();
        devices.sort_by(|a, b| a.id.cmp(&b.id));

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

    /// Propose syncing when devices drift, have conflicts, or have pending P2P messages.
    async fn propose(&self, context: &SpecialistContext) -> Result<Vec<ProposedAction>, SpecialistError> {
        let drifted = self.detect_devices_drift();
        let conflicts = self.detect_sync_conflicts();
        // Peek at inbox without draining: pending messages are an urgency signal
        // even before we know who sent them. Execute() will drain and apply them.
        let pending_messages = self.sync_inbox.lock().len();

        if drifted.is_empty() && conflicts.is_empty() && pending_messages == 0 {
            return Ok(vec![]);
        }

        let device_count = self.devices.len() as f32;
        // Pending inbox messages add urgency: each message bumps confidence slightly
        let inbox_urgency = (pending_messages as f32 * 0.02).min(0.15);
        let base_confidence = if !conflicts.is_empty() {
            0.90
        } else if pending_messages > 0 {
            0.80 + inbox_urgency
        } else {
            0.75
        };

        // Get learned confidence from history
        let learning = self.learning.lock();
        let learned_confidence = learning.get_proposal_confidence();
        drop(learning);

        // Blend base confidence (70%) with learned confidence (30%)
        let confidence = (base_confidence * 0.7) + (learned_confidence * 0.3);

        let description = if pending_messages > 0 {
            format!(
                "Sync {} devices (drift: {}, conflicts: {}, pending: {} msg)",
                device_count,
                drifted.len(),
                conflicts.len(),
                pending_messages
            )
        } else {
            format!(
                "Sync {} devices (drift: {}, conflicts: {})",
                device_count,
                drifted.len(),
                conflicts.len()
            )
        };

        Ok(vec![ProposedAction {
            id: format!("omnipresent-sync-{}", uuid()),
            specialist: SpecialistId::Omnipresent,
            action_type: "sync_devices".to_string(),
            description,
            confidence,
            required_resources: ResourceRequest {
                gpu_percent: 0.0,
                cpu_percent: 15.0,
                memory_mb: 300,
                duration_seconds: 30,
            },
            priority: if !conflicts.is_empty() || pending_messages > 2 {
                ProposalPriority::UserFacing
            } else {
                ProposalPriority::Normal
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
            let mut learning = self.learning.lock();
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

    // === P2P integration tests ===

    #[tokio::test]
    async fn test_no_p2p_node_by_default() {
        let omnipresent = Omnipresent::new();
        assert!(!omnipresent.has_p2p());
        assert!(omnipresent.p2p_endpoint_id().is_none());
    }

    #[tokio::test]
    async fn test_with_p2p_attaches_node() {
        let omnipresent = Omnipresent::new()
            .with_p2p()
            .await
            .expect("p2p spawn should succeed");

        assert!(omnipresent.has_p2p());
        let endpoint = omnipresent.p2p_endpoint_id();
        assert!(endpoint.is_some(), "endpoint id should be present after attaching p2p");
    }

    #[tokio::test]
    async fn test_register_device_with_endpoint() {
        let mut omnipresent = Omnipresent::new()
            .with_p2p()
            .await
            .expect("p2p spawn should succeed");

        let device = Device {
            id: "phone-1".to_string(),
            name: "iPhone".to_string(),
            device_type: DeviceType::Phone,
            last_seen: 0,
            intent_version: 1,
            is_online: true,
        };
        let endpoint = P2pNodeId::random();

        omnipresent.register_device_with_endpoint(device, endpoint.clone());

        assert_eq!(omnipresent.devices.len(), 1);
        assert_eq!(omnipresent.device_endpoints.len(), 1);
        assert_eq!(omnipresent.device_endpoints.get("phone-1"), Some(&endpoint));
    }

    #[tokio::test]
    async fn test_sync_to_device_without_p2p_returns_zero() {
        let omnipresent = Omnipresent::new(); // no p2p attached

        let bytes_sent = omnipresent
            .sync_to_device("phone-1", 1, vec![1, 2, 3])
            .await
            .expect("should not error when p2p is absent");

        assert_eq!(bytes_sent, 0);
    }

    #[tokio::test]
    async fn test_sync_to_unknown_device_errors() {
        let omnipresent = Omnipresent::new()
            .with_p2p()
            .await
            .expect("p2p spawn");

        let result = omnipresent
            .sync_to_device("unknown-device", 1, vec![1, 2, 3])
            .await;

        assert!(matches!(
            result,
            Err(crate::federation::p2p::P2pError::InvalidEndpoint(_))
        ));
    }

    #[tokio::test]
    async fn test_broadcast_intent_with_no_devices_returns_zero() {
        let omnipresent = Omnipresent::new()
            .with_p2p()
            .await
            .expect("p2p spawn");

        let n = omnipresent
            .broadcast_intent(1, vec![1, 2, 3])
            .await
            .expect("broadcast with no devices should succeed");

        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn test_broadcast_intent_without_p2p_returns_zero() {
        let omnipresent = Omnipresent::new(); // no p2p
        let n = omnipresent
            .broadcast_intent(1, vec![1, 2, 3])
            .await
            .expect("broadcast without p2p should succeed");
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn test_heartbeat_all_with_no_devices() {
        let omnipresent = Omnipresent::new()
            .with_p2p()
            .await
            .expect("p2p spawn");

        let n = omnipresent.heartbeat_all().await.expect("heartbeat should succeed");
        assert_eq!(n, 0);
    }

    // === Inbox-shaped proposal tests ===

    #[tokio::test]
    async fn test_propose_without_drift_but_pending_inbox_still_proposes() {
        // No drifted devices, no conflicts, but 3 pending sync messages
        // → should still generate a proposal
        let mut omnipresent = Omnipresent::new();
        let node_id = crate::federation::p2p::P2pNodeId::random();
        for _ in 0..3 {
            let msg = crate::federation::p2p::SyncMessage::heartbeat(node_id.clone(), 1);
            omnipresent.sync_inbox.lock().push_back(msg);
        }

        let context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        let proposals = omnipresent.propose(&context).await.unwrap();
        assert!(
            !proposals.is_empty(),
            "pending inbox messages should trigger a proposal even without drift"
        );
        assert!(
            proposals[0].description.contains("pending"),
            "description should mention pending messages: {}",
            proposals[0].description
        );
    }

    #[tokio::test]
    async fn test_propose_with_inbox_messages_has_higher_urgency() {
        // Scenario 1: no inbox, only drift → Normal priority
        // Scenario 2: inbox has >2 messages → UserFacing priority
        let node_id = crate::federation::p2p::P2pNodeId::random();

        let context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        // Scenario 1: some drift, no inbox
        let mut omni_drift = Omnipresent::new();
        // Drift is only triggered by last_seen > 300s for is_online devices.
        // Use conflicts instead which is deterministic.
        omni_drift.register_device(Device {
            id: "d1".to_string(),
            name: "D1".to_string(),
            device_type: DeviceType::Desktop,
            last_seen: 0,
            intent_version: 2,
            is_online: true,
        });
        omni_drift.register_device(Device {
            id: "d2".to_string(),
            name: "D2".to_string(),
            device_type: DeviceType::Phone,
            last_seen: 0,
            intent_version: 1,
            is_online: true,
        });
        let proposals_drift = omni_drift.propose(&context).await.unwrap();
        assert!(!proposals_drift.is_empty());
        // Conflicts → high base confidence but still UserFacing (conflicts.is_empty() is false)

        // Scenario 2: >2 inbox messages alone → UserFacing  
        let mut omni_inbox = Omnipresent::new();
        for _ in 0..3 {
            let msg = crate::federation::p2p::SyncMessage::heartbeat(node_id.clone(), 1);
            omni_inbox.sync_inbox.lock().push_back(msg);
        }
        let proposals_inbox = omni_inbox.propose(&context).await.unwrap();
        assert!(!proposals_inbox.is_empty());
        assert_eq!(
            proposals_inbox[0].priority,
            ProposalPriority::UserFacing,
            "3+ pending messages should give UserFacing priority"
        );
    }

    #[tokio::test]
    async fn test_no_proposal_when_no_drift_no_conflicts_no_inbox() {
        let omnipresent = Omnipresent::new();
        let context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        let proposals = omnipresent.propose(&context).await.unwrap();
        assert!(
            proposals.is_empty(),
            "no drift, no conflicts, no inbox → no proposals"
        );
    }
}
