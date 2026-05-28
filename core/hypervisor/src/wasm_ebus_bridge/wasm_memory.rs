/// WASM Linear Memory Management
/// 
/// Provides shared memory between Rust and WASM runtime
/// Responsibility: Manage zero-copy memory regions

use parking_lot::RwLock;
use std::sync::Arc;

/// WASM linear memory (shared between Rust and WASM runtime)
pub struct WasmMemory {
    /// Linear memory buffer (accessible by both Rust and WASM)
    memory: Arc<RwLock<Vec<u8>>>,
    
    /// Total size (pages are 64KB each)
    size_bytes: usize,
}

impl WasmMemory {
    /// Create new WASM linear memory
    pub fn new(size_bytes: usize) -> Self {
        // Round to nearest page (64KB)
        let size_bytes = ((size_bytes + 65535) / 65536) * 65536;
        
        Self {
            memory: Arc::new(RwLock::new(vec![0u8; size_bytes])),
            size_bytes,
        }
    }
    
    /// Get total size in bytes
    pub fn size(&self) -> usize {
        self.size_bytes
    }
    
    /// Write data at offset (used by Rust to pass data to WASM)
    pub fn write(&self, offset: usize, data: &[u8]) -> Result<(), String> {
        if offset + data.len() > self.size_bytes {
            return Err(format!(
                "Write out of bounds: offset={}, len={}, capacity={}",
                offset,
                data.len(),
                self.size_bytes
            ));
        }
        
        let mut mem = self.memory.write();
        mem[offset..offset + data.len()].copy_from_slice(data);
        
        Ok(())
    }
    
    /// Read data at offset (used by Rust to read WASM output)
    pub fn read(&self, offset: usize, len: usize) -> Result<Vec<u8>, String> {
        if offset + len > self.size_bytes {
            return Err(format!(
                "Read out of bounds: offset={}, len={}, capacity={}",
                offset, len, self.size_bytes
            ));
        }
        
        let mem = self.memory.read();
        Ok(mem[offset..offset + len].to_vec())
    }
    
    /// Access memory with a raw pointer (safe: lock held for closure duration)
    pub fn with_ptr<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R {
        let mem = self.memory.read();
        f(&mem)
    }
    
    /// Access memory mutably with a raw pointer (safe: lock held for closure duration)
    pub fn with_ptr_mut<R>(&self, f: impl FnOnce(&mut [u8]) -> R) -> R {
        let mut mem = self.memory.write();
        f(&mut mem)
    }
    
    /// Allocate region in WASM memory
    /// 
    /// Returns offset of allocated region
    pub fn allocate(&self, size: usize) -> Result<usize, String> {
        // Simple allocation strategy: find first free block
        // For now, we mark allocated regions by setting the first byte
        
        let mut mem = self.memory.write();
        
        // Find first sequence of zeros (free space)
        for i in 0..mem.len().saturating_sub(size) {
            if i + size <= mem.len() {
                let slice = &mem[i..i + size];
                if slice.iter().all(|&b| b == 0) {
                    // Mark as allocated (set first byte to non-zero)
                    mem[i] = 0xFF;
                    return Ok(i);
                }
            }
        }
        
        Err("No contiguous free space available".to_string())
    }
    
    /// Free allocated region (mark as available)
    pub fn free(&self, offset: usize, size: usize) -> Result<(), String> {
        self.write(offset, &vec![0u8; size])
    }
    
    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        let mem = self.memory.read();
        
        let total = mem.len();
        let used = mem.iter().filter(|&&b| b != 0).count();
        let free = total - used;
        
        MemoryStats {
            total_bytes: total,
            used_bytes: used,
            free_bytes: free,
            utilization: (used as f32 / total as f32) * 100.0,
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_bytes: usize,
    pub used_bytes: usize,
    pub free_bytes: usize,
    pub utilization: f32,
}

impl Clone for WasmMemory {
    fn clone(&self) -> Self {
        Self {
            memory: self.memory.clone(),
            size_bytes: self.size_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasm_memory_creation() {
        let mem = WasmMemory::new(1024 * 1024); // 1MB
        
        assert_eq!(mem.size(), 1024 * 1024);
    }
    
    #[test]
    fn test_wasm_memory_write_read() {
        let mem = WasmMemory::new(1024);
        
        let data = vec![1, 2, 3, 4, 5];
        mem.write(0, &data).expect("Write failed");
        
        let read_back = mem.read(0, 5).expect("Read failed");
        assert_eq!(read_back, data);
    }
    
    #[test]
    fn test_wasm_memory_bounds_check() {
        let mem = WasmMemory::new(10);
        let size = mem.size();  // Actual size after page rounding
        
        // Try to write past the end
        let result = mem.write(size - 5, &vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_wasm_memory_allocate() {
        let mem = WasmMemory::new(1024);
        
        let offset1 = mem.allocate(100).expect("Allocate 1 failed");
        assert_eq!(offset1, 0);
        
        let offset2 = mem.allocate(100).expect("Allocate 2 failed");
        assert!(offset2 > 0);
    }
}
