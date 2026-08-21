/// Registry adapters that implement the SubRegistry trait for the hybrid master registry.
///
/// These adapters provide a unified trait interface to existing registries that
/// have different internal structures. They enable cross-registry synchronization
/// by exposing a common query_entity() and synchronize_state() interface.
///
/// Phase 6D: Hybrid Master Registry Composition Strategy
///
/// Each adapter:
/// 1. Wraps an existing registry type
/// 2. Implements SubRegistry trait methods
/// 3. Provides a common interface for master registry coordination
/// 4. Handles type conversions and data mapping
///
/// Note: Some adapters currently return None for query_entity() because the
/// underlying registries don't expose a simple `models: HashMap<String, ModelInfo>`
/// field. Real synchronization requires adding public accessor methods to the
/// underlying registries.
use crate::registry::{EntityInfo, EntryHealth, RegistryType, SubRegistry, WorkspaceContext};

/// Adapter for UnifiedRegistry to SubRegistry trait.
///
/// UnifiedRegistry has a public `entries: HashMap<String, RegistryEntry<T>>` field
/// that can be directly accessed for synchronization.
pub struct UnifiedRegistryAdapter<
    T: Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
> {
    inner: crate::unified_registry::Registry<T>,
}

impl<T: Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync>
    UnifiedRegistryAdapter<T>
{
    pub fn new(registry: crate::unified_registry::Registry<T>) -> Self {
        Self { inner: registry }
    }
}

fn map_health(h: crate::unified_registry::EntryHealth) -> EntryHealth {
    match h {
        crate::unified_registry::EntryHealth::Healthy => EntryHealth::Healthy,
        crate::unified_registry::EntryHealth::Degraded => EntryHealth::Degraded,
        crate::unified_registry::EntryHealth::Failed => EntryHealth::Failed,
        crate::unified_registry::EntryHealth::Unknown => EntryHealth::Unknown,
    }
}

impl<T: Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync> SubRegistry
    for UnifiedRegistryAdapter<T>
{
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // UnifiedRegistry.entries is public and accessible
        if let Some(entry) = self.inner.get(id) {
            Some(EntityInfo {
                id: entry.id.clone(),
                name: Some(serde_json::to_string(&entry.data).unwrap_or_default()),
                version: Some(entry.meta.version.clone()),
                health: map_health(entry.meta.health),
                last_seen: entry.meta.last_seen,
            })
        } else {
            None
        }
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        // Evict expired entries and update health
        self.inner.evict_expired();
        Ok(())
    }

    fn list(&self) -> Vec<EntityInfo> {
        // List all entries from UnifiedRegistry
        self.inner
            .list()
            .into_iter()
            .map(|entry| EntityInfo {
                id: entry.id.clone(),
                name: Some(serde_json::to_string(&entry.data).unwrap_or_default()),
                version: Some(entry.meta.version.clone()),
                health: map_health(entry.meta.health),
                last_seen: entry.meta.last_seen,
            })
            .collect()
    }

    fn registry_type(&self) -> RegistryType {
        RegistryType::Unified
    }
}

/// Adapter for FederationModelRegistry to SubRegistry trait.
///
/// FederationModelRegistry does not expose a `models` HashMap field;
/// return None to satisfy the SubRegistry surface. Real sync can be
/// wired by adding a public accessor to the federation model registry.
pub struct FederationModelRegistryAdapter {
    _inner: crate::federation::model_registry::FederationModelRegistry,
}

impl FederationModelRegistryAdapter {
    pub fn new(registry: crate::federation::model_registry::FederationModelRegistry) -> Self {
        Self { _inner: registry }
    }
}

impl SubRegistry for FederationModelRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // FederationModelRegistry does not expose a `models` HashMap field;
        // return None to satisfy the SubRegistry surface. Real sync can be
        // wired by adding a public accessor to the federation model registry.
        let _ = id;
        None
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn registry_type(&self) -> RegistryType {
        RegistryType::FederationModel
    }

    fn list(&self) -> Vec<EntityInfo> {
        // FederationModelRegistry does not expose a list API; return empty
        Vec::new()
    }
}

/// Adapter for LinkRegistry to SubRegistry trait.
///
/// LinkRegistry has a public `specialists: HashMap<SpecialistId, Arc<dyn Specialist>>`
/// field, but Specialist has no `name`/`last_active` fields.
/// Return None to satisfy the SubRegistry surface.
pub struct LinkRegistryAdapter {
    _inner: crate::federation::links::LinkRegistry,
}

impl LinkRegistryAdapter {
    pub fn new(registry: crate::federation::links::LinkRegistry) -> Self {
        Self { _inner: registry }
    }
}

impl SubRegistry for LinkRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // LinkRegistry.specialists is private and stores Arc<dyn Specialist>;
        // return None. Real sync requires a public accessor method.
        let _ = id;
        None
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn registry_type(&self) -> RegistryType {
        RegistryType::FederationLinks
    }

    fn list(&self) -> Vec<EntityInfo> {
        // LinkRegistry does not expose a list API; return empty
        Vec::new()
    }
}

/// Adapter for LLMModelRegistry to SubRegistry trait.
///
/// ModelRegistry.models is private; return None. Real sync requires
/// adding a public accessor (e.g., `pub fn get(&self, id: &str) -> Option<&ModelInfo>`)
/// to the llm model_registry module.
pub struct LLMModelRegistryAdapter {
    _inner: crate::llm::model_registry::ModelRegistry,
}

impl LLMModelRegistryAdapter {
    pub fn new(registry: crate::llm::model_registry::ModelRegistry) -> Self {
        Self { _inner: registry }
    }
}

impl SubRegistry for LLMModelRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // ModelRegistry.models is private; return None. Real sync requires
        // adding a public accessor (e.g., `pub fn get(&self, id: &str) -> Option<&ModelInfo>`)
        // to the llm model_registry module.
        let _ = id;
        None
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn registry_type(&self) -> RegistryType {
        RegistryType::LLMModel
    }

    fn list(&self) -> Vec<EntityInfo> {
        // ModelRegistry does not expose a list API; return empty
        Vec::new()
    }
}

/// Adapter for ComponentRegistry to SubRegistry trait.
///
/// ComponentRegistry does not expose a `components` field; return None.
pub struct ComponentRegistryAdapter {
    _inner: crate::federation::component_registry::ComponentRegistry,
}

impl ComponentRegistryAdapter {
    pub fn new(registry: crate::federation::component_registry::ComponentRegistry) -> Self {
        Self { _inner: registry }
    }
}

impl SubRegistry for ComponentRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // ComponentRegistry does not expose a `components` field; return None.
        let _ = id;
        None
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn registry_type(&self) -> RegistryType {
        RegistryType::Component
    }

    fn list(&self) -> Vec<EntityInfo> {
        // ComponentRegistry does not expose a list API; return empty
        Vec::new()
    }
}

/// Adapter for SpecialistRegistry to SubRegistry trait.
///
/// SpecialistRegistry.specialists is private and stores Arc<dyn Specialist>;
/// return None. Real sync requires a public accessor method.
pub struct SpecialistRegistryAdapter {
    _inner: crate::federation::specialist::SpecialistRegistry,
}

impl SpecialistRegistryAdapter {
    pub fn new(registry: crate::federation::specialist::SpecialistRegistry) -> Self {
        Self { _inner: registry }
    }
}

impl SubRegistry for SpecialistRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // SpecialistRegistry.specialists is private and stores Arc<dyn Specialist>;
        // return None. Real sync requires a public accessor method.
        let _ = id;
        None
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn registry_type(&self) -> RegistryType {
        RegistryType::Specialist
    }

    fn list(&self) -> Vec<EntityInfo> {
        // SpecialistRegistry does not expose a list API; return empty
        Vec::new()
    }
}

/// Adapter for ChromosomeRegistry to SubRegistry trait.
///
/// ChromosomeRegistry has `profiles: HashMap<String, HoxChromosome>` (not `chromosomes`).
/// Stubbed to None; wire a public `get` accessor to enable real sync.
pub struct ChromosomeRegistryAdapter {
    _inner: crate::chromosome_registry::ChromosomeRegistry,
}

impl ChromosomeRegistryAdapter {
    pub fn new(registry: crate::chromosome_registry::ChromosomeRegistry) -> Self {
        Self { _inner: registry }
    }
}

impl SubRegistry for ChromosomeRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // ChromosomeRegistry has `profiles: HashMap<String, HoxChromosome>` (not `chromosomes`).
        // Stubbed to None; wire a public `get` accessor to enable real sync.
        let _ = id;
        None
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn registry_type(&self) -> RegistryType {
        RegistryType::Chromosome
    }

    fn list(&self) -> Vec<EntityInfo> {
        // ChromosomeRegistry does not expose a list API; return empty
        Vec::new()
    }
}

/// Adapter for HoxRegistry to SubRegistry trait.
///
/// Uses the public HoxRegistry API (no internal field access).
pub struct HoxCapabilityRegistryAdapter {
    inner: crate::hox_registry::HoxRegistry,
}

impl HoxCapabilityRegistryAdapter {
    pub fn new(registry: crate::hox_registry::HoxRegistry) -> Self {
        Self { inner: registry }
    }
}

impl SubRegistry for HoxCapabilityRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // HoxRegistry stores capabilities in SQLite; the public `get_capability`
        // API returns a Result. We swallow it for the SubRegistry surface.
        let cap = self.inner.get_capability(id).ok().flatten()?;
        Some(EntityInfo {
            id: cap.name.clone(),
            name: Some(cap.name),
            version: None,
            health: EntryHealth::Healthy,
            last_seen: 0,
        })
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn list(&self) -> Vec<EntityInfo> {
        // List all capabilities from HoxRegistry
        self.inner
            .list_capabilities()
            .ok()
            .unwrap_or_default()
            .into_iter()
            .map(|cap| EntityInfo {
                id: cap.name.clone(),
                name: Some(cap.name),
                version: None,
                health: EntryHealth::Healthy,
                last_seen: 0,
            })
            .collect()
    }

    fn registry_type(&self) -> RegistryType {
        RegistryType::HoxCapability
    }
}

/// Adapter for DistributedSpecialistRegistry to SubRegistry trait.
///
/// DistributedSpecialistRegistry.specialists is keyed by (SpecialistId, String)
/// not a bare &str, and RemoteSpecialist has no `name`/`last_active` fields.
/// Stubbed to None; requires public `find_by_id(&str)` accessor to enable real sync.
pub struct DistributedSpecialistRegistryAdapter {
    _inner: crate::federation::multi_hive::distributed_registry::DistributedSpecialistRegistry,
}

impl DistributedSpecialistRegistryAdapter {
    pub fn new(
        registry: crate::federation::multi_hive::distributed_registry::DistributedSpecialistRegistry,
    ) -> Self {
        Self { _inner: registry }
    }
}

impl SubRegistry for DistributedSpecialistRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // DistributedSpecialistRegistry.specialists is keyed by (SpecialistId, String)
        // not a bare &str, and RemoteSpecialist has no `name`/`last_active` fields.
        // Stubbed to None; requires public `find_by_id(&str)` accessor to enable real sync.
        let _ = id;
        None
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }

    fn registry_type(&self) -> RegistryType {
        RegistryType::DistributedSpecialist
    }

    fn list(&self) -> Vec<EntityInfo> {
        // DistributedSpecialistRegistry does not expose a list API; return empty
        Vec::new()
    }
}
