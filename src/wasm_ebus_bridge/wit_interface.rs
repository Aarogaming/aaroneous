/// WebAssembly Interface Types (WIT) definitions
/// 
/// This module defines the ABI (Application Binary Interface) between Rust and WASM
/// Responsibility: Function export registry, type definitions, marshalling

use std::collections::HashMap;

/// Exported WASM function descriptor
#[derive(Debug, Clone)]
pub struct WasmExport {
    /// Function name
    pub name: String,
    
    /// Parameter types
    pub param_types: Vec<ValueType>,
    
    /// Return type
    pub return_type: ValueType,
    
    /// Function pointer (for future dynamic invocation)
    pub function_id: u32,
}

impl WasmExport {
    /// Create new export descriptor
    pub fn new(
        name: impl Into<String>,
        param_types: Vec<ValueType>,
        return_type: ValueType,
        function_id: u32,
    ) -> Self {
        Self {
            name: name.into(),
            param_types,
            return_type,
            function_id,
        }
    }
    
    /// Call the function (in real implementation, would invoke WASM)
    pub fn call(&self, _args: &[u32]) -> Result<u32, String> {
        // In a real execution environment, this calls into the Wasmtime instance.
        // For the interface registry, this remains a stub.
        Ok(0)
    }
}

/// WASM value types (subset of what WIT supports)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    I32,
    I64,
    F32,
    F64,
    None,
}

impl ValueType {
    /// Size in bytes
    pub fn size(&self) -> usize {
        match self {
            ValueType::I32 => 4,
            ValueType::I64 => 8,
            ValueType::F32 => 4,
            ValueType::F64 => 8,
            ValueType::None => 0,
        }
    }
}

/// WIT Interface: Registry of WASM exports
#[derive(Debug, Clone)]
pub struct WitInterface {
    /// Exported functions
    exports: HashMap<String, WasmExport>,
}

impl WitInterface {
    /// Create new WIT interface
    pub fn new() -> Self {
        Self {
            exports: HashMap::new(),
        }
    }
    
    /// Register an exported function
    pub fn register_export(&mut self, export: WasmExport) {
        self.exports.insert(export.name.clone(), export);
    }
    
    /// Get function by name
    pub fn get_function(&self, name: &str) -> Option<&WasmExport> {
        self.exports.get(name)
    }
    
    /// List all exports
    pub fn list_exports(&self) -> Vec<&str> {
        self.exports.keys().map(|s| s.as_str()).collect()
    }
    
    /// Default interface with standard exports
    pub fn with_defaults() -> Self {
        let mut iface = Self::new();
        
        // process_events: Takes event count, returns status
        iface.register_export(WasmExport::new(
            "process_events",
            vec![ValueType::I32],
            ValueType::I32,
            1,
        ));
        
        // execute_action: Sends action to marionette
        iface.register_export(WasmExport::new(
            "execute_action",
            vec![ValueType::I32, ValueType::I32], // ptr, len
            ValueType::I32,
            2,
        ));
        
        // query_state: Gets current game state
        iface.register_export(WasmExport::new(
            "query_state",
            vec![],
            ValueType::I32, // Returns offset of state buffer
            3,
        ));
        
        // learn_observation: Record surprise/learning
        iface.register_export(WasmExport::new(
            "learn_observation",
            vec![ValueType::I32, ValueType::I32], // prediction_error, discovery_ptr
            ValueType::I32,
            4,
        ));
        
        // get_memory_stats: Check WASM memory usage
        iface.register_export(WasmExport::new(
            "get_memory_stats",
            vec![],
            ValueType::I32,
            5,
        ));
        
        iface
    }
}

impl Default for WitInterface {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard WIT type definitions (for future import/export type checking)
pub mod types {
    use super::ValueType;
    
    /// Event that WASM receives from O3DE
    #[derive(Debug, Clone)]
    pub struct WasmEvent {
        pub event_type: u32,
        pub timestamp_ns: u64,
        pub payload_ptr: u32,  // Pointer in WASM memory
        pub payload_len: u32,
    }
    
    /// Action that WASM sends to O3DE
    #[derive(Debug, Clone)]
    pub struct WasmAction {
        pub action_type: u32,
        pub data_ptr: u32,     // Pointer to action data
        pub data_len: u32,
    }
    
    /// Game state snapshot (visual+entity data)
    #[derive(Debug, Clone)]
    pub struct GameState {
        pub frame_number: u32,
        pub timestamp_ns: u64,
        pub vision_hash: u64,
        pub entities_count: u32,
        pub ui_state_hash: u64,
    }
    
    /// Discovery insight (for curiosity learning)
    #[derive(Debug, Clone)]
    pub struct Discovery {
        pub action_type: u32,
        pub prediction_error: f32,
        pub surprise_value: f32,
        pub metadata_ptr: u32,
        pub metadata_len: u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_value_type_sizes() {
        assert_eq!(ValueType::I32.size(), 4);
        assert_eq!(ValueType::I64.size(), 8);
        assert_eq!(ValueType::F32.size(), 4);
        assert_eq!(ValueType::F64.size(), 8);
        assert_eq!(ValueType::None.size(), 0);
    }
    
    #[test]
    fn test_wit_interface_registration() {
        let mut iface = WitInterface::new();
        
        let export = WasmExport::new(
            "test_func",
            vec![ValueType::I32],
            ValueType::I32,
            1,
        );
        
        iface.register_export(export);
        
        assert!(iface.get_function("test_func").is_some());
        assert_eq!(iface.list_exports().len(), 1);
    }
    
    #[test]
    fn test_wit_interface_defaults() {
        let iface = WitInterface::with_defaults();
        
        assert!(iface.get_function("process_events").is_some());
        assert!(iface.get_function("execute_action").is_some());
        assert!(iface.get_function("query_state").is_some());
        assert!(iface.get_function("learn_observation").is_some());
    }
}
