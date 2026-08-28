// Machine-Native Linking Protocol (MNLP) - Machine Packet System
// Zero-allocation aligned binary packet topology for direct linear memory crossing.
// Treats linear memory and host memory as a unified high-speed packet bus.
//
// Integrated with SlabAllocator for fragmentation-free memory management.

use std::mem;

use crate::slab_allocator::{PacketSlot, SlabAllocator};

/// Uniform packet topology - strict binary layout understood by host and guest runtimes
/// #[repr(C, align(8))] ensures predictable memory layout across FFI and IPC boundaries
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct MachinePacket {
    /// Packet type identifier (magic: 0xAA55_0001)
    pub magic: u32,
    /// Packet sequence number (monotonic)
    pub sequence: u64,
    /// Source specialist/agent ID hash
    pub source_id: u64,
    /// Packet type: 0=intent, 1=state_read, 2=notification, 3=response
    pub packet_type: u8,
    /// Priority: 0=low, 1=normal, 2=high, 3=critical
    pub priority: u8,
    /// Schema version for validation
    pub schema_version: u16,
    /// Offset into linear memory where payload begins
    pub payload_offset: u32,
    /// Length of payload in bytes
    pub payload_length: u32,
    /// Checksum for integrity verification
    pub checksum: u32,
    /// Reserved for future use (aligned to 8 bytes)
    pub reserved: [u8; 8],
}

/// Backward compatibility alias
pub type NucleotidePacket = MachinePacket;

impl MachinePacket {
    pub const MAGIC: u32 = 0xAA55_0001;
    pub const HEADER_SIZE: usize = mem::size_of::<MachinePacket>();

    pub fn new(
        sequence: u64,
        source_id: u64,
        packet_type: u8,
        priority: u8,
        payload_offset: u32,
        payload_length: u32,
    ) -> Self {
        let mut pkt = Self {
            magic: Self::MAGIC,
            sequence,
            source_id,
            packet_type,
            priority,
            schema_version: crate::swmr_synapse::SCHEMA_VERSION as u16,
            payload_offset,
            payload_length,
            checksum: 0,
            reserved: [0; 8],
        };
        pkt.checksum = pkt.compute_checksum();
        pkt
    }

    /// Compute simple checksum for integrity verification (excluding checksum field)
    pub fn compute_checksum(&self) -> u32 {
        let mut acc = 0u32;
        acc = acc.wrapping_add(self.magic);
        acc = acc.wrapping_add(self.sequence as u32);
        acc = acc.wrapping_add((self.sequence >> 32) as u32);
        acc = acc.wrapping_add(self.source_id as u32);
        acc = acc.wrapping_add((self.source_id >> 32) as u32);
        acc = acc.wrapping_add(self.packet_type as u32);
        acc = acc.wrapping_add(self.priority as u32);
        acc = acc.wrapping_add(self.schema_version as u32);
        acc = acc.wrapping_add(self.payload_offset);
        acc = acc.wrapping_add(self.payload_length);
        acc = acc.wrapping_add(self.reserved[0] as u32);
        acc = acc.wrapping_add(self.reserved[1] as u32);
        acc = acc.wrapping_add(self.reserved[2] as u32);
        acc = acc.wrapping_add(self.reserved[3] as u32);
        acc = acc.wrapping_add(self.reserved[4] as u32);
        acc = acc.wrapping_add(self.reserved[5] as u32);
        acc = acc.wrapping_add(self.reserved[6] as u32);
        acc = acc.wrapping_add(self.reserved[7] as u32);
        acc
    }

    /// Verify packet integrity
    pub fn verify(&self) -> bool {
        self.magic == Self::MAGIC && self.checksum == self.compute_checksum()
    }

    /// Convert packet to raw bytes for FFI transfer
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts((self as *const Self) as *const u8, mem::size_of::<Self>())
        }
    }

    /// Reconstruct packet from raw bytes (zero-copy)
    pub fn from_bytes(bytes: &[u8]) -> Option<&Self> {
        if bytes.len() < mem::size_of::<Self>() {
            return None;
        }
        let ptr = bytes.as_ptr();
        if (ptr as usize) % mem::align_of::<Self>() != 0 {
            return None;
        }
        unsafe { Some(&*(ptr as *const Self)) }
    }
}

/// Linear Memory FFI Bridge
/// Maps a slice of linear memory directly for zero-allocation crossings
pub struct LinearMemoryBridge<'a> {
    /// Raw pointer to linear memory (mutable for guest writes)
    memory: &'a mut [u8],
    /// Current packet sequence counter
    sequence: u64,
}

/// Backward compatibility alias
pub type WASMLinearMemoryBridge<'a> = LinearMemoryBridge<'a>;

impl<'a> LinearMemoryBridge<'a> {
    pub fn new(memory: &'a mut [u8]) -> Self {
        Self {
            memory,
            sequence: 0,
        }
    }

    /// Read a packet from memory at the given offset
    pub fn read_packet(&self, offset: usize) -> Option<&MachinePacket> {
        if offset + MachinePacket::HEADER_SIZE > self.memory.len() {
            return None;
        }
        let packet_bytes = &self.memory[offset..];
        let packet = MachinePacket::from_bytes(packet_bytes)?;

        if !packet.verify() {
            return None;
        }

        Some(packet)
    }

    /// Read payload data referenced by a packet
    pub fn read_payload<'p>(&'p self, packet: &MachinePacket) -> Option<&'p [u8]> {
        let start = packet.payload_offset as usize;
        let end = start + packet.payload_length as usize;

        if end > self.memory.len() {
            return None;
        }

        Some(&self.memory[start..end])
    }

    /// Write a packet header into memory (guest writes, host reads notification pointer)
    pub fn write_packet_header(
        &mut self,
        offset: usize,
        packet_type: u8,
        priority: u8,
        payload_offset: u32,
        payload_length: u32,
        source_id: u64,
    ) -> Option<u64> {
        if offset + MachinePacket::HEADER_SIZE > self.memory.len() {
            return None;
        }

        self.sequence += 1;
        let packet = MachinePacket::new(
            self.sequence,
            source_id,
            packet_type,
            priority,
            payload_offset,
            payload_length,
        );

        let bytes = packet.as_bytes();
        let dest = &mut self.memory[offset..offset + bytes.len()];
        dest.copy_from_slice(bytes);

        Some(self.sequence)
    }

    /// Get the current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}

/// Packet type constants
pub mod packet_types {
    pub const INTENT: u8 = 0;
    pub const STATE_READ: u8 = 1;
    pub const NOTIFICATION: u8 = 2;
    pub const RESPONSE: u8 = 3;
}

/// Priority constants
pub mod priorities {
    pub const LOW: u8 = 0;
    pub const NORMAL: u8 = 1;
    pub const HIGH: u8 = 2;
    pub const CRITICAL: u8 = 3;
}

/// Slab-backed WASM Linear Memory Bridge
/// Combines the zero-allocation packet bus with slab allocator for fragmentation-free operation
pub struct SlabBackedBridge<'a> {
    /// Slab allocator for packet slots
    slab: &'a mut SlabAllocator,
    /// Optional payload arena for overflow
    arena: Option<&'a mut [u8]>,
    /// Current sequence counter
    sequence: u64,
}

impl<'a> SlabBackedBridge<'a> {
    pub fn new(slab: &'a mut SlabAllocator) -> Self {
        Self {
            slab,
            arena: None,
            sequence: 0,
        }
    }

    pub fn with_arena(slab: &'a mut SlabAllocator, arena: &'a mut [u8]) -> Self {
        Self {
            slab,
            arena: Some(arena),
            sequence: 0,
        }
    }

    /// Allocate a new packet slot and write header
    pub fn allocate_packet(
        &mut self,
        packet_type: u8,
        priority: u8,
        source_id: u64,
        payload: &[u8],
    ) -> Option<&mut PacketSlot> {
        let slot = self.slab.allocate(packet_type, priority, source_id)?;
        self.sequence += 1;
        slot.sequence = self.sequence;

        // Write payload
        if payload.len() <= slot.payload.len() {
            slot.write_payload(payload);
            slot.payload_offset = 0;
        } else if let Some(_arena) = &self.arena {
            // Check if arena has space
            // For simplicity, we use inline payload only in this bridge
            // Full arena management is in SlabAllocatorWithArena
            slot.write_payload(&payload[..slot.payload.len()]);
            slot.payload_offset = 0;
        } else {
            // Truncate to fit
            slot.write_payload(&payload[..slot.payload.len()]);
            slot.payload_offset = 0;
        }

        Some(slot)
    }

    /// Get a committed packet for host consumption
    pub fn get_committed_packet(&self) -> Option<(usize, &PacketSlot)> {
        self.slab.find_committed()
    }

    /// Free a packet slot after host processing
    pub fn free_packet(&mut self, slot_idx: usize) -> bool {
        self.slab.free_by_index(slot_idx)
    }

    /// Free all committed packets (bulk reclaim)
    pub fn free_all_committed(&mut self) -> u16 {
        self.slab.free_committed()
    }

    /// Get slab utilization
    pub fn utilization(&self) -> f32 {
        self.slab.utilization()
    }

    /// Get slab stats
    pub fn stats(&self) -> crate::slab_allocator::SlabStats {
        self.slab.stats()
    }

    /// Get current sequence
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_roundtrip() {
        let packet = NucleotidePacket::new(1, 42, packet_types::INTENT, priorities::HIGH, 100, 64);
        assert!(packet.verify());

        let bytes = packet.as_bytes();
        let reconstructed = NucleotidePacket::from_bytes(bytes).unwrap();
        assert_eq!(reconstructed.sequence, 1);
        assert_eq!(reconstructed.source_id, 42);
        assert!(reconstructed.verify());
    }

    #[test]
    fn test_packet_size() {
        // Should be aligned to 8 bytes
        assert_eq!(NucleotidePacket::HEADER_SIZE % 8, 0);
    }

    #[test]
    fn test_from_bytes_unaligned_rejected() {
        let buffer = vec![0u8; NucleotidePacket::HEADER_SIZE + 8];
        let base_ptr = buffer.as_ptr() as usize;
        let misaligned_offset = if base_ptr % 8 == 0 { 1 } else { 8 - (base_ptr % 8) + 1 };
        let slice = &buffer[misaligned_offset..misaligned_offset + NucleotidePacket::HEADER_SIZE];
        assert_eq!((slice.as_ptr() as usize) % 8, 1);
        assert!(NucleotidePacket::from_bytes(slice).is_none());
    }

    #[test]
    fn test_linear_memory_bridge() {
        let mut memory = vec![0u8; 4096];

        // Write payload at offset 256
        let payload = b"test payload data";
        memory[256..256 + payload.len()].copy_from_slice(payload);

        // Write packet header at offset 0
        let mut bridge = WASMLinearMemoryBridge::new(&mut memory);
        let seq = bridge.write_packet_header(
            0,
            packet_types::INTENT,
            priorities::NORMAL,
            256,
            payload.len() as u32,
            123,
        );
        assert!(seq.is_some());

        // Read back
        let packet = bridge.read_packet(0).unwrap();
        assert_eq!(packet.sequence, seq.unwrap());
        assert_eq!(packet.source_id, 123);

        let read_payload = bridge.read_payload(packet).unwrap();
        assert_eq!(read_payload, payload);
    }

    #[test]
    fn test_slab_backed_bridge_allocate() {
        use crate::slab_allocator::SlabAllocator;

        let mut slab = SlabAllocator::new(8);
        let mut bridge = SlabBackedBridge::new(&mut slab);

        // Allocate a packet
        let payload = b"test intent";
        let slot = bridge.allocate_packet(packet_types::INTENT, priorities::HIGH, 42, payload);
        assert!(slot.is_some());

        let slot = slot.unwrap();
        assert_eq!(slot.sequence, 1);
        assert_eq!(slot.source_id, 42);
        assert_eq!(slot.packet_type, packet_types::INTENT);
        assert_eq!(slot.priority, priorities::HIGH);

        // Commit the packet
        slot.commit();

        // Find committed packet
        let (idx, committed) = bridge.get_committed_packet().unwrap();
        assert_eq!(committed.sequence, 1);
        assert_eq!(committed.source_id, 42);

        // Free the packet
        assert!(bridge.free_packet(idx));
        assert!(bridge.get_committed_packet().is_none());
    }

    #[test]
    fn test_slab_backed_bridge_utilization() {
        use crate::slab_allocator::SlabAllocator;

        let mut slab = SlabAllocator::new(4);
        let mut bridge = SlabBackedBridge::new(&mut slab);

        assert_eq!(bridge.utilization(), 0.0);

        // Allocate 2 slots
        bridge.allocate_packet(0, 1, 1, b"a");
        bridge.allocate_packet(0, 1, 2, b"b");

        assert_eq!(bridge.utilization(), 0.5);

        let stats = bridge.stats();
        assert_eq!(stats.committed_count, 0);
        assert_eq!(stats.free_count, 2);
    }

    #[test]
    fn test_slab_backed_bridge_bulk_free() {
        use crate::slab_allocator::SlabAllocator;

        let mut slab = SlabAllocator::new(8);
        let mut bridge = SlabBackedBridge::new(&mut slab);

        // Allocate and commit 3 packets
        for i in 0..3 {
            let slot = bridge.allocate_packet(0, 1, i + 1, b"data").unwrap();
            slot.commit();
        }

        assert_eq!(bridge.stats().committed_count, 3);

        // Bulk free
        let freed = bridge.free_all_committed();
        assert_eq!(freed, 3);
        assert_eq!(bridge.stats().committed_count, 0);
    }
}
