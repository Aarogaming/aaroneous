// Aaroneous Nervous System - SWMR rkyv Architecture
// Single Writer, Multi-Reader zero-copy shared memory with mutation intent validation.

pub mod disruptor;
pub mod intent_log;
pub mod machine_packet;
pub mod metrics;
pub mod mutation_intent;
pub mod specialist_bus;
pub mod persistent_grimoire;
pub mod preparedness_notice;
pub mod scheme_router;
pub mod slab_allocator;
pub mod spmc_synapse_bus;
pub mod swmr_synapse;

// Backward compatibility module alias
pub mod nucleotide_packet {
    pub use crate::machine_packet::*;
}

pub use specialist_bus::{install_specialist_panic_hook, SpecialistSynapseBus, SpecialistSpmcChannel, TensorSlot, TENSOR_DIM};
pub use spmc_synapse_bus::{SharedSynapseBus, SpmcSynapseBus, SynapsePacket};

pub use persistent_grimoire::{GrimoireRecord, PersistentGrimoireStore, PersistentWalStore, WalRecord};
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
pub use machine_packet::{
    packet_types, priorities, LinearMemoryBridge, MachinePacket, NucleotidePacket,
    SlabBackedBridge, WASMLinearMemoryBridge,
};
pub use preparedness_notice::{NoticeBroadcast, PreparednessNotice};
pub use slab_allocator::{
    PacketSlot, SlabAllocator, SlabAllocatorWithArena, SlabStats, SLOT_ACTIVE, SLOT_COMMITTED,
    SLOT_ERROR, SLOT_FREE,
};
pub use swmr_synapse::{
    McpToolCallFrame, SWMRSynapse, SpecialistDialogue, SynapseReader, SynapseState,
    SynapseWriterHandle,
};

// Engineering & CS Terminology Aliases (Machine-Native Linking Protocol & IPC)
pub use machine_packet::MachinePacket as IpcPacket;
pub use swmr_synapse::SWMRSynapse as SharedMemoryChannel;
pub use swmr_synapse::SWMRSynapse as SharedMemorySynapse;
pub use specialist_bus::SpecialistSynapseBus as SpecialistIpcBus;
pub use spmc_synapse_bus::SharedSynapseBus as SharedIpcBus;

pub mod ipc_bus {
    pub use crate::specialist_bus::*;
    pub use crate::spmc_synapse_bus::*;
}

pub mod shared_memory_channel {
    pub use crate::swmr_synapse::*;
}
