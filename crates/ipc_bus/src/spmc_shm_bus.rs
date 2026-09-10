//! crates/ipc_bus/src/spmc_synapse_bus.rs
//! Lock-Free Single-Producer Multi-Consumer (SPMC) Synapse Bus using Crossbeam ArrayQueue.
//! 
//! Key Performance & Architectural Pillars:
//! 1. Lock-Free CAS: Bounded array queue with zero mutex locking.
//! 2. 128-Byte Cache Alignment: Prevents hardware cache-line bouncing (false sharing) across multi-core consumers.
//! 3. Zero-Allocation Hot Path: Pre-allocated 4096-packet ring buffer.

use std::sync::Arc;
use crossbeam::queue::ArrayQueue;

/// A fixed-size message packet transmitted across the synapse bus.
/// Designed to fit compactly within cache lines for sub-microsecond transit.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct SynapsePacket {
    pub source_id: u32,          // Originating subsystem identifier (e.g. Cortex = 1, Adaptation Engine = 2)
    pub timestamp_ns: u64,       // High-resolution hardware timestamp
    pub intent_vector: [f32; 4], // Compact summary of active R^256 intent or state
    pub opcode_trigger: u32,     // Associated machine opcode or diagnostic flag
}

impl Default for SynapsePacket {
    fn default() -> Self {
        Self {
            source_id: 0,
            timestamp_ns: 0,
            intent_vector: [0.0; 4],
            opcode_trigger: 0,
        }
    }
}

/// 128-byte alignment attribute prevents CPU cache-line false sharing 
/// between the single producer thread and multiple consumer specialist threads.
#[repr(align(128))]
pub struct SpmcSynapseBus {
    // A bounded, lock-free ring buffer. 
    // Capacity set to 4096 packets to prevent allocation under heavy load.
    channel: ArrayQueue<SynapsePacket>,
}

impl Default for SpmcSynapseBus {
    fn default() -> Self {
        Self::new()
    }
}

impl SpmcSynapseBus {
    pub fn new() -> Self {
        Self {
            channel: ArrayQueue::new(4096),
        }
    }

    /// Creates a bus with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            channel: ArrayQueue::new(capacity),
        }
    }

    /// [Producer Only] Push a new state packet onto the bus.
    /// Operates via atomic Compare-And-Swap (CAS) with zero mutex locks.
    #[inline(always)]
    pub fn broadcast(&self, packet: SynapsePacket) -> Result<(), SynapsePacket> {
        self.channel.push(packet)
    }

    /// [Consumer] Pull the next available packet for processing 
    /// (Used by Desktop Emulator, Adaptation Engine, or the egui Telemetry HUD).
    #[inline(always)]
    pub fn consume(&self) -> Option<SynapsePacket> {
        self.channel.pop()
    }

    /// Check current backlog pressure on the bus
    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.channel.len()
    }

    /// Check if the bus is currently empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.channel.is_empty()
    }

    /// Check total queue capacity
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.channel.capacity()
    }
}

/// Shared thread-safe handle for multi-threaded consumer access
pub type SharedSynapseBus = Arc<SpmcSynapseBus>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_spmc_bus_broadcast_and_consume() {
        let bus = SpmcSynapseBus::new();
        assert_eq!(bus.pending_count(), 0);
        assert!(bus.is_empty());

        let pkt = SynapsePacket {
            source_id: 1,
            timestamp_ns: 123456789,
            intent_vector: [0.1, 0.2, 0.3, 0.4],
            opcode_trigger: 0x42,
        };

        assert!(bus.broadcast(pkt.clone()).is_ok());
        assert_eq!(bus.pending_count(), 1);
        assert!(!bus.is_empty());

        let consumed = bus.consume().expect("Packet should be available");
        assert_eq!(consumed, pkt);
        assert_eq!(bus.pending_count(), 0);
    }

    #[test]
    fn test_spmc_bus_multi_consumer_concurrency() {
        let bus = Arc::new(SpmcSynapseBus::with_capacity(512));
        let num_messages = 200;

        // Produce 200 messages
        for i in 0..num_messages {
            let pkt = SynapsePacket {
                source_id: 1,
                timestamp_ns: i as u64,
                intent_vector: [i as f32, 0.0, 0.0, 0.0],
                opcode_trigger: i,
            };
            bus.broadcast(pkt).unwrap();
        }

        // Spawn 4 concurrent consumer threads
        let mut handles = Vec::new();
        for _ in 0..4 {
            let bus_clone = Arc::clone(&bus);
            handles.push(thread::spawn(move || {
                let mut count = 0;
                while let Some(_pkt) = bus_clone.consume() {
                    count += 1;
                }
                count
            }));
        }

        let total_consumed: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
        assert_eq!(total_consumed, num_messages as usize);
        assert!(bus.is_empty());
    }

    #[test]
    fn test_spmc_bus_alignment() {
        assert_eq!(std::mem::align_of::<SpmcSynapseBus>(), 128);
    }
}
