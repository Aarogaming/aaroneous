pub mod agent_bridge;
pub mod communication;
pub mod conflict_resolution;
pub mod proposal;
pub mod sentinel;
/// Federation Module: Core specialist hive architecture
///
/// This module implements the federated specialist hive pattern where:
/// - Each specialist is independent with its own GGUF model
/// - Sentinel orchestrates decisions without being a bottleneck
/// - Specialists propose actions bidirectionally (top-down + bottom-up)
/// - Specialists can self-organize and negotiate with peers
///
/// Architecture: 6 core specialists
/// - Sentinel (2GB): Orchestrator, conflict resolver, resource allocator
/// - Visionary (1GB): Design generation, aesthetic learning
/// - Omnipresent (1GB): P2P sync, multi-device coordination
/// - Symbiotic (500MB): Biometric polling, state classification
/// - Phygital (1GB): AR/VR, depth processing, landmarks
/// - Archivist (500MB): ArtifactRegistry persistence, reflection
// ── Core specialist system ──────────────────────────────────────────────────
pub mod specialist;
pub mod specialists;

// ── Lifecycle & orchestration ───────────────────────────────────────────────
pub mod hive;
pub mod host;

// ── Intent & session management ─────────────────────────────────────────────
pub mod intent;
pub mod session;

// ── Persistence bridge ──────────────────────────────────────────────────────
pub mod hive_db;
pub mod learn_persist;

// ── Model DNA & sovereign packages ──────────────────────────────────────────
pub mod dna;
pub mod model_registry;
pub mod sovereign_package;

// ── Forge (GGUF crystallization) ────────────────────────────────────────────
pub mod forge;

// ── HTTP status API ─────────────────────────────────────────────────────────
pub mod http;

// ── Enterprise observability (audit, compliance) ────────────────────────────
pub mod enterprise;

// ── Speculative modules (compile but not wired into active paths) ───────────
pub mod ar;
pub mod biometric;
pub mod bootstrap;
pub mod cli;
pub mod component_registry;
pub mod deployment_examples;
pub mod artifact_registry;
pub mod graph;
pub mod links;
pub mod multi_hive;
pub mod optimization;
pub mod p2p;
pub mod fleet_scheduler;
pub mod tasks;
pub mod tensor_vault;

pub use fleet_scheduler::{FleetScheduler, FleetTask, PeerLoadMetric};

// runtime.rs was a standalone design prototype (model cache + deployment manifest runtime)
// that was never connected to any production code. Its useful concepts (ModelManager,
// ExecutionMetrics) are available for future extraction into a monitoring module.
// The file is retained on disk but not compiled in to avoid dead-code warnings.
pub mod runtime;

// ── Re-exports: Core specialist system ──────────────────────────────────────
pub use agent_bridge::{SpecialistAgentBridge, agent_name_to_specialist_id};
pub use communication::{CommunicationBus, MessageChannel, SpecialistMessage};
pub use conflict_resolution::{
    ConflictArbitrator, ConflictDetector, ConflictResolution, ResourceAllocation,
};
pub use proposal::{Proposal, ProposalId, ProposalSet, ProposalStatus};
pub use sentinel::{ArbitrationResult, Sentinel, SentinelConfig};
pub use specialist::{
    Conflict, Decision, DelegateRequest, ExecutionResult, ExecutionStatus, ProposalPriority,
    ProposedAction, Specialist, SpecialistConfig, SpecialistContext, SpecialistId,
    SpecialistRegistry, SystemResources, UserState,
};
pub use specialists::{Archivist, GenericSpecialist, Omnipresent, Phygital, Symbiotic, Visionary};

// ── Re-exports: Lifecycle & orchestration ───────────────────────────────────
pub use hive::{
    Federation, FederationBuilder, FederationConfig, FederationErrors, LearningSummary,
    SpecialistLearningSummary,
};
pub use host::{HostConfig, HostError, HostState, HostableSpecialist, SpecialistHost};

// ── Re-exports: Intent & session ────────────────────────────────────────────
pub use intent::{Intent, IntentPriority, IntentResult, IntentScaling, IntentSource, IntentStatus};
pub use session::{Session, SessionManager, SessionState};

// ── Re-exports: Persistence ─────────────────────────────────────────────────
pub use learn_persist::{
    LearnPersistError, LearningSnapshot, PersistableLearning, load_learning, save_learning,
};

// ── Re-exports: Model DNA & sovereign ───────────────────────────────────────
pub use dna::{BlockDNA, ModelDNA, dissect_model, load_dna_sidecar};
pub use model_registry::register_as_specialist;
pub use sovereign_package::{
    ImportResult, LearningStateSnapshot, PackageOptions, SovereignManifest, export_sovereign,
    import_sovereign, read_manifest,
};

// ── Re-exports: Forge ───────────────────────────────────────────────────────
pub use forge::{
    CrystallizationResult, Forge, ForgeError, ForgeRecipe, GgufIndex, GgufMeta, SplicingSegment,
    TensorMeta,
};

// ── Re-exports: HTTP ────────────────────────────────────────────────────────
pub use http::{HttpServerError, HttpStatusServer};

// ── Re-exports: Enterprise ──────────────────────────────────────────────────
pub use enterprise::{
    AccessControl, Analytics, AnalyticsEvent, AuditEvent, AuditLevel, AuditLog, AuditQuery,
    AuthToken, ComplianceMonitor, ComplianceRule, ComplianceStatus, DataEncryption,
    EnterpriseContext, Permission, QuotaLimit, RateLimiter, Role, SecurityConfig, TLSConfig,
};

// ── Re-exports: Speculative modules ─────────────────────────────────────────
pub use ar::{
    ArError, ArProvider, ArSessionState, ArSystemInfo, FormFactor as ArFormFactor,
    ViewConfiguration as ArViewConfiguration,
};
pub use biometric::{
    BiometricDevice, BiometricKind, BiometricProvider, BiometricSample, BiometricStream, BleError,
    DeviceFilter, StandardServices,
};
pub use bootstrap::{
    BootstrapResult, BootstrapSystem, DeploymentConfig, DeploymentTarget, Manifest,
    SpecialistModule,
};
pub use cli::{AaroneosCLI, CLIResult, Command, ExpandArgs, InitArgs, PortableArgs, StatusArgs};
pub use component_registry::{AgentBundle, ComponentRegistry};
pub use artifact_registry::{
    BackupInfo, ConsolidationStats, ArtifactRegistry, ArtifactRegistryStats, ArtifactEvent, EventQuery, Pattern,
};
pub use multi_hive::{
    ClusterConfig, ConsensusEngine, DistributedSpecialistRegistry, FederatedLearningEngine,
    GossipMessage, GradientUpdate, HiveCluster, HiveNode, MessageType, ModelMerger,
    MultihiveFederation, P2PNetwork, PeerMessage, RemoteSpecialist,
};
pub use optimization::{
    AccessPattern, BatchConfig, BatchManager, CacheWarmingStrategy, CacheWarmingTracker,
    GPUAccelerationStrategy, GPUInferenceContext, GPUInfo, GPUMemoryManager, GPUType,
    OptimizationProfile, OptimizationStats, ProposalBatch, QuantizationConfig,
    QuantizationStrategy, QuantizationType, QuantizedModel, WarmingSchedule,
};
pub use p2p::{P2pError, P2pNode, P2pNodeId, SyncMessage};
pub use runtime::{ExecutionMetrics, HealthReport, LoadedModel, ModelManager, RuntimeStats};
pub use tasks::{
    BackgroundTaskHandle, OmnipresentDrainTask, OmnipresentRecvTask, SymbioticBleTask,
    SymbioticDrainTask,
};

#[cfg(test)]
mod tests;
