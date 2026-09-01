// WGPU Reflex Pipeline
// Hardware-accelerated bitwise genome processing via WebGPU compute shaders.
//
// Memory-maps the universal gaming genome binary, uploads to GPU storage buffers,
// dispatches the reflex kernel compute shader with epigenetic gate masking,
// and reads back motor intents for HID execution.

use std::path::Path;
use std::sync::Arc;

use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, ComputePassDescriptor, ComputePipeline,
    ComputePipelineDescriptor, Device, Queue, ShaderModuleDescriptor, ShaderSource,
    util::DeviceExt,
};

use crate::spatial_delta_gate::SpatialDeltaGateMatrix;
use crate::win32_intercept::hid_bridge::MotorIntent;

pub const GRID_SIZE: usize = 128 * 128;
pub const TOTAL_SECTORS: usize = 256;
pub const MAX_VOXELS: usize = 1_200_000_000; // 1.2B voxel capacity
pub const MAX_BUFFER_SIZE: u64 = 128 * 1024 * 1024; // 128 MB wgpu max binding size

/// The WGPU Reflex Pipeline - executes bitwise genome processing on GPU
pub struct WgpuReflexPipeline {
    device: Arc<Device>,
    queue: Arc<Queue>,
    genome_buffers: Vec<Buffer>,
    pixel_buffer: Buffer,
    intent_buffer: Buffer,
    gate_mask_buffer: Buffer,
    compute_pipeline: ComputePipeline,
    voxel_count: u32,
    staging_buffer: Buffer,
}

impl WgpuReflexPipeline {
    /// Initialize the WGPU reflex pipeline with genome file and shader sources
    pub async fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        genome_path: &Path,
        reflex_shader_source: &str,
    ) -> Result<Self, String> {
        // Memory-map the genome file
        let genome_data = Self::load_genome_mmap(genome_path)?;
        let voxel_count = genome_data.len() as u32;

        // Chunk genome into multiple buffers (wgpu has 256MB limit per buffer)
        let max_voxels_per_buffer = (MAX_BUFFER_SIZE / 4) as usize; // 4 bytes per u32
        let mut genome_buffers = Vec::new();
        let mut offset = 0;
        let mut chunk_idx = 0;
        while offset < genome_data.len() {
            let end = (offset + max_voxels_per_buffer).min(genome_data.len());
            let chunk = &genome_data[offset..end];
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Genome Storage Buffer {}", chunk_idx)),
                contents: bytemuck::cast_slice(chunk),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });
            genome_buffers.push(buffer);
            offset = end;
            chunk_idx += 1;
        }

        let intent_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Motor Intent Output Buffer"),
            size: (voxel_count as usize * std::mem::size_of::<f32>()).min(MAX_BUFFER_SIZE as usize)
                as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let gate_mask_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Epigenetic Gate Mask Buffer"),
            size: (TOTAL_SECTORS * std::mem::size_of::<u32>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (voxel_count as usize * std::mem::size_of::<f32>()).min(1024 * 1024) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Compile reflex kernel shader
        let reflex_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Reflex Kernel Shader"),
            source: ShaderSource::Wgsl(reflex_shader_source.into()),
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Reflex Compute Pipeline"),
            layout: None,
            module: &reflex_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let pixel_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Pixel Input Buffer"),
            size: (GRID_SIZE * std::mem::size_of::<f32>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            device,
            queue,
            genome_buffers,
            pixel_buffer,
            intent_buffer,
            gate_mask_buffer,
            compute_pipeline,
            voxel_count,
            staging_buffer,
        })
    }

    /// Load genome from binary file via memory mapping
    fn load_genome_mmap(path: &Path) -> Result<Vec<u32>, String> {
        let file =
            std::fs::File::open(path).map_err(|e| format!("Failed to open genome file: {}", e))?;

        let mmap = unsafe {
            memmap2::Mmap::map(&file).map_err(|e| format!("Failed to memory-map genome: {}", e))?
        };

        // Parse header: AASv1 magic (5 bytes) + voxel_count (8 bytes) + weight_count (8 bytes)
        if mmap.len() < 21 {
            return Err("Genome file too small".to_string());
        }

        let magic = &mmap[0..5];
        if magic != b"AASv1" {
            // Try raw binary format without header
            let voxels: Vec<u32> = bytemuck::pod_collect_to_vec(&mmap);
            return Ok(voxels);
        }

        let voxel_count = u64::from_le_bytes([
            mmap[5], mmap[6], mmap[7], mmap[8], mmap[9], mmap[10], mmap[11], mmap[12],
        ]) as usize;

        let data_start = 21;
        let data_end = data_start + voxel_count * 4;

        if data_end > mmap.len() {
            return Err(format!(
                "Genome file truncated: expected {} bytes, got {}",
                data_end,
                mmap.len()
            ));
        }

        let voxel_data = &mmap[data_start..data_end];
        let voxels: Vec<u32> = bytemuck::pod_collect_to_vec(voxel_data);

        Ok(voxels)
    }

    /// Execute the reflex kernel on a new frame
    ///
    /// Args:
    ///   pixels: 128x128 normalized float grid from screen capture
    ///   gate_matrix: epigenetic gating state (optional, uses full compute if None)
    ///
    /// Returns:
    ///   Computed intent values for motor execution
    pub async fn execute_frame(
        &self,
        pixels: &[f32; GRID_SIZE],
        gate_matrix: Option<&SpatialDeltaGateMatrix>,
    ) -> Vec<f32> {
        // Upload pixel data to GPU
        self.queue.write_buffer(
            &self.pixel_buffer,
            0,
            bytemuck::cast_slice(pixels.as_slice()),
        );

        // Upload gate mask if provided
        if let Some(gate) = gate_matrix {
            let mask = gate.get_gpu_mask();
            self.queue
                .write_buffer(&self.gate_mask_buffer, 0, bytemuck::cast_slice(&mask));
        }

        // Process each genome chunk
        let max_voxels_per_buffer = (MAX_BUFFER_SIZE / 4) as usize;
        let mut all_intents = Vec::with_capacity(self.voxel_count as usize);

        for (chunk_idx, genome_buf) in self.genome_buffers.iter().enumerate() {
            let chunk_voxels = max_voxels_per_buffer
                .min(self.voxel_count as usize - chunk_idx * max_voxels_per_buffer);
            if chunk_voxels == 0 {
                break;
            }

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(&format!("Reflex Frame Encoder Chunk {}", chunk_idx)),
                });

            // Create bind group for this chunk
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("Reflex Bind Group Chunk {}", chunk_idx)),
                layout: &self.compute_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: genome_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.pixel_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.intent_buffer.as_entire_binding(),
                    },
                ],
            });

            {
                let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some(&format!("Reflex Compute Pass Chunk {}", chunk_idx)),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);

                // Dispatch with workgroup count within limits (max 65535)
                let threads_per_workgroup = 256u32;
                let workgroup_count = (chunk_voxels as u32)
                    .div_ceil(threads_per_workgroup)
                    .min(65535);
                compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
            }

            // Copy results to staging buffer
            let read_size = (chunk_voxels * std::mem::size_of::<f32>())
                .min(self.staging_buffer.size() as usize) as u64;

            encoder.copy_buffer_to_buffer(
                &self.intent_buffer,
                0,
                &self.staging_buffer,
                0,
                read_size,
            );

            self.queue.submit(Some(encoder.finish()));

            // Read back chunk results
            let chunk_intents = self.readback_intents(read_size as usize).await;
            all_intents.extend(chunk_intents);
        }

        all_intents
    }

    async fn readback_intents(&self, size: usize) -> Vec<f32> {
        let buffer_slice = self.staging_buffer.slice(..size as u64);

        let (tx, rx) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
        rx.await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let intents: Vec<f32> = bytemuck::pod_collect_to_vec(&data);
        drop(data);

        self.staging_buffer.unmap();

        intents
    }

    /// Convert computed intent values to MotorIntent for HID execution
    pub fn compute_motor_intent(
        &self,
        intents: &[f32],
        gate_matrix: &SpatialDeltaGateMatrix,
    ) -> MotorIntent {
        // Aggregate intent values across active genome tracks
        let mut sum_x: f32 = 0.0;
        let mut sum_y: f32 = 0.0;
        let mut max_magnitude: f32 = 0.0;

        let active_count = gate_matrix.active_sector_count().max(1);

        for (i, &intent) in intents.iter().take(GRID_SIZE).enumerate() {
            let x = (i % 128) as f32 / 128.0;
            let y = (i / 128) as f32 / 128.0;

            if gate_matrix.is_pixel_active(i % 128, i / 128) {
                sum_x += intent * (x - 0.5);
                sum_y += intent * (y - 0.5);
                max_magnitude = max_magnitude.max(intent.abs());
            }
        }

        // Normalize by active sector count
        let scale = if max_magnitude > 0.0 {
            1.0 / max_magnitude
        } else {
            1.0
        };

        MotorIntent {
            delta_x: sum_x * scale / active_count as f32 * 100.0,
            delta_y: sum_y * scale / active_count as f32 * 100.0,
            binary_action_register: self.compute_action_flags(max_magnitude),
        }
    }

    fn compute_action_flags(&self, max_magnitude: f32) -> u64 {
        use crate::win32_intercept::hid_bridge::*;

        let mut flags: u64 = ACTION_MOUSE_MOVE;

        if max_magnitude > 2.0 {
            flags |= ACTION_CLICK;
        }

        if max_magnitude > 3.5 {
            flags |= ACTION_DOUBLE_CLICK;
        }

        flags
    }

    /// Get the number of genome voxels loaded
    pub fn voxel_count(&self) -> u32 {
        self.voxel_count
    }

    /// Get GPU memory usage estimate
    pub fn gpu_memory_usage_mb(&self) -> f32 {
        let genome_mb = self.voxel_count as f32 * 4.0 / 1024.0 / 1024.0;
        let pixel_mb = GRID_SIZE as f32 * 4.0 / 1024.0 / 1024.0;
        let intent_mb = self.voxel_count as f32 * 4.0 / 1024.0 / 1024.0;
        genome_mb + pixel_mb + intent_mb
    }
}
