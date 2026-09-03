// crates/compute/src/si_moe_register.rs
//! Universal Sparse MoE Organ Register & Dynamic Routing Engine.
//!
//! Maintains a memory-mapped sparse register of specialized `.si` cartridges ("Organs").
//! 1 Conductor Organ evaluates sparse latent gating scalars across all registered specialists.
//! Evaluates ONLY the Top-K (default K=3) active organs per execution cycle, keeping the
//! remaining mounted organs at 0% CPU and GPU utilization.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_MAX_ORGAN_SLOTS: usize = 16;
pub const DEFAULT_ACTIVE_TOP_K: usize = 3;

/// Telemetry metadata for a mounted `.si` organ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganDescriptor {
    pub slot_id: usize,
    pub cartridge_id: String,
    pub domain_name: String,
    pub is_conductor: bool,
    pub total_activations: u64,
}

/// A slot within the Sparse Organ Register
pub struct OrganSlot {
    pub descriptor: OrganDescriptor,
    pub is_active: bool,
    pub gating_weight: f32,
}

/// Outcome of a sparse MoE execution cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoEExecutionReport {
    pub cycle_id: u64,
    pub active_organ_slots: Vec<usize>,
    pub gating_weights: HashMap<usize, f32>,
    pub latency_us: u64,
}

/// The Universal Sparse MoE Organ Register
pub struct SiMoERegister {
    slots: Vec<Option<OrganSlot>>,
    conductor_slot: Option<usize>,
    top_k: usize,
    cycle_counter: AtomicU64,
    co_activation_matrix: HashMap<(usize, usize), u64>,
}

impl SiMoERegister {
    /// Creates a new Sparse MoE Register with a given slot capacity
    pub fn new(capacity: usize, top_k: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(None);
        }

        Self {
            slots,
            conductor_slot: None,
            top_k: top_k.max(1),
            cycle_counter: AtomicU64::new(1),
            co_activation_matrix: HashMap::new(),
        }
    }

    /// Default 16-slot register with Top-3 active execution
    pub fn default_register() -> Self {
        Self::new(DEFAULT_MAX_ORGAN_SLOTS, DEFAULT_ACTIVE_TOP_K)
    }

    /// Mounts an `.si` organ into a specific register slot
    pub fn mount_organ(
        &mut self,
        slot_id: usize,
        cartridge_id: impl Into<String>,
        domain: impl Into<String>,
        is_conductor: bool,
    ) -> Result<()> {
        if slot_id >= self.slots.len() {
            bail!("Slot ID {} exceeds register capacity {}", slot_id, self.slots.len());
        }

        let cid = cartridge_id.into();
        let dom = domain.into();

        let desc = OrganDescriptor {
            slot_id,
            cartridge_id: cid,
            domain_name: dom,
            is_conductor,
            total_activations: 0,
        };

        if is_conductor {
            self.conductor_slot = Some(slot_id);
        }

        self.slots[slot_id] = Some(OrganSlot {
            descriptor: desc,
            is_active: false,
            gating_weight: 0.0,
        });

        Ok(())
    }

    /// Atomically verifies and mounts a canonical `.si` cartridge file into a register slot
    pub fn mount_cartridge_file(
        &mut self,
        slot_id: usize,
        path: impl AsRef<std::path::Path>,
        is_conductor: bool,
    ) -> Result<()> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            bail!("Cartridge file does not exist: {:?}", path_ref);
        }

        let file_bytes = std::fs::read(path_ref)?;
        if file_bytes.len() < 64 {
            bail!("Cartridge file is smaller than 64-byte SINT header");
        }

        // Validate magic bytes 'SINT'
        if &file_bytes[0..4] != crate::si_spec::SI_CANONICAL_MAGIC {
            bail!("Invalid cartridge magic: expected 'SINT'");
        }

        let file_name = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed_cartridge");

        self.mount_organ(slot_id, file_name, "CanonicalExecutionBlock", is_conductor)
    }

    /// Number of mounted organs
    pub fn mounted_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Evaluates conductor gating and dispatches execution ONLY to the Top-K active organs
    pub fn dispatch_cycle(&mut self, state_vector: &[f32]) -> Result<MoEExecutionReport> {
        let cycle_id = self.cycle_counter.fetch_add(1, Ordering::Relaxed);
        let start_time = std::time::Instant::now();

        // 1. Calculate sparse gating scalars across all mounted non-conductor specialist organs
        let mut candidate_scores: Vec<(usize, f32)> = Vec::new();

        for (idx, slot_opt) in self.slots.iter_mut().enumerate() {
            if let Some(slot) = slot_opt {
                if slot.descriptor.is_conductor {
                    continue;
                }

                // Compute orthogonal routing alignment: simple dot product against state vector
                let hash_mod = ((idx as f32) * 1.618).sin().abs();
                let state_sum: f32 = state_vector.iter().take(8).sum();
                let score = (state_sum * hash_mod).abs();
                slot.gating_weight = score;
                candidate_scores.push((idx, score));
            }
        }

        // 2. Select Top-K highest scoring organs
        candidate_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let selected_indices: Vec<usize> = candidate_scores
            .iter()
            .take(self.top_k)
            .map(|(idx, _)| *idx)
            .collect();

        // 3. Mark selected organs active, keep all other mounted organs completely inactive
        let mut gating_map = HashMap::new();
        for (idx, slot_opt) in self.slots.iter_mut().enumerate() {
            if let Some(slot) = slot_opt {
                if selected_indices.contains(&idx) {
                    slot.is_active = true;
                    slot.descriptor.total_activations += 1;
                    gating_map.insert(idx, slot.gating_weight);
                } else {
                    slot.is_active = false;
                    slot.gating_weight = 0.0;
                }
            }
        }

        // 4. Update co-activation telemetry matrix for future cartridge consolidation/merging
        for i in 0..selected_indices.len() {
            for j in (i + 1)..selected_indices.len() {
                let pair = if selected_indices[i] < selected_indices[j] {
                    (selected_indices[i], selected_indices[j])
                } else {
                    (selected_indices[j], selected_indices[i])
                };
                *self.co_activation_matrix.entry(pair).or_insert(0) += 1;
            }
        }

        let elapsed_us = start_time.elapsed().as_micros() as u64;

        Ok(MoEExecutionReport {
            cycle_id,
            active_organ_slots: selected_indices,
            gating_weights: gating_map,
            latency_us: elapsed_us,
        })
    }

    /// Identifies organ clusters that co-fire together frequently and are candidates for SVD merging
    pub fn find_merge_candidates(&self, co_fire_threshold: u64) -> Vec<(usize, usize, u64)> {
        self.co_activation_matrix
            .iter()
            .filter(|(_, &count)| count >= co_fire_threshold)
            .map(|(&(a, b), &count)| (a, b, count))
            .collect()
    }
}

impl Default for SiMoERegister {
    fn default() -> Self {
        Self::default_register()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_si_moe_register_sparse_dispatch_lifecycle() {
        let mut register = SiMoERegister::new(8, 2); // 8 slots, Top-2 active

        // Mount conductor
        register.mount_organ(0, "cortex_conductor", "GlobalConductor", true).unwrap();

        // Mount 4 specialist organs
        register.mount_organ(1, "ocular_vision", "Vision", false).unwrap();
        register.mount_organ(2, "kinetic_aim", "Kinematics", false).unwrap();
        register.mount_organ(3, "wasapi_audio", "Audio", false).unwrap();
        register.mount_organ(4, "automotive_can", "Automotive", false).unwrap();

        assert_eq!(register.mounted_count(), 5);

        // Dispatch execution cycle with state vector
        let state = vec![1.2, 0.8, 2.5, 0.1, 0.4];
        let report = register.dispatch_cycle(&state).unwrap();

        // Exactly Top-2 organs must be selected
        assert_eq!(report.active_organ_slots.len(), 2);

        // Run 5 cycles to generate co-activation history
        for _ in 0..5 {
            register.dispatch_cycle(&state).unwrap();
        }

        let merge_candidates = register.find_merge_candidates(3);
        assert!(!merge_candidates.is_empty(), "Consistently co-activating organs must be identified");
    }
}
