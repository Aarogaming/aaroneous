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
/// - Archivist (500MB): DNA Bank persistence, reflection

// ── Core specialist system ──────────────────────────────────────────────────
pub mod specialist;
pub mod sentinel;
pub mod proposal;
pub mod communication;
pub mod conflict_resolution;
pub mod agent_bridge;
pub mod specialists;

// ── Lifecycle & orchestration ───────────────────────────────────────────────
pub mod host;
pub mod hive;

// ── Intent & session management ─────────────────────────────────────────────
pub mod intent;
pub mod session;

// ── Persistence bridge ──────────────────────────────────────────────────────
pub mod learn_persist;
pub mod hive_db;

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
pub mod bootstrap;
pub mod component_registry;
pub mod deployment_examples;
pub mod cli;
pub mod dna_bank;
pub mod optimization;
pub mod multi_hive;
pub mod p2p;
pub mod biometric;
pub mod ar;
pub mod tasks;
pub mod graph;
pub mod tensor_vault;
pub mod links;

// runtime.rs was a standalone design prototype (model cache + deployment manifest runtime)
// that was never connected to any production code. Its useful concepts (ModelManager,
// ExecutionMetrics) are available for future extraction into a monitoring module.
// The file is retained on disk but not compiled in to avoid dead-code warnings.
pub mod runtime;

// ── Re-exports: Core specialist system ──────────────────────────────────────
pub use specialist::{
    Specialist, SpecialistId, SpecialistConfig, SpecialistRegistry,
    SpecialistContext, Decision, DelegateRequest, Conflict,
    ProposedAction, SystemResources, UserState,
    ExecutionStatus, ExecutionResult, ProposalPriority,
};
pub use sentinel::{Sentinel, SentinelConfig, ArbitrationResult};
pub use proposal::{Proposal, ProposalId, ProposalStatus, ProposalSet};
pub use communication::{SpecialistMessage, MessageChannel, CommunicationBus};
pub use conflict_resolution::{ConflictDetector, ResourceAllocation, ConflictResolution, ConflictArbitrator};
pub use agent_bridge::{SpecialistAgentBridge, agent_name_to_specialist_id};
pub use specialists::{Visionary, Omnipresent, Symbiotic, Phygital, Archivist, GenericSpecialist};

// ── Re-exports: Lifecycle & orchestration ───────────────────────────────────
pub use host::{SpecialistHost, HostConfig, HostState, HostError, HostableSpecialist};
pub use hive::{
    Federation, FederationBuilder, FederationConfig, FederationErrors,
    LearningSummary, SpecialistLearningSummary,
};

// ── Re-exports: Intent & session ────────────────────────────────────────────
pub use intent::{
    Intent, IntentPriority, IntentStatus, IntentSource, IntentScaling, IntentResult,
};
pub use session::{Session, SessionState, SessionManager};

// ── Re-exports: Persistence ─────────────────────────────────────────────────
pub use learn_persist::{
    LearningSnapshot, PersistableLearning, LearnPersistError,
    save_learning, load_learning,
};

// ── Re-exports: Model DNA & sovereign ───────────────────────────────────────
pub use dna::{ModelDNA, BlockDNA, dissect_model, load_dna_sidecar};
pub use model_registry::register_as_specialist;
pub use sovereign_package::{
    SovereignManifest, PackageOptions, LearningStateSnapshot,
    export_sovereign, import_sovereign, ImportResult, read_manifest,
};

// ── Re-exports: Forge ───────────────────────────────────────────────────────
pub use forge::{Forge, ForgeRecipe, SplicingSegment, GgufIndex, GgufMeta, TensorMeta, ForgeError, CrystallizationResult};

// ── Re-exports: HTTP ────────────────────────────────────────────────────────
pub use http::{HttpStatusServer, HttpServerError};

// ── Re-exports: Enterprise ──────────────────────────────────────────────────
pub use enterprise::{
    AuditLog, AuditEvent, AuditLevel, AuditQuery,
    ComplianceMonitor, ComplianceRule, ComplianceStatus,
    SecurityConfig, TLSConfig, DataEncryption,
    RateLimiter, QuotaLimit,
    AccessControl, Role, Permission, AuthToken,
    Analytics, AnalyticsEvent,
    EnterpriseContext,
};

// ── Re-exports: Speculative modules ─────────────────────────────────────────
pub use component_registry::{ComponentRegistry, AgentBundle};
pub use bootstrap::{SpecialistModule, DeploymentTarget, Manifest, DeploymentConfig, BootstrapResult, BootstrapSystem};
pub use cli::{Command, AaroneosCLI, CLIResult, InitArgs, ExpandArgs, PortableArgs, StatusArgs};
pub use dna_bank::{DNABank, DNAEvent, EventQuery, Pattern, DNABankStats, ConsolidationStats, BackupInfo};
pub use optimization::{
    QuantizationType, QuantizationStrategy, QuantizationConfig, QuantizedModel,
    GPUType, GPUInfo, GPUMemoryManager, GPUInferenceContext, GPUAccelerationStrategy,
    CacheWarmingStrategy, AccessPattern, CacheWarmingTracker, WarmingSchedule,
    BatchConfig, ProposalBatch, BatchManager,
    OptimizationProfile, OptimizationStats,
};
pub use multi_hive::{
    HiveCluster, HiveNode, ClusterConfig, P2PNetwork, PeerMessage, MessageType,
    GossipMessage, ConsensusEngine, GradientUpdate, ModelMerger, FederatedLearningEngine,
    RemoteSpecialist, DistributedSpecialistRegistry, MultihiveFederation,
};
pub use p2p::{P2pNode, P2pNodeId, P2pError, SyncMessage};
pub use biometric::{
    BiometricProvider, BiometricDevice, BiometricSample, BiometricKind,
    BleError, DeviceFilter, BiometricStream, StandardServices,
};
pub use ar::{ArProvider, ArSystemInfo, ArSessionState, ArError, FormFactor as ArFormFactor, ViewConfiguration as ArViewConfiguration};
pub use tasks::{BackgroundTaskHandle, OmnipresentRecvTask, SymbioticBleTask, OmnipresentDrainTask, SymbioticDrainTask};
pub use runtime::{ModelManager, ExecutionMetrics, LoadedModel, RuntimeStats, HealthReport};

#[cfg(test)]
mod tests;
