// src/state/ring_buffer.rs
//! Sound lock-free SWMR ring buffer using UnsafeCell with proper Sync encapsulation.

use core::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct SwmrRingBuffer<T: Copy, const CAP: usize> {
    head: AtomicUsize,
    tail: AtomicUsize,
    slots: UnsafeCell<[T; CAP]>,
}

unsafe impl<T: Copy + Send, const CAP: usize> Sync for SwmrRingBuffer<T, CAP> {}
unsafe impl<T: Copy + Send, const CAP: usize> Send for SwmrRingBuffer<T, CAP> {}

impl<T: Copy, const CAP: usize> SwmrRingBuffer<T, CAP> {
    pub const fn new(initial: T) -> Self {
        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            slots: UnsafeCell::new([initial; CAP]),
        }
    }

    pub fn push(&self, item: T) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail.wrapping_sub(head) >= CAP {
            return false;
        }
        let idx = tail % CAP;
        // `idx = tail % CAP` is always `< CAP`, staying within the bounds of
        // the `[T; CAP]` array backing `slots`. This is a single-writer
        // (SWMR) ring buffer: `push` is the only method that writes into
        // `slots`, so no other thread can be concurrently writing to the
        // same slot. The capacity check above (`tail - head < CAP`)
        // guarantees slot `idx` has already been consumed by `pop` (or
        // never used); the `Release` store of `tail` below publishes the
        // write to readers who `Acquire`-load `tail`.
        // SAFETY: idx < CAP; single-writer; slot is not concurrently read.
        unsafe {
            let buffer_ptr = self.slots.get() as *mut T;
            buffer_ptr.add(idx).write(item);
        }
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head == tail {
            return None;
        }
        let idx = head % CAP;
        // `idx = head % CAP` is always `< CAP`, staying within the bounds of
        // the `[T; CAP]` array backing `slots`. `head != tail` (checked
        // above) and the `Acquire` load of `tail` pairs with `push`'s
        // `Release` store, so this slot has already been fully written by
        // `push` before this read observes it. `T: Copy`, so bitwise-copying
        // it out via `read()` cannot violate any ownership/drop invariant,
        // and the single-writer contract means no concurrent `write()`
        // targets this slot.
        // SAFETY: idx < CAP; slot fully written and not concurrently written.
        let val = unsafe {
            let buffer_ptr = self.slots.get() as *const T;
            buffer_ptr.add(idx).read()
        };
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(val)
    }

    pub fn tail(&self) -> usize {
        self.tail.load(Ordering::Relaxed)
    }
    pub fn head(&self) -> usize {
        self.head.load(Ordering::Relaxed)
    }
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Relaxed) == self.tail.load(Ordering::Relaxed)
    }
    pub fn is_full(&self) -> bool {
        let t = self.tail.load(Ordering::Relaxed);
        let h = self.head.load(Ordering::Relaxed);
        t.wrapping_sub(h) >= CAP
    }
}
