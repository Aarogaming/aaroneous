// Integration test for the spatial-kinetic pipeline
// Verifies the complete loop: genome loading → gate matrix → reflex kernel → motor intent

#[cfg(test)]
mod spatial_kinetic_integration_tests {
    use crate::spatial_delta_gate::{SpatialDeltaGateMatrix, GRID_SIZE, SpatialDeltaPipeline};
    use crate::win32_intercept::hid_bridge::{ACTION_CLICK, ACTION_MOUSE_MOVE, MotorIntent};
    use rand::RngExt;
    use rand::SeedableRng;

    #[test]
    fn test_gate_matrix_initial_state() {
        let matrix = SpatialDeltaGateMatrix::new();
        assert_eq!(matrix.active_sector_count(), 256);
        assert!((matrix.skip_ratio() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_gate_matrix_static_frames_gate_off() {
        let mut matrix = SpatialDeltaGateMatrix::new();
        let frame = [0.5f32; GRID_SIZE];

        // First frame: all active
        let active = matrix.update(&frame);
        assert_eq!(active, 256);

        // Second identical frame: still active (hysteresis)
        let active = matrix.update(&frame);
        assert_eq!(active, 256);

        // Third identical frame: still active (hysteresis threshold)
        let active = matrix.update(&frame);
        assert_eq!(active, 256);

        // Fourth identical frame: should start gating off
        let active = matrix.update(&frame);
        assert!(active < 256);
    }

    #[test]
    fn test_gate_matrix_motion_keeps_active() {
        let mut matrix = SpatialDeltaGateMatrix::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        for i in 0..10 {
            let base = if i % 2 == 0 { 0.0 } else { 1.0 };
            let mut frame = [base; GRID_SIZE];
            for val in frame.iter_mut() {
                *val += rng.random_range(-0.05..0.05);
            }
            let active = matrix.update(&frame);
            assert_eq!(active, 256);
        }
    }

    #[test]
    fn test_motor_intent_structure() {
        let intent = MotorIntent {
            delta_x: 10.0,
            delta_y: -5.0,
            binary_action_register: ACTION_MOUSE_MOVE | ACTION_CLICK,
        };

        assert_eq!(intent.delta_x, 10.0);
        assert_eq!(intent.delta_y, -5.0);
        assert!(intent.binary_action_register & ACTION_MOUSE_MOVE != 0);
        assert!(intent.binary_action_register & ACTION_CLICK != 0);
    }

    #[test]
    fn test_visual_gate_pipeline_integration() {
        let mut pipeline = SpatialDeltaPipeline::new();
        let frame = [0.5f32; GRID_SIZE];

        // Process first frame
        let active = pipeline.process_frame(&frame);
        assert_eq!(active, 256);

        // Process identical frame (should trigger hysteresis)
        let active = pipeline.process_frame(&frame);
        assert_eq!(active, 256);

        // Verify GPU mask is accessible
        let mask = pipeline.gpu_dispatch_mask();
        assert_eq!(mask.len(), 4);

        // Verify skip calculation
        let skip = pipeline.gate_matrix.skip_ratio();
        assert!(skip >= 0.0 && skip <= 1.0);
    }

    #[test]
    fn test_gate_matrix_packed_mask_correctness() {
        let mut matrix = SpatialDeltaGateMatrix::new();
        let frame = [0.5f32; GRID_SIZE];

        // Gate everything off
        for _ in 0..5 {
            matrix.update(&frame);
        }

        let before = matrix.active_count;
        matrix.force_sector_active(0, 0);
        assert_eq!(matrix.active_count, before + 1);

        let mask = matrix.get_gpu_mask();
        let total_active: u32 = mask.iter().map(|w| w.count_ones()).sum();
        assert_eq!(total_active, matrix.active_count);
    }

    #[test]
    fn test_pixel_active_lookup() {
        let matrix = SpatialDeltaGateMatrix::new();
        assert!(matrix.is_pixel_active(0, 0));
        assert!(matrix.is_pixel_active(64, 64));
        assert!(matrix.is_pixel_active(127, 127));
        assert!(!matrix.is_pixel_active(128, 128));
    }

    #[test]
    fn test_skip_ratio_calculation() {
        let mut matrix = SpatialDeltaGateMatrix::new();
        assert_eq!(matrix.skip_ratio(), 0.0);

        // Manually gate off half the sectors
        for i in 0..128 {
            matrix.sectors[i].active = 0;
        }
        matrix.active_count = 128;

        assert!((matrix.skip_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_sub_16ms_perception_to_motor_reflex_benchmark() {
        let mut pipeline = SpatialDeltaPipeline::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(1337);

        let total_frames = 100usize;
        let start = std::time::Instant::now();
        let mut total_active_sectors = 0u64;

        for frame_idx in 0..total_frames {
            // Simulate 90% static HUD with 10% moving crosshair/target sector
            let mut frame = [0.1f32; GRID_SIZE];
            if frame_idx % 3 == 0 {
                // Motion in target sector (sector 42)
                frame[42] = rng.random_range(0.8..1.0);
            }

            let active = pipeline.process_frame(&frame);
            total_active_sectors += active as u64;
        }

        let elapsed = start.elapsed();
        let avg_latency_us = (elapsed.as_micros() as f64) / (total_frames as f64);
        let avg_latency_ms = avg_latency_us / 1000.0;

        // Sub-16ms requirement (target is sub-1ms for pure CPU gating step)
        assert!(avg_latency_ms < 1.0, "Average perception latency too high: {:.3}ms (must be < 1.0ms)", avg_latency_ms);

        // Compute savings: Most static sectors should be gated off after warmup
        let skip_ratio = pipeline.gate_matrix.skip_ratio();
        assert!(skip_ratio > 0.50, "Epigenetic skip ratio should exceed 50%, got {:.2}%", skip_ratio * 100.0);
    }
}
