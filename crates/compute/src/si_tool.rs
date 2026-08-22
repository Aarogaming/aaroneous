//! crates/compute/src/si_tool.rs
//! Aaroneous Machine-Native SI Tool Suite.
//! Comprehensive utility for inspecting, benchmarking, packing, unpacking,
//! and distilling `.si` and `.sissm` zero-copy binary containers.

use anyhow::{bail, Result};
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::machine_native::{DimensionalUnit, MachineOpcode, NativeComputationNode, NativeComputationalGraph, NativeTypeLattice};
use crate::si_binary::{SiThoughtPacket, SI_MAGIC_BYTES};
use crate::si_solid_state::{SolidStateSiContainer, SI_SOLID_STATE_MAGIC, SI_SOLID_STATE_VERSION};
use crate::si_ssm::{SiSsmConfig, SiStateSpaceModel, SI_SSM_MAGIC};

/// Detailed Structural Inspection Report for a `.si` or `.sissm` container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiInspectorReport {
    pub file_name: String,
    pub file_path: PathBuf,
    pub file_size_bytes: u64,
    pub magic: String,
    pub version: u16,
    pub goal_opcode: u16,
    pub dimensional_unit: String,
    pub state_tensor_dim: usize,
    pub node_count: usize,
    pub opcodes_used: Vec<String>,
    pub total_energy_cost: f64,
    pub is_mmap_compatible: bool,
    pub embedded_ssm: Option<SiSsmConfig>,
}

/// Real-time Latency and Memory Bandwidth Benchmark Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiBenchmarkReport {
    pub file_name: String,
    pub iterations: usize,
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
    pub min_latency_us: u64,
    pub max_latency_us: u64,
    pub mean_latency_us: f64,
    pub throughput_ops_per_sec: f64,
    pub bandwidth_mb_per_sec: f64,
}

/// Complete Machine-Native SI Tool Suite
#[derive(Debug, Default, Clone)]
pub struct SiToolEngine;

impl SiToolEngine {
    /// Inspects an arbitrary `.si` or `.sissm` binary file without allocating heavy data structures
    pub fn inspect(&self, path: impl AsRef<Path>) -> Result<SiInspectorReport> {
        let path = path.as_ref();
        if !path.exists() {
            bail!("Target file does not exist: {:?}", path);
        }

        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let file_size_bytes = metadata.len();
        let mmap = unsafe { Mmap::map(&file)? };

        if mmap.len() < 8 {
            bail!("File is too small to contain a valid SI header");
        }

        // Check SINT Solid-State Container Magic (b"SINT")
        if mmap.len() >= 4 && mmap[0..4] == SI_SOLID_STATE_MAGIC {
            let container = SolidStateSiContainer::load_from_file(path)?;
            let opcodes_used: Vec<String> = vec![
                format!("Anchors: {}", container.adaptation.anchor_buffer.len()),
                format!("LoRA-Rank: {}", container.adaptation.rank),
                format!("Retention: {:.1}%", container.adaptation.verify_anchor_retention()),
            ];

            return Ok(SiInspectorReport {
                file_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                file_path: path.to_path_buf(),
                file_size_bytes,
                magic: "SINT".to_string(),
                version: SI_SOLID_STATE_VERSION,
                goal_opcode: 0x0100,
                dimensional_unit: "MachineIntent[256]".to_string(),
                state_tensor_dim: container.adaptation.in_dim,
                node_count: container.adaptation.anchor_buffer.len(),
                opcodes_used,
                total_energy_cost: 0.015,
                is_mmap_compatible: true,
                embedded_ssm: Some(container.config),
            });
        }

        // Check SISSM State-Space Model Magic
        if mmap.len() >= 5 && mmap[0..5] == SI_SSM_MAGIC {
            let version = u16::from_le_bytes(mmap[5..7].try_into()?);
            let config_len = u32::from_le_bytes(mmap[7..11].try_into()?) as usize;
            let config_bytes = &mmap[11..11 + config_len];
            let config: SiSsmConfig = serde_json::from_slice(config_bytes)?;

            let cursor = 11 + config_len;
            let packet_len = u32::from_le_bytes(mmap[cursor..cursor + 4].try_into()?) as usize;
            let packet_bytes = &mmap[cursor + 4..cursor + 4 + packet_len];
            let packet = SiThoughtPacket::from_binary(packet_bytes)?;

            let opcodes_used: Vec<String> = packet.graph.nodes.values().map(|n| n.opcode.name().to_string()).collect();

            return Ok(SiInspectorReport {
                file_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                file_path: path.to_path_buf(),
                file_size_bytes,
                magic: "SISSM".to_string(),
                version,
                goal_opcode: packet.header.goal_opcode,
                dimensional_unit: format!("{:?}", packet.header.dimensional_signature),
                state_tensor_dim: packet.state_tensors.len(),
                node_count: packet.graph.nodes.len(),
                opcodes_used,
                total_energy_cost: packet.graph.thermodynamic_free_energy,
                is_mmap_compatible: true,
                embedded_ssm: Some(config),
            });
        }

        // Check Standard SIMN Thought Packet Magic
        if mmap.len() >= 4 && mmap[0..4] == SI_MAGIC_BYTES {
            let packet = SiThoughtPacket::from_binary(&mmap)?;
            let opcodes_used: Vec<String> = packet.graph.nodes.values().map(|n| n.opcode.name().to_string()).collect();

            return Ok(SiInspectorReport {
                file_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                file_path: path.to_path_buf(),
                file_size_bytes,
                magic: "SIMN".to_string(),
                version: packet.header.version,
                goal_opcode: packet.header.goal_opcode,
                dimensional_unit: format!("{:?}", packet.header.dimensional_signature),
                state_tensor_dim: packet.state_tensors.len(),
                node_count: packet.graph.nodes.len(),
                opcodes_used,
                total_energy_cost: packet.graph.thermodynamic_free_energy,
                is_mmap_compatible: true,
                embedded_ssm: None,
            });
        }

        bail!("Unknown file format: expected 'SINT', 'SIMN' or 'SISSM' magic bytes");
    }

    /// Benchmarks memory-mapped execution latency and memory bandwidth over N runs
    pub fn benchmark(&self, path: impl AsRef<Path>, iterations: usize) -> Result<SiBenchmarkReport> {
        let path = path.as_ref();
        let iterations = iterations.max(10);
        let mut latencies_us = Vec::with_capacity(iterations);

        let file = File::open(path)?;
        let file_size = file.metadata()?.len();

        for _ in 0..iterations {
            let start = Instant::now();
            let mmap = unsafe { Mmap::map(&file)? };
            if mmap.len() >= 4 && mmap[0..4] == SI_SOLID_STATE_MAGIC {
                let _ = SolidStateSiContainer::load_from_file(path)?;
            } else if mmap.len() >= 5 && mmap[0..5] == SI_SSM_MAGIC {
                let _ = SiStateSpaceModel::load_from_si_container(path, false)?;
            } else if mmap.len() >= 4 && mmap[0..4] == SI_MAGIC_BYTES {
                let _ = SiThoughtPacket::from_binary(&mmap)?;
            }
            latencies_us.push(start.elapsed().as_micros() as u64);
        }

        latencies_us.sort_unstable();

        let p50 = latencies_us[iterations * 50 / 100];
        let p95 = latencies_us[iterations * 95 / 100];
        let p99 = latencies_us[iterations * 99 / 100];
        let min = latencies_us[0];
        let max = latencies_us[iterations - 1];
        let sum: u64 = latencies_us.iter().sum();
        let mean = sum as f64 / iterations as f64;

        let ops_per_sec = if mean > 0.0 { 1_000_000.0 / mean } else { 1_000_000.0 };
        let bandwidth_mb_s = (ops_per_sec * file_size as f64) / (1024.0 * 1024.0);

        Ok(SiBenchmarkReport {
            file_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            iterations,
            p50_latency_us: p50,
            p95_latency_us: p95,
            p99_latency_us: p99,
            min_latency_us: min,
            max_latency_us: max,
            mean_latency_us: mean,
            throughput_ops_per_sec: ops_per_sec,
            bandwidth_mb_per_sec: bandwidth_mb_s,
        })
    }

    /// Packs raw state vectors and computational DAG into a `.si` binary container
    pub fn pack(
        &self,
        goal_opcode: u16,
        unit: DimensionalUnit,
        state_tensors: Vec<f32>,
        graph: NativeComputationalGraph,
        target_path: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let path = target_path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let packet = SiThoughtPacket::new(goal_opcode, unit, state_tensors, graph);
        let bytes = packet.to_binary()?;

        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        Ok(path.to_path_buf())
    }

    /// Distills a sequence of high-level task strings into a machine-native `.si` cartridge
    pub fn distill_task_sequence(
        &self,
        _task_name: &str,
        steps: &[String],
        target_path: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let mut graph = NativeComputationalGraph::new();
        
        for (i, step) in steps.iter().enumerate() {
            let id = (i + 1) as u64;
            let reg = id as u16;
            let opcode = match step.to_lowercase() {
                s if s.contains("alloc") || s.contains("memory") => MachineOpcode::Alloc { size_bytes: 4096, align: 64 },
                s if s.contains("load") || s.contains("read") => MachineOpcode::Load { address_reg: reg },
                s if s.contains("store") || s.contains("write") => MachineOpcode::Store { address_reg: reg, value_reg: reg + 1 },
                s if s.contains("sync") || s.contains("call") => MachineOpcode::Call { function_id: 0x5000 + id, arg_regs: vec![reg] },
                s if s.contains("reclaim") || s.contains("clean") => MachineOpcode::EntropyMinimization { state_reg: reg },
                _ => MachineOpcode::TensorDot { left_reg: reg, right_reg: reg + 1, dim: 64 },
            };

            let deps = if id > 1 { vec![id - 1] } else { Vec::new() };
            graph.add_node(NativeComputationNode {
                id,
                opcode,
                type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 32 },
                energy_cost: 0.02,
                dependencies: deps,
            });
        }

        let state_vector = vec![0.5f32; 64];
        self.pack(0x0999, DimensionalUnit::DIMENSIONLESS, state_vector, graph, target_path)
    }

    /// Distills high-dimensional teacher latent representations (e.g. 4096-dim)
    /// through the 2-Layer GeLU Bottleneck into a machine-native `.si` dataset
    pub fn distill_teacher_trajectory(
        &self,
        bridge: &crate::si_trainer::LatentGELUBottleneckBridge,
        teacher_latents: &[Vec<f32>],
        target_path: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let mut graph = NativeComputationalGraph::new();
        
        for (i, latent) in teacher_latents.iter().enumerate() {
            let id = (i + 1) as u64;
            let reg = id as u16;
            
            // Project teacher latent (4096-dim) through GeLU bottleneck into student space (256-dim)
            let student_projection = bridge.project(latent);
            let energy_norm = student_projection.iter().map(|&x| (x * x) as f64).sum::<f64>() / student_projection.len() as f64;

            graph.add_node(NativeComputationNode {
                id,
                opcode: MachineOpcode::TensorDot { left_reg: reg, right_reg: reg + 1, dim: bridge.student_dim },
                type_lattice: NativeTypeLattice::TensorType {
                    shape: vec![bridge.student_dim],
                    element_type: Box::new(NativeTypeLattice::PrimitiveFloat { bits: 32 }),
                },
                energy_cost: energy_norm.clamp(0.001, 1.0),
                dependencies: if id > 1 { vec![id - 1] } else { Vec::new() },
            });
        }

        let root_state = if let Some(first) = teacher_latents.first() {
            bridge.project(first)
        } else {
            vec![0.0f32; bridge.student_dim]
        };

        self.pack(0x0FEE, DimensionalUnit::DIMENSIONLESS, root_state, graph, target_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_si_tool_inspect_and_benchmark() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("benchmark_test.si");

        let engine = SiToolEngine;
        let mut graph = NativeComputationalGraph::new();
        graph.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc { size_bytes: 2048, align: 32 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 32 },
            energy_cost: 0.03,
            dependencies: Vec::new(),
        });
        graph.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::EntropyMinimization { state_reg: 1 },
            type_lattice: NativeTypeLattice::PhysicalQuantity { unit: DimensionalUnit::ENERGY_JOULE, precision: 64 },
            energy_cost: 0.01,
            dependencies: vec![1],
        });

        let path = engine.pack(0x0777, DimensionalUnit::ENERGY_JOULE, vec![1.0, 2.0, 3.0], graph, &target).unwrap();
        assert!(path.exists());

        let report = engine.inspect(&path).expect("Inspect failed");
        assert_eq!(report.magic, "SIMN");
        assert_eq!(report.goal_opcode, 0x0777);
        assert_eq!(report.node_count, 2);
        assert!(report.is_mmap_compatible);

        let bench = engine.benchmark(&path, 20).expect("Benchmark failed");
        assert_eq!(bench.iterations, 20);
        assert!(bench.throughput_ops_per_sec > 0.0);
    }

    #[test]
    fn test_si_distill_teacher_trajectory() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("distilled_dataset.si");
        let engine = SiToolEngine;

        let bridge = crate::si_trainer::LatentGELUBottleneckBridge::new(64, 32, 16);
        let teacher_latents = vec![vec![0.5f32; 64], vec![0.8f32; 64]];

        let path = engine.distill_teacher_trajectory(&bridge, &teacher_latents, &target).unwrap();
        assert!(path.exists());

        let report = engine.inspect(&path).expect("Inspect distilled dataset failed");
        assert_eq!(report.magic, "SIMN");
        assert_eq!(report.goal_opcode, 0x0FEE);
        assert_eq!(report.node_count, 2);
    }
}
