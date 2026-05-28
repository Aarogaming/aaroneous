/// Lock-free ringbuffer for Hypervisor → WASM event passing
/// 
/// Single producer (Hypervisor), single consumer (WASM)
/// No locks, atomic operations only

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use super::ebus_event::EbusEvent;

/// Lock-free ringbuffer for passing events
pub struct RingBuffer {
    /// Circular buffer of events
    buffer: Vec<Option<EbusEvent>>,
    
    /// Write pointer (Hypervisor writes here)
    write_pos: Arc<AtomicUsize>,
    
    /// Read pointer (WASM reads from here)
    read_pos: Arc<AtomicUsize>,
    
    /// Mask for efficient modulo
    mask: usize,
}

impl RingBuffer {
    /// Create new ringbuffer with capacity (must be power of 2)
    pub fn new(capacity: usize) -> Self {
        // Round up to nearest power of 2
        let capacity = capacity.next_power_of_two();
        
        Self {
            buffer: vec![None; capacity],
            write_pos: Arc::new(AtomicUsize::new(0)),
            read_pos: Arc::new(AtomicUsize::new(0)),
            mask: capacity - 1,
        }
    }
    
    /// Push event (non-blocking, from Hypervisor)
    pub fn push(&self, event: EbusEvent) -> Result<(), String> {
        let write = self.write_pos.load(Ordering::Acquire);
        let next_write = (write + 1) & self.mask;
        let read = self.read_pos.load(Ordering::Acquire);
        
        // Check if buffer is full
        if next_write == read {
            return Err("Ringbuffer full".to_string());
        }
        
        // Safety: We own the write position, no race
        unsafe {
            let ptr = self.buffer.as_ptr() as *mut Option<EbusEvent>;
            *ptr.add(write) = Some(event);
        }
        
        // Update write pointer atomically
        self.write_pos.store(next_write, Ordering::Release);
        
        Ok(())
    }
    
    /// Pop event (non-blocking, from WASM)
    pub fn pop(&self) -> Result<EbusEvent, String> {
        let read = self.read_pos.load(Ordering::Acquire);
        let write = self.write_pos.load(Ordering::Acquire);
        
        // Check if buffer is empty
        if read == write {
            return Err("Ringbuffer empty".to_string());
        }
        
        // Safety: We own the read position, no race
        let event = unsafe {
            let ptr = self.buffer.as_ptr() as *mut Option<EbusEvent>;
            (*ptr.add(read)).take()
        };
        
        let next_read = (read + 1) & self.mask;
        self.read_pos.store(next_read, Ordering::Release);
        
        event.ok_or_else(|| "Event slot was empty".to_string())
    }
    
    /// Get current number of events in buffer
    pub fn len(&self) -> usize {
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.read_pos.load(Ordering::Acquire);
        
        if write >= read {
            write - read
        } else {
            (self.mask + 1) - (read - write)
        }
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.mask + 1
    }
}

// RingBuffer cannot be cloned; multiple consumers would race
// Instead, we use Arc<RingBuffer>

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ebus_event::EbusEventType;
    
    #[test]
    fn test_ringbuffer_creation() {
        let rb = RingBuffer::new(1024);
        
        assert_eq!(rb.capacity(), 1024);
        assert!(rb.is_empty());
    }
    
    #[test]
    fn test_ringbuffer_push_pop() {
        let rb = RingBuffer::new(1024);
        
        let event = EbusEvent::new(EbusEventType::InputEvent, vec![1, 2, 3]);
        
        rb.push(event.clone()).expect("Push failed");
        assert_eq!(rb.len(), 1);
        
        let popped = rb.pop().expect("Pop failed");
        assert_eq!(popped.event_type, event.event_type);
        assert!(rb.is_empty());
    }
    
    #[test]
    fn test_ringbuffer_multiple_events() {
        let rb = RingBuffer::new(16);
        
        for i in 0..10 {
            let event = EbusEvent::new(EbusEventType::InputEvent, vec![i as u8]);
            rb.push(event).expect("Push failed");
        }
        
        assert_eq!(rb.len(), 10);
        
        for i in 0..10 {
            let event = rb.pop().expect("Pop failed");
            assert_eq!(event.payload[0], i as u8);
        }
        
        assert!(rb.is_empty());
    }
    
    #[test]
    fn test_ringbuffer_wraparound() {
        let rb = RingBuffer::new(4); // Small buffer to test wraparound
        
        // Fill and drain several times
        for cycle in 0..5 {
            for i in 0..3 {
                let event = EbusEvent::new(
                    EbusEventType::InputEvent,
                    vec![(cycle * 3 + i) as u8],
                );
                rb.push(event).expect("Push failed");
            }
            
            for _ in 0..3 {
                rb.pop().expect("Pop failed");
            }
        }
        
        assert!(rb.is_empty());
    }
    
    #[test]
    fn test_ringbuffer_full() {
        let rb = RingBuffer::new(4);
        
        // Fill to capacity - 1
        for i in 0..3 {
            let event = EbusEvent::new(EbusEventType::InputEvent, vec![i as u8]);
            rb.push(event).expect("Push failed");
        }
        
        // Next push should fail
        let event = EbusEvent::new(EbusEventType::InputEvent, vec![99]);
        let result = rb.push(event);
        
        assert!(result.is_err());
    }
}
