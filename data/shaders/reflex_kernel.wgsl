// Universal Bitwise GPU Compute Shader
// Executes parallel general-purpose bitwise calculations on the universal gaming genome

@group(0) @binding(0) var<storage, read> universal_genome: array<u32>;
@group(0) @binding(1) var<storage, read> visual_pixels: array<f32>;
@group(0) @binding(2) var<storage, read_write> computed_intent: array<f32>;

fn lut_lookup(idx: u32) -> f32 {
    switch idx {
        case 0u: { return -2.5; }
        case 1u: { return -0.5; }
        case 2u: { return 0.5; }
        case 3u: { return 2.5; }
        default: { return 0.0; }
    }
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index >= arrayLength(&universal_genome)) { return; }
    
    let packed_u32_voxel = universal_genome[index];
    var accumulated_force: f32 = 0.0;
    
    for (var i: u32 = 0u; i < 16u; i = i + 1u) {
        let pixel_index = (index * 16u) + i;
        if (pixel_index >= arrayLength(&visual_pixels)) { break; }
        
        // Rapid right bit-shifting and hexadecimal masking to isolate the 2-bit parameter
        let extracted_base = (packed_u32_voxel >> (i * 2u)) & 0x03u;
        
        // Execute direct sensory mapping vectors at bare-metal speeds
        accumulated_force += visual_pixels[pixel_index] * lut_lookup(extracted_base);
    }
    
    computed_intent[index] = accumulated_force;
}
