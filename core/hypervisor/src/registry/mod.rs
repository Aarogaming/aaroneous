/// Core registry module for Aaroneous hybrid master registry system.
/// 
/// This module implements the core `SubRegistry` trait that all registry adapters must implement,
/// providing a unified interface for the hybrid master registry to interact with all sub-registries.

use serde::{Serialize, Deserialize};

/// Trait interface for sub-registries in the hybrid master registry system.
/// 
/// Defines lifecycle methods that all registry implementations must support:
/// - `initialize`: Setup and initialization with workspace context
/// - `query_entity`: Query an entity by ID, returning None if not found
/// - `synchronize_state`: Synchronize state across all sub-registries
pub trait SubRegistry: Send + Sync {
    /// Initialize the registry with workspace context.
    fn initialize(&mut self, ctx: &WorkspaceContext) -> Result<(), String>;
    
    /// Query an entity by ID. Returns None if not found.
    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        None
    }
    
    /// Synchronize state and return all entities managed by this sub-registry.
    /// 
    /// Called by master registry during synchronization.
    /// Returns a Vec of EntityInfo for all entities in this registry.
    fn list_entities(&self) -> Vec<EntityInfo> {
        Vec::new()
    }
    
    /// Synchronize state across all sub-registries.
    fn synchronize_state(&mut self, ctx: &WorkspaceContext) -> Result<(), String>;
    
    /// Get registry type identifier.
    fn registry_type(&self) -> RegistryType;
}

/// Information about a queried entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub id: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub health: EntryHealth,
    pub last_seen: u64,
}

/// Health status of a registered entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryHealth {
    Healthy,
    Degraded,
    Failed,
    Unknown,
}

impl Default for EntryHealth {
    fn default() -> Self { Self::Unknown }
}

/// Type identifier for registry implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryType {
    Unified,
    FederationModel,
    FederationLinks,
    LLMModel,
    Component,
    Specialist,
    Chromosome,
    HoxCapability,
    DistributedSpecialist,
}

impl Default for RegistryType {
    fn default() -> Self { Self::Unified }
}

/// Workspace context for registry operations.
#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    /// Current active era (e.g., Phase 6D)
    pub current_era: PhaseEra,
    /// Registry version identifier
    pub registry_version: String,
}

impl Default for WorkspaceContext {
    fn default() -> Self {
        Self {
            current_era: PhaseEra::SixD,
            registry_version: "1.0.0".to_string(),
        }
    }
}

/// Era identifier for the system lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PhaseEra {
    /// Era 1: Foundation
    OneA,
    /// Era 2: Nervous System
    TwoB,
    /// Era 3: Digestive System (current)
    ThreeC,
    /// Era 4: Genetic System
    FourD,
    /// Era 5: Biological System
    FiveE,
    /// Era 6: WASM/Sentinel GuestOS Layer
    SixD,
}

impl PhaseEra {
    pub fn current() -> Self {
        Self::SixD
    }
}

impl std::fmt::Display for PhaseEra {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::OneA => "1A",
            Self::TwoB => "2B",
            Self::ThreeC => "3C",
            Self::FourD => "4D",
            Self::FiveE => "5E",
            Self::SixD => "6D",
        };
        write!(f, "Phase {}", s)
    }
}

impl std::fmt::Display for RegistryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Unified => "unified",
            Self::FederationModel => "federation_model",
            Self::FederationLinks => "federation_links",
            Self::LLMModel => "llm_model",
            Self::Component => "component",
            Self::Specialist => "specialist",
            Self::Chromosome => "chromosome",
            Self::HoxCapability => "hox_capability",
            Self::DistributedSpecialist => "distributed_specialist",
        };
        write!(f, "{}", s)
    }
}
