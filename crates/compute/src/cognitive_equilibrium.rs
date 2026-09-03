// crates/compute/src/cognitive_equilibrium.rs
//! Generalized Cognitive Equilibrium, Attention Spectrum & Somatic Proprioception.
//!
//! Eliminates "one-track mind" failure modes by maintaining continuous balanced awareness:
//! 1. `AttentionSpectrum`: Allocates cognitive resources across Focal, Ambient, and Reflexive domains.
//! 2. `SomaticVitals`: Self-monitoring of memory pressure, cycle jitter, and execution thermodynamics.
//! 3. `TriModalReasoningGate`: Evaluates actions simultaneously across Linguistic, Spatial, and Physical bounds.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Balanced allocation of cognitive attention bandwidth (sums to 1.0)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AttentionSpectrum {
    /// Foreground primary task execution (e.g. active user command)
    pub focal_ratio: f32,
    /// Background environmental awareness & system health monitoring
    pub ambient_ratio: f32,
    /// Low-latency safety interlock & emergency interrupt sentry
    pub reflexive_ratio: f32,
}

impl Default for AttentionSpectrum {
    fn default() -> Self {
        Self {
            focal_ratio: 0.60,
            ambient_ratio: 0.30,
            reflexive_ratio: 0.10,
        }
    }
}

impl AttentionSpectrum {
    /// Creates a validated attention spectrum
    pub fn new(focal: f32, ambient: f32, reflexive: f32) -> Result<Self> {
        let total = focal + ambient + reflexive;
        if (total - 1.0).abs() > 0.01 {
            bail!("Attention spectrum ratios must sum to 1.0 (got {total:.3})");
        }
        if focal < 0.0 || ambient < 0.0 || reflexive < 0.0 {
            bail!("Attention ratios cannot be negative");
        }
        Ok(Self {
            focal_ratio: focal,
            ambient_ratio: ambient,
            reflexive_ratio: reflexive,
        })
    }
}

/// Somatic Hardware Health & Execution Vitals (Body Awareness)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SomaticVitals {
    pub memory_heap_mb: usize,
    pub cycle_jitter_us: u32,
    pub thermodynamic_free_energy: f32,
    pub is_thermally_throttled: bool,
}

impl Default for SomaticVitals {
    fn default() -> Self {
        Self {
            memory_heap_mb: 64,
            cycle_jitter_us: 5,
            thermodynamic_free_energy: 0.015,
            is_thermally_throttled: false,
        }
    }
}

/// Multi-Modal Evaluation Across Linguistic, Spatial, and Physical Axes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriModalDecisionReport {
    pub linguistic_intent_score: f32, // Intent alignment (0.0 to 1.0)
    pub spatial_feasibility_score: f32, // Geometric/physical clearance (0.0 to 1.0)
    pub thermodynamic_cost_score: f32,  // Energy/cycle footprint (0.0 to 1.0, 1.0 = optimal)
    pub composite_confidence: f32,
    pub action_approved: bool,
}

/// The Cognitive Equilibrium Coordinator
pub struct CognitiveEquilibriumCoordinator {
    pub spectrum: AttentionSpectrum,
    pub vitals: SomaticVitals,
    total_evaluations: u64,
}

impl Default for CognitiveEquilibriumCoordinator {
    fn default() -> Self {
        Self::new(AttentionSpectrum::default(), SomaticVitals::default())
    }
}

impl CognitiveEquilibriumCoordinator {
    pub fn new(spectrum: AttentionSpectrum, vitals: SomaticVitals) -> Self {
        Self {
            spectrum,
            vitals,
            total_evaluations: 0,
        }
    }

    /// Dynamically shifts attention based on somatic pressure
    pub fn recalibrate_attention_from_vitals(&mut self) {
        if self.vitals.thermodynamic_free_energy > 0.04 || self.vitals.is_thermally_throttled {
            // Under high stress/heat: shift focus into reflexive safety & ambient throttling
            self.spectrum.focal_ratio = 0.30;
            self.spectrum.ambient_ratio = 0.40;
            self.spectrum.reflexive_ratio = 0.30;
        } else {
            // Under nominal conditions: restore standard balanced spectrum
            self.spectrum = AttentionSpectrum::default();
        }
    }

    /// Evaluates a proposed action across all three cognitive axes
    pub fn evaluate_trimodal_action(
        &mut self,
        linguistic_intent: f32,
        spatial_feasibility: f32,
        energy_cost: f32,
    ) -> TriModalDecisionReport {
        self.total_evaluations += 1;

        // Thermodynamic score is inverse of cost: higher cost = lower score
        let thermodynamic_cost_score = (1.0 - energy_cost).clamp(0.0, 1.0);

        // Weighted composite confidence influenced by current attention spectrum
        let composite_confidence = (linguistic_intent * self.spectrum.focal_ratio)
            + (spatial_feasibility * self.spectrum.ambient_ratio)
            + (thermodynamic_cost_score * (1.0 - self.spectrum.reflexive_ratio));

        // Action is approved only if it passes all individual dimensional baselines
        let action_approved = linguistic_intent >= 0.5
            && spatial_feasibility >= 0.5
            && thermodynamic_cost_score >= 0.5
            && composite_confidence >= 0.6;

        TriModalDecisionReport {
            linguistic_intent_score: linguistic_intent,
            spatial_feasibility_score: spatial_feasibility,
            thermodynamic_cost_score,
            composite_confidence,
            action_approved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attention_spectrum_validation() {
        let valid = AttentionSpectrum::new(0.5, 0.3, 0.2);
        assert!(valid.is_ok());

        let invalid = AttentionSpectrum::new(0.5, 0.5, 0.5);
        assert!(invalid.is_err());
    }

    #[test]
    fn test_trimodal_decision_approval() {
        let mut coordinator = CognitiveEquilibriumCoordinator::default();

        // Balanced high-confidence proposal
        let approved = coordinator.evaluate_trimodal_action(0.85, 0.90, 0.10);
        assert!(approved.action_approved);
        assert!(approved.composite_confidence > 0.7);

        // Action with high language intent but hazardous spatial feasibility
        let rejected = coordinator.evaluate_trimodal_action(0.95, 0.20, 0.10);
        assert!(!rejected.action_approved);
    }

    #[test]
    fn test_somatic_vitals_recalibration() {
        let mut coordinator = CognitiveEquilibriumCoordinator::default();
        assert_eq!(coordinator.spectrum.focal_ratio, 0.60);

        // Simulate thermal pressure
        coordinator.vitals.is_thermally_throttled = true;
        coordinator.recalibrate_attention_from_vitals();

        assert_eq!(coordinator.spectrum.focal_ratio, 0.30);
        assert_eq!(coordinator.spectrum.reflexive_ratio, 0.30);
    }
}
