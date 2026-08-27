//! crates/compute/src/si_distillation_harness.rs
//! Offline Bootstrap & Distillation Harness for Zero-to-One .si Base Models.
//! Distills 4096-dimensional teacher reasoning states from the Translation Dataset
//! into frozen base Selective SSM weights using CKA + InfoNCE loss, and packages the result into
//! a sovereign, bootable .si model container.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::translation_dataset::{TranslationDataset, ROSETTA_LATENT_DIM, ROSETTA_TEACHER_DIM};
use crate::si_solid_state::SolidStateSiContainer;
use crate::si_ssm::SiSsmConfig;
use crate::si_trainer::LatentGELUBottleneckBridge;

use crate::si_trainer::{run_bootstrapper, BootstrapperConfig};

/// Bootstrap Training Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub model_name: String,
    pub epochs: usize,
    pub batch_size: usize,
    pub learning_rate: f32,
    pub teacher_dim: usize,
    pub latent_dim: usize,
    pub target_cka_threshold: f64,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            model_name: "base_router_v1".to_string(),
            epochs: 5,
            batch_size: 16,
            learning_rate: 0.001,
            teacher_dim: ROSETTA_TEACHER_DIM,
            latent_dim: ROSETTA_LATENT_DIM,
            target_cka_threshold: 0.85,
        }
    }
}

/// Training Report from Bootstrap Distillation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapReport {
    pub model_name: String,
    pub samples_processed: usize,
    pub final_cka_alignment: f64,
    pub final_infonce_loss: f64,
    pub final_mse_delta_loss: f64,
    pub total_duration_ms: u64,
    pub output_si_path: PathBuf,
}

/// Offline Bootstrap Distillation Harness
pub struct SiDistillationHarness {
    pub config: BootstrapConfig,
    /// Pre-distillation bridge kept for direct single-sample projection calls.
    /// The full bootstrap path uses `run_bootstrapper` which trains its own bridge.
    #[allow(dead_code)]
    pub bridge: LatentGELUBottleneckBridge,
}

impl SiDistillationHarness {
    pub fn new(config: BootstrapConfig) -> Self {
        let bottleneck_dim = 1024;
        let bridge = LatentGELUBottleneckBridge::new(config.teacher_dim, bottleneck_dim, config.latent_dim);
        Self { config, bridge }
    }

    /// Distills a Translation Dataset into a bootable .si base model container
    pub fn bootstrap_base_model<P: AsRef<Path>>(
        &mut self,
        dataset: &TranslationDataset,
        out_path: P,
    ) -> Result<BootstrapReport> {
        let start = Instant::now();

        // 1. Initialize Solid-State SI Container with Selective SSM configuration
        let ssm_config = SiSsmConfig {
            model_name: self.config.model_name.clone(),
            state_dim: self.config.latent_dim,
            d_model: 32,
            d_state: 16,
            d_conv: 4,
            dt_rank: 8,
            num_layers: 2,
            num_opcodes: 16,
            param_count: 50_000,
        };

        let mut container = SolidStateSiContainer::new(&self.config.model_name, ssm_config);

        // 2. Run the full Bootstrapper training loop (bridge + classifier head)
        //    Epochs, batch size, and LR are inherited from BootstrapConfig.
        let bs_config = BootstrapperConfig {
            num_opcodes: 16,
            learning_rate: self.config.learning_rate,
            epochs: self.config.epochs,
            batch_size: self.config.batch_size,
            cka_weight: 0.5,
            infonce_weight: 0.1,
            temperature: 0.07,
        };

        let (trained_model, epoch_reports) = run_bootstrapper(dataset, bs_config);

        // Use the trained bridge for final projections and anchor seeding
        let final_bridge = &trained_model.bridge;

        // 3. Project all dataset steps through the *trained* bridge and seed anchors
        for step in &dataset.steps {
            let student_latent = final_bridge.project(&step.teacher_hidden_state);
            container.adaptation.add_anchor_state(
                student_latent,
                step.expected_opcode,
                step.target_state_delta.clone(),
            );
        }

        // 4. Extract real metrics from the final epoch
        let last = epoch_reports.last();
        let final_cka     = last.map(|r| (1.0 - r.cka_loss as f64).clamp(0.0, 1.0)).unwrap_or(0.0);
        let final_infonce = last.map(|r| r.infonce_loss as f64).unwrap_or(0.0);
        let final_mse     = last.map(|r| r.ce_loss as f64).unwrap_or(0.0);

        // 5. Save bootable .si container to disk with 64-byte alignment and magic headers
        let target_path = out_path.as_ref().to_path_buf();
        container.save_to_file(&target_path)?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(BootstrapReport {
            model_name: self.config.model_name.clone(),
            samples_processed: dataset.steps.len(),
            final_cka_alignment: final_cka,
            final_infonce_loss: final_infonce,
            final_mse_delta_loss: final_mse,
            total_duration_ms: duration_ms,
            output_si_path: target_path,
        })
    }

    /// Distills all 9 Sovereign Domain Specialists and compiles their zero-copy .si model containers
    pub fn distill_all_9_specialists<P: AsRef<Path>>(
        out_dir: P,
        samples_per_specialist: usize,
        epochs: usize,
    ) -> Result<Vec<BootstrapReport>> {
        use std::fs;
        let out_dir_path = out_dir.as_ref();
        fs::create_dir_all(out_dir_path)?;

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

        let mut reports = Vec::new();
        for (name, opcode) in specs {
            let dataset = TranslationDataset::synthesize_specialist_corpus(name, opcode, samples_per_specialist);
            let config = BootstrapConfig {
                model_name: format!("{}_sovereign_v1", name),
                epochs,
                batch_size: 16,
                learning_rate: 0.001,
                teacher_dim: ROSETTA_TEACHER_DIM,
                latent_dim: ROSETTA_LATENT_DIM,
                target_cka_threshold: 0.80,
            };

            let mut harness = SiDistillationHarness::new(config);
            let target_file = out_dir_path.join(format!("{}.si", name));
            let report = harness.bootstrap_base_model(&dataset, &target_file)?;
            reports.push(report);
        }

        Ok(reports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_bootstrap_base_model_from_rosetta_dataset() {
        let dataset = TranslationDataset::synthesize_synthetic_corpus(8);
        let config = BootstrapConfig::default();
        let mut harness = SiDistillationHarness::new(config);

        let temp_dir = std::env::temp_dir();
        let out_path = temp_dir.join("test_base_router.si");

        let report = harness.bootstrap_base_model(&dataset, &out_path).unwrap();
        assert_eq!(report.samples_processed, 8);
        assert!(report.final_cka_alignment >= 0.80);
        assert!(out_path.exists());

        // Verify loaded container
        let loaded = SolidStateSiContainer::load_from_file(&out_path).unwrap();
        assert_eq!(loaded.adaptation.anchor_buffer.len(), 8);
        assert!(loaded.adaptation.verify_anchor_retention() >= 95.0);

        let _ = fs::remove_file(out_path);
    }

    #[test]
    fn test_distill_all_9_specialists() {
        let temp_dir = std::env::temp_dir().join("test_distill_9");
        let reports = SiDistillationHarness::distill_all_9_specialists(&temp_dir, 4, 1).unwrap();
        assert_eq!(reports.len(), 9);

        for report in &reports {
            assert!(report.output_si_path.exists());
            assert_eq!(report.samples_processed, 4);
        }

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
