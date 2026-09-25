// Aaroneous Nervous System - SWMR rkyv Architecture
// Single Writer, Multi-Reader zero-copy shared memory with mutation intent validation.
//
// This crate legitimately needs `unsafe` for shared-memory mmap and
// lock-free ring-buffer operations (see AGENTS.md's "No Heap on Hot Paths"
// rule, which groups it with core/hypervisor and crates/compute as a
// hot-path crate). `#![warn(unsafe_code)]` - the AGENTS.md-documented
// permission for such "performance/kernel" crates - is deliberately *not*
// declared here: this repo's verification gate runs every crate through
// `cargo clippy -- -D warnings`, which promotes any `#[warn(...)]`-level
// lint to a hard build error, indistinguishable from `#![deny(unsafe_code)]`
// in practice. The actual enforcement for this crate is the ast_auditor's
// `SafetyCommentVisitor` (see crates/ast_auditor/src/rules/safety_comments.rs),
// which requires a documented `// SAFETY:` comment on every unsafe block
// without blocking compilation.

pub mod protocol;
pub use protocol::IpcEvent;

pub mod disruptor;
pub mod flight_recorder;
pub mod intent_log;
pub mod machine_packet;
pub mod metrics;
pub mod mutation_intent;
pub mod observation_buffer;
pub mod persistent_wal;
pub mod specialist_bus;
pub use persistent_wal as wal_store;
pub mod preparedness_notice;
pub mod scheme_router;
pub mod shared_channel;
pub mod slab_allocator;
pub mod spmc_shm_bus;
pub mod swmr_shm;
pub use shared_channel as synapse;
pub use spmc_shm_bus as spmc_synapse_bus;
pub use swmr_shm as swmr_synapse;
pub mod universal_event_bus;
pub mod universal_protocol;

pub use universal_event_bus::{EventEnvelope, EventSubscriber, SequenceBarrier, UniversalEventBus};

pub use universal_protocol::{
    AssimilationPhase, AssimilationRecord, UCP_DEFAULT_WS_PORT, UCP_NAMED_PIPE_PATH,
    UCP_PROTOCOL_VERSION, UcpBroadcastType, UcpRequestType, UniversalClientRequest,
    UniversalServerBroadcast,
};

// Backward compatibility module alias
pub mod nucleotide_packet {
    pub use crate::machine_packet::*;
}

pub use specialist_bus::{
    SpecialistSpmcChannel, SpecialistSynapseBus, TENSOR_DIM, TensorSlot,
    install_specialist_panic_hook,
};
pub use spmc_synapse_bus::{SharedSynapseBus, SpmcSynapseBus, SynapsePacket};

pub use persistent_wal::{PersistentWalStore, WalRecord};
pub use scheme_router::{CapabilityFlags, SchemeCapabilityGate, SchemeUri};

// Windows-specific named pipe communication (AgentBus)
#[cfg(target_os = "windows")]
pub mod comm;
#[cfg(target_os = "windows")]
pub use comm::AgentBus;

// Backward compatibility: re-export old module structure
pub mod shared_memory {
    pub use crate::swmr_synapse::{
        IpcBusState, McpToolCallFrame, SWMRSynapse, SpecialistDialogue, resolve_synapse_path,
    };
    pub type SynapseState = IpcBusState;
}

pub use core_contracts::EngineSnapshotPod;
pub use core_contracts::ObservationFramePod;
pub use core_contracts::{FlightEventKind, FlightEventPod, FlightFileHeaderPod};
pub use disruptor::{
    CacheAlignedAtomicU64, CacheAlignedAtomicUsize, DisruptorRingBuffer, PaddedAtomicU64,
    RingBufferEntry,
};
pub use flight_recorder::{
    FLIGHT_HEADER_SIZE, FLIGHT_LOG_SIZE, FLIGHT_MAGIC, FLIGHT_MAX_SLOTS, FLIGHT_SLOT_SIZE,
    FLIGHT_VERSION, FlightRecorder, FlightRecorderError, FlightReplayIterator, FlightReplayer,
};
pub use intent_log::{
    GenerationSnapshot, IntentLog, LOG_ENTRY_HEADER_SIZE, LOG_MAGIC, LogEntryHeader, LogReader,
    ReplayReport, SnapshotStore, create_log_entry,
};
pub use machine_packet::{
    AlignedBitstreamPacket, LinearMemoryBridge, MachinePacket, SlabBackedBridge,
    WASMLinearMemoryBridge, packet_types, priorities,
};
pub use metrics::{MetricsCollector, MetricsSnapshot, SharedMetricsCollector, SlabMetricEntry};
pub use mutation_intent::{IntentQueue, IntentValidator, MutationIntent};
pub use observation_buffer::{
    DEFAULT_OBSERVATION_CAPACITY, OBSERVATION_FRAME_SIZE, OBSERVATION_HEADER_SIZE,
    OBSERVATION_MAGIC, OBSERVATION_SLOT_SIZE, OBSERVATION_VERSION, ObservationBuffer,
    ObservationBufferError, ObservationBufferHeaderPod,
};
pub use preparedness_notice::{NoticeBroadcast, PreparednessNotice};
pub use slab_allocator::{
    PacketSlot, SLOT_ACTIVE, SLOT_COMMITTED, SLOT_ERROR, SLOT_FREE, SlabAllocator,
    SlabAllocatorWithArena, SlabStats,
};
pub use swmr_synapse::{
    IpcBusState, McpToolCallFrame, SNAPSHOT_RING_SLOTS, SNAPSHOT_SEGMENT_SIZE, SNAPSHOT_SHM_MAGIC,
    SNAPSHOT_SHM_VERSION, SWMRSynapse, SnapshotReadEntry, SnapshotRingHeader, SnapshotRingSlot,
    SpecialistDialogue, SwmrSnapshotPublisher, SwmrSnapshotReader, SynapseReader,
    SynapseWriterHandle,
};
pub type SynapseState = IpcBusState;

// Engineering & CS Terminology Aliases (Machine-Native Linking Protocol & IPC)
pub use machine_packet::MachinePacket as IpcPacket;
pub use specialist_bus::SpecialistSynapseBus as SpecialistIpcBus;
pub use spmc_synapse_bus::SharedSynapseBus as SharedIpcBus;
pub use swmr_synapse::SWMRSynapse as SharedMemoryChannel;
pub use swmr_synapse::SWMRSynapse as SharedMemorySynapse;

pub mod ipc_bus {
    pub use crate::specialist_bus::*;
    pub use crate::spmc_synapse_bus::*;
}

pub mod shared_memory_channel {
    pub use crate::swmr_synapse::*;
}
