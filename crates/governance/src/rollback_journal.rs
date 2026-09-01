//! crates/governance/src/rollback_journal.rs
//! Generational Rollback Journal & Adaptive Safety Guardrails.
//! Maintains an append-only ring of verified state snapshots, LoRA weights, and JIT function pointers.
//! Automatically intercepts thermodynamic violations or execution traps and rolls back to the
//! preceding stable generation with zero hypervisor downtime.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// A snapshot entry representing a verified system generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationSnapshot<T> {
    pub generation_id: u64,
    pub timestamp_ms: u64,
    pub thermodynamic_free_energy: f64,
    pub state: T,
    pub is_verified_stable: bool,
}

/// Generational Journal holding an append-only bounded ring of verified state checkpoints
#[derive(Debug, Clone)]
pub struct GenerationalJournal<T> {
    snapshots: VecDeque<GenerationSnapshot<T>>,
    max_history_capacity: usize,
    current_generation: u64,
    rollback_count: u64,
    max_free_energy_bound: f64,
}

impl<T: Clone> Default for GenerationalJournal<T> {
    fn default() -> Self {
        Self::new(32, 0.05)
    }
}

impl<T: Clone> GenerationalJournal<T> {
    /// Creates a new GenerationalJournal with capacity and free-energy bound
    pub fn new(capacity: usize, max_free_energy: f64) -> Self {
        Self {
            snapshots: VecDeque::with_capacity(capacity),
            max_history_capacity: capacity,
            current_generation: 0,
            rollback_count: 0,
            max_free_energy_bound: max_free_energy,
        }
    }

    /// Records a new verified state checkpoint into the journal
    pub fn record_checkpoint(
        &mut self,
        generation_id: u64,
        state: T,
        free_energy: f64,
    ) -> Result<()> {
        if free_energy > self.max_free_energy_bound {
            return Err(anyhow!(
                "Cannot checkpoint unstable state: free energy {:.4} exceeds bound {:.4}",
                free_energy,
                self.max_free_energy_bound
            ));
        }

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if self.snapshots.len() >= self.max_history_capacity {
            self.snapshots.pop_front();
        }

        self.snapshots.push_back(GenerationSnapshot {
            generation_id,
            timestamp_ms: ts,
            thermodynamic_free_energy: free_energy,
            state,
            is_verified_stable: true,
        });

        self.current_generation = generation_id;
        Ok(())
    }

    /// Rolls back to the most recent stable generation before the current failing state (Cold Path)
    #[cold]
    #[inline(never)]
    pub fn rollback_to_last_stable(&mut self) -> Option<T> {
        if self.snapshots.is_empty() {
            return None;
        }

        // Remove failing current generation if present
        if self.snapshots.len() > 1 {
            let _failing = self.snapshots.pop_back();
        }

        if let Some(stable) = self.snapshots.back() {
            self.current_generation = stable.generation_id;
            self.rollback_count += 1;
            Some(stable.state.clone())
        } else {
            None
        }
    }

    /// Rolls back to a specific generation ID if present in the journal (Cold Path)
    #[cold]
    #[inline(never)]
    pub fn rollback_to_generation(&mut self, generation_id: u64) -> Option<T> {
        let idx = self.snapshots.iter().position(|s| s.generation_id == generation_id)?;
        self.snapshots.truncate(idx + 1);
        let snapshot = self.snapshots.back()?;
        self.current_generation = snapshot.generation_id;
        self.rollback_count += 1;
        Some(snapshot.state.clone())
    }

    /// Executes an adaptation or JIT compilation closure inside a thermodynamic safety guardrail.
    /// If the closure fails or produces an unstable free energy state, automatically rolls back.
    pub fn execute_with_guardrail<F, R>(
        &mut self,
        current_state: &mut T,
        action: F,
    ) -> Result<R>
    where
        F: FnOnce(&mut T) -> Result<(R, f64)>,
    {
        let pre_state = current_state.clone();

        match action(current_state) {
            Ok((result, new_free_energy)) => {
                if new_free_energy > self.max_free_energy_bound {
                    // Thermodynamic violation: Restore pre-state and rollback
                    *current_state = pre_state;
                    self.rollback_count += 1;
                    return Err(anyhow!(
                        "Thermodynamic guardrail triggered: Delta F {:.4} > {:.4}. Rolled back.",
                        new_free_energy,
                        self.max_free_energy_bound
                    ));
                }

                // Stable: Record new generation
                let next_gen = self.current_generation + 1;
                let _ = self.record_checkpoint(next_gen, current_state.clone(), new_free_energy);
                Ok(result)
            }
            Err(e) => {
                // Trap / error: Restore pre-state
                *current_state = pre_state;
                self.rollback_count += 1;
                Err(anyhow!("Execution trap encountered: {e}. Rolled back to generation {}.", self.current_generation))
            }
        }
    }

    /// Total number of rollbacks executed across runtime lifecycle
    pub fn rollback_count(&self) -> u64 {
        self.rollback_count
    }

    /// Current active generation ID
    pub fn current_generation(&self) -> u64 {
        self.current_generation
    }

    /// Number of active snapshots in the ring buffer
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct MockNeuralState {
        lora_weights: Vec<f32>,
        model_version: u32,
    }

    #[test]
    fn test_generational_journal_record_and_rollback() {
        let mut journal = GenerationalJournal::new(10, 0.05);

        let state_v1 = MockNeuralState {
            lora_weights: vec![0.1, 0.2],
            model_version: 1,
        };

        let state_v2 = MockNeuralState {
            lora_weights: vec![0.3, 0.4],
            model_version: 2,
        };

        journal.record_checkpoint(1, state_v1.clone(), 0.01).unwrap();
        journal.record_checkpoint(2, state_v2.clone(), 0.02).unwrap();

        assert_eq!(journal.current_generation(), 2);
        assert_eq!(journal.snapshot_count(), 2);

        // Rollback to last stable
        let restored = journal.rollback_to_last_stable().unwrap();
        assert_eq!(restored, state_v1);
        assert_eq!(journal.current_generation(), 1);
        assert_eq!(journal.rollback_count(), 1);
    }

    #[test]
    fn test_execute_with_guardrail_thermodynamic_violation_rollback() {
        let mut journal = GenerationalJournal::new(10, 0.05);

        let mut current_state = MockNeuralState {
            lora_weights: vec![1.0, 1.0],
            model_version: 1,
        };

        journal.record_checkpoint(1, current_state.clone(), 0.01).unwrap();

        // Attempt an adaptation that destabilizes free energy to 0.15 (> 0.05)
        let result = journal.execute_with_guardrail(&mut current_state, |state| {
            state.lora_weights = vec![99.0, 99.0]; // Poisoned weights
            Ok(("adapted_token", 0.15)) // Free energy violation
        });

        assert!(result.is_err());
        // State must be completely restored to safe pre-state
        assert_eq!(current_state.lora_weights, vec![1.0, 1.0]);
        assert_eq!(journal.rollback_count(), 1);
    }
}
