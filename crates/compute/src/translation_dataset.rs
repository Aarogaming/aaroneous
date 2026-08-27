//! crates/compute/src/translation_dataset.rs
//! Translation Dataset Synthetic Engine.
//! Translates standard software and OS operations into 4096-dimensional teacher reasoning states
//! paired with 256-dimensional target state deltas and discrete machine opcodes for offline distillation.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

pub const ROSETTA_TEACHER_DIM: usize = 4096;
pub const ROSETTA_LATENT_DIM: usize = 256;
pub const ROSETTA_MAGIC: [u8; 4] = [b'R', b'O', b'S', b'T'];

/// A single step in a software or OS micro-task trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosettaTrajectoryStep {
    pub task_id: u64,
    pub description: String,
    pub teacher_hidden_state: Vec<f32>, // 4096-dim Oracle reasoning vector
    pub expected_opcode: u16,           // Discrete MachineOpcode (e.g. 0x01: Alloc, 0x04: TensorDot)
    pub target_state_delta: Vec<f32>,   // 256-dim next-state delta (ΔS = S_{t+1} - S_t)
}

/// The Translation Dataset containing thousands of trajectory steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationDataset {
    pub name: String,
    pub sample_count: usize,
    pub teacher_dim: usize,
    pub latent_dim: usize,
    pub steps: Vec<RosettaTrajectoryStep>,
}

impl TranslationDataset {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            sample_count: 0,
            teacher_dim: ROSETTA_TEACHER_DIM,
            latent_dim: ROSETTA_LATENT_DIM,
            steps: Vec::new(),
        }
    }

    /// Synthesizes N realistic micro-task trajectory steps across AST and UI domains
    pub fn synthesize_synthetic_corpus(sample_count: usize) -> Self {
        Self::synthesize_specialist_corpus("router", 0x0700, sample_count)
    }

    /// Synthesizes tailored micro-task trajectories for a specific Sovereign Specialist Domain
    pub fn synthesize_specialist_corpus(specialist_name: &str, domain_opcode: u16, sample_count: usize) -> Self {
        let mut dataset = Self::new(&format!("Translation-{}-0x{:04X}", specialist_name, domain_opcode));
        dataset.sample_count = sample_count;

        let templates: &[(&str, u16, f32)] = match specialist_name.to_lowercase().as_str() {
            "orchestrator" => &[
                ("Orchestrator: Schedule distributed specialist pipeline execution", 0x01, 0.05),
                ("Orchestrator: Evaluate Byzantine quorum consensus vote threshold", 0x02, 0.04),
                ("Orchestrator: Allocate metabolic energy tokens across federation", 0x03, 0.03),
            ],
            "synthesizer" => &[
                ("Synthesizer: Query 3D Omni Galaxy knowledge subgraph", 0x02, 0.05),
                ("Synthesizer: Link cross-domain ontology concepts in AST", 0x04, 0.06),
                ("Synthesizer: Ingest scientific research paper into semantic index", 0x01, 0.04),
            ],
            "presenter" => &[
                ("Presenter: Render 60Hz 3D Star-Graph constellation viewport", 0x01, 0.06),
                ("Presenter: Project latent state activations onto 256-bar oscilloscope", 0x04, 0.05),
                ("Presenter: Compose reactive HUD dashboard widget", 0x03, 0.04),
            ],
            "fabricator" => &[
                ("Fabricator: Generate SIMD-quantized Q4_K_M forward kernel", 0x04, 0.08),
                ("Fabricator: Compile and link native WASM bytecode module", 0x01, 0.05),
                ("Fabricator: Optimize AST computational DAG node ordering", 0x03, 0.04),
            ],
            "sentinel" => &[
                ("Sentinel: Audit candidate action state tensor against SVDD safe manifold", 0x05, 0.07),
                ("Sentinel: Orthogonally project rogue latent vector onto safe boundary", 0x06, 0.09),
                ("Sentinel: Verify memory-mapped container zero-copy bounds check", 0x02, 0.03),
            ],
            "archivist" => &[
                ("Archivist: Trigger Compaction Engine zero-copy memory compaction on NVMe", 0x01, 0.05),
                ("Archivist: Step 4-channel neurochemical homeostatic decay", 0x04, 0.04),
                ("Archivist: Calculate proactive curiosity drive impulse", 0x03, 0.06),
            ],
            "router" => &[
                ("Router: Broadcast zero-copy tensor packet across SPMC synapse", 0x01, 0.04),
                ("Router: Route multi-node gossip proposal to P2P peer", 0x02, 0.05),
                ("Router: Synchronize state across federated hive nodes", 0x03, 0.03),
            ],
            "aligner" => &[
                ("Aligner: Synchronize relativistic chrono-scheduler clocks", 0x02, 0.03),
                ("Aligner: Align temporal resonance frequency across specialist loops", 0x04, 0.04),
                ("Aligner: Predict time-to-completion for autonomous chimera cycle", 0x03, 0.05),
            ],
            "perceiver" => &[
                ("Perceiver: Evaluate 16x16 epigenetic visual motion gating delta", 0x01, 0.07),
                ("Perceiver: Skip dormant screen sectors to achieve >90% compute savings", 0x06, 0.06),
                ("Perceiver: Project raw visual luminance into R^256 spatial latent intent", 0x04, 0.08),
            ],
            _ => &[
                ("General: Execute computational instruction step", 0x01, 0.05),
                ("General: Evaluate latent state transition", 0x04, 0.04),
            ],
        };

        for i in 0..sample_count {
            let (desc, opcode, scale) = templates[i % templates.len()];

            // Generate structured 4096-dim teacher hidden state with deterministic pseudo-random harmonics
            let mut teacher_state = vec![0.0f32; ROSETTA_TEACHER_DIM];
            for (j, value) in teacher_state.iter_mut().enumerate() {
                let freq = ((i * 13 + j * 17 + (domain_opcode as usize) * 31) as f32).sin();
                *value = freq * 0.5 + ((j % 64) as f32 * 0.001);
            }

            // Normalize teacher state onto hypersphere
            let norm: f32 = teacher_state.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
            for x in teacher_state.iter_mut() {
                *x /= norm;
            }

            // Generate structured 256-dim target state delta
            let mut delta = vec![0.0f32; ROSETTA_LATENT_DIM];
            for (j, value) in delta.iter_mut().enumerate() {
                *value = (((i * 7 + j * 11 + (domain_opcode as usize) * 19) as f32).cos()) * scale;
            }

            dataset.steps.push(RosettaTrajectoryStep {
                task_id: (i + 1) as u64,
                description: desc.to_string(),
                teacher_hidden_state: teacher_state,
                expected_opcode: opcode,
                target_state_delta: delta,
            });
        }

        dataset
    }

    /// Synthesizes distinct datasets for all 9 Sovereign Domain Specialists
    pub fn synthesize_all_9_specialists(sample_count_per_domain: usize) -> Vec<TranslationDataset> {
        let specs = [
            ("orchestrator", 0x0100),
            ("synthesizer", 0x0200),
            ("presenter", 0x0300),
            ("fabricator", 0x0400),
            ("sentinel", 0x0500),
            ("archivist", 0x0600),
            ("router", 0x0700),
            ("aligner", 0x0800),
            ("perceiver", 0x0900),
        ];

        specs.iter()
            .map(|(name, opcode)| Self::synthesize_specialist_corpus(name, *opcode, sample_count_per_domain))
            .collect()
    }

    /// Synthesizes a unified multi-thousand sample training corpus spanning all 9 Sovereign Domain Specialists
    pub fn synthesize_full_federation_corpus(sample_count_per_domain: usize) -> Self {
        let mut unified = Self::new("Translation-SpecialistFederation-FullCorpus");
        let all_datasets = Self::synthesize_all_9_specialists(sample_count_per_domain);
        for ds in all_datasets {
            unified.steps.extend(ds.steps);
        }
        unified.sample_count = unified.steps.len();
        unified
    }

    /// Saves the translation dataset to a binary file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }

        let encoded = serde_json::to_vec(self)?;
        let mut file = File::create(path)?;
        file.write_all(&ROSETTA_MAGIC)?;
        file.write_all(&(encoded.len() as u64).to_le_bytes())?;
        file.write_all(&encoded)?;
        Ok(())
    }

    /// Loads the Translation Dataset from a binary file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if magic != ROSETTA_MAGIC {
            anyhow::bail!("Invalid translation dataset magic header");
        }

        let mut len_bytes = [0u8; 8];
        file.read_exact(&mut len_bytes)?;
        let payload_len = u64::from_le_bytes(len_bytes) as usize;

        let mut payload = vec![0u8; payload_len];
        file.read_exact(&mut payload)?;

        let dataset: Self = serde_json::from_slice(&payload)?;
        Ok(dataset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_dataset_synthesis_and_roundtrip() {
        let dataset = TranslationDataset::synthesize_synthetic_corpus(10);
        assert_eq!(dataset.steps.len(), 10);
        assert_eq!(dataset.steps[0].teacher_hidden_state.len(), ROSETTA_TEACHER_DIM);
        assert_eq!(dataset.steps[0].target_state_delta.len(), ROSETTA_LATENT_DIM);

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_rosetta_stone.bin");
        dataset.save_to_file(&path).unwrap();

        let loaded = TranslationDataset::load_from_file(&path).unwrap();
        assert_eq!(loaded.sample_count, 10);
        assert_eq!(loaded.steps[0].description, dataset.steps[0].description);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_translation_dataset_full_federation_corpus() {
        let unified = TranslationDataset::synthesize_full_federation_corpus(20);
        assert_eq!(unified.sample_count, 180); // 9 specialists * 20 samples
        assert_eq!(unified.steps.len(), 180);

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_rosetta_federation_corpus.rost");
        unified.save_to_file(&path).unwrap();

        let loaded = TranslationDataset::load_from_file(&path).unwrap();
        assert_eq!(loaded.sample_count, 180);
        assert_eq!(loaded.steps[179].expected_opcode, unified.steps[179].expected_opcode);
        let _ = fs::remove_file(path);
    }
}
