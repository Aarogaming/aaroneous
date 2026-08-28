// Aaroneous Nervous System - SWMR rkyv Architecture
// Single Writer, Multi-Reader zero-copy shared memory with mutation intent validation.

pub mod disruptor;
pub mod intent_log;
pub mod metrics;
pub mod mutation_intent;
pub mod nucleotide_packet;
pub mod specialist_bus;
pub mod persistent_grimoire;
pub mod preparedness_notice;
pub mod scheme_router;
pub mod slab_allocator;
pub mod spmc_synapse_bus;
pub mod swmr_synapse;

pub use specialist_bus::{install_specialist_panic_hook, SpecialistSynapseBus, SpecialistSpmcChannel, TensorSlot, TENSOR_DIM};
pub use spmc_synapse_bus::{SharedSynapseBus, SpmcSynapseBus, SynapsePacket};

pub use persistent_grimoire::{GrimoireRecord, PersistentGrimoireStore};
pub use scheme_router::{CapabilityFlags, SchemeCapabilityGate, SchemeUri};

// Windows-specific named pipe communication (AgentBus)
#[cfg(target_os = "windows")]
pub mod comm;
#[cfg(target_os = "windows")]
pub use comm::AgentBus;

// Backward compatibility: re-export old module structure
pub mod shared_memory {
    pub use crate::swmr_synapse::{
        resolve_synapse_path, McpToolCallFrame, SWMRSynapse, SpecialistDialogue, SynapseState,
    };
}

pub use disruptor::{DisruptorRingBuffer, PaddedAtomicU64, RingBufferEntry};
pub use intent_log::{
    create_log_entry, GenerationSnapshot, IntentLog, LogEntryHeader, LogReader, ReplayReport,
    SnapshotStore, LOG_ENTRY_HEADER_SIZE, LOG_MAGIC,
};
pub use metrics::{MetricsCollector, MetricsSnapshot, SharedMetricsCollector, SlabMetricEntry};
pub use mutation_intent::{IntentQueue, IntentValidator, MutationIntent};
pub use nucleotide_packet::{
    packet_types, priorities, NucleotidePacket, SlabBackedBridge, WASMLinearMemoryBridge,
};
pub use preparedness_notice::{NoticeBroadcast, PreparednessNotice};
pub use slab_allocator::{
    PacketSlot, SlabAllocator, SlabAllocatorWithArena, SlabStats, SLOT_ACTIVE, SLOT_COMMITTED,
    SLOT_ERROR, SLOT_FREE,
};
// Engineering & CS Terminology Aliases (Machine-Native Linking Protocol & IPC)
pub use nucleotide_packet::NucleotidePacket as MachinePacket;
pub use nucleotide_packet::NucleotidePacket as IpcPacket;
pub use nucleotide_packet::WASMLinearMemoryBridge as LinearMemoryBridge;
pub use swmr_synapse::SWMRSynapse as SharedMemoryChannel;
pub use swmr_synapse::SWMRSynapse as SharedMemorySynapse;
pub use specialist_bus::SpecialistSynapseBus as SpecialistIpcBus;
pub use spmc_synapse_bus::SharedSynapseBus as SharedIpcBus;

pub mod machine_packet {
    pub use crate::nucleotide_packet::*;
}

pub mod ipc_bus {
    pub use crate::specialist_bus::*;
    pub use crate::spmc_synapse_bus::*;
}

pub mod shared_memory_channel {
    pub use crate::swmr_synapse::*;
}
