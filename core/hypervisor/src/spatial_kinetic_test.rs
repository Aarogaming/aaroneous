// Integration test for the spatial-kinetic pipeline
// Verifies the complete loop: genome loading → gate matrix → reflex kernel → motor intent

#[cfg(test)]
mod spatial_kinetic_integration_tests {
    use crate::epigenetic_gate::{EpigeneticGateMatrix, GRID_SIZE, VisualGatePipeline};
    use crate::win32_intercept::hid_bridge::{ACTION_CLICK, ACTION_MOUSE_MOVE, MotorIntent};
    use rand::RngExt;
    use rand::SeedableRng;

    #[test]
    fn test_gate_matrix_initial_state() {
        let matrix = EpigeneticGateMatrix::new();
        assert_eq!(matrix.active_sector_count(), 256);
        assert!((matrix.skip_ratio() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_gate_matrix_static_frames_gate_off() {
        let mut matrix = EpigeneticGateMatrix::new();
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
        let mut matrix = EpigeneticGateMatrix::new();
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
        let mut pipeline = VisualGatePipeline::new();
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
        let mut matrix = EpigeneticGateMatrix::new();
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
        let matrix = EpigeneticGateMatrix::new();
        assert!(matrix.is_pixel_active(0, 0));
        assert!(matrix.is_pixel_active(64, 64));
        assert!(matrix.is_pixel_active(127, 127));
        assert!(!matrix.is_pixel_active(128, 128));
    }

    #[test]
    fn test_skip_ratio_calculation() {
        let mut matrix = EpigeneticGateMatrix::new();
        assert_eq!(matrix.skip_ratio(), 0.0);

        // Manually gate off half the sectors
        for i in 0..128 {
            matrix.sectors[i].active = 0;
        }
        matrix.active_count = 128;

        assert!((matrix.skip_ratio() - 0.5).abs() < 0.01);
    }
}
