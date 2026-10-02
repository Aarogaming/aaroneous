//! crates/compute/src/episodic_accumulator.rs
//! Background episodic thought packet accumulator.
//! Consumes observation frames from `ObservationBuffer`, synthesizes continuous trajectories,
//! generates machine-native `SiThoughtPacket`s, and persists them into WAL / Block 3 skill stack.

use crate::si_binary::SiThoughtPacket;
use crate::si_solid_state::SolidStateSiContainer;
use anyhow::Result;
use ipc_bus::{ObservationBuffer, ObservationFramePod};
use serde::{Deserialize, Serialize};
use si_ir::{
    DimensionalUnit, MachineOpcode, NativeComputationNode, NativeComputationalGraph,
    NativeTypeLattice,
};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Configuration parameters for the episodic thought accumulator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicAccumulatorConfig {
    /// Minimum steps required to crystallize an episodic thought trajectory
    pub min_episode_len: usize,
    /// Maximum steps in a single trajectory before forced crystallization
    pub max_episode_len: usize,
    /// Minimum cumulative reward threshold to retain an episode as a crystallized skill
    pub min_crystallize_reward: f32,
    /// Directory for persisting episodic trajectory WAL records
    pub wal_dir: Option<PathBuf>,
}

impl Default for EpisodicAccumulatorConfig {
    fn default() -> Self {
        Self {
            min_episode_len: 4,
            max_episode_len: 32,
            min_crystallize_reward: 0.0,
            wal_dir: None,
        }
    }
}

/// Background worker accumulating observation frames into machine-native thought packets.
pub struct EpisodicThoughtAccumulator {
    config: EpisodicAccumulatorConfig,
    last_sequence: u64,
    current_trajectory: Vec<ObservationFramePod>,
    crystallized_count: usize,
}

impl EpisodicThoughtAccumulator {
    /// Creates a new accumulator with the given configuration.
    pub fn new(config: EpisodicAccumulatorConfig) -> Self {
        Self {
            config,
            last_sequence: 0,
            current_trajectory: Vec::new(),
            crystallized_count: 0,
        }
    }

    /// Sets the cursor sequence to begin consuming from.
    pub fn set_last_sequence(&mut self, seq: u64) {
        self.last_sequence = seq;
    }

    /// Returns the total number of thought packets crystallized by this accumulator.
    pub fn crystallized_count(&self) -> usize {
        self.crystallized_count
    }

    /// Non-blockingly polls the observation buffer for new frames and crystallizes completed episodes.
    pub fn poll_and_accumulate(
        &mut self,
        buffer: &ObservationBuffer,
    ) -> Result<Vec<SiThoughtPacket>> {
        let current_head = buffer.current_sequence();
        if current_head <= self.last_sequence {
            return Ok(Vec::new());
        }

        let mut crystallized = Vec::new();

        // Advance cursor if buffer wrapped past our position
        if current_head > buffer.capacity() as u64
            && self.last_sequence < current_head - buffer.capacity() as u64
        {
            self.last_sequence = current_head - buffer.capacity() as u64;
            self.current_trajectory.clear();
        }

        let start_seq = self.last_sequence + 1;
        for seq in start_seq..=current_head {
            if let Some(frame) = buffer.read_by_sequence(seq) {
                let is_action_trigger = frame.actual_opcode != 0;
                self.current_trajectory.push(frame);
                self.last_sequence = seq;

                let reached_max = self.current_trajectory.len() >= self.config.max_episode_len;
                let reached_min = self.current_trajectory.len() >= self.config.min_episode_len;

                // Crystallize episode when an action completes or max length is reached
                if (is_action_trigger && reached_min) || reached_max {
                    if let Some(packet) = self.crystallize_trajectory(&self.current_trajectory)? {
                        if let Some(ref wal_dir) = self.config.wal_dir {
                            self.append_to_wal(wal_dir, &packet)?;
                        }
                        crystallized.push(packet);
                        self.crystallized_count += 1;
                    }
                    self.current_trajectory.clear();
                }
            } else {
                // Gap or overrun; update cursor
                self.last_sequence = seq;
            }
        }

        Ok(crystallized)
    }

    /// Crystallizes a slice of observation frames into a machine-native `SiThoughtPacket`.
    pub fn crystallize_trajectory(
        &self,
        trajectory: &[ObservationFramePod],
    ) -> Result<Option<SiThoughtPacket>> {
        if trajectory.is_empty() {
            return Ok(None);
        }

        let total_reward: f32 = trajectory.iter().map(|f| f.reward).sum();
        if total_reward < self.config.min_crystallize_reward && !trajectory.is_empty() {
            // Below retention threshold
            return Ok(None);
        }

        let last_frame = trajectory.last().unwrap();
        let goal_opcode = if last_frame.actual_opcode != 0 {
            last_frame.actual_opcode
        } else {
            0x0100 // Default state progression opcode
        };

        // Compute mean state vector across the trajectory
        let mut mean_state = vec![0.0f32; 256];
        let n = trajectory.len() as f32;
        for frame in trajectory {
            for (mean, feature) in mean_state.iter_mut().zip(frame.state_features.iter()) {
                *mean += feature / n;
            }
        }

        // Construct native computational DAG for this episodic habit
        let mut graph = NativeComputationalGraph::new();

        // Node 1: Memory state tensor input
        let node1 = NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc {
                size_bytes: 1024,
                align: 64,
            },
            type_lattice: NativeTypeLattice::TensorType {
                shape: vec![256],
                element_type: Box::new(NativeTypeLattice::PrimitiveFloat { bits: 32 }),
            },
            energy_cost: 0.05,
            dependencies: vec![],
        };
        graph.add_node(node1);

        // Node 2: Action decision transition
        let node2 = NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Call {
                function_id: goal_opcode as u64,
                arg_regs: vec![],
            },
            type_lattice: NativeTypeLattice::PrimitiveInt {
                bits: 16,
                signed: false,
            },
            energy_cost: (last_frame.free_energy as f64).max(0.01),
            dependencies: vec![1],
        };
        graph.add_node(node2);

        graph.entry_node = 1;
        graph.exit_node = 2;
        graph.shannon_entropy = (1.0 - last_frame.confidence as f64).max(0.001);

        let packet = SiThoughtPacket::new(
            goal_opcode,
            DimensionalUnit::DIMENSIONLESS,
            mean_state,
            graph,
        );

        Ok(Some(packet))
    }

    /// Appends newly crystallized thought packets into a container's Block 3 skill stack.
    pub fn consolidate_into_container(
        &self,
        packets: &[SiThoughtPacket],
        container: &mut SolidStateSiContainer,
    ) {
        container.skill_stack.extend_from_slice(packets);
    }

    /// Appends a crystallized thought packet to disk WAL.
    fn append_to_wal(&self, wal_dir: &Path, packet: &SiThoughtPacket) -> Result<()> {
        std::fs::create_dir_all(wal_dir)?;
        let wal_path = wal_dir.join("episodes.wal");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(wal_path)?;

        let bytes = serde_json::to_vec(packet)?;
        let len_bytes = (bytes.len() as u32).to_le_bytes();
        file.write_all(&len_bytes)?;
        file.write_all(&bytes)?;
        file.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episodic_thought_accumulation() {
        let mut buffer = ObservationBuffer::new_in_memory(32).unwrap();
        let config = EpisodicAccumulatorConfig {
            min_episode_len: 2,
            max_episode_len: 8,
            min_crystallize_reward: 0.0,
            wal_dir: None,
        };

        let mut accumulator = EpisodicThoughtAccumulator::new(config);

        // Record 5 idle frames followed by an action frame
        for i in 0..5 {
            let mut state_features = [0.0f32; 256];
            state_features[0] = i as f32;
            let f = ObservationFramePod {
                actual_opcode: 0,
                reward: 0.5,
                state_features,
                ..Default::default()
            };
            buffer.record(f).unwrap();
        }

        let mut state_features = [0.0f32; 256];
        state_features[0] = 5.0;
        let action_frame = ObservationFramePod {
            actual_opcode: 0x0200, // Routing action
            reward: 2.0,
            confidence: 0.98,
            state_features,
            ..Default::default()
        };
        buffer.record(action_frame).unwrap();

        // Poll accumulator
        let packets = accumulator.poll_and_accumulate(&buffer).unwrap();
        assert_eq!(packets.len(), 1);

        let packet = &packets[0];
        assert_eq!(packet.header.goal_opcode, 0x0200);
        assert_eq!(packet.state_tensors.len(), 256);
        assert_eq!(packet.graph.nodes.len(), 2);
        assert_eq!(accumulator.crystallized_count(), 1);

        // Verify consolidation into container
        let mut container = SolidStateSiContainer::factory_default_reflex().unwrap();
        assert_eq!(container.skill_stack.len(), 0);
        accumulator.consolidate_into_container(&packets, &mut container);
        assert_eq!(container.skill_stack.len(), 1);
        assert_eq!(container.skill_stack[0].header.goal_opcode, 0x0200);
    }
}
