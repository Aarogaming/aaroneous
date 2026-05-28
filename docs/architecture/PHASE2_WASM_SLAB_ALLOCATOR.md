# Phase 2: WASM Slab Allocator & Grim Reaper Pattern

## Problem Statement
Zero-allocation WASM packet bus creates long-term memory fragmentation. Agents running continuously will fragment their linear memory, causing unbounded growth with no native shrink mechanism.

## Architecture

### 1. Slab Allocator (WASM Guest Side)

```rust
// Fixed-grid uniform packet slots inside WASM linear memory
#[repr(C, align(16))]
pub struct PacketSlot {
    pub sequence: u64,      // Monotonic sequence number
    pub status: u8,         // 0=Free, 1=Active, 2=Committed, 3=Error
    pub packet_type: u8,    // Intent/StateRead/Notification/Response
    pub priority: u8,       // 0=Low, 1=Normal, 2=High, 3=Critical
    pub _padding: u8,
    pub payload_offset: u32, // Offset into payload arena
    pub payload_length: u32, // Length in bytes
    pub checksum: u32,       // Integrity verification
    pub payload: [u8; 240],  // Inline payload (fits in single slot)
}

pub const SLOT_SIZE: usize = 256; // 256 bytes per slot
pub const MAX_SLOTS: usize = 1024; // 256 KB total slab

pub struct SlabAllocator {
    slots: [PacketSlot; MAX_SLOTS],
    free_list: [u16; MAX_SLOTS], // Indices of free slots
    free_count: u16,
    next_sequence: u64,
}

impl SlabAllocator {
    pub fn allocate(&mut self, packet_type: u8, priority: u8) -> Option<&mut PacketSlot> {
        if self.free_count == 0 {
            return None; // Slab exhausted - trigger Grim Reaper
        }
        
        let idx = self.free_count as usize - 1;
        let slot_idx = self.free_list[idx] as usize;
        self.free_count -= 1;
        
        let slot = &mut self.slots[slot_idx];
        slot.sequence = self.next_sequence;
        self.next_sequence += 1;
        slot.status = 1; // Active
        slot.packet_type = packet_type;
        slot.priority = priority;
        slot.payload_length = 0;
        slot.checksum = 0;
        
        Some(slot)
    }
    
    pub fn free(&mut self, slot: &PacketSlot) {
        let idx = (slot as *const PacketSlot as usize - 
                   self.slots.as_ptr() as usize) / SLOT_SIZE;
        
        if idx < MAX_SLOTS {
            self.slots[idx].status = 0; // Free
            self.free_list[self.free_count as usize] = idx as u16;
            self.free_count += 1;
        }
    }
    
    pub fn utilization(&self) -> f32 {
        1.0 - (self.free_count as f32 / MAX_SLOTS as f32)
    }
}
```

### 2. Grim Reaper Pattern (Host Side)

```rust
pub struct GrimReaper {
    /// Threshold to trigger reaping (80% slab utilization)
    utilization_threshold: f32,
    /// Snapshot buffer for agent state preservation
    snapshot_buffer: Vec<u8>,
}

impl GrimReaper {
    pub fn should_reap(&self, slab_utilization: f32) -> bool {
        slab_utilization > self.utilization_threshold
    }
    
    pub async fn reap_agent(
        &mut self,
        agent_id: &str,
        instance: &mut WasmtimeInstance,
        state_extractor: impl FnOnce(&mut Store<AgentData>) -> AgentState,
        state_restorer: impl FnOnce(&mut Store<AgentData>, AgentState),
    ) -> Result<NewWasmtimeInstance> {
        // 1. Snapshot core state
        let state = state_extractor(&mut instance.store);
        
        // 2. Serialize state to snapshot buffer
        let snapshot = rkyv::to_bytes::<_, 256>(&state)?;
        self.snapshot_buffer.clear();
        self.snapshot_buffer.extend_from_slice(&snapshot);
        
        // 3. Kill the fragmented instance
        drop(instance);
        
        // 4. Spin up fresh clone
        let new_instance = self.spawn_fresh_instance(agent_id).await?;
        
        // 5. Restore state into clean memory
        state_restorer(&mut new_instance.store, state);
        
        tracing::info!(
            agent_id,
            "Grim Reaper: Reaped and resurrected agent with clean memory"
        );
        
        Ok(new_instance)
    }
    
    async fn spawn_fresh_instance(&self, agent_id: &str) -> Result<NewWasmtimeInstance> {
        // Load WASM module fresh, allocate new linear memory
        // State will be restored from snapshot
        todo!()
    }
}
```

### 3. Integration with NucleotidePacket System

The Slab Allocator replaces the current `WASMLinearMemoryBridge` write mechanism:

```
[Agent] ── allocate slot ──> [SlabAllocator]
     │                              │
     │   write packet data          │
     └─────────────────────────────>│
                                    │
[Host] <── notification pointer ────┘
     │
     │   read slot directly (zero-copy)
     │
     └─> process intent ──> free slot
```

### 4. Memory Layout

```
WASM Linear Memory (4 MB default)
┌─────────────────────────────────────┐
│ Code Section (imports, functions)   │
├─────────────────────────────────────┤
│ Slab Allocator (256 KB)             │
│   [Slot 0][Slot 1]...[Slot 1023]   │
├─────────────────────────────────────┤
│ Payload Arena (3 MB)                │
│   Variable-size overflow payloads   │
├─────────────────────────────────────┤
│ Stack (512 KB)                      │
└─────────────────────────────────────┘
```

### 5. Metrics & Observability

- `slab_utilization`: Percentage of slots in use
- `allocation_failures`: Count of failed allocations (triggers Grim Reaper)
- `reap_count`: Number of agent resurrections
- `avg_slot_lifetime`: Time from allocation to free
- `fragmentation_ratio`: Payload arena fragmentation metric

## Implementation Order
1. Define `PacketSlot` struct with `#[repr(C, align(16))]`
2. Implement `SlabAllocator` with free list
3. Add utilization monitoring to WASM loader
4. Implement `GrimReaper` with state snapshot/restore
5. Integrate with existing `NucleotidePacket` system
6. Add metrics dashboard panel
