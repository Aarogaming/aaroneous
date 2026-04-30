/// WASM-EBus Bridge: Zero-copy communication between WASM runtime and O3DE
/// 
/// This module provides:
/// - Event ringbuffer (O3DE EBus → WASM linear memory)
/// - WASM memory export (shared memory between Rust/WASM)
/// - WIT (WebAssembly Interface Types) interface definitions
/// - Action execution (WASM → O3DE effects)

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use bytes::{Bytes, BytesMut};

pub mod ebus_event;
pub mod wasm_memory;
pub mod wit_interface;
pub mod action_executor;
pub mod ringbuffer;

pub use ebus_event::{EbusEvent, EbusEventType};
pub use wasm_memory::WasmMemory;
pub use wit_interface::WitInterface;
pub use action_executor::ActionExecutor;
pub use ringbuffer::RingBuffer;

/// Main WASM-EBus Bridge
/// 
/// Responsibility: Coordinate zero-copy message passing between O3DE and WASM
#[derive(Clone)]
pub struct WasmEbusBridge {
    /// Shared linear memory with WASM runtime
    pub wasm_memory: Arc<WasmMemory>,
    
    /// Event queue (O3DE → WASM)
    pub ebus_ringbuffer: Arc<RingBuffer>,
    
    /// WASM exported functions registry
    pub wasm_exports: Arc<RwLock<WitInterface>>,
    
    /// Sync point: WASM polls this to know events are ready
    pub sync_point: Arc<AtomicUsize>,
    
    /// Action executor (WASM → O3DE)
    pub action_executor: Arc<ActionExecutor>,
}

impl WasmEbusBridge {
    /// Create new bridge
    pub fn new(
        wasm_memory: Arc<WasmMemory>,
        ebus_ringbuffer: Arc<RingBuffer>,
        action_executor: Arc<ActionExecutor>,
    ) -> Self {
        Self {
            wasm_memory,
            ebus_ringbuffer,
            wasm_exports: Arc::new(RwLock::new(WitInterface::default())),
            sync_point: Arc::new(AtomicUsize::new(0)),
            action_executor,
        }
    }
    
    /// O3DE pushes event to WASM ringbuffer
    /// 
    /// This is called from O3DE's EBus handlers (e.g., input events, visual changes)
    pub fn on_ebus_event(&self, event: EbusEvent) -> Result<(), String> {
        // Push to ringbuffer (non-blocking)
        self.ebus_ringbuffer.push(event)
            .map_err(|e| format!("Ringbuffer push failed: {}", e))?;
        
        // Signal that events are available
        self.sync_point.fetch_add(1, Ordering::Release);
        
        Ok(())
    }
    
    /// WASM calls exported function (e.g., process_events)
    pub fn invoke_wasm_export(
        &self,
        export_name: &str,
        args: &[u32],
    ) -> Result<u32, String> {
        let exports = self.wasm_exports.read();
        
        exports.get_function(export_name)
            .ok_or_else(|| format!("Export '{}' not found", export_name))?
            .call(args)
    }
    
    /// WASM requests action execution (e.g., move mouse, press key)
    pub async fn execute_wasm_action(&self, action_bytes: &[u8]) -> Result<Bytes, String> {
        self.action_executor.execute(action_bytes).await
    }
    
    /// Get current event count (for WASM to poll)
    pub fn event_count(&self) -> usize {
        self.ebus_ringbuffer.len()
    }
    
    /// Drain all events (WASM calls this to batch-process)
    pub fn drain_events(&self) -> Vec<EbusEvent> {
        let mut events = Vec::new();
        
        while let Ok(event) = self.ebus_ringbuffer.pop() {
            events.push(event);
        }
        
        // Reset sync point
        self.sync_point.store(0, Ordering::Release);
        
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bridge_creation() {
        let wasm_mem = Arc::new(WasmMemory::new(1024 * 1024)); // 1MB
        let ringbuf = Arc::new(RingBuffer::new(1024));
        let executor = Arc::new(ActionExecutor::default());
        
        let bridge = WasmEbusBridge::new(wasm_mem, ringbuf, executor);
        
        assert_eq!(bridge.event_count(), 0);
    }
    
    #[test]
    fn test_event_push_increments_sync() {
        let wasm_mem = Arc::new(WasmMemory::new(1024 * 1024));
        let ringbuf = Arc::new(RingBuffer::new(1024));
        let executor = Arc::new(ActionExecutor::default());
        
        let bridge = WasmEbusBridge::new(wasm_mem, ringbuf, executor);
        
        let event = EbusEvent::new(EbusEventType::InputEvent, vec![1, 2, 3]);
        
        bridge.on_ebus_event(event).expect("Push failed");
        assert!(bridge.sync_point.load(Ordering::Acquire) > 0);
    }
}
