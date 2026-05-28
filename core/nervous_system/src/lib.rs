// Aaroneous Nervous System - SWMR rkyv Architecture
// Single Writer, Multi-Reader zero-copy shared memory with mutation intent validation.

pub mod swmr_synapse;
pub mod mutation_intent;
pub mod preparedness_notice;
pub mod nucleotide_packet;
pub mod slab_allocator;
pub mod intent_log;
pub mod metrics;

// Windows-specific named pipe communication (AgentBus)
#[cfg(target_os = "windows")]
pub mod comm;
#[cfg(target_os = "windows")]
pub use comm::AgentBus;

// Backward compatibility: re-export old module structure
pub mod shared_memory {
    pub use crate::swmr_synapse::{SWMRSynapse, SynapseState, McpToolCallFrame, SpecialistDialogue, resolve_synapse_path};
}

pub use swmr_synapse::{SWMRSynapse, SynapseReader, SynapseWriterHandle};
pub use mutation_intent::{MutationIntent, IntentValidator, IntentQueue};
pub use preparedness_notice::{PreparednessNotice, NoticeBroadcast};
pub use nucleotide_packet::{NucleotidePacket, WASMLinearMemoryBridge, SlabBackedBridge, packet_types, priorities};
pub use slab_allocator::{SlabAllocator, SlabAllocatorWithArena, PacketSlot, SlabStats, SLOT_FREE, SLOT_ACTIVE, SLOT_COMMITTED, SLOT_ERROR};
pub use intent_log::{IntentLog, LogReader, LogEntryHeader, GenerationSnapshot, SnapshotStore, ReplayReport, create_log_entry, LOG_MAGIC, LOG_ENTRY_HEADER_SIZE};
pub use metrics::{MetricsCollector, MetricsSnapshot, SlabMetricEntry, SharedMetricsCollector};

// Backward compatibility alias
pub use swmr_synapse::SWMRSynapse as SharedMemorySynapse;
