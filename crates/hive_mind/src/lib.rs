#![allow(unused)]

pub mod action_executor;
pub mod agentic_players;
pub mod assimilation;
pub mod capability_broker;
pub mod cartridge_compiler;
pub mod decision_engine;
pub mod delta_orchestrator;
pub mod executive_plan;
#[cfg(feature = "fault_injector")]
pub mod fault_injector;
pub mod federation;
pub mod hybrid_master_registry;
pub mod intent_orchestrator;
#[cfg(feature = "knowledge_gap")]
pub mod knowledge_gap_detector;
pub mod lora_adapter_vault;
pub mod mcp_service;
pub mod neural_pruning;
pub mod onboarding;
pub mod orchestration_daemon;
pub mod profile_persistence;
pub mod raft_consensus;
pub mod registry_adapters;
pub mod reward_system;
pub mod simulation_testbed;
pub mod splicing_engine;
pub mod supervisory_loop;
pub mod task_routing;
pub mod unified_learning;
pub mod worker_types;
pub use crate::intent_orchestrator as prefrontal_cortex;
pub use crate::profile_persistence as hox_persistence;
pub use crate::reward_system as dopamine_system;
pub use crate::supervisory_loop as autonomic_loop;
pub use crate::worker_types as enzyme_types;
