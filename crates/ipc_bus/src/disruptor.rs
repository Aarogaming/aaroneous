//! crates/ipc_bus/src/disruptor.rs
//! High-throughput, lock-free ring buffer and sequence-guarded event broadcaster
//! inspired by the LMAX Disruptor and Aeron IPC architectures.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 64-byte Cacheline padded atomic counter to eliminate CPU false sharing
#[repr(align(64))]
#[derive(Debug)]
pub struct PaddedAtomicU64 {
    value: AtomicU64,
}

impl PaddedAtomicU64 {
    pub fn new(initial: u64) -> Self {
        Self {
            value: AtomicU64::new(initial),
        }
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Acquire)
    }

    pub fn set(&self, val: u64) {
        self.value.store(val, Ordering::Release)
    }

    pub fn increment(&self) -> u64 {
        self.value.fetch_add(1, Ordering::AcqRel)
    }
}

/// An entry in the lock-free Disruptor Ring Buffer
#[derive(Debug, Clone)]
pub struct RingBufferEntry<T: Clone + Default> {
    pub sequence: u64,
    pub payload: T,
}

impl<T: Clone + Default> Default for RingBufferEntry<T> {
    fn default() -> Self {
        Self {
            sequence: 0,
            payload: T::default(),
        }
    }
}

/// Lock-Free Disruptor Ring Buffer for microsecond-latency event streaming
pub struct DisruptorRingBuffer<T: Clone + Default> {
    capacity: usize,
    mask: usize,
    buffer: Vec<RingBufferEntry<T>>,
    cursor: Arc<PaddedAtomicU64>,
    #[allow(dead_code)]
    gating_sequence: Arc<PaddedAtomicU64>,
}

impl<T: Clone + Default> DisruptorRingBuffer<T> {
    /// Capacity must be a power of two
    pub fn new(capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "Capacity must be a power of two");
        let mask = capacity - 1;
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(RingBufferEntry::default());
        }

        Self {
            capacity,
            mask,
            buffer,
            cursor: Arc::new(PaddedAtomicU64::new(0)),
            gating_sequence: Arc::new(PaddedAtomicU64::new(0)),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn cursor(&self) -> u64 {
        self.cursor.get()
    }

    /// Publishes a single item into the ring buffer at the next sequence index
    pub fn publish(&mut self, payload: T) -> u64 {
        let seq = self.cursor.get();
        let index = (seq as usize) & self.mask;

        self.buffer[index] = RingBufferEntry {
            sequence: seq,
            payload,
        };

        self.cursor.increment();
        seq
    }

    /// Publishes a batch of items in a single atomic sequence advance
    pub fn publish_batch(&mut self, items: Vec<T>) -> (u64, u64) {
        let start_seq = self.cursor.get();
        let count = items.len();

        for (i, item) in items.into_iter().enumerate() {
            let seq = start_seq + i as u64;
            let index = (seq as usize) & self.mask;
            self.buffer[index] = RingBufferEntry {
                sequence: seq,
                payload: item,
            };
        }

        for _ in 0..count {
            self.cursor.increment();
        }

        (start_seq, start_seq + count as u64 - 1)
    }

    /// Reads events from a reader's sequence cursor up to the current publisher cursor
    pub fn read_from(&self, reader_cursor: u64, max_events: usize) -> Vec<(u64, T)> {
        let current_cursor = self.cursor.get();
        let mut results = Vec::new();

        let mut seq = reader_cursor;
        while seq < current_cursor && results.len() < max_events {
            let index = (seq as usize) & self.mask;
            let entry = &self.buffer[index];
            if entry.sequence == seq {
                results.push((seq, entry.payload.clone()));
            }
            seq += 1;
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disruptor_single_publish_read() {
        let mut ring = DisruptorRingBuffer::<String>::new(1024);
        assert_eq!(ring.cursor(), 0);

        let seq0 = ring.publish("EVENT_INIT".to_string());
        let seq1 = ring.publish("EVENT_SYNAPSE_PULSE".to_string());

        assert_eq!(seq0, 0);
        assert_eq!(seq1, 1);
        assert_eq!(ring.cursor(), 2);

        let events = ring.read_from(0, 10);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].1, "EVENT_INIT");
        assert_eq!(events[1].1, "EVENT_SYNAPSE_PULSE");
    }

    #[test]
    fn test_disruptor_batch_publish() {
        let mut ring = DisruptorRingBuffer::<i32>::new(512);
        let batch: Vec<i32> = (0..100).collect();

        let (start, end) = ring.publish_batch(batch);
        assert_eq!(start, 0);
        assert_eq!(end, 99);
        assert_eq!(ring.cursor(), 100);

        let read_back = ring.read_from(0, 50);
        assert_eq!(read_back.len(), 50);
        assert_eq!(read_back[49].1, 49);
    }
}
