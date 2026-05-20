use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;
use anyhow::Result;
use bytes::Bytes;
use crate::federation::Federation;

/// WASM-HypervisorBridge: Zero-copy communication between WASM runtime and Hypervisor
/// 
/// This module provides:
/// - Event ringbuffer (Hypervisor → WASM linear memory)
/// - WASM memory export (shared memory between Rust/WASM)
/// - WIT (WebAssembly Interface Types) interface definitions
/// - Action execution (WASM → System effects)

pub mod ebus_event;
pub mod wasm_memory;
pub mod wit_interface;
pub mod action_executor;
pub mod ringbuffer;
pub mod instruction_set;

pub use ebus_event::{EbusEvent, EbusEventType};
pub use wasm_memory::WasmMemory;
pub use wit_interface::WitInterface;
pub use action_executor::ActionExecutor;
pub use ringbuffer::RingBuffer;
pub use instruction_set::{Instruction, OpCode};

/// Main WASM-HypervisorBridge
/// 
/// Responsibility: Coordinate zero-copy message passing between Hypervisor and WASM
#[derive(Clone)]
pub struct WasmEbusBridge {
    /// Shared linear memory with WASM runtime
    pub wasm_memory: Arc<WasmMemory>,
    
    /// Event queue (Hypervisor → WASM)
    pub ebus_ringbuffer: Arc<RingBuffer>,
    
    /// WASM exported functions registry
    pub wasm_exports: Arc<RwLock<WitInterface>>,
    
    /// Sync point: WASM polls this to know events are ready
    pub sync_point: Arc<AtomicUsize>,
    
    /// Action executor (WASM → System)
    pub action_executor: Arc<ActionExecutor>,
    
    /// Reference to the federation for cross-agent signaling
    pub federation: Option<Arc<Federation>>,
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
            federation: None,
        }
    }

    /// Attach federation to the bridge
    pub fn with_federation(mut self, federation: Arc<Federation>) -> Self {
        self.federation = Some(federation);
        self
    }
    
    /// Hypervisor pushes event to WASM ringbuffer
    /// 
    /// This is called from the Hypervisor's event handlers (e.g., input events, visual changes)
    pub fn on_internal_event(&self, event: EbusEvent) -> Result<(), String> {
        // Push to ringbuffer (non-blocking)
        self.ebus_ringbuffer.push(event)
            .map_err(|e| format!("Ringbuffer push failed: {}", e))?;
        
        // Signal that events are available
        self.sync_point.fetch_add(1, Ordering::Release);
        
        Ok(())
    }
    
    /// WASM calls exported function (e.g., process_events)
    pub async fn invoke_wasm_export(
        &self,
        export_name: &str,
        args: &[u32],
    ) -> Result<u32, String> {
        let exports = self.wasm_exports.read().await;
        
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

    /// Emit a signal from WASM to the AAS Shards (Python)
    pub async fn emit_to_shards(&self, signal_type: &str, payload: serde_json::Value) -> Result<(), String> {
        if let Some(ref fed) = self.federation {
            let intent_content = format!("SIGNAL: {} | PAYLOAD: {}", signal_type, payload);
            let intent = crate::federation::intent::Intent::new(intent_content);
            fed.submit_intent(intent).await;
            Ok(())
        } else {
            Err("Federation not attached to EBus Bridge".to_string())
        }
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
        
        bridge.on_internal_event(event).expect("Push failed");
        assert!(bridge.sync_point.load(Ordering::Acquire) > 0);
    }
}
