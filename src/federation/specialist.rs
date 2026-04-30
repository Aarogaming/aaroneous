/// Core Specialist Trait and Types
/// 
/// Every entity in the Aaroneous hive is a Specialist:
/// - Can propose actions asynchronously
/// - Can execute decisions when assigned
/// - Can delegate work to other specialists
/// - Can negotiate with peers to resolve conflicts
/// - Has its own compact GGUF model (0.5-2GB)

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a specialist in the hive
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum SpecialistId {
    Sentinel,
    Visionary,
    Omnipresent,
    Symbiotic,
    Phygital,
    Archivist,
}

impl SpecialistId {
    pub fn name(&self) -> &'static str {
        match self {
            SpecialistId::Sentinel => "Sentinel",
            SpecialistId::Visionary => "Visionary",
            SpecialistId::Omnipresent => "Omnipresent",
            SpecialistId::Symbiotic => "Symbiotic",
            SpecialistId::Phygital => "Phygital",
            SpecialistId::Archivist => "Archivist",
        }
    }

    pub fn model_size_mb(&self) -> u32 {
        match self {
            SpecialistId::Sentinel => 2000,
            SpecialistId::Visionary => 1000,
            SpecialistId::Omnipresent => 1000,
            SpecialistId::Symbiotic => 500,
            SpecialistId::Phygital => 1000,
            SpecialistId::Archivist => 500,
        }
    }

    pub fn is_core(&self) -> bool {
        true // All 6 are core specialists
    }
}

/// Configuration for a specialist instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistConfig {
    pub id: SpecialistId,
    pub enabled: bool,
    pub model_path: String,
    pub max_concurrent_proposals: usize,
    pub proposal_timeout_ms: u64,
    pub execution_timeout_ms: u64,
    pub memory_limit_mb: u32,
}

impl SpecialistConfig {
    pub fn default_for(id: SpecialistId) -> Self {
        Self {
            id,
            enabled: true,
            model_path: format!("/models/{}.gguf", id.name().to_lowercase()),
            max_concurrent_proposals: 10,
            proposal_timeout_ms: 5000,
            execution_timeout_ms: 30000,
            memory_limit_mb: id.model_size_mb(),
        }
    }
}

/// The core trait that all specialists implement
/// 
/// A specialist is:
/// - Autonomous: can propose actions without being asked
/// - Delegatable: can execute decisions from Sentinel
/// - Collaborative: can negotiate with other specialists
/// - Transparent: decisions are logged and auditable
#[async_trait]
pub trait Specialist: Send + Sync {
    /// Returns the specialist's unique ID
    fn id(&self) -> SpecialistId;

    /// Propose actions based on current context
    /// 
    /// Called asynchronously when the specialist has ideas.
    /// Returns multiple proposals ranked by confidence.
    /// Does NOT require Sentinel's permission (bottom-up signal).
    async fn propose(&self, context: &SpecialistContext) -> Result<Vec<ProposedAction>, SpecialistError>;

    /// Execute a decision assigned by Sentinel
    /// 
    /// Called when Sentinel decides this specialist should act.
    /// Specialist commits to the decision and reports results (top-down signal).
    /// REVIVED: Specialists use interior mutability (Mutex) to learn from results
    async fn execute(&self, decision: &Decision) -> Result<ExecutionResult, SpecialistError>;

    /// Delegate work to another specialist
    /// 
    /// Called when this specialist needs help from a peer.
    /// Can happen without Sentinel involvement (lateral signal).
    async fn delegate(&self, request: &DelegateRequest) -> Result<DelegateResponse, SpecialistError>;

    /// Negotiate a conflict with another specialist
    /// 
    /// Called when two specialists have competing proposals.
    /// Both must agree or Sentinel arbitrates.
    async fn negotiate(&self, other_id: SpecialistId, conflict: &Conflict) -> Result<NegotiationResult, SpecialistError>;

    /// Optional: Get specialist's current state for diagnostics
    async fn status(&self) -> Result<SpecialistStatus, SpecialistError> {
        Ok(SpecialistStatus {
            id: self.id(),
            enabled: true,
            load: 0.5,
            last_proposal: None,
            last_execution: None,
            error_count: 0,
        })
    }

    /// Optional: Get specialist's capabilities
    fn capabilities(&self) -> Vec<SpecialistCapability> {
        vec![]
    }
}

/// Context passed to a specialist when proposing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistContext {
    pub timestamp: u64,
    pub user_state: UserState,
    pub system_resources: SystemResources,
    pub active_specialists: Vec<SpecialistId>,
    pub recent_decisions: Vec<String>, // Last N decisions in system
}

/// User state (biometric, activity, focus level, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserState {
    pub stress_level: f32,     // 0.0-1.0
    pub focus_level: f32,      // 0.0-1.0
    pub fatigue_level: f32,    // 0.0-1.0
    pub activity: String,      // "working", "gaming", "idle", etc.
}

impl Default for UserState {
    fn default() -> Self {
        Self {
            stress_level: 0.5,
            focus_level: 0.5,
            fatigue_level: 0.5,
            activity: "idle".to_string(),
        }
    }
}

/// System resource availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    pub gpu_available_percent: f32,    // 0-100
    pub cpu_available_percent: f32,    // 0-100
    pub memory_available_mb: u32,
    pub thermal_headroom: f32,         // 0.0-1.0
}

impl Default for SystemResources {
    fn default() -> Self {
        Self {
            gpu_available_percent: 100.0,
            cpu_available_percent: 100.0,
            memory_available_mb: 8192,
            thermal_headroom: 1.0,
        }
    }
}

/// Action proposed by a specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedAction {
    pub id: String,
    pub specialist: SpecialistId,
    pub action_type: String,
    pub description: String,
    pub confidence: f32,           // 0.0-1.0
    pub required_resources: ResourceRequest,
    pub priority: ProposalPriority,
    pub tags: Vec<String>,
}

/// Resource request for an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequest {
    pub gpu_percent: f32,
    pub cpu_percent: f32,
    pub memory_mb: u32,
    pub duration_seconds: u32,
}

impl Default for ResourceRequest {
    fn default() -> Self {
        Self {
            gpu_percent: 0.0,
            cpu_percent: 10.0,
            memory_mb: 100,
            duration_seconds: 60,
        }
    }
}

/// Priority levels for proposals
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ProposalPriority {
    Background = 0,
    Normal = 1,
    UserFacing = 2,
    Urgent = 3,
}

/// Decision from Sentinel for a specialist to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub proposal_id: String,
    pub specialist: SpecialistId,
    pub action: String,
    pub allocated_resources: ResourceRequest,
    pub deadline_ms: u64,
    pub context: HashMap<String, String>,
}

/// Result of executing a decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub specialist: SpecialistId,
    pub proposal_id: String,
    pub status: ExecutionStatus,
    pub output: String,
    pub resources_used: ResourceRequest,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    PartialSuccess,
    Failed,
    Timeout,
}

/// Request to delegate work to another specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateRequest {
    pub requester: SpecialistId,
    pub target: SpecialistId,
    pub task: String,
    pub context: HashMap<String, String>,
    pub timeout_ms: u64,
}

/// Response from delegated work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateResponse {
    pub requester: SpecialistId,
    pub target: SpecialistId,
    pub success: bool,
    pub result: String,
    pub duration_ms: u64,
}

/// Conflict between two specialists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub specialist_a: SpecialistId,
    pub specialist_b: SpecialistId,
    pub conflict_type: String,
    pub context: HashMap<String, String>,
}

/// Result of negotiation between specialists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationResult {
    pub resolved: bool,
    pub resolution: String,
    pub winner: Option<SpecialistId>,
    pub compromise: Option<String>,
}

/// Specialist status for monitoring and diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistStatus {
    pub id: SpecialistId,
    pub enabled: bool,
    pub load: f32,                      // 0.0-1.0
    pub last_proposal: Option<u64>,     // timestamp
    pub last_execution: Option<u64>,    // timestamp
    pub error_count: u32,
}

/// Capabilities that a specialist offers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistCapability {
    pub name: String,
    pub description: String,
    pub required_resources: ResourceRequest,
    pub estimated_duration_ms: u64,
}

/// Errors that can occur during specialist operations
#[derive(Debug, Clone)]
pub enum SpecialistError {
    Timeout,
    ResourceUnavailable,
    ModelNotLoaded,
    ExecutionFailed(String),
    ConflictResolutionFailed,
    DelegationFailed(String),
}

impl std::fmt::Display for SpecialistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialistError::Timeout => write!(f, "Specialist operation timed out"),
            SpecialistError::ResourceUnavailable => write!(f, "Required resources unavailable"),
            SpecialistError::ModelNotLoaded => write!(f, "Specialist model not loaded"),
            SpecialistError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            SpecialistError::ConflictResolutionFailed => write!(f, "Could not resolve conflict"),
            SpecialistError::DelegationFailed(msg) => write!(f, "Delegation failed: {}", msg),
        }
    }
}

impl std::error::Error for SpecialistError {}

/// Registry of all specialists in the hive
#[derive(Clone)]
pub struct SpecialistRegistry {
    specialists: HashMap<SpecialistId, Arc<dyn Specialist>>,
}

impl SpecialistRegistry {
    pub fn new() -> Self {
        Self {
            specialists: HashMap::new(),
        }
    }

    pub fn register(&mut self, specialist: Arc<dyn Specialist>) {
        let id = specialist.id();
        self.specialists.insert(id, specialist);
    }

    pub fn get(&self, id: SpecialistId) -> Option<Arc<dyn Specialist>> {
        self.specialists.get(&id).cloned()
    }

    pub fn all(&self) -> Vec<Arc<dyn Specialist>> {
        self.specialists.values().cloned().collect()
    }

    pub fn count(&self) -> usize {
        self.specialists.len()
    }
}

impl Default for SpecialistRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialist_id_names() {
        assert_eq!(SpecialistId::Sentinel.name(), "Sentinel");
        assert_eq!(SpecialistId::Visionary.name(), "Visionary");
        assert_eq!(SpecialistId::Omnipresent.name(), "Omnipresent");
    }

    #[test]
    fn test_specialist_model_sizes() {
        assert_eq!(SpecialistId::Sentinel.model_size_mb(), 2000);
        assert_eq!(SpecialistId::Visionary.model_size_mb(), 1000);
        assert_eq!(SpecialistId::Symbiotic.model_size_mb(), 500);
    }

    #[test]
    fn test_total_hive_size() {
        let total: u32 = vec![
            SpecialistId::Sentinel,
            SpecialistId::Visionary,
            SpecialistId::Omnipresent,
            SpecialistId::Symbiotic,
            SpecialistId::Phygital,
            SpecialistId::Archivist,
        ]
        .iter()
        .map(|id| id.model_size_mb())
        .sum();

        assert_eq!(total, 6000); // Full hive = 6GB
    }

    #[test]
    fn test_specialist_config_defaults() {
        let config = SpecialistConfig::default_for(SpecialistId::Visionary);
        assert_eq!(config.id, SpecialistId::Visionary);
        assert!(config.enabled);
        assert_eq!(config.memory_limit_mb, 1000);
    }

    #[test]
    fn test_registry_register_and_get() {
        // Mock specialist for testing
        #[derive(Debug)]
        struct MockSpecialist;

        #[async_trait]
        impl Specialist for MockSpecialist {
            fn id(&self) -> SpecialistId {
                SpecialistId::Sentinel
            }

            async fn propose(&self, _context: &SpecialistContext) -> Result<Vec<ProposedAction>, SpecialistError> {
                Ok(vec![])
            }

            async fn execute(&self, _decision: &Decision) -> Result<ExecutionResult, SpecialistError> {
                Ok(ExecutionResult {
                    specialist: SpecialistId::Sentinel,
                    proposal_id: "test".to_string(),
                    status: ExecutionStatus::Success,
                    output: "ok".to_string(),
                    resources_used: ResourceRequest::default(),
                    duration_ms: 100,
                    error: None,
                })
            }

            async fn delegate(&self, _request: &DelegateRequest) -> Result<DelegateResponse, SpecialistError> {
                Ok(DelegateResponse {
                    requester: SpecialistId::Sentinel,
                    target: SpecialistId::Visionary,
                    success: true,
                    result: "delegated".to_string(),
                    duration_ms: 50,
                })
            }

            async fn negotiate(&self, _other_id: SpecialistId, _conflict: &Conflict) -> Result<NegotiationResult, SpecialistError> {
                Ok(NegotiationResult {
                    resolved: true,
                    resolution: "agreed".to_string(),
                    winner: None,
                    compromise: Some("both agree".to_string()),
                })
            }
        }

        let mut registry = SpecialistRegistry::new();
        let specialist = Arc::new(MockSpecialist);
        registry.register(specialist.clone());

        assert_eq!(registry.count(), 1);
        assert!(registry.get(SpecialistId::Sentinel).is_some());
    }

    #[test]
    fn test_user_state_defaults() {
        let state = UserState::default();
        assert_eq!(state.stress_level, 0.5);
        assert_eq!(state.activity, "idle");
    }

    #[test]
    fn test_system_resources_defaults() {
        let resources = SystemResources::default();
        assert_eq!(resources.gpu_available_percent, 100.0);
        assert_eq!(resources.memory_available_mb, 8192);
    }
}
