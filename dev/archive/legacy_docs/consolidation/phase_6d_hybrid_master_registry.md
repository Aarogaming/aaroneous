# Phase 6D: Hybrid Master Registry Composition Strategy

## Overview

This document describes the implementation of the Hybrid Master Registry Composition Strategy for Phase 6D of the Aaroneous Defragmentation project.

## Goal

Combine Trait-Based Discovery Pattern with Registry-of-Registries Container for the Phase 6D WASM/Sentinel GuestOS layer.

## Architecture

### Trait-Based Discovery Pattern

The `SubRegistry` trait provides a unified interface for all registry implementations:

```rust
pub trait SubRegistry: Send + Sync {
    fn initialize(&mut self, ctx: &WorkspaceContext) -> Result<(), String>;
    fn query_entity(&self, id: &str) -> Option<EntityInfo>;
    fn list(&self) -> Vec<EntityInfo>;
    fn synchronize_state(&mut self, ctx: &WorkspaceContext) -> Result<(), String>;
    fn registry_type(&self) -> RegistryType;
}
```

### Registry-of-Registries Container

The `MasterRegistry` struct implements the Registry-of-Registries pattern:

```rust
pub struct MasterRegistry {
    sub_registries: Vec<Box<dyn SubRegistry>>,
    ctx: WorkspaceContext,
    meta: RegistryMeta,
    synced_entities: HashMap<String, EntityInfo>,
}
```

## Registry Adapters

Each adapter wraps an existing registry type and implements the `SubRegistry` trait:

### UnifiedRegistryAdapter

- **Registry Type**: Unified
- **Internal Structure**: `entries: HashMap<String, RegistryEntry<T>>`
- **Query Support**: Full (via `self.inner.get(id)`)
- **List Support**: Full (via `self.inner.list()`)
- **Sync Support**: Full (via `self.inner.evict_expired()`)

### FederationModelRegistryAdapter

- **Registry Type**: FederationModel
- **Internal Structure**: Private `models` field
- **Query Support**: Stubbed (None)
- **List Support**: Stubbed (empty Vec)
- **Sync Support**: Stubbed (empty)

### LinkRegistryAdapter

- **Registry Type**: FederationLinks
- **Internal Structure**: Private `specialists` field
- **Query Support**: Stubbed (None)
- **List Support**: Stubbed (empty Vec)
- **Sync Support**: Stubbed (empty)

### LLMModelRegistryAdapter

- **Registry Type**: LLMModel
- **Internal Structure**: Private `models` field
- **Query Support**: Stubbed (None)
- **List Support**: Stubbed (empty Vec)
- **Sync Support**: Stubbed (empty)

### ComponentRegistryAdapter

- **Registry Type**: Component
- **Internal Structure**: Private `components` field
- **Query Support**: Stubbed (None)
- **List Support**: Stubbed (empty Vec)
- **Sync Support**: Stubbed (empty)

### SpecialistRegistryAdapter

- **Registry Type**: Specialist
- **Internal Structure**: Private `specialists` field
- **Query Support**: Stubbed (None)
- **List Support**: Stubbed (empty Vec)
- **Sync Support**: Stubbed (empty)

### ChromosomeRegistryAdapter

- **Registry Type**: Chromosome
- **Internal Structure**: Private `profiles` field
- **Query Support**: Stubbed (None)
- **List Support**: Stubbed (empty Vec)
- **Sync Support**: Stubbed (empty)

### HoxCapabilityRegistryAdapter

- **Registry Type**: HoxCapability
- **Internal Structure**: SQLite-backed via `get_capability` API
- **Query Support**: Full (via `self.inner.get_capability(id)`)
- **List Support**: Full (via `self.inner.list_capabilities()`)
- **Sync Support**: Stubbed (empty)

### DistributedSpecialistRegistryAdapter

- **Registry Type**: DistributedSpecialist
- **Internal Structure**: Private `specialists` field
- **Query Support**: Stubbed (None)
- **List Support**: Stubbed (empty Vec)
- **Sync Support**: Stubbed (empty)

## Composition Strategy

The `RegistryCompositionStrategy` struct provides a fluent API for wiring existing registries to the hybrid master container:

```rust
pub struct RegistryCompositionStrategy {
    adapters: Vec<Box<dyn SubRegistry>>,
}

impl RegistryCompositionStrategy {
    pub fn new() -> Self {
        Self { adapters: Vec::new() }
    }
    
    pub fn with_unified_registry(mut self, registry: Registry<String>) -> Self {
        let adapter = UnifiedRegistryAdapter::new(registry);
        self.adapters.push(Box::new(adapter));
        self
    }
    
    // ... other registry adapters ...
    
    pub fn build_master_registry(self, ctx: &WorkspaceContext) -> MasterRegistry {
        let mut master = MasterRegistry::new();
        for adapter in self.adapters {
            master.add_registry(adapter);
        }
        master.initialize();
        master
    }
}
```

## Synchronization Flow

1. **Initialize**: Call `initialize()` on all sub-registries with workspace context
2. **Synchronize**: Call `synchronize_state()` on all sub-registries
3. **Collect**: Call `list()` on all sub-registries to collect entities
4. **Cache**: Store entities in `synced_entities` HashMap
5. **Query**: Check `synced_entities` cache first, then fall back to sub-registries

## EntityInfo Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub id: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub health: EntryHealth,
    pub last_seen: u64,
}
```

## Health Status

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryHealth {
    Healthy,
    Degraded,
    Failed,
    Unknown,
}
```

## Phase Era

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PhaseEra {
    OneA,
    TwoB,
    ThreeC,
    FourD,
    FiveE,
    SixD,  // Current era
}
```

## Workspace Context

```rust
#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    pub current_era: PhaseEra,
    pub registry_version: String,
}
```

## Files Modified

- `core/hypervisor/src/registry/mod.rs` - Core registry trait definitions
- `core/hypervisor/src/hybrid_master_registry.rs` - Master registry container
- `core/hypervisor/src/registry_adapters.rs` - Registry adapters

## Next Steps

1. Add public accessor methods to underlying registries for real synchronization
2. Implement `list()` methods for registries that support it
3. Wire adapters to master registry in application initialization
4. Add cross-registry synchronization logic
5. Implement entity health monitoring
6. Add registry version tracking

## Notes

- Some adapters currently return `None` for `query_entity()` because the underlying registries don't expose a simple `models: HashMap<String, ModelInfo>` field
- Real synchronization requires adding public accessor methods to the underlying registries
- The `list()` method is implemented for registries that support it (UnifiedRegistry, HoxRegistry)
- Stubbed adapters return empty Vec for `list()` to satisfy the trait surface
