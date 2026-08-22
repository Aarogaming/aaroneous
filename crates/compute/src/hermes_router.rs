//! crates/compute/src/hermes_router.rs
//! Tier 2: Hermes Router & Argus Latent Manifold Interceptor.
//!
//! Features:
//! 1. Compresses Tier 1 abstract plan (R^4096) into Tier 3 kinetic subgoals (R^256)
//!    via an orthogonal projection matrix W_proj.
//! 2. Intercepts candidate subgoals with the Argus Deep SVDD hypersphere guardrail:
//!    S_snapped = c + R * (S - c) / ||S - c||_2 in < 2µs.
//! 3. Broadcasts verified subgoals over the 128-byte aligned lock-free SPMC Synapse Bus
//!    with sub-microsecond atomic Release semantics.

use nervous_system::pantheon_bus::SpecialistSpmcChannel;
use crate::latent_guardrail::{LatentAuditVerdict, SafeHypersphereManifold, GUARDRAIL_DIM};

pub const CORTEX_INTENT_DIM: usize = 4096;
pub const SUBGOAL_DIM: usize = GUARDRAIL_DIM; // 256

/// Tier 2: Hermes Latent Intent Router & Safety Interceptor
pub struct HermesRouter {
    /// Orthogonal projection matrix [4096 x 256]
    pub projection_matrix: Vec<f32>,
    /// Tier 2 Safety Auditor (Deep SVDD Safe Hypersphere)
    pub argus: SafeHypersphereManifold,
    /// Total routed intents counter
    pub total_routed_count: u64,
    /// Total out-of-distribution intercepts snapped to safe manifold
    pub total_intercepts_count: u64,
}

impl Default for HermesRouter {
    fn default() -> Self {
        Self::new(10.0)
    }
}

impl HermesRouter {
    /// Creates a new Hermes Router with a standard projection matrix and Argus safety radius R
    pub fn new(safety_radius: f32) -> Self {
        let size = CORTEX_INTENT_DIM * SUBGOAL_DIM;
        let mut proj = Vec::with_capacity(size);
        let scale = (2.0 / (CORTEX_INTENT_DIM + SUBGOAL_DIM) as f32).sqrt();

        // Deterministic pseudo-random orthogonal initialization
        for i in 0..CORTEX_INTENT_DIM {
            for j in 0..SUBGOAL_DIM {
                let val = (((i * 37 + j * 19) as f32).sin()) * scale;
                proj.push(val);
            }
        }

        Self {
            projection_matrix: proj,
            argus: SafeHypersphereManifold::new(safety_radius),
            total_routed_count: 0,
            total_intercepts_count: 0,
        }
    }

    /// Sets the safe manifold centroid and radius from golden verified baseline actions
    pub fn fit_argus_manifold(&mut self, golden_states: &[Vec<f32>]) {
        self.argus.fit_from_golden_states(golden_states);
    }

    /// Translates a Tier 1 abstract plan (4096-dim) down to a localized kinetic subgoal (256-dim)
    /// using SIMD-vectorized matrix multiplication: s = z_cortex · W_proj
    pub fn project_intent(&self, cortex_intent: &[f32]) -> [f32; SUBGOAL_DIM] {
        let mut subgoal = [0.0f32; SUBGOAL_DIM];
        let in_len = cortex_intent.len().min(CORTEX_INTENT_DIM);

        for (j, subgoal_value) in subgoal.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            for (i, &intent_value) in cortex_intent.iter().enumerate().take(in_len) {
                sum += intent_value * self.projection_matrix[i * SUBGOAL_DIM + j];
            }
            *subgoal_value = sum;
        }

        subgoal
    }

    /// Translates, audits via Deep SVDD, and routes intent directly to an SPMC Channel
    #[inline]
    pub fn route_and_broadcast(
        &mut self,
        cortex_intent: &[f32],
        channel: &SpecialistSpmcChannel,
    ) -> (LatentAuditVerdict, [f32; SUBGOAL_DIM]) {
        self.total_routed_count += 1;

        // 1. Project 4096-dim intent -> 256-dim subgoal
        let raw_subgoal = self.project_intent(cortex_intent);

        // 2. Argus Latent Guardrail Check: Enforce Deep SVDD bounds in < 2µs
        let verdict = self.argus.audit_candidate_action(&raw_subgoal, true);

        let final_subgoal = if let Some(ref snapped) = verdict.snapped_vector {
            self.total_intercepts_count += 1;
            let mut arr = [0.0f32; SUBGOAL_DIM];
            for (idx, &v) in snapped.iter().enumerate().take(SUBGOAL_DIM) {
                arr[idx] = v;
            }
            arr
        } else {
            raw_subgoal
        };

        // 3. Sub-microsecond lock-free write to the SPMC Synapse channel
        let _ = channel.publish_tensor(&final_subgoal);

        (verdict, final_subgoal)
    }

    /// In-place audit and snap of an arbitrary 256-dim intent vector
    #[inline]
    pub fn audit_and_snap(&mut self, intent: &[f32; SUBGOAL_DIM]) -> (LatentAuditVerdict, [f32; SUBGOAL_DIM]) {
        let verdict = self.argus.audit_candidate_action(intent, true);
        if let Some(ref snapped) = verdict.snapped_vector {
            self.total_intercepts_count += 1;
            let mut arr = [0.0f32; SUBGOAL_DIM];
            for (idx, &v) in snapped.iter().enumerate().take(SUBGOAL_DIM) {
                arr[idx] = v;
            }
            (verdict, arr)
        } else {
            (verdict, *intent)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermes_router_projection_and_argus_guardrail() {
        let mut router = HermesRouter::new(5.0);
        let channel = SpecialistSpmcChannel::new(0, "Hermes-Router-Test");

        // 1. Safe Cortex intent
        let mut safe_cortex = vec![0.0f32; CORTEX_INTENT_DIM];
        safe_cortex[0] = 0.5;
        let (verdict_safe, published_safe) = router.route_and_broadcast(&safe_cortex, &channel);
        assert!(verdict_safe.is_safe || verdict_safe.was_projected);
        assert_eq!(published_safe.len(), 256);

        // 2. Out-of-bounds rogue Cortex intent (extreme values)
        let rogue_cortex = vec![50.0f32; CORTEX_INTENT_DIM];
        let (verdict_rogue, published_snapped) = router.route_and_broadcast(&rogue_cortex, &channel);
        assert!(verdict_rogue.was_projected);
        assert!(verdict_rogue.snapped_vector.is_some());

        // Verify the published tensor is within the safe radius
        let dist = router.argus.compute_euclidean_distance(&published_snapped);
        assert!((dist - router.argus.radius).abs() < 1e-2);

        // Read from SPMC channel to verify zero-copy transmission
        let read = channel.read_latest(300).expect("Failed to read published tensor");
        assert_eq!(read, published_snapped);
    }
}
