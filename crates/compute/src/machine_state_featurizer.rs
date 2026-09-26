//! crates/compute/src/machine_state_featurizer.rs
//! High-performance, zero-allocation machine state featurizer.
//! Projects system snapshots, Win32/OS telemetry, and hypervisor signals into
//! normalized continuous state vectors (R^256) for machine-native reflex models.

use ipc_bus::{EngineSnapshotPod, ObservationFramePod};

/// Contextual and environmental signals augmenting point-in-time engine snapshots
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeaturizerContext {
    /// Autonomic thermal factor [0.0..1.0] (1.0 = nominal, 0.0 = critical)
    pub thermal_factor: f32,
    /// System memory pressure [0.0..100.0]
    pub memory_pressure: f32,
    /// Concept drift divergence [0.0..1.0]
    pub concept_drift: f32,
    /// Curiosity / exploration drive [0.0..100.0]
    pub curiosity_drive: f32,
    /// Tick execution duration in microseconds
    pub tick_duration_us: u32,
    /// Reinforcement / reward signal
    pub reward: f32,
    /// Thermodynamic free energy or entropy delta
    pub free_energy: f32,
    /// Seed or hash of active latent vector / intent
    pub latent_seed: u64,
}

impl Default for FeaturizerContext {
    fn default() -> Self {
        Self {
            thermal_factor: 1.0,
            memory_pressure: 25.0,
            concept_drift: 0.0,
            curiosity_drive: 50.0,
            tick_duration_us: 250,
            reward: 0.0,
            free_energy: 0.01,
            latent_seed: 0,
        }
    }
}

/// Zero-allocation featurizer mapping runtime state into continuous `[f32; 256]` vectors.
#[derive(Debug, Clone, Copy, Default)]
pub struct MachineStateFeaturizer;

impl MachineStateFeaturizer {
    /// Creates a new machine state featurizer instance.
    pub const fn new() -> Self {
        Self
    }

    /// Featurizes an engine snapshot and runtime context directly into a pre-allocated `[f32; 256]` buffer.
    ///
    /// Memory geometry (256 dimensions):
    /// - [0..16]:   Core hypervisor performance & telemetry (FPS, integrity, understanding, pacing)
    /// - [16..48]:  Active specialist identity projection (32-dim hash embedding)
    /// - [48..80]:  Profile & operator context projection (32-dim hash embedding)
    /// - [80..144]: Event description character trigram hash projection (64-dim)
    /// - [144..208]: Latent intent hash projection (64-dim)
    /// - [208..240]: Sinusoidal positional temporal frequency bands (32-dim)
    /// - [240..256]: Thermodynamic stability & reward invariants (16-dim)
    #[inline]
    pub fn featurize(
        &self,
        snapshot: &EngineSnapshotPod,
        ctx: &FeaturizerContext,
        out: &mut [f32; 256],
    ) {
        // --- [0..16]: Core Hypervisor Performance & Telemetry ---
        out[0] = (snapshot.measured_fps / 120.0).clamp(0.0, 2.0);
        out[1] = (snapshot.bus_integrity / 100.0).clamp(0.0, 1.0);
        out[2] = (snapshot.bus_understanding / 100.0).clamp(0.0, 1.0);
        out[3] = snapshot.flow_score.clamp(0.0, 1.0);
        out[4] = (snapshot.user_level as f32 / 100.0).clamp(0.0, 10.0);
        out[5] = ((snapshot.user_xp % 10_000) as f32 / 10_000.0).clamp(0.0, 1.0);
        out[6] = (snapshot.active_companions_count as f32 / 16.0).clamp(0.0, 1.0);
        out[7] = (snapshot.running_macros_count as f32 / 32.0).clamp(0.0, 1.0);
        out[8] = if snapshot.pacing == 0 { 1.0 } else { 0.0 };
        out[9] = if snapshot.pacing == 1 { 1.0 } else { 0.0 };
        out[10] = if snapshot.pacing >= 2 { 1.0 } else { 0.0 };
        out[11] = ctx.thermal_factor.clamp(0.0, 1.0);
        out[12] = (ctx.memory_pressure / 100.0).clamp(0.0, 1.0);
        out[13] = ctx.concept_drift.clamp(0.0, 1.0);
        out[14] = (ctx.curiosity_drive / 100.0).clamp(0.0, 1.0);
        // Normalized execution tick budget (8333 µs = 120Hz nominal)
        out[15] = (ctx.tick_duration_us as f32 / 8333.0).clamp(0.0, 4.0);

        // --- [16..48]: Active Specialist Embedding (32 dimensions) ---
        project_fixed_bytes_into_slice(&snapshot.active_specialist, &mut out[16..48]);

        // --- [48..80]: Profile & Operator Context (32 dimensions) ---
        project_fixed_bytes_into_slice(&snapshot.active_profile_name, &mut out[48..80]);

        // --- [80..144]: Event Description Trigram Projection (64 dimensions) ---
        project_fixed_bytes_into_slice(&snapshot.last_event_desc, &mut out[80..144]);

        // --- [144..208]: Latent Intent Projection (64 dimensions) ---
        let mut seed = ctx.latent_seed.wrapping_add(snapshot.bus_generation);
        for i in 0..64 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let val = ((seed >> 33) as f32) / (u32::MAX as f32);
            out[144 + i] = (val * 2.0) - 1.0;
        }

        // --- [208..240]: Sinusoidal Temporal Positional Encodings (32 dimensions) ---
        let tick = snapshot.bus_generation as f32;
        for i in 0..16 {
            let freq = 1.0 / (10000.0f32.powf((2 * i) as f32 / 32.0));
            out[208 + (i * 2)] = (tick * freq).sin();
            out[208 + (i * 2) + 1] = (tick * freq).cos();
        }

        // --- [240..256]: Thermodynamic Stability & Reward Invariants (16 dimensions) ---
        out[240] = ctx.reward.clamp(-10.0, 10.0);
        out[241] = ctx.free_energy.clamp(0.0, 10.0);
        out[242] = (out[0] * out[3]).clamp(0.0, 2.0); // Flow-FPS product
        out[243] = (out[1] * out[2]).clamp(0.0, 1.0); // Integrity-Understanding coherence
        for i in 4..16 {
            out[240 + i] = 0.0;
        }
    }

    /// Featurizes state directly into a fully populated `ObservationFramePod`.
    pub fn featurize_frame(
        &self,
        snapshot: &EngineSnapshotPod,
        ctx: &FeaturizerContext,
        actual_opcode: u16,
        predicted_opcode: u16,
        confidence: f32,
    ) -> ObservationFramePod {
        let mut frame = ObservationFramePod {
            sequence: snapshot.bus_generation,
            timestamp_ns: snapshot.timestamp_ms.saturating_mul(1_000_000),
            tick_duration_us: ctx.tick_duration_us,
            actual_opcode,
            predicted_opcode,
            actor_tier: if actual_opcode == 0 { 0 } else { 3 },
            flags: if actual_opcode == predicted_opcode {
                0x03
            } else {
                0x01
            }, // bit 0: shadow, bit 1: match
            concurrence: if actual_opcode == predicted_opcode {
                1
            } else {
                0
            },
            _pad0: 0,
            bus_integrity: snapshot.bus_integrity,
            bus_understanding: snapshot.bus_understanding,
            flow_score: snapshot.flow_score,
            thermal_factor: ctx.thermal_factor,
            reward: ctx.reward,
            confidence,
            free_energy: ctx.free_energy,
            _reserved: [0; 2],
            state_features: [0.0; 256],
        };

        self.featurize(snapshot, ctx, &mut frame.state_features);
        frame
    }
}

/// Projects arbitrary bytes into a target slice using a deterministic polynomial rolling hash.
/// Zero heap allocations, pure arithmetic.
#[inline]
fn project_fixed_bytes_into_slice(input: &[u8], target: &mut [f32]) {
    target.fill(0.0);
    if input.is_empty() || target.is_empty() {
        return;
    }

    let target_len = target.len();
    let mut acc: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for (idx, &byte) in input.iter().enumerate() {
        if byte == 0 {
            break;
        }
        acc ^= byte as u64;
        acc = acc.wrapping_mul(0x100000001b3); // FNV prime

        let slot = (acc as usize + idx) % target_len;
        let norm_val = ((acc >> 32) as f32 / (u32::MAX as f32)) * 2.0 - 1.0;
        target[slot] = (target[slot] * 0.5) + (norm_val * 0.5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_featurizer_determinism_and_bounds() {
        let featurizer = MachineStateFeaturizer::new();
        let mut snapshot = EngineSnapshotPod {
            measured_fps: 119.8,
            bus_integrity: 99.4,
            bus_understanding: 98.6,
            flow_score: 0.88,
            ..Default::default()
        };
        snapshot.set_active_specialist("Fabricator");
        snapshot.set_last_event_desc("Compiled WASM linear memory block");

        let ctx = FeaturizerContext {
            thermal_factor: 0.95,
            memory_pressure: 42.0,
            tick_duration_us: 320,
            reward: 1.25,
            ..Default::default()
        };

        let mut features1 = [0.0f32; 256];
        let mut features2 = [0.0f32; 256];

        featurizer.featurize(&snapshot, &ctx, &mut features1);
        featurizer.featurize(&snapshot, &ctx, &mut features2);

        // Strict determinism
        assert_eq!(features1, features2);

        // All values must be finite numbers (no NaNs or Infs)
        for (i, &val) in features1.iter().enumerate() {
            assert!(val.is_finite(), "Dimension {} is non-finite: {}", i, val);
        }

        // Verify key telemetry dimensions
        assert!((features1[0] - (119.8 / 120.0)).abs() < 1e-4);
        assert!((features1[1] - 0.994).abs() < 1e-4);
        assert!((features1[2] - 0.986).abs() < 1e-4);
        assert!((features1[3] - 0.88).abs() < 1e-4);
        assert_eq!(features1[8], 1.0); // nominal pacing
        assert!((features1[11] - 0.95).abs() < 1e-4);
        assert!((features1[12] - 0.42).abs() < 1e-4);

        // Frame generation
        let frame = featurizer.featurize_frame(&snapshot, &ctx, 0x0400, 0x0400, 0.96);
        assert_eq!(frame.actual_opcode, 0x0400);
        assert_eq!(frame.predicted_opcode, 0x0400);
        assert_eq!(frame.concurrence, 1);
        assert_eq!(frame.confidence, 0.96);
        assert_eq!(frame.state_features, features1);
    }
}
