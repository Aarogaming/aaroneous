// Epigenetic Visual Gating Shader
// Runs on WebGPU compute to perform parallel sector delta detection and bitmask generation.
//
// Input: Two consecutive 128x128 float pixel grids (current + previous frame)
// Output: 256-bit packed sector activity mask (4x u64)
//
// Each workgroup processes one 8x8 pixel sector. If delta < threshold,
// the sector is flagged dormant and excluded from downstream compute.

// Grid constants
const GRID_WIDTH: u32 = 128u;
const GRID_HEIGHT: u32 = 128u;
const SECTOR_SIZE: u32 = 8u;
const SECTORS_PER_ROW: u32 = 16u;
const SECTORS_PER_COL: u32 = 16u;
const TOTAL_SECTORS: u32 = 256u;

// Delta threshold for activity detection
const DELTA_THRESHOLD: f32 = 0.02;

// Hysteresis: frames required before gating off
const HYSTERESIS_FRAMES: u32 = 3u;

@group(0) @binding(0) var<storage, read> frame_current: array<f32>;
@group(0) @binding(1) var<storage, read> frame_previous: array<f32>;
@group(0) @binding(2) var<storage, read> sector_means_prev: array<f32>;
@group(0) @binding(3) var<storage, read_write> sector_means_new: array<f32>;
@group(0) @binding(4) var<storage, read_write> sector_frames_static: array<u32>;
@group(0) @binding(5) var<storage, read_write> sector_active: array<u32>;
@group(0) @binding(6) var<storage, read_write> packed_mask: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let sector_idx = id.x;
    if (sector_idx >= TOTAL_SECTORS) { return; }

    // Calculate sector position in grid
    let sector_x = sector_idx % SECTORS_PER_ROW;
    let sector_y = sector_idx / SECTORS_PER_ROW;

    // Compute mean intensity for this sector in current frame
    var sum_current: f32 = 0.0;
    var sum_previous: f32 = 0.0;
    var count: u32 = 0u;

    for (var dy: u32 = 0u; dy < SECTOR_SIZE; dy = dy + 1u) {
        for (var dx: u32 = 0u; dx < SECTOR_SIZE; dx = dx + 1u) {
            let px = sector_x * SECTOR_SIZE + dx;
            let py = sector_y * SECTOR_SIZE + dy;
            let grid_idx = py * GRID_WIDTH + px;

            sum_current = sum_current + frame_current[grid_idx];
            sum_previous = sum_previous + frame_previous[grid_idx];
            count = count + 1u;
        }
    }

    let mean_current = sum_current / f32(count);
    let mean_prev = sector_means_prev[sector_idx];

    // Compute delta between current mean and stored previous mean
    let delta = abs(mean_current - mean_prev);

    // Update epigenetic flag with hysteresis
    var is_active: u32 = sector_active[sector_idx];
    var frames_static = sector_frames_static[sector_idx];

    if (delta > DELTA_THRESHOLD) {
        is_active = 1u;
        frames_static = 0u;
    } else {
        frames_static = frames_static + 1u;
        if (frames_static >= HYSTERESIS_FRAMES) {
            is_active = 0u;
        }
    }

    // Write updated state
    sector_means_new[sector_idx] = mean_current;
    sector_frames_static[sector_idx] = frames_static;
    sector_active[sector_idx] = is_active;

    // Pack into bitmask (32 sectors per u32 word)
    let word_idx = sector_idx / 32u;
    let bit_idx = sector_idx % 32u;

    // Atomic-like update: read-modify-write
    let old_word = packed_mask[word_idx];
    var new_word = old_word;

    if (is_active == 1u) {
        new_word = new_word | (1u << bit_idx);
    } else {
        new_word = new_word & ~(1u << bit_idx);
    }

    packed_mask[word_idx] = new_word;
}
