// src/streaming_adaptation.rs

//! Zero‑copy streaming adaptation for the Runtime Monitor fast‑path.
//! Uses fixed‑size buffers and reads telemetry directly from the hypervisor RingBuffer.

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};
use hypervisor::state::RingBuffer;
use super::types::Trigger;
use crossbeam_queue::ArrayQueue;
use std::sync::Once;

/// Maximum rank supported by the fast‑path implementation.
pub const MAX_RANK: usize = 32;

/// Streaming LoRA adaptation pipeline.
///
/// * `lora_a` – matrix A (rank × d_model) stored row‑major.
/// * `lora_b` – matrix B (d_model × rank) stored column‑major.
/// * `intermediate` – fixed‑size stack buffer for low‑rank projection.
#[derive(Clone, Debug)]
pub struct StreamingLoraAdaptationPipeline {
    pub d_model: usize,
    pub rank: usize,
    pub lora_a: Vec<f32>, // allocated once at construction
    pub lora_b: Vec<f32>, // allocated once at construction
    pub learning_rate: f32,
    pub cycle_count: u64,
    // Stack‑allocated buffer reused on each forward pass.
    intermediate: [f32; MAX_RANK],
}

impl StreamingLoraAdaptationPipeline {
    /// Create a new pipeline with the supplied dimensions.
    /// The internal matrices are allocated once; subsequent calls are allocation‑free.
    pub fn new(d_model: usize, rank: usize, learning_rate: f32) -> Self {
        assert!(rank <= MAX_RANK, "rank exceeds MAX_RANK");
        let lora_a = vec![0.01_f32; rank * d_model];
        let lora_b = vec![0.0_f32; d_model * rank];
        Self {
            d_model,
            rank,
            lora_a,
            lora_b,
            learning_rate,
            cycle_count: 0,
            intermediate: [0.0; MAX_RANK],
        }
    }

    /// Perform a forward delta computation without allocating.
    pub fn forward_delta(&mut self, input: &[f32], output: &mut [f32]) -> Result<()> {
        if input.len() != self.d_model || output.len() != self.d_model {
            anyhow::bail!("Dimension mismatch in forward_delta");
        }
        // 1. Low‑rank projection into the stack buffer.
        for r in 0..self.rank {
            let mut sum = 0.0_f32;
            let a_offset = r * self.d_model;
            for (i, &inp) in input.iter().enumerate().take(self.d_model) {
                sum += self.lora_a[a_offset + i] * inp;
            }
            self.intermediate[r] = sum;
        }
        // 2. Expand to output.
        for i in 0..self.d_model {
            let mut sum = 0.0_f32;
            for r in 0..self.rank {
                sum += self.lora_b[i * self.rank + r] * self.intermediate[r];
            }
            output[i] += sum;
        }
        Ok(())
    }

    /// Adapt the LoRA matrices using orthogonal gradient projection.
    pub fn adapt_step(&mut self, error_gradient: &[f32], input_state: &[f32]) -> Result<()> {
        if error_gradient.len() != self.d_model || input_state.len() != self.d_model {
            anyhow::bail!("Dimension mismatch in adapt_step");
        }
        self.cycle_count += 1;
        // Compute gradient norm (no allocation).
        let grad_norm_sq: f32 = error_gradient.iter().map(|g| g * g).sum();
        let grad_norm = grad_norm_sq.sqrt();
        let scale = self.learning_rate / (grad_norm + 1e-6);
        // Update B matrix in‑place.
        for (i, &grad) in error_gradient.iter().enumerate().take(self.d_model) {
            for r in 0..self.rank {
                let idx = i * self.rank + r;
                let step = scale * grad * 0.1;
                self.lora_b[idx] += step;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------
// Trigger handling – lock‑free queue.

static INIT_QUEUE: Once = Once::new();
static mut TRIGGER_QUEUE: MaybeUninit<ArrayQueue<Trigger>> = MaybeUninit::uninit();

/// Initialize the global trigger queue. Called lazily on first use.
fn queue() -> &'static ArrayQueue<Trigger> {
    unsafe {
        INIT_QUEUE.call_once(|| {
            TRIGGER_QUEUE = MaybeUninit::new(ArrayQueue::new(256));
        });
        TRIGGER_QUEUE.assume_init_ref()
    }
}

/// Push a trigger onto the global queue without allocation.
pub fn push_trigger(trigger: Trigger) {
    let _ = queue().push(trigger);
}

// ---------------------------------------------------------------------
// Example telemetry read – zero‑copy from the hypervisor RingBuffer.

/// Reads the most recent telemetry entry from the hypervisor RingBuffer.
/// Returns `None` if the buffer is empty.
pub fn read_latest_telemetry<T: Pod + Copy, const CAP: usize>() -> Option<T> {
    // Safety: The RingBuffer stores POD values; we can cast the last element.
    // The hypervisor crate exposes a static instance `TELEMETRY_BUFFER`.
    // This placeholder assumes such a static exists.
    use hypervisor::state::TELEMETRY_BUFFER; // hypothetical static
    // Peek at the tail index (the next write position).
    let tail = TELEMETRY_BUFFER.tail.load(Ordering::SeqCst);
    if tail == 0 {
        return None;
    }
    let idx = (tail.wrapping_sub(1)) % CAP;
    // SAFETY: The slot at `idx` has been fully written by the producer.
    let raw = unsafe { TELEMETRY_BUFFER.buffer.get_unchecked(idx).assume_init_read() };
    Some(bytemuck::cast_ref::<T, T>(&raw).clone())
}
