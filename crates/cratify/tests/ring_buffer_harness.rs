//! Integration test harness for Inter-ACC Ring Buffer.
//!
//! Simulates concurrent multi-threaded high-velocity message passing
//! through SPSC and MPSC ring buffers to validate lock-free correctness,
//! cache-line isolation, and zero-copy payload integrity.

use cratify::ring::{
    self, AtomicPaddedUsize, ErrorFlags, MpscConsumer, MpscProducer, MpscRing,
    SpscConsumer, SpscProducer, SpscRing, SlotSize,
};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

/// Wrapper to make raw pointers Send for threaded tests.
/// The pointer must remain valid for the lifetime of the threads.
struct SendSlice {
    ptr: *mut u8,
    len: usize,
}
// SAFETY: SendSlice is used only in test harnesses where the underlying
// memory is a stack-allocated Vec that outlives all threads.
unsafe impl Send for SendSlice {}
unsafe impl Sync for SendSlice {}

impl SendSlice {
    /// Reconstruct a mutable slice from the raw pointer.
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid for `self.len` bytes
    /// and that no other thread is writing to the same region.
    unsafe fn as_mut_slice(&self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.ptr, self.len)
    }
}

use std::thread;

// ══════════════════════════════════════════════════════════════════════
//  GROUP 1 — T-Shirt Sized Pool Validation
// ══════════════════════════════════════════════════════════════════════

#[test]
fn all_tier_capacities_are_powers_of_two() {
    for size in [SlotSize::XS, SlotSize::S, SlotSize::M, SlotSize::L, SlotSize::XL] {
        let cap = size.capacity();
        assert!(cap > 0 && cap.is_power_of_two(),
            "{:?} capacity {} is not a power of two", size, cap);
    }
}

#[test]
fn slot_sizes_monotonically_increasing() {
    let sizes = [SlotSize::XS, SlotSize::S, SlotSize::M, SlotSize::L, SlotSize::XL];
    for window in sizes.windows(2) {
        assert!(window[0].capacity() < window[1].capacity(),
            "{:?} ({}) >= {:?} ({})",
            window[0], window[0].capacity(), window[1], window[1].capacity());
    }
}

#[test]
fn xs_slot_fits_u64() {
    let ring = SpscRing::new(4, SlotSize::XS.capacity()).unwrap();
    assert!(std::mem::size_of::<u64>() + 8 <= ring.slot_size);
}

#[test]
fn xl_slot_fits_large_struct() {
    let ring = SpscRing::new(4, SlotSize::XL.capacity()).unwrap();
    // XL = 64KB — easily fits a 1KB payload.
    assert!(1024 + 8 <= ring.slot_size);
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 2 — Cache-Line Padding Verification
// ══════════════════════════════════════════════════════════════════════

#[test]
fn atomic_padded_usize_size_at_least_two_cache_lines() {
    let size = std::mem::size_of::<AtomicPaddedUsize>();
    assert!(size >= 128,
        "AtomicPaddedUsize is {} bytes, expected >= 128 (2 cache lines)", size);
}

#[test]
fn two_padded_atomics_do_not_share_cache_line() {
    let a = AtomicPaddedUsize::new(0);
    let b = AtomicPaddedUsize::new(0);
    let a_addr = &a as *const _ as usize;
    let b_addr = &b as *const _ as usize;
    let distance = if a_addr > b_addr { a_addr - b_addr } else { b_addr - a_addr };
    // They should be at least 128 bytes apart (each is 128+ bytes).
    assert!(distance >= 128,
        "two AtomicPaddedUsize instances are only {} bytes apart", distance);
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 3 — SPSC Concurrent Single-Thread Roundtrip
// ══════════════════════════════════════════════════════════════════════

#[test]
fn spsc_1000_sequential_messages() {
    let ring = SpscRing::new(1024, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        for i in 0..1000u64 {
            producer.write_raw(&i).unwrap();
        }
    }

    {
        let consumer = SpscConsumer::new(&ring, &data);
        for i in 0..1000u64 {
            let (val, _hdr) = consumer.read::<u64>().unwrap();
            assert_eq!(val, i, "slot {} mismatch", i);
        }
        assert!(consumer.read::<u64>().is_none());
    }
}

#[test]
fn spsc_large_payload() {
    let slot = SlotSize::L.capacity();
    let ring = SpscRing::new(4, slot).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    // Write a payload that fills most of the slot.
    let payload = vec![0xABu8; slot - 8]; // minus 8 for header
    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        producer.write_bytes(&payload).unwrap();
    }

    {
        let consumer = SpscConsumer::new(&ring, &data);
        let (_idx, read_payload, _hdr) = consumer.try_read_bytes().unwrap();
        assert_eq!(&read_payload[..payload.len()], payload.as_slice());
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 4 — SPSC Multi-Threaded Producer/Consumer
// ══════════════════════════════════════════════════════════════════════

#[test]
fn spsc_threaded_throughput() {
    let n: usize = 10_000;
    let ring = Arc::new(SpscRing::new(1024, 64).unwrap());
    let mut data = vec![0u8; ring.data_bytes()];

    // Producer runs on a thread, consumer on main thread — concurrent.
    let ring_p = ring.clone();
    let ss = SendSlice { ptr: data.as_mut_ptr(), len: data.len() };
    let producer_handle = thread::spawn(move || {
        let buf = unsafe { ss.as_mut_slice() };
        let mut producer = SpscProducer::new(&ring_p, buf);
        for i in 0..n as u64 {
            producer.write_raw(&i).unwrap();
        }
    });

    // Consumer on main thread — drains concurrently with producer.
    {
        let consumer = SpscConsumer::new(&ring, &data);
        let mut received = Vec::new();
        while received.len() < n {
            if let Some((val, _hdr)) = consumer.read::<u64>() {
                received.push(val);
            } else {
                std::hint::spin_loop();
            }
        }
        assert_eq!(received.len(), n);
        for (i, val) in received.iter().enumerate() {
            assert_eq!(*val, i as u64, "message {} mismatch: got {}", i, val);
        }
    }

    producer_handle.join().unwrap();
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 5 — MPSC Multi-Threaded Producers
// ══════════════════════════════════════════════════════════════════════

#[test]
fn mpsc_two_producers_no_collision() {
    let n_per_producer = 500;
    let total = n_per_producer * 2;
    let ring = Arc::new(MpscRing::new(1024, 64).unwrap());
    let mut data = vec![0u8; ring.data_bytes()];

    // Producers run concurrently, consumer on main thread drains concurrently.
    {
        let mut handles = Vec::new();
        for producer_id in 0u64..2 {
            let ring_c = ring.clone();
            let ss = SendSlice { ptr: data.as_mut_ptr(), len: data.len() };
            handles.push(thread::spawn(move || {
                let buf = unsafe { ss.as_mut_slice() };
                let mut producer = MpscProducer::new(&ring_c, buf);
                for i in 0..n_per_producer as u64 {
                    let msg = producer_id * n_per_producer as u64 + i;
                    producer.write_raw(&msg).unwrap();
                }
            }));
        }

        // Consumer on main thread — drains concurrently.
        {
            let consumer = MpscConsumer::new(&ring, &data);
            let mut received = Vec::new();
            while received.len() < total {
                if let Some((val, _hdr)) = consumer.read::<u64>() {
                    received.push(val);
                } else {
                    std::hint::spin_loop();
                }
            }

            assert_eq!(received.len(), total);
            received.sort();
            for i in 0..total as u64 {
                assert!(received.contains(&i), "missing message {}", i);
            }
        }

        for h in handles {
            h.join().unwrap();
        }
    }
}

#[test]
fn mpsc_four_producers_high_contention() {
    let n_per_producer = 250;
    let num_producers = 4;
    let total = n_per_producer * num_producers;
    let ring = Arc::new(MpscRing::new(512, 64).unwrap());
    let mut data = vec![0u8; ring.data_bytes()];

    {
        let mut handles = Vec::new();
        for pid in 0u64..num_producers as u64 {
            let ring_c = ring.clone();
            let ss = SendSlice { ptr: data.as_mut_ptr(), len: data.len() };
            handles.push(thread::spawn(move || {
                let buf = unsafe { ss.as_mut_slice() };
                let mut producer = MpscProducer::new(&ring_c, buf);
                for i in 0..n_per_producer as u64 {
                    let msg = pid * n_per_producer as u64 + i;
                    producer.write_raw(&msg).unwrap();
                }
            }));
        }

        // Consumer on main thread — drains concurrently.
        {
            let consumer = MpscConsumer::new(&ring, &data);
            let mut received = Vec::new();
            while received.len() < total {
                if let Some((val, _hdr)) = consumer.read::<u64>() {
                    received.push(val);
                } else {
                    std::hint::spin_loop();
                }
            }

            assert_eq!(received.len(), total);
            received.sort();
            for i in 0..total as u64 {
                assert!(received.contains(&i), "missing message {}", i);
            }
        }

        for h in handles {
            h.join().unwrap();
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 6 — Error Flag Propagation
// ══════════════════════════════════════════════════════════════════════

#[test]
fn error_flag_payload_exceeded_roundtrip() {
    let ring = SpscRing::new(4, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        producer.write_with_flags(b"bad", ErrorFlags::PAYLOAD_EXCEEDED).unwrap();
    }

    {
        let consumer = SpscConsumer::new(&ring, &data);
        let (_idx, _payload, hdr) = consumer.try_read_bytes().unwrap();
        let flags = ErrorFlags(hdr.flags);
        assert!(flags.contains(ErrorFlags::PAYLOAD_EXCEEDED));
        assert!(!flags.contains(ErrorFlags::CRC_MISMATCH));
        assert!(!flags.contains(ErrorFlags::CONSUMER_TIMEOUT));
    }
}

#[test]
fn error_flag_combined_flags_roundtrip() {
    let ring = SpscRing::new(4, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];
    let combined = ErrorFlags::PAYLOAD_EXCEEDED | ErrorFlags::CONSUMER_TIMEOUT;

    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        producer.write_with_flags(b"err", combined).unwrap();
    }

    {
        let consumer = SpscConsumer::new(&ring, &data);
        let (_idx, _payload, hdr) = consumer.try_read_bytes().unwrap();
        let flags = ErrorFlags(hdr.flags);
        assert!(flags.contains(ErrorFlags::PAYLOAD_EXCEEDED));
        assert!(flags.contains(ErrorFlags::CONSUMER_TIMEOUT));
        assert!(!flags.contains(ErrorFlags::CRC_MISMATCH));
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 7 — IpcHeader Integration
// ══════════════════════════════════════════════════════════════════════

#[test]
fn ipc_header_pod_roundtrip_through_ring() {
    use core_contracts::IpcHeader;

    let ring = SpscRing::new(4, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    let header = IpcHeader {
        dst: 1,
        src: 2,
        len: 32,
        flags: 0,
    };

    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        producer.write_raw(&header).unwrap();
    }

    {
        let consumer = SpscConsumer::new(&ring, &data);
        let (read_header, _hdr) = consumer.read::<IpcHeader>().unwrap();
        assert_eq!(read_header.dst, 1);
        assert_eq!(read_header.src, 2);
        assert_eq!(read_header.len, 32);
        assert_eq!(read_header.flags, 0);
    }
}

#[test]
fn ipc_header_with_flags_through_ring() {
    use core_contracts::IpcHeader;

    let ring = SpscRing::new(4, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    let header = IpcHeader {
        dst: 10,
        src: 20,
        len: 64,
        flags: 0x1, // PAYLOAD_EXCEEDED
    };

    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        producer.write_raw(&header).unwrap();
    }

    {
        let consumer = SpscConsumer::new(&ring, &data);
        let (read_header, _hdr) = consumer.read::<IpcHeader>().unwrap();
        assert_eq!(read_header.dst, 10);
        assert_eq!(read_header.flags, 0x1);
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 8 — Slot Header Sequence Number Integrity
// ══════════════════════════════════════════════════════════════════════

#[test]
fn sequence_numbers_are_monotonically_increasing() {
    let ring = SpscRing::new(8, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        for _ in 0..8 {
            producer.write_raw(&0_u64).unwrap();
        }
    }

    {
        let consumer = SpscConsumer::new(&ring, &data);
        for i in 0..8u32 {
            let hdr = consumer.try_peek_header().unwrap();
            assert_eq!(hdr.sequence, i + 1,
                "slot {} has sequence {}, expected {}", i, hdr.sequence, i + 1);
            consumer.consume();
        }
    }
}

#[test]
fn peek_header_does_not_advance_cursor() {
    let ring = SpscRing::new(4, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        producer.write_raw(&42_u64).unwrap();
    }

    {
        let consumer = SpscConsumer::new(&ring, &data);
        let hdr1 = consumer.try_peek_header().unwrap();
        let hdr2 = consumer.try_peek_header().unwrap();
        assert_eq!(hdr1.sequence, hdr2.sequence);
        assert_eq!(consumer.available(), 1);
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 9 — Wrap-Around correctness
// ══════════════════════════════════════════════════════════════════════

#[test]
fn spsc_wrap_around_100_cycles() {
    let ring = SpscRing::new(4, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    for cycle in 0..100u64 {
        {
            let mut producer = SpscProducer::new(&ring, &mut data);
            producer.write_raw(&cycle).unwrap();
        }
        {
            let consumer = SpscConsumer::new(&ring, &data);
            let (val, _hdr) = consumer.read::<u64>().unwrap();
            assert_eq!(val, cycle);
        }
    }
}

#[test]
fn mpsc_wrap_around_100_cycles() {
    let ring = MpscRing::new(4, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    for cycle in 0..100u64 {
        {
            let mut producer = MpscProducer::new(&ring, &mut data);
            producer.write_raw(&cycle).unwrap();
        }
        {
            let consumer = MpscConsumer::new(&ring, &data);
            let (val, _hdr) = consumer.read::<u64>().unwrap();
            assert_eq!(val, cycle);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 10 — Mixed Payload Types
// ══════════════════════════════════════════════════════════════════════

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
struct TestPacket {
    value: u64,
    id: u32,
    flag: u8,
    _pad: [u8; 3], // Explicit padding to ensure Pod compliance.
}

#[test]
fn mixed_payload_struct_roundtrip() {
    let ring = SpscRing::new(4, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    let packets = [
        TestPacket { value: 100, id: 1, flag: 0xFF, _pad: [0; 3] },
        TestPacket { value: 200, id: 2, flag: 0x00, _pad: [0; 3] },
        TestPacket { value: u64::MAX, id: 3, flag: 0x42, _pad: [0; 3] },
    ];

    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        for pkt in &packets {
            producer.write_raw(pkt).unwrap();
        }
    }

    {
        let consumer = SpscConsumer::new(&ring, &data);
        for expected in &packets {
            let (pkt, _hdr) = consumer.read::<TestPacket>().unwrap();
            assert_eq!(&pkt, expected);
        }
    }
}

#[test]
fn heterogeneous_sizes_interleaved() {
    let ring = SpscRing::new(8, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        producer.write_raw(&1_u8).unwrap();
        producer.write_raw(&0xDEAD_BEEF_u32).unwrap();
        producer.write_raw(&0x0102030405060708_u64).unwrap();
        producer.write_raw(&99999_u32).unwrap();
    }

    {
        let consumer = SpscConsumer::new(&ring, &data);
        assert_eq!(consumer.read::<u8>().unwrap().0, 1);
        assert_eq!(consumer.read::<u32>().unwrap().0, 0xDEAD_BEEF);
        assert_eq!(consumer.read::<u64>().unwrap().0, 0x0102030405060708);
        assert_eq!(consumer.read::<u32>().unwrap().0, 99999);
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 11 — Configuration Validation
// ══════════════════════════════════════════════════════════════════════

#[test]
fn validate_configs_for_all_tier_sizes() {
    for tier in [SlotSize::XS, SlotSize::S, SlotSize::M, SlotSize::L, SlotSize::XL] {
        assert!(ring::validate_config(8, tier.capacity()).is_ok(),
            "failed to validate config for {:?}", tier);
    }
}

#[test]
fn shared_memory_size_matches_product() {
    for tier in [SlotSize::XS, SlotSize::S, SlotSize::M, SlotSize::L, SlotSize::XL] {
        let cap = 8;
        let expected = cap * tier.capacity();
        assert_eq!(ring::shared_memory_size(cap, tier.capacity()), expected);
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 12 — Stress: Full Buffer Drain & Refill
// ══════════════════════════════════════════════════════════════════════

#[test]
fn spsc_fill_drain_fill_drain() {
    let ring = SpscRing::new(4, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    // Fill.
    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        for i in 0..4u64 {
            producer.write_raw(&i).unwrap();
        }
    }
    // Drain.
    {
        let consumer = SpscConsumer::new(&ring, &data);
        for i in 0..4u64 {
            assert_eq!(consumer.read::<u64>().unwrap().0, i);
        }
        assert!(consumer.read::<u64>().is_none());
    }
    // Refill.
    {
        let mut producer = SpscProducer::new(&ring, &mut data);
        for i in 100..104u64 {
            producer.write_raw(&i).unwrap();
        }
    }
    // Drain again.
    {
        let consumer = SpscConsumer::new(&ring, &data);
        for i in 100..104u64 {
            assert_eq!(consumer.read::<u64>().unwrap().0, i);
        }
    }
}

#[test]
fn mpsc_fill_drain_fill_drain() {
    let ring = MpscRing::new(4, 64).unwrap();
    let mut data = vec![0u8; ring.data_bytes()];

    {
        let mut producer = MpscProducer::new(&ring, &mut data);
        for i in 0..4u64 {
            producer.write_raw(&i).unwrap();
        }
    }
    {
        let consumer = MpscConsumer::new(&ring, &data);
        for i in 0..4u64 {
            assert_eq!(consumer.read::<u64>().unwrap().0, i);
        }
    }
    {
        let mut producer = MpscProducer::new(&ring, &mut data);
        for i in 200..204u64 {
            producer.write_raw(&i).unwrap();
        }
    }
    {
        let consumer = MpscConsumer::new(&ring, &data);
        for i in 200..204u64 {
            assert_eq!(consumer.read::<u64>().unwrap().0, i);
        }
    }
}
