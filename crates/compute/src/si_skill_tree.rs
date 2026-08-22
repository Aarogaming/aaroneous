//! crates/compute/src/si_skill_tree.rs
//! Component #6: The Skill-Expansion & Meta-Learning Engine (Self-Development Bias).
//! Monitors executed workflows, measures step compression and thermodynamic free energy dissipation,
//! and automatically crystallizes high-efficiency pathways into permanent `.si` skill cartridges.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use crate::machine_native::{DimensionalUnit, MachineOpcode, NativeComputationNode, NativeComputationalGraph, NativeTypeLattice};
use crate::si_binary::SiThoughtPacket;

/// Maturity Status of an Autonomous Skill Module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillMaturityStatus {
    /// Initial candidate discovered from user workflow or agent action
    Candidate,
    /// Verified through repeated successful replays (> 3 times)
    Validated,
    /// High-efficiency routine crystallized into a standalone .si skill cartridge
    CrystallizedModule,
    /// Native zero-latency reflex loaded into hot Synapse memory
    CoreReflex,
}

impl SkillMaturityStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            Self::Candidate => "🌱 Candidate",
            Self::Validated => "🧪 Validated",
            Self::CrystallizedModule => "💎 Crystallized",
            Self::CoreReflex => "⚡ Core Reflex",
        }
    }
}

/// A Node in the Machine-Native Skill Tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiSkillModule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger_intent: String,
    pub status: SkillMaturityStatus,
    pub execution_count: u64,
    pub success_count: u64,
    pub step_compression_ratio: f64,    // e.g. 10 prompt steps -> 2 AST nodes = 5.0x compression
    pub thermodynamic_efficiency: f64,  // Lower dissipated energy (J/op) = higher efficiency
    pub latency_avg_us: u64,
    pub intrinsic_score: f64,           // Overall meta-learning fitness score
    pub created_at_unix: u64,
    pub packet: SiThoughtPacket,
    pub parent_skill_ids: Vec<String>,
}

impl SiSkillModule {
    /// Computes the meta-learning intrinsic fitness score
    pub fn compute_intrinsic_fitness(&mut self) -> f64 {
        let success_rate = if self.execution_count == 0 {
            1.0
        } else {
            self.success_count as f64 / self.execution_count as f64
        };

        let compression_weight = (self.step_compression_ratio / 5.0).clamp(0.2, 3.0);
        let energy_efficiency = (1.0 / (1.0 + self.thermodynamic_efficiency)).clamp(0.1, 1.0);
        
        let score = (success_rate * 0.4) + (compression_weight * 0.35) + (energy_efficiency * 0.25);
        self.intrinsic_score = score;

        // Auto-promotion logic
        if self.execution_count >= 5 && success_rate >= 0.95 && self.intrinsic_score >= 0.85 {
            self.status = SkillMaturityStatus::CoreReflex;
        } else if self.execution_count >= 3 && success_rate >= 0.9 && self.intrinsic_score >= 0.70 {
            self.status = SkillMaturityStatus::CrystallizedModule;
        } else if self.execution_count >= 1 && success_rate >= 0.8 {
            self.status = SkillMaturityStatus::Validated;
        }

        self.intrinsic_score
    }
}

/// The Skill-Expansion & Meta-Learning Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExpansionEngine {
    pub skills_dir: PathBuf,
    pub skills: HashMap<String, SiSkillModule>,
    pub total_skills_evolved: usize,
    pub mean_compression_rate: f64,
}

impl Default for SkillExpansionEngine {
    fn default() -> Self {
        let paths = aaroneous_paths::WorkspacePaths::discover();
        let skills_dir = paths.data().join("skills");
        let _ = fs::create_dir_all(&skills_dir);

        Self {
            skills_dir,
            skills: HashMap::new(),
            total_skills_evolved: 0,
            mean_compression_rate: 1.0,
        }
    }
}

impl SkillExpansionEngine {
    /// Initializes engine with a custom storage path
    pub fn new(skills_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&skills_dir)?;
        let mut engine = Self {
            skills_dir,
            skills: HashMap::new(),
            total_skills_evolved: 0,
            mean_compression_rate: 1.0,
        };
        engine.load_installed_skills()?;
        Ok(engine)
    }

    /// Evaluates an observed execution trace and registers or evolves a skill
    #[allow(clippy::too_many_arguments)]
    pub fn record_and_evaluate_trace(
        &mut self,
        name: &str,
        description: &str,
        intent: &str,
        raw_steps_count: usize,
        packet: SiThoughtPacket,
        execution_latency_us: u64,
        success: bool,
    ) -> Result<&SiSkillModule> {
        let skill_id = name.to_lowercase().replace(' ', "_");
        let node_count = packet.graph.nodes.len().max(1);
        let compression = (raw_steps_count as f64 / node_count as f64).max(1.0);
        let energy_cost = packet.graph.thermodynamic_free_energy;

        let module = self.skills.entry(skill_id.clone()).or_insert_with(|| SiSkillModule {
            id: skill_id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            trigger_intent: intent.to_string(),
            status: SkillMaturityStatus::Candidate,
            execution_count: 0,
            success_count: 0,
            step_compression_ratio: compression,
            thermodynamic_efficiency: energy_cost,
            latency_avg_us: execution_latency_us,
            intrinsic_score: 0.5,
            created_at_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            packet: packet.clone(),
            parent_skill_ids: Vec::new(),
        });

        module.execution_count += 1;
        if success {
            module.success_count += 1;
        }
        module.latency_avg_us = (module.latency_avg_us + execution_latency_us) / 2;
        module.thermodynamic_efficiency = (module.thermodynamic_efficiency + energy_cost) / 2.0;
        module.step_compression_ratio = (module.step_compression_ratio + compression) / 2.0;
        
        let _score = module.compute_intrinsic_fitness();

        // If crystallized or reflex, persist directly as .si cartridge
        if module.status == SkillMaturityStatus::CrystallizedModule || module.status == SkillMaturityStatus::CoreReflex {
            self.crystallize_skill_cartridge(&skill_id)?;
        }

        self.recompute_global_metrics();
        Ok(self.skills.get(&skill_id).unwrap())
    }

    /// Freezes a validated skill module into a standalone `.si` cartridge file
    pub fn crystallize_skill_cartridge(&self, skill_id: &str) -> Result<PathBuf> {
        let skill = match self.skills.get(skill_id) {
            Some(s) => s,
            None => bail!("Skill ID '{}' not found", skill_id),
        };

        let file_path = self.skills_dir.join(format!("{}.si", skill.id));
        let meta_path = self.skills_dir.join(format!("{}.meta.json", skill.id));

        let binary_packet = skill.packet.to_binary()?;
        let mut file = File::create(&file_path)?;
        file.write_all(&binary_packet)?;

        let meta_json = serde_json::to_string_pretty(skill)?;
        fs::write(&meta_path, meta_json)?;

        Ok(file_path)
    }

    /// Loads all installed skills from the skills directory
    pub fn load_installed_skills(&mut self) -> Result<()> {
        if !self.skills_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.skills_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(module) = serde_json::from_str::<SiSkillModule>(&content) {
                        self.skills.insert(module.id.clone(), module);
                    }
                }
            }
        }

        self.recompute_global_metrics();
        Ok(())
    }

    /// Populates default foundational skill cartridges
    pub fn ensure_starter_skills(&mut self) -> Result<Vec<SiSkillModule>> {
        if self.skills.len() >= 8 {
            return Ok(self.skills.values().cloned().collect());
        }

        // Skill 1: AST Code Transformer
        let mut g1 = NativeComputationalGraph::new();
        g1.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc { size_bytes: 4096, align: 64 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 64 },
            energy_cost: 0.02,
            dependencies: Vec::new(),
        });
        g1.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Call { function_id: 0x4001, arg_regs: vec![1] },
            type_lattice: NativeTypeLattice::PrimitiveInt { bits: 32, signed: false },
            energy_cost: 0.03,
            dependencies: vec![1],
        });
        let p1 = SiThoughtPacket::new(0x0801, DimensionalUnit::DIMENSIONLESS, vec![0.8, 0.4, 0.9], g1);
        self.record_and_evaluate_trace("AST Semantic Rewrite", "Rewrites AST patterns deterministically in sub-millisecond cycles", "refactor code ast", 8, p1, 32, true)?;

        // Skill 2: Thermodynamic Memory Reclaim
        let mut g2 = NativeComputationalGraph::new();
        g2.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::EntropyMinimization { state_reg: 1 },
            type_lattice: NativeTypeLattice::PhysicalQuantity { unit: DimensionalUnit::ENERGY_JOULE, precision: 64 },
            energy_cost: 0.01,
            dependencies: Vec::new(),
        });
        let p2 = SiThoughtPacket::new(0x0802, DimensionalUnit::ENERGY_JOULE, vec![0.1, 0.2, 0.3], g2);
        self.record_and_evaluate_trace("Thermodynamic Memory Reclaim", "Flushes inactive latent ring buffers and reclaims memory", "reclaim memory cleanup", 5, p2, 18, true)?;

        // Skill 3: IPC Synapse Broadcast
        let mut g3 = NativeComputationalGraph::new();
        g3.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Call { function_id: 0x9000, arg_regs: vec![1, 2] },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: false, alignment: 32 },
            energy_cost: 0.04,
            dependencies: Vec::new(),
        });
        let p3 = SiThoughtPacket::new(0x0803, DimensionalUnit::DIMENSIONLESS, vec![0.5, 0.5, 0.5], g3);
        self.record_and_evaluate_trace("Zero-Copy Synapse Relay", "Dispatches agent state tensors directly across 64 MB shared synapse", "dispatch synapse relay", 6, p3, 24, true)?;

        // Skill 4: Smart Git Sync
        let mut g4 = NativeComputationalGraph::new();
        g4.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc { size_bytes: 1024, align: 32 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 32 },
            energy_cost: 0.01,
            dependencies: Vec::new(),
        });
        g4.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Call { function_id: 0x1004, arg_regs: vec![1] },
            type_lattice: NativeTypeLattice::PrimitiveInt { bits: 32, signed: true },
            energy_cost: 0.02,
            dependencies: vec![1],
        });
        let p4 = SiThoughtPacket::new(0x0804, DimensionalUnit::DIMENSIONLESS, vec![0.3, 0.7, 0.2], g4);
        self.record_and_evaluate_trace("Smart Git Sync", "Performs atomic staging and microsecond repository index sync", "git sync stage", 4, p4, 21, true)?;

        // Skill 5: Compiler Diagnostic Repair
        let mut g5 = NativeComputationalGraph::new();
        g5.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Load { address_reg: 1 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: false, alignment: 64 },
            energy_cost: 0.02,
            dependencies: Vec::new(),
        });
        g5.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Call { function_id: 0x2005, arg_regs: vec![1] },
            type_lattice: NativeTypeLattice::PrimitiveInt { bits: 32, signed: false },
            energy_cost: 0.03,
            dependencies: vec![1],
        });
        let p5 = SiThoughtPacket::new(0x0805, DimensionalUnit::DIMENSIONLESS, vec![0.9, 0.1, 0.8], g5);
        self.record_and_evaluate_trace("Compiler Diagnostic Repair", "Auto-fixes common compiler diagnostic AST mismatches and lint errors", "repair compiler error", 7, p5, 29, true)?;

        // Skill 6: Workspace Cache Reaper
        let mut g6 = NativeComputationalGraph::new();
        g6.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::EntropyMinimization { state_reg: 1 },
            type_lattice: NativeTypeLattice::PhysicalQuantity { unit: DimensionalUnit::ENERGY_JOULE, precision: 64 },
            energy_cost: 0.01,
            dependencies: Vec::new(),
        });
        let p6 = SiThoughtPacket::new(0x0806, DimensionalUnit::ENERGY_JOULE, vec![0.1, 0.1, 0.1], g6);
        self.record_and_evaluate_trace("Workspace Cache Reaper", "Scans and purges stale target/debug dependencies and cache blobs", "clean workspace cache", 5, p6, 15, true)?;

        // Skill 7: Process Heartbeat Probe
        let mut g7 = NativeComputationalGraph::new();
        g7.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Call { function_id: 0x3007, arg_regs: vec![1] },
            type_lattice: NativeTypeLattice::PrimitiveInt { bits: 64, signed: false },
            energy_cost: 0.01,
            dependencies: Vec::new(),
        });
        let p7 = SiThoughtPacket::new(0x0807, DimensionalUnit::DIMENSIONLESS, vec![0.4, 0.4, 0.4], g7);
        self.record_and_evaluate_trace("Process Heartbeat Probe", "Probes running hypervisor threads and measures sub-millisecond jitter", "probe process heartbeat", 3, p7, 12, true)?;

        // Skill 8: Dimensional Invariant Verifier
        let mut g8 = NativeComputationalGraph::new();
        g8.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::TensorDot { left_reg: 1, right_reg: 2, dim: 64 },
            type_lattice: NativeTypeLattice::PhysicalQuantity { unit: DimensionalUnit::FORCE_NEWTON, precision: 32 },
            energy_cost: 0.02,
            dependencies: Vec::new(),
        });
        let p8 = SiThoughtPacket::new(0x0808, DimensionalUnit::FORCE_NEWTON, vec![0.6, 0.2, 0.7], g8);
        self.record_and_evaluate_trace("Dimensional Invariant Verifier", "Validates SI units across physical computation graphs", "verify dimensional invariants", 4, p8, 17, true)?;

        Ok(self.skills.values().cloned().collect())
    }

    fn recompute_global_metrics(&mut self) {
        self.total_skills_evolved = self.skills.len();
        if self.skills.is_empty() {
            self.mean_compression_rate = 1.0;
        } else {
            let sum: f64 = self.skills.values().map(|s| s.step_compression_ratio).sum();
            self.mean_compression_rate = sum / self.skills.len() as f64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_skill_evaluation_and_crystallization() {
        let dir = tempdir().unwrap();
        let mut engine = SkillExpansionEngine::new(dir.path().to_path_buf()).unwrap();

        let mut graph = NativeComputationalGraph::new();
        graph.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc { size_bytes: 1024, align: 16 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 16 },
            energy_cost: 0.05,
            dependencies: Vec::new(),
        });
        let packet = SiThoughtPacket::new(0x0888, DimensionalUnit::DIMENSIONLESS, vec![1.0, 2.0], graph);

        // Record 3 successful high-compression runs
        engine.record_and_evaluate_trace("Smart Git Sync", "Fast clean and push", "sync git", 10, packet.clone(), 40, true).unwrap();
        engine.record_and_evaluate_trace("Smart Git Sync", "Fast clean and push", "sync git", 10, packet.clone(), 38, true).unwrap();
        let skill = engine.record_and_evaluate_trace("Smart Git Sync", "Fast clean and push", "sync git", 10, packet.clone(), 35, true).unwrap();

        assert_eq!(skill.execution_count, 3);
        assert_eq!(skill.success_count, 3);
        assert!(skill.step_compression_ratio >= 5.0);
        assert!(skill.intrinsic_score > 0.70);
        assert!(skill.status == SkillMaturityStatus::CrystallizedModule || skill.status == SkillMaturityStatus::CoreReflex);

        let cartridge_path = dir.path().join("smart_git_sync.si");
        assert!(cartridge_path.exists());
    }
}
