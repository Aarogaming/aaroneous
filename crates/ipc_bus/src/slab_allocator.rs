// WASM Slab Allocator
// Fixed-grid uniform packet slots inside WASM linear memory.
// Prevents memory fragmentation by pre-allocating uniform slots
// and using a free list for O(1) allocation/deallocation.

/// Slot status flags
pub const SLOT_FREE: u8 = 0;
pub const SLOT_ACTIVE: u8 = 1;
pub const SLOT_COMMITTED: u8 = 2;
pub const SLOT_ERROR: u8 = 3;

/// Fixed-grid uniform packet slot - 256 bytes each
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct PacketSlot {
    /// Monotonic sequence number
    pub sequence: u64,
    /// Slot status: 0=Free, 1=Active, 2=Committed, 3=Error
    pub status: u8,
    /// Packet type: 0=Intent, 1=StateRead, 2=Notification, 3=Response
    pub packet_type: u8,
    /// Priority: 0=Low, 1=Normal, 2=High, 3=Critical
    pub priority: u8,
    /// Reserved for alignment
    pub _padding: u8,
    /// Offset into payload arena for overflow data
    pub payload_offset: u32,
    /// Length of payload in bytes
    pub payload_length: u32,
    /// Checksum for integrity verification
    pub checksum: u32,
    /// Source agent/specialist ID hash
    pub source_id: u64,
    /// Generation counter at time of allocation
    pub generation: u64,
    /// Inline payload buffer (fits in single slot)
    pub payload: [u8; 208],
}

impl PacketSlot {
    pub const SIZE: usize = 256;

    pub fn new() -> Self {
        Self {
            sequence: 0,
            status: SLOT_FREE,
            packet_type: 0,
            priority: 0,
            _padding: 0,
            payload_offset: 0,
            payload_length: 0,
            checksum: 0,
            source_id: 0,
            generation: 0,
            payload: [0; 208],
        }
    }

    pub fn is_free(&self) -> bool {
        self.status == SLOT_FREE
    }

    pub fn is_active(&self) -> bool {
        self.status == SLOT_ACTIVE
    }

    pub fn is_committed(&self) -> bool {
        self.status == SLOT_COMMITTED
    }

    pub fn is_error(&self) -> bool {
        self.status == SLOT_ERROR
    }

    /// Write payload data into the slot's inline buffer
    pub fn write_payload(&mut self, data: &[u8]) -> bool {
        if data.len() > self.payload.len() {
            return false;
        }
        self.payload[..data.len()].copy_from_slice(data);
        self.payload_length = data.len() as u32;
        true
    }

    /// Read payload data from the slot's inline buffer
    pub fn read_payload(&self) -> &[u8] {
        &self.payload[..self.payload_length as usize]
    }

    /// Compute checksum over header fields + payload (excluding checksum field itself)
    pub fn compute_checksum(&self) -> u32 {
        let mut acc = 0u32;
        // Hash fields before checksum
        acc = acc.wrapping_add(self.sequence as u32);
        acc = acc.wrapping_add(self.status as u32);
        acc = acc.wrapping_add(self.packet_type as u32);
        acc = acc.wrapping_add(self.priority as u32);
        acc = acc.wrapping_add(self.payload_offset);
        acc = acc.wrapping_add(self.payload_length);
        // Hash fields after checksum
        acc = acc.wrapping_add(self.source_id as u32);
        acc = acc.wrapping_add((self.source_id >> 32) as u32);
        acc = acc.wrapping_add(self.generation as u32);
        acc = acc.wrapping_add((self.generation >> 32) as u32);
        // Hash payload
        for chunk in self.payload[..self.payload_length as usize].chunks(4) {
            let mut val = 0u32;
            for (i, &b) in chunk.iter().enumerate() {
                val |= (b as u32) << (i * 8);
            }
            acc = acc.wrapping_add(val);
        }
        acc
    }

    /// Verify slot integrity
    pub fn verify(&self) -> bool {
        self.checksum == self.compute_checksum()
    }

    /// Mark slot as committed (ready for host consumption)
    pub fn commit(&mut self) {
        self.status = SLOT_COMMITTED;
        self.checksum = self.compute_checksum();
    }

    /// Reset slot to free state, zeroing all data including payload
    pub fn reset(&mut self) {
        self.sequence = 0;
        self.status = SLOT_FREE;
        self.packet_type = 0;
        self.priority = 0;
        self.payload_offset = 0;
        self.payload_length = 0;
        self.checksum = 0;
        self.source_id = 0;
        self.generation = 0;
        self.payload = [0; 208];
    }
}

impl Default for PacketSlot {
    fn default() -> Self {
        Self::new()
    }
}

/// Slab Allocator - manages fixed-grid packet slots
pub struct SlabAllocator {
    /// Fixed array of packet slots
    slots: Box<[PacketSlot]>,
    /// Free list - indices of available slots
    free_list: Box<[u16]>,
    /// Number of free slots
    free_count: u16,
    /// Next sequence number for allocation
    next_sequence: u64,
    /// Total slots allocated since creation
    total_allocations: u64,
    /// Total slots freed since creation
    total_frees: u64,
    /// Current generation counter
    current_generation: u64,
}

impl SlabAllocator {
    /// Default slab size: 1024 slots = 256 KB
    pub const DEFAULT_CAPACITY: usize = 1024;

    pub fn new(capacity: usize) -> Self {
        let slots = vec![PacketSlot::new(); capacity].into_boxed_slice();
        let mut free_list = (0..capacity as u16).collect::<Vec<_>>().into_boxed_slice();

        // Initialize free list in reverse order for LIFO allocation
        free_list.reverse();

        Self {
            slots,
            free_list,
            free_count: capacity as u16,
            next_sequence: 1,
            total_allocations: 0,
            total_frees: 0,
            current_generation: 0,
        }
    }

    /// Allocate a new slot for the given packet type and priority
    pub fn allocate(
        &mut self,
        packet_type: u8,
        priority: u8,
        source_id: u64,
    ) -> Option<&mut PacketSlot> {
        if self.free_count == 0 {
            return None; // Slab exhausted
        }

        let idx = self.free_count as usize - 1;
        let slot_idx = self.free_list[idx] as usize;
        self.free_count -= 1;

        let slot = &mut self.slots[slot_idx];
        slot.reset();
        slot.sequence = self.next_sequence;
        self.next_sequence += 1;
        slot.status = SLOT_ACTIVE;
        slot.packet_type = packet_type;
        slot.priority = priority;
        slot.source_id = source_id;
        slot.generation = self.current_generation;

        self.total_allocations += 1;

        Some(slot)
    }

    /// Free a slot by its index
    pub fn free_by_index(&mut self, slot_idx: usize) -> bool {
        if slot_idx >= self.slots.len() {
            return false;
        }

        let slot = &mut self.slots[slot_idx];
        if slot.is_free() {
            return false; // Already free
        }

        slot.reset();
        self.free_list[self.free_count as usize] = slot_idx as u16;
        self.free_count += 1;
        self.total_frees += 1;

        true
    }

    /// Free a slot by reference
    pub fn free(&mut self, slot: &PacketSlot) -> bool {
        let slot_idx = self.slot_index(slot);
        self.free_by_index(slot_idx)
    }

    /// Free all committed slots (bulk reclaim)
    pub fn free_committed(&mut self) -> u16 {
        let mut freed = 0;
        for i in 0..self.slots.len() {
            if self.slots[i].is_committed() && self.free_by_index(i) {
                freed += 1;
            }
        }
        freed
    }

    /// Get a slot by index (read-only)
    pub fn get(&self, index: usize) -> Option<&PacketSlot> {
        self.slots.get(index)
    }

    /// Get a slot by index (mutable)
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PacketSlot> {
        self.slots.get_mut(index)
    }

    /// Get the index of a slot by pointer arithmetic
    pub fn slot_index(&self, slot: &PacketSlot) -> usize {
        let base = self.slots.as_ptr() as usize;
        let ptr = slot as *const PacketSlot as usize;
        (ptr - base) / PacketSlot::SIZE
    }

    /// Find the next committed slot for host consumption
    pub fn find_committed(&self) -> Option<(usize, &PacketSlot)> {
        self.slots
            .iter()
            .enumerate()
            .find(|(_, s)| s.is_committed())
    }

    /// Get utilization percentage (0.0 to 1.0)
    pub fn utilization(&self) -> f32 {
        1.0 - (self.free_count as f32 / self.slots.len() as f32)
    }

    /// Get number of free slots
    pub fn free_count(&self) -> u16 {
        self.free_count
    }

    /// Get total slot capacity
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Get total memory usage in bytes
    pub fn memory_usage_bytes(&self) -> usize {
        self.slots.len() * PacketSlot::SIZE
    }

    /// Get allocation statistics
    pub fn stats(&self) -> SlabStats {
        SlabStats {
            capacity: self.slots.len(),
            free_count: self.free_count,
            active_count: self.slots.iter().filter(|s| s.is_active()).count() as u16,
            committed_count: self.slots.iter().filter(|s| s.is_committed()).count() as u16,
            error_count: self.slots.iter().filter(|s| s.is_error()).count() as u16,
            utilization: self.utilization(),
            total_allocations: self.total_allocations,
            total_frees: self.total_frees,
            next_sequence: self.next_sequence,
            current_generation: self.current_generation,
        }
    }

    /// Increment the generation counter
    pub fn advance_generation(&mut self) {
        self.current_generation += 1;
    }

    /// Get current generation
    pub fn generation(&self) -> u64 {
        self.current_generation
    }

    /// Check if slab is critically full (>90%)
    pub fn is_critical(&self) -> bool {
        self.utilization() > 0.9
    }

    /// Check if slab is exhausted
    pub fn is_exhausted(&self) -> bool {
        self.free_count == 0
    }

    /// Reset the entire slab (emergency clear)
    pub fn reset(&mut self) {
        for slot in self.slots.iter_mut() {
            slot.reset();
        }
        for (i, entry) in self.free_list.iter_mut().enumerate() {
            *entry = i as u16;
        }
        self.free_count = self.slots.len() as u16;
        self.total_allocations = 0;
        self.total_frees = 0;
    }
}

/// Slab allocation statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct SlabStats {
    pub capacity: usize,
    pub free_count: u16,
    pub active_count: u16,
    pub committed_count: u16,
    pub error_count: u16,
    pub utilization: f32,
    pub total_allocations: u64,
    pub total_frees: u64,
    pub next_sequence: u64,
    pub current_generation: u64,
}

/// Slab allocator with payload arena for overflow data
pub struct SlabAllocatorWithArena {
    pub slab: SlabAllocator,
    pub payload_arena: Vec<u8>,
    pub arena_offset: usize,
}

impl SlabAllocatorWithArena {
    pub fn new(slab_capacity: usize, arena_size: usize) -> Self {
        Self {
            slab: SlabAllocator::new(slab_capacity),
            payload_arena: vec![0u8; arena_size],
            arena_offset: 0,
        }
    }

    /// Allocate a slot with payload that may overflow into the arena
    pub fn allocate_with_payload(
        &mut self,
        packet_type: u8,
        priority: u8,
        source_id: u64,
        payload: &[u8],
    ) -> Option<&mut PacketSlot> {
        // Check arena capacity first before allocating
        if payload.len() > 208 && self.arena_offset + payload.len() > self.payload_arena.len() {
            return None; // Arena full
        }

        let slot = self.slab.allocate(packet_type, priority, source_id)?;

        if payload.len() <= slot.payload.len() {
            // Fits in inline buffer
            slot.write_payload(payload);
            slot.payload_offset = 0; // 0 means inline
        } else {
            // Overflow into arena - use offset + 1 to distinguish from inline
            let offset = self.arena_offset;
            self.payload_arena[offset..offset + payload.len()].copy_from_slice(payload);
            slot.payload_offset = (offset + 1) as u32; // +1 to distinguish from inline (0)
            slot.payload_length = payload.len() as u32;
            self.arena_offset += payload.len();
        }

        Some(slot)
    }

    /// Read payload from slot (inline or arena)
    pub fn read_payload<'a>(&'a self, slot: &'a PacketSlot) -> &'a [u8] {
        if slot.payload_offset == 0 {
            slot.read_payload()
        } else {
            let offset = (slot.payload_offset - 1) as usize; // Subtract 1 to get actual arena offset
            let len = slot.payload_length as usize;
            &self.payload_arena[offset..offset + len]
        }
    }

    /// Reset arena (call after processing all committed slots)
    pub fn reset_arena(&mut self) {
        self.arena_offset = 0;
    }

    /// Get arena utilization
    pub fn arena_utilization(&self) -> f32 {
        self.arena_offset as f32 / self.payload_arena.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_packet_slot_size() {
        assert_eq!(PacketSlot::SIZE, 256);
        assert_eq!(mem::size_of::<PacketSlot>(), 256);
    }

    #[test]
    fn test_slab_allocation() {
        let mut slab = SlabAllocator::new(16);
        assert_eq!(slab.free_count(), 16);
        assert_eq!(slab.utilization(), 0.0);

        // Allocate a slot
        let slot = slab.allocate(0, 1, 42).unwrap();
        assert!(slot.is_active());
        assert_eq!(slot.sequence, 1);
        assert_eq!(slot.source_id, 42);
        assert_eq!(slab.free_count(), 15);
        assert!(slab.utilization() > 0.0);
    }

    #[test]
    fn test_slab_free() {
        let mut slab = SlabAllocator::new(4);

        let slot = slab.allocate(0, 1, 1).unwrap();
        slot.commit();

        let (idx, _) = slab.find_committed().unwrap();
        assert!(slab.free_by_index(idx));
        assert_eq!(slab.free_count(), 4);
    }

    #[test]
    fn test_slab_exhaustion() {
        let mut slab = SlabAllocator::new(2);

        assert!(slab.allocate(0, 1, 1).is_some());
        assert!(slab.allocate(0, 1, 2).is_some());
        assert!(slab.allocate(0, 1, 3).is_none()); // Exhausted
    }

    #[test]
    fn test_slab_commit_and_find() {
        let mut slab = SlabAllocator::new(4);

        let slot = slab.allocate(0, 1, 1).unwrap();
        slot.write_payload(b"test");
        slot.commit();

        let (idx, committed) = slab.find_committed().unwrap();
        assert!(committed.is_committed());
        assert_eq!(committed.read_payload(), b"test");

        slab.free_by_index(idx);
        assert!(slab.find_committed().is_none());
    }

    #[test]
    fn test_slab_stats() {
        let mut slab = SlabAllocator::new(10);

        slab.allocate(0, 1, 1).unwrap();
        slab.allocate(1, 2, 2).unwrap();

        let stats = slab.stats();
        assert_eq!(stats.capacity, 10);
        assert_eq!(stats.active_count, 2);
        assert_eq!(stats.free_count, 8);
        assert_eq!(stats.total_allocations, 2);
    }

    #[test]
    fn test_slab_reset() {
        let mut slab = SlabAllocator::new(4);

        slab.allocate(0, 1, 1).unwrap();
        slab.allocate(0, 1, 2).unwrap();
        slab.reset();

        assert_eq!(slab.free_count(), 4);
        assert_eq!(slab.utilization(), 0.0);
    }

    #[test]
    fn test_slab_with_arena() {
        let mut allocator = SlabAllocatorWithArena::new(4, 1024);

        // Small payload fits inline
        {
            let slot = allocator.allocate_with_payload(0, 1, 1, b"small").unwrap();
            assert_eq!(slot.payload_offset, 0);
            assert_eq!(slot.payload_length, 5);
        }

        // Large payload goes to arena
        let large_payload = vec![0xAB; 300];
        let (slot_idx, payload_len) = {
            let slot = allocator
                .allocate_with_payload(0, 1, 2, &large_payload)
                .unwrap();
            assert!(
                slot.payload_offset > 0,
                "Expected payload_offset > 0 for large payload"
            );
            // payload_offset is arena_offset + 1, so actual arena offset is payload_offset - 1
            (
                (slot.payload_offset - 1) as usize,
                slot.payload_length as usize,
            )
        };

        let read_back = &allocator.payload_arena[slot_idx..slot_idx + payload_len];
        assert_eq!(read_back, large_payload.as_slice());
    }

    #[test]
    fn test_packet_slot_checksum() {
        let mut slot = PacketSlot::new();
        slot.sequence = 42;
        slot.source_id = 123;
        assert!(slot.write_payload(b"test data"));
        slot.commit();

        assert!(slot.verify());

        // Corrupt the data
        slot.sequence = 99;
        assert!(!slot.verify());
    }

    #[test]
    fn test_free_committed_bulk() {
        let mut slab = SlabAllocator::new(8);

        for i in 0..5 {
            let slot = slab.allocate(0, 1, i).unwrap();
            slot.commit();
        }

        let freed = slab.free_committed();
        assert_eq!(freed, 5);
        assert_eq!(slab.free_count(), 8);
    }
}
