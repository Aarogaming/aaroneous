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

pub mod specialist;
pub mod sentinel;
pub mod proposal;
pub mod communication;
pub mod conflict_resolution;
pub mod agent_bridge;
pub mod specialists;
pub mod bootstrap;
pub mod deployment_examples;
pub mod cli;
pub mod runtime;
pub mod dna_bank;

pub use specialist::{Specialist, SpecialistId, SpecialistConfig, SpecialistRegistry, SpecialistContext, Decision, DelegateRequest, Conflict, ProposedAction, SystemResources, UserState, ExecutionStatus, ExecutionResult, ProposalPriority};
pub use sentinel::{Sentinel, SentinelConfig, ArbitrationResult};
pub use proposal::{Proposal, ProposalId, ProposalStatus, ProposalSet};
pub use communication::{SpecialistMessage, MessageChannel, CommunicationBus};
pub use conflict_resolution::{ConflictDetector, ResourceAllocation, ConflictResolution, ConflictArbitrator};
pub use agent_bridge::{SpecialistAgentBridge, agent_name_to_specialist_id};
pub use specialists::Visionary;
pub use bootstrap::{SpecialistModule, DeploymentTarget, Manifest, DeploymentConfig, BootstrapResult, BootstrapSystem};
pub use cli::{Command, AaroneosCLI, CLIResult, InitArgs, ExpandArgs, PortableArgs, StatusArgs};
pub use runtime::{HiveRuntime, ModelManager, ExecutionMetrics, SpecialistHealth, HealthStatus, RuntimeStats, HealthReport, LoadedModel};
pub use dna_bank::{DNABank, DNAEvent, EventQuery, Pattern, DNABankStats, ConsolidationStats, BackupInfo};

#[cfg(test)]
mod tests;
