// Aaroneous Nervous System - SWMR rkyv Architecture
// Single Writer, Multi-Reader zero-copy shared memory with mutation intent validation.

pub mod intent_log;
pub mod metrics;
pub mod mutation_intent;
pub mod nucleotide_packet;
pub mod preparedness_notice;
pub mod slab_allocator;
pub mod swmr_synapse;

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
pub use swmr_synapse::{SWMRSynapse, SynapseReader, SynapseWriterHandle};

// Backward compatibility alias
pub use swmr_synapse::SWMRSynapse as SharedMemorySynapse;
