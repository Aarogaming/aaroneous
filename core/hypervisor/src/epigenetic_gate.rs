// Epigenetic Visual Gating Matrix
// Implements zero-compute skipping on static screen regions via binary bitmask overlay.
//
// Maps a 128x128 screen pixel grid into sectors, tracks visual delta frame-over-frame,
// and flips dormant region flags to 0. The WASM hypervisor drops flagged memory
// addresses entirely from the execution thread, reducing compute overhead by up to 90%.

use std::sync::atomic::{AtomicU64, Ordering};

/// Grid dimensions for the sensory input
pub const GRID_WIDTH: usize = 128;
pub const GRID_HEIGHT: usize = 128;
pub const GRID_SIZE: usize = GRID_WIDTH * GRID_HEIGHT;

/// Sector size for epigenetic gating (8x8 pixel blocks)
pub const SECTOR_SIZE: usize = 8;
pub const SECTORS_PER_ROW: usize = GRID_WIDTH / SECTOR_SIZE; // 16
pub const SECTORS_PER_COL: usize = GRID_HEIGHT / SECTOR_SIZE; // 16
pub const TOTAL_SECTORS: usize = SECTORS_PER_ROW * SECTORS_PER_COL; // 256

/// Delta threshold to consider a sector "active" (changed)
pub const DELTA_THRESHOLD: f32 = 0.02;

/// Epigenetic gate state for a single sector
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SectorGate {
    /// 1 = active (compute), 0 = dormant (skip)
    pub active: u8,
    /// Running average of pixel values in this sector (for delta comparison)
    pub mean_intensity: f32,
    /// Frame counter since last state change
    pub frames_since_change: u32,
}

/// The complete epigenetic gating matrix overlay
/// Maps directly over the 128x128 input screen grid as a 256-sector bitmask
#[repr(C)]
#[derive(Debug, Clone)]
pub struct EpigeneticGateMatrix {
    /// 256 sector gates (16x16 grid of 8x8 pixel blocks)
    pub sectors: [SectorGate; TOTAL_SECTORS],
    /// Packed 256-bit mask for fast SIMD/WebGPU transfer (4x u64)
    pub packed_mask: [u64; 4],
    /// Total active sectors this frame
    pub active_count: u32,
    /// Frame counter
    pub frame_id: u64,
}

impl EpigeneticGateMatrix {
    pub fn new() -> Self {
        let mut sectors = [SectorGate::default(); TOTAL_SECTORS];
        for s in sectors.iter_mut() {
            s.active = 1;
        }
        Self {
            sectors,
            packed_mask: [0u64; 4],
            active_count: TOTAL_SECTORS as u32,
            frame_id: 0,
        }
    }

    /// Update the gating matrix based on new frame data.
    ///
    /// Compares the incoming 128x128 float grid against the stored sector means,
    /// flips epigenetic flags for dormant regions, and rebuilds the packed bitmask.
    ///
    /// Returns the number of active sectors (compute targets) for this frame.
    pub fn update(&mut self, frame: &[f32; GRID_SIZE]) -> u32 {
        self.frame_id += 1;
        let mut active_count = 0u32;

        for sector_y in 0..SECTORS_PER_COL {
            for sector_x in 0..SECTORS_PER_ROW {
                let sector_idx = sector_y * SECTORS_PER_ROW + sector_x;
                let gate = &mut self.sectors[sector_idx];

                // Compute sector mean intensity
                let mut sum = 0.0f32;
                let mut count = 0usize;

                for dy in 0..SECTOR_SIZE {
                    for dx in 0..SECTOR_SIZE {
                        let px = sector_x * SECTOR_SIZE + dx;
                        let py = sector_y * SECTOR_SIZE + dy;
                        let idx = py * GRID_WIDTH + px;
                        sum += frame[idx];
                        count += 1;
                    }
                }

                let mean = sum / count as f32;
                let delta = (mean - gate.mean_intensity).abs();

                // Update epigenetic flag based on visual delta
                if delta > DELTA_THRESHOLD {
                    gate.active = 1;
                    gate.frames_since_change = 0;
                } else {
                    gate.frames_since_change += 1;
                    // Hysteresis: require 3 consecutive static frames before gating off
                    if gate.frames_since_change >= 3 {
                        gate.active = 0;
                    }
                }

                gate.mean_intensity = mean;
                if gate.active == 1 {
                    active_count += 1;
                }
            }
        }

        self.active_count = active_count;
        self.rebuild_packed_mask();
        active_count
    }

    /// Rebuild the 256-bit packed mask from sector states
    /// Organized as 4x u64 for efficient WebGPU storage buffer transfer
    fn rebuild_packed_mask(&mut self) {
        self.packed_mask = [0u64; 4];
        for i in 0..TOTAL_SECTORS {
            let word = i / 64;
            let bit = i % 64;
            if self.sectors[i].active == 1 {
                self.packed_mask[word] |= 1u64 << bit;
            }
        }
    }

    /// Check if a specific pixel coordinate is in an active sector
    #[inline]
    pub fn is_pixel_active(&self, x: usize, y: usize) -> bool {
        if x >= GRID_WIDTH || y >= GRID_HEIGHT {
            return false;
        }
        let sector_x = x / SECTOR_SIZE;
        let sector_y = y / SECTOR_SIZE;
        let sector_idx = sector_y * SECTORS_PER_ROW + sector_x;
        self.sectors[sector_idx].active == 1
    }

    /// Get the packed bitmask for direct GPU transfer
    pub fn get_gpu_mask(&self) -> [u64; 4] {
        self.packed_mask
    }

    /// Get active sector count for compute load estimation
    pub fn active_sector_count(&self) -> u32 {
        self.active_count
    }

    /// Get compute skip ratio (percentage of sectors gated off)
    pub fn skip_ratio(&self) -> f32 {
        1.0 - (self.active_count as f32 / TOTAL_SECTORS as f32)
    }

    /// Force all sectors active (emergency override)
    pub fn force_all_active(&mut self) {
        for gate in self.sectors.iter_mut() {
            gate.active = 1;
            gate.frames_since_change = 0;
        }
        self.active_count = TOTAL_SECTORS as u32;
        self.rebuild_packed_mask();
    }

    /// Force specific sector active (e.g., HUD region override)
    pub fn force_sector_active(&mut self, sector_x: usize, sector_y: usize) {
        if sector_x < SECTORS_PER_ROW && sector_y < SECTORS_PER_COL {
            let idx = sector_y * SECTORS_PER_ROW + sector_x;
            if self.sectors[idx].active == 0 {
                self.sectors[idx].active = 1;
                self.sectors[idx].frames_since_change = 0;
                self.active_count += 1;
                self.rebuild_packed_mask();
            }
        }
    }
}

/// Frame buffer for delta comparison between consecutive frames
pub struct FrameBuffer {
    pub current: Box<[f32; GRID_SIZE]>,
    pub previous: Box<[f32; GRID_SIZE]>,
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self {
            current: Box::new([0.0f32; GRID_SIZE]),
            previous: Box::new([0.0f32; GRID_SIZE]),
        }
    }

    /// Swap current to previous and return mutable reference to new current
    pub fn swap(&mut self) -> &mut [f32; GRID_SIZE] {
        std::mem::swap(&mut self.current, &mut self.previous);
        &mut self.current
    }

    pub fn get_current(&self) -> &[f32; GRID_SIZE] {
        &self.current
    }
}

/// Atomic counter for multi-threaded gate state tracking
pub struct AtomicGateState {
    pub active_sectors: AtomicU64,
    pub total_sectors: AtomicU64,
    pub frame_id: AtomicU64,
}

impl AtomicGateState {
    pub fn new() -> Self {
        Self {
            active_sectors: AtomicU64::new(TOTAL_SECTORS as u64),
            total_sectors: AtomicU64::new(TOTAL_SECTORS as u64),
            frame_id: AtomicU64::new(0),
        }
    }

    pub fn update(&self, active: u64) {
        self.active_sectors.store(active, Ordering::Relaxed);
        self.frame_id.fetch_add(1, Ordering::Relaxed);
    }

    pub fn skip_ratio(&self) -> f64 {
        let active = self.active_sectors.load(Ordering::Relaxed);
        let total = self.total_sectors.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        1.0 - (active as f64 / total as f64)
    }
}

/// Integration struct for the WASM hypervisor reflex loop
/// Combines gate matrix with frame buffer for complete epigenetic processing
pub struct VisualGatePipeline {
    pub gate_matrix: EpigeneticGateMatrix,
    pub frame_buffer: FrameBuffer,
    pub atomic_state: AtomicGateState,
}

impl VisualGatePipeline {
    pub fn new() -> Self {
        Self {
            gate_matrix: EpigeneticGateMatrix::new(),
            frame_buffer: FrameBuffer::new(),
            atomic_state: AtomicGateState::new(),
        }
    }

    /// Process a new frame and return the active sector count
    /// This is the main entry point called each frame by the hypervisor
    pub fn process_frame(&mut self, new_frame: &[f32; GRID_SIZE]) -> u32 {
        // Copy new frame data
        let current = self.frame_buffer.swap();
        current.copy_from_slice(new_frame);

        // Update epigenetic gates
        let active = self.gate_matrix.update(self.frame_buffer.get_current());

        // Update atomic state for cross-thread visibility
        self.atomic_state.update(active as u64);

        active
    }

    /// Get the GPU-ready bitmask for WebGPU compute shader dispatch
    pub fn gpu_dispatch_mask(&self) -> [u64; 4] {
        self.gate_matrix.get_gpu_mask()
    }

    /// Check if compute can be skipped entirely (all sectors static)
    pub fn can_skip_compute(&self) -> bool {
        self.gate_matrix.active_count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngExt;
    use rand::SeedableRng;

    #[test]
    fn test_gate_matrix_initial_state() {
        let matrix = EpigeneticGateMatrix::new();
        assert_eq!(matrix.active_count, TOTAL_SECTORS as u32);
        assert_eq!(matrix.skip_ratio(), 0.0);
    }

    #[test]
    fn test_gate_matrix_static_frames_gate_off() {
        let mut matrix = EpigeneticGateMatrix::new();
        let frame = [0.5f32; GRID_SIZE];

        // First frame: all active (initial state)
        let active = matrix.update(&frame);
        assert_eq!(active, TOTAL_SECTORS as u32);

        // Second identical frame: still active (hysteresis)
        let active = matrix.update(&frame);
        assert_eq!(active, TOTAL_SECTORS as u32);

        // Third identical frame: still active (hysteresis threshold)
        let active = matrix.update(&frame);
        assert_eq!(active, TOTAL_SECTORS as u32);

        // Fourth identical frame: should start gating off
        let active = matrix.update(&frame);
        assert!(active < TOTAL_SECTORS as u32);
    }

    #[test]
    fn test_gate_matrix_motion_keeps_active() {
        let mut matrix = EpigeneticGateMatrix::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        // Alternate between near-0 and near-1 base frames to guarantee
        // every sector's mean intensity changes by more than DELTA_THRESHOLD.
        for i in 0..10 {
            let base = if i % 2 == 0 { 0.0 } else { 1.0 };
            let mut frame = [base; GRID_SIZE];
            for val in frame.iter_mut() {
                *val += rng.random_range(-0.05..0.05);
            }
            let active = matrix.update(&frame);
            assert_eq!(active, TOTAL_SECTORS as u32);
        }
    }

    #[test]
    fn test_packed_mask_correctness() {
        let mut matrix = EpigeneticGateMatrix::new();
        let frame = [0.5f32; GRID_SIZE];

        matrix.update(&frame);
        matrix.update(&frame);
        matrix.update(&frame);
        matrix.update(&frame);

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
        assert!(!matrix.is_pixel_active(128, 128)); // Out of bounds
    }

    #[test]
    fn test_force_sector_active() {
        let mut matrix = EpigeneticGateMatrix::new();
        let frame = [0.5f32; GRID_SIZE];

        // Gate everything off
        for _ in 0..5 {
            matrix.update(&frame);
        }

        let before = matrix.active_count;
        matrix.force_sector_active(0, 0);
        assert_eq!(matrix.active_count, before + 1);
    }

    #[test]
    fn test_frame_buffer_swap() {
        let mut fb = FrameBuffer::new();
        let new_data = [1.0f32; GRID_SIZE];

        fb.current.copy_from_slice(&new_data);
        let current = fb.swap();
        current.copy_from_slice(&[2.0f32; GRID_SIZE]);

        assert_eq!(fb.previous[0], 1.0);
        assert_eq!(fb.current[0], 2.0);
    }

    #[test]
    fn test_skip_ratio_calculation() {
        let mut matrix = EpigeneticGateMatrix::new();
        assert_eq!(matrix.skip_ratio(), 0.0);

        // Manually gate off half the sectors
        for i in 0..TOTAL_SECTORS / 2 {
            matrix.sectors[i].active = 0;
        }
        matrix.active_count = (TOTAL_SECTORS / 2) as u32;
        matrix.rebuild_packed_mask();

        assert!((matrix.skip_ratio() - 0.5).abs() < 0.01);
    }
}
