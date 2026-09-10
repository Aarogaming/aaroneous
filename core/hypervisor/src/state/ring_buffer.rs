// src/state/ring_buffer.rs
//! A lock‑free fixed‑capacity ring buffer used for task queues and telemetry.
//!
//! The implementation mirrors the design used in other lock‑free structures in the
//! codebase (e.g. token bucket limiter). It stores up to `CAP` items of type `T`
//! where `T: Copy + Default + bytemuck::Pod`. The buffer uses two `AtomicUsize`
//! indices (`head` for consuming, `tail` for producing) and a `[MaybeUninit<T>; CAP]`
//! storage array. All operations are `SeqCst` to guarantee safety on Windows.
//!
//! The buffer does **not** allocate on the heap – the storage lives inline in the
//! struct, satisfying the zero‑copy POD requirements.

use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};
use bytemuck::{Pod, Zeroable};

/// Fixed‑capacity lock‑free ring buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct RingBuffer<T, const CAP: usize>
where
    T: Copy + Default + Pod,
{
    // Storage for items. `MaybeUninit` avoids constructing `T` unnecessarily.
    buffer: [MaybeUninit<T>; CAP],
    // Index of the next element to read.
    head: AtomicUsize,
    // Index of the next slot to write.
    tail: AtomicUsize,
}

impl<T, const CAP: usize> RingBuffer<T, CAP>
where
    T: Copy + Default + Pod,
{
    /// Create a new empty buffer.
    pub const fn new() -> Self {
        // SAFETY: An array of `MaybeUninit` is always valid.
        Self {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Return true if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::SeqCst) == self.tail.load(Ordering::SeqCst)
    }

    /// Return true if the buffer is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        let tail = self.tail.load(Ordering::SeqCst);
        let head = self.head.load(Ordering::SeqCst);
        (tail + 1) % CAP == head % CAP
    }

    /// Push a value onto the buffer. Returns `Ok(())` on success or `Err(value)`
    /// if the buffer is full.
    #[inline]
    pub fn push(&self, value: T) -> Result<(), T> {
        let tail = self.tail.load(Ordering::SeqCst);
        let head = self.head.load(Ordering::SeqCst);
        if (tail + 1) % CAP == head % CAP {
            // Full.
            return Err(value);
        }
        let idx = tail % CAP;
        // SAFETY: We have exclusive write access to this slot because `tail` is
        // only advanced after the write.
        unsafe { *self.buffer.get_unchecked(idx).as_ptr() = value };
        self.tail.store(tail.wrapping_add(1), Ordering::SeqCst);
        Ok(())
    }

    /// Pop a value from the buffer. Returns `Some(value)` if not empty.
    #[inline]
    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        if head == tail {
            return None; // Empty.
        }
        let idx = head % CAP;
        // SAFETY: The slot has been written by a prior `push`.
        let value = unsafe { self.buffer.get_unchecked(idx).assume_init_read() };
        self.head.store(head.wrapping_add(1), Ordering::SeqCst);
        Some(value)
    }
}

// Unit tests for the ring buffer – they return `Result<()>` per the new testing style.
#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[derive(Copy, Clone, Default, Debug, Pod, Zeroable, PartialEq, Eq)]
    #[repr(C)]
    struct TestItem(u64);

    #[test]
    fn basic_push_pop() -> Result<(), String> {
        const CAP: usize = 4;
        let buf = RingBuffer::<TestItem, CAP>::new();
        assert!(buf.is_empty());
        buf.push(TestItem(42)).map_err(|_| "push failed".to_string())?;
        assert!(!buf.is_empty());
        let v = buf.pop().ok_or("pop returned None")?;
        assert_eq!(v, TestItem(42));
        assert!(buf.is_empty());
        Ok(())
    }

    #[test]
    fn overflow_behaviour() -> Result<(), String> {
        const CAP: usize = 2;
        let buf = RingBuffer::<TestItem, CAP>::new();
        buf.push(TestItem(1)).map_err(|_| "first push".to_string())?;
        // Buffer is now full because CAP=2 and we reserve one slot for distinction.
        assert!(buf.is_full());
        // Pushing when full should return Err with the original value.
        let err = buf.push(TestItem(2)).err().ok_or("expected Err")?;
        assert_eq!(err, TestItem(2));
        Ok(())
    }
}
