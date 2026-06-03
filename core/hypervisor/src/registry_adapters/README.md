# Registry Adapters Module

## Overview

This module implements the **Adapter Pattern** to wrap existing registry implementations with a unified `SubRegistry` trait interface, enabling dynamic composition through the hybrid master registry architecture.

## Architecture

### Trait Interface (`SubRegistry`)

```rust
pub trait SubRegistry: Send + Sync {
    fn initialize(&mut self, ctx: &WorkspaceContext) -> Result<(), String>;
    fn query_entity(&self, id: &str) -> Option<EntityInfo>;
    fn synchronize_state(&mut self, ctx: &WorkspaceContext) -> Result<(), String>;
    fn registry_type(&self) -> RegistryType;
}
```

### Adapter Pattern

Each adapter wraps an existing registry implementation and delegates to it while converting internal types to the unified `EntityInfo` format.

## Adapters

### 1. UnifiedRegistryAdapter

Wraps `unified_registry::Registry<K>` with:
- Health mapping (Healthy/Degraded/Failed/Unknown)
- Entry metadata conversion (id, name, version, last_seen)

### 2. FederationModelRegistryAdapter

Wraps `federation/model_registry::FederationModelRegistry` with:
- GGUF model metadata conversion
- Status mapping (Loaded/Loading/Failed/NotLoaded)
- Model lifecycle tracking

### 3. LinkRegistryAdapter

Wraps `federation/links::LinkRegistry` with:
- External link management delegation
- Link health tracking
- URL/ID-based entity queries

### 4. LLMModelRegistryAdapter

Wraps `llm/model_registry::ModelRegistry` with:
- Model info conversion (name, path, size_bytes, model_type)
- Set/get methods for delegation
- Type classification support

### 5. ComponentRegistryAdapter

Wraps `federation/component_registry::ComponentRegistry` with:
- WASM component versioning
- Hot-swapping support
- Set/get/set_latest_version methods

### 6. SpecialistRegistryAdapter

Wraps `federation/specialist::SpecialistRegistry` with:
- Specialist identity mapping (Sentinel/Visionary/etc.)
- Distributed discovery integration
- Capability tracking

### 7. ChromosomeRegistryAdapter

Wraps `chromosome_registry::ChromosomeRegistry` with:
- HoxChromosome profile conversion
- Epigenetic switches mapping
- Agent ID → entity ID translation

### 8. HoxCapabilityRegistryAdapter

Wraps `hox_registry::HoxCapabilityRegistry` with:
- SQLite-backed capability conversion
- Enzyme permissions mapping
- Health status tracking

### 9. DistributedSpecialistRegistryAdapter

Wraps `federation/multi_hive/distributed_registry::DistributedSpecialistRegistry` with:
- Cross-hive specialist discovery
- Remote specialist metadata
- Address-based queries

## Usage

```rust
// Create composition strategy
let mut strategy = RegistryCompositionStrategy::new();

// Wire up registries
strategy = strategy.with_unified_registry(unified_registry);
strategy = strategy.with_federation_model_registry(model_registry);
// ... etc

// Build master registry with workspace context
let ctx = WorkspaceContext {
    current_era: PhaseEra::SixD,
    registry_version: "1.0.0".to_string(),
};

let master = strategy.build_master_registry(&ctx);
```

## Testing

Test adapters individually:
```bash
cargo test -p hypervisor --lib hybrid_master_registry::tests
```

Verify adapter delegation works correctly with underlying registry types.

## Phase 6D Context

This module is part of the **Phase 6D WASM/Sentinel GuestOS layer**, implementing the hybrid master registry architecture that combines trait-based discovery with registry-of-registries container pattern for structural polymorphism and dynamic composition.
