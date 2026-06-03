/// UnifiedRegistryAdapter - wraps unified_registry::Registry
pub mod unified_registry_adapter;

/// FederationModelRegistryAdapter - wraps federation/model_registry::FederationModelRegistry
pub mod federation_model_registry_adapter;

/// LinkRegistryAdapter - wraps federation/links::LinkRegistry
pub mod link_registry_adapter;

/// LLMModelRegistryAdapter - wraps llm/model_registry::ModelRegistry
pub mod llm_model_registry_adapter;

/// ComponentRegistryAdapter - wraps federation/component_registry::ComponentRegistry
pub mod component_registry_adapter;

/// SpecialistRegistryAdapter - wraps federation/specialist::SpecialistRegistry
pub mod specialist_registry_adapter;

/// ChromosomeRegistryAdapter - wraps chromosome_registry::ChromosomeRegistry
pub mod chromosome_registry_adapter;

/// HoxCapabilityRegistryAdapter - wraps hox_registry::HoxCapabilityRegistry
pub mod hox_registry_adapter;
pub use hox_registry_adapter::HoxCapabilityRegistryAdapter;

/// DistributedSpecialistRegistryAdapter - wraps federation/multi_hive/distributed_registry::DistributedSpecialistRegistry
pub mod distributed_specialist_registry_adapter;
