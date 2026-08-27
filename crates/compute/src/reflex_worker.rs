//! crates/compute/src/reflex_worker.rs
//! Tier 3: Kinetic Specialist & Reflex Worker Hot Loop.
//!
//! Features:
//! 1. Sub-microsecond lock-free spin-wait for new subgoals from Router on SPMC channel.
//! 2. Ingests current sensory state (AST diff, UI tree, or sensory vector).
//! 3. Conditions input: x_t = x_sensory + s_subgoal.
//! 4. Continuous state-space recurrence execution (h_t = A·h_{t-1} + B·x_t) in < 180µs.
//! 5. Decodes and returns physical MachineOpcode actions.

use std::hint::spin_loop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use anyhow::Result;

use crate::machine_native::MachineOpcode;
use crate::si_solid_state::{SiOnlineLearner, SolidStateSiContainer};
use crate::si_ssm::SsmStatePrediction;
use nervous_system::specialist_bus::SpecialistSpmcChannel;

/// Configuration and telemetry for a Tier 3 Reflex Worker
pub struct ReflexWorker {
    pub worker_id: u16,
    pub name: String,
    pub learner: SiOnlineLearner,
    pub last_processed_seq: u64,
    pub total_steps_executed: u64,
    pub avg_step_latency_us: u64,
}

impl ReflexWorker {
    /// Creates a new ReflexWorker from a loaded .si container
    pub fn new(worker_id: u16, name: &str, container: SolidStateSiContainer, use_gpu: bool) -> Result<Self> {
        let learner = SiOnlineLearner::new(container, use_gpu)?;
        Ok(Self {
            worker_id,
            name: name.to_string(),
            learner,
            last_processed_seq: 0,
            total_steps_executed: 0,
            avg_step_latency_us: 0,
        })
    }

    /// Single non-blocking tick of the kinetic pursuit loop
    pub fn tick_step(
        &mut self,
        channel: &SpecialistSpmcChannel,
        sensory_state: &[f32],
    ) -> Result<Option<(MachineOpcode, SsmStatePrediction)>> {
        let write_head = channel.write_cursor.value.load(Ordering::Acquire);
        if write_head == self.last_processed_seq || write_head == 0 {
            return Ok(None);
        }

        // 1. Read the 256-dim subgoal from the 128-byte aligned SPMC channel
        if let Some(subgoal) = channel.read_latest(250) {
            let start = Instant::now();
            self.last_processed_seq = write_head;

            // 2. Condition sensory state with incoming subgoal: x_t = x_sensory + s_subgoal
            let state_dim = self.learner.container.config.state_dim;
            let mut x_t = vec![0.0f32; state_dim];
            for (i, &s) in sensory_state.iter().enumerate().take(state_dim) {
                x_t[i] = s;
            }
            for (i, &sg) in subgoal.iter().enumerate().take(state_dim) {
                x_t[i] += sg;
            }

            // 3. Sub-180µs continuous state-space step
            let pred = self.learner.forward_adapted_step(&x_t)?;
            let duration_us = start.elapsed().as_micros() as u64;

            self.total_steps_executed += 1;
            self.avg_step_latency_us = if self.total_steps_executed == 1 {
                duration_us
            } else {
                (self.avg_step_latency_us * 9 + duration_us) / 10
            };

            let opcode = pred.predicted_opcode.clone();
            Ok(Some((opcode, pred)))
        } else {
            Ok(None)
        }
    }

    /// Continuous hot loop running until the shutdown signal is set
    pub fn run_continuous(
        &mut self,
        channel: &SpecialistSpmcChannel,
        shutdown: Arc<AtomicBool>,
        max_iterations: Option<u64>,
    ) -> Result<u64> {
        let mut iterations = 0u64;
        let dummy_sensory = vec![0.1f32; self.learner.container.config.state_dim];

        while !shutdown.load(Ordering::Relaxed) {
            if let Some(limit) = max_iterations {
                if iterations >= limit {
                    break;
                }
            }

            let write_head = channel.write_cursor.value.load(Ordering::Acquire);
            if write_head == self.last_processed_seq {
                spin_loop(); // Yield CPU execution pipeline without OS context switch
                continue;
            }

            if let Some((_opcode, _pred)) = self.tick_step(channel, &dummy_sensory)? {
                iterations += 1;
            }
        }

        Ok(iterations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::si_ssm::SiSsmConfig;
    use nervous_system::specialist_bus::TENSOR_DIM;

    #[test]
    fn test_reflex_worker_step_and_spin_read() {
        let config = SiSsmConfig {
            model_name: "ReflexWorker-Test".to_string(),
            state_dim: 128,
            d_model: 32,
            d_state: 8,
            d_conv: 2,
            dt_rank: 4,
            num_layers: 1,
            num_opcodes: 8,
            param_count: 10_000,
        };

        let container = SolidStateSiContainer::new("Reflex Test", config);
        let mut worker = ReflexWorker::new(1, "DesktopEmulator-Worker", container, false).unwrap();
        let channel = SpecialistSpmcChannel::new(0, "Router-Channel");

        // Step before any publishing -> returns None
        let sensory = vec![0.05f32; 128];
        let res1 = worker.tick_step(&channel, &sensory).unwrap();
        assert!(res1.is_none());

        // Publish a subgoal
        let subgoal = [0.2f32; TENSOR_DIM];
        channel.publish_tensor(&subgoal).unwrap();

        // Step after publishing -> processes and returns action
        let res2 = worker.tick_step(&channel, &sensory).unwrap();
        assert!(res2.is_some());
        let (_opcode, pred) = res2.unwrap();
        assert_eq!(pred.predicted_state.len(), 128);
        assert!(worker.total_steps_executed == 1);
    }
}
