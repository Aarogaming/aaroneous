// Spatial-Kinetic Engine - Main Reflex Loop
// Orchestrates the complete pipeline: capture → gate → GPU compute → HID execution.
//
// This is the central execution loop that treats any Win32 workspace as a game interface,
// processing screen pixels through the universal gaming genome to produce motor actions.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::epigenetic_gate::{VisualGatePipeline, GRID_SIZE};
use crate::win32_intercept::hid_bridge::{HIDOutputBridge, MotorIntent};
use crate::win32_intercept::capture::Win32ScreenCapture;
use crate::wgpu_reflex_pipeline::WgpuReflexPipeline;

/// Configuration for the spatial-kinetic engine
#[derive(Clone)]
pub struct SpatialKineticConfig {
    pub genome_path: PathBuf,
    pub reflex_shader_path: PathBuf,
    pub gate_shader_path: Option<PathBuf>,
    pub target_fps: f32,
    pub mouse_sensitivity: f32,
    pub enable_hid_output: bool,
    pub enable_epigenetic_gating: bool,
    pub capture_region: Option<(i32, i32, i32, i32)>, // x, y, width, height
}

impl Default for SpatialKineticConfig {
    fn default() -> Self {
        Self {
            genome_path: PathBuf::from("chromosomes/universal_gaming_core.bin"),
            reflex_shader_path: PathBuf::from("shaders/reflex_kernel.wgsl"),
            gate_shader_path: None,
            target_fps: 30.0,
            mouse_sensitivity: 1.0,
            enable_hid_output: true,
            enable_epigenetic_gating: true,
            capture_region: None,
        }
    }
}

/// Telemetry data for dashboard display
#[derive(Clone, Debug)]
pub struct EngineTelemetry {
    pub frame_id: u64,
    pub fps: f32,
    pub compute_latency_us: f32,
    pub active_sectors: u32,
    pub total_sectors: u32,
    pub skip_ratio: f32,
    pub genome_voxels: u64,
    pub vram_usage_mb: f32,
    pub motor_intents_executed: u64,
    pub last_intent: Option<MotorIntent>,
}

/// The Spatial-Kinetic Engine - main reflex loop
pub struct SpatialKineticEngine {
    config: SpatialKineticConfig,
    capture: Win32ScreenCapture,
    gate_pipeline: VisualGatePipeline,
    hid_bridge: HIDOutputBridge,
    wgpu_pipeline: Option<Arc<WgpuReflexPipeline>>,
    telemetry: EngineTelemetry,
    running: bool,
    frame_history: Vec<f32>,
    motor_intent_count: u64,
}

impl SpatialKineticEngine {
    pub fn new(config: SpatialKineticConfig) -> Self {
        Self {
            config: config.clone(),
            capture: Win32ScreenCapture::new(),
            gate_pipeline: VisualGatePipeline::new(),
            hid_bridge: HIDOutputBridge::new().with_sensitivity(config.mouse_sensitivity),
            wgpu_pipeline: None,
            telemetry: EngineTelemetry {
                frame_id: 0,
                fps: 0.0,
                compute_latency_us: 0.0,
                active_sectors: 256,
                total_sectors: 256,
                skip_ratio: 0.0,
                genome_voxels: 0,
                vram_usage_mb: 0.0,
                motor_intents_executed: 0,
                last_intent: None,
            },
            running: false,
            frame_history: Vec::with_capacity(60),
            motor_intent_count: 0,
        }
    }

    /// Initialize the engine (capture devices, GPU pipeline, genome loading)
    pub async fn initialize(&mut self) -> Result<(), String> {
        // Initialize screen capture
        self.capture.initialize()?;

        // Load shaders
        let reflex_shader = std::fs::read_to_string(&self.config.reflex_shader_path)
            .map_err(|e| format!("Failed to load reflex shader: {}", e))?;

        // Initialize WGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(|_| "Failed to find GPU adapter")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Spatial-Kinetic Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    experimental_features: wgpu::ExperimentalFeatures::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                },
            )
            .await
            .map_err(|e| format!("Failed to create WGPU device: {}", e))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Load genome and create reflex pipeline
        let pipeline = WgpuReflexPipeline::new(
            device,
            queue,
            &self.config.genome_path,
            &reflex_shader,
        )
        .await?;

        self.telemetry.genome_voxels = pipeline.voxel_count() as u64;
        self.telemetry.vram_usage_mb = pipeline.gpu_memory_usage_mb();
        self.wgpu_pipeline = Some(Arc::new(pipeline));

        println!(
            "[SpatialKineticEngine] Initialized with {} voxels ({:.0} MB VRAM)",
            self.telemetry.genome_voxels, self.telemetry.vram_usage_mb
        );

        Ok(())
    }

    /// Run the main reflex loop
    pub async fn run(&mut self) -> Result<(), String> {
        if self.wgpu_pipeline.is_none() {
            self.initialize().await?;
        }

        self.running = true;
        let frame_interval = Duration::from_secs_f32(1.0 / self.config.target_fps);
        let mut fps_counter = 0;
        let mut fps_timer = Instant::now();

        println!(
            "[SpatialKineticEngine] Reflex loop starting at {} FPS",
            self.config.target_fps
        );

        while self.running {
            let frame_start = Instant::now();

            // Execute one frame of the reflex loop
            match self.execute_frame().await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[SpatialKineticEngine] Frame error: {}", e);
                }
            }

            // FPS tracking
            fps_counter += 1;
            if fps_timer.elapsed() >= Duration::from_secs(1) {
                self.telemetry.fps = fps_counter as f32 / fps_timer.elapsed().as_secs_f32();
                fps_counter = 0;
                fps_timer = Instant::now();
            }

            // Frame rate limiting
            let elapsed = frame_start.elapsed();
            if elapsed < frame_interval {
                tokio::time::sleep(frame_interval - elapsed).await;
            }

            // Update frame history for telemetry
            self.frame_history.push(self.telemetry.fps);
            if self.frame_history.len() > 60 {
                self.frame_history.remove(0);
            }
        }

        Ok(())
    }

    /// Execute a single frame of the reflex pipeline
    async fn execute_frame(&mut self) -> Result<(), String> {
        let frame_start = Instant::now();

        // Step 1: Capture screen as 128x128 float grid
        let pixels = self.capture.capture_frame()?;
        let pixel_array: [f32; GRID_SIZE] = pixels
            .try_into()
            .map_err(|_| "Pixel array size mismatch".to_string())?;

        // Step 2: Update epigenetic gate matrix
        let active_sectors = if self.config.enable_epigenetic_gating {
            self.gate_pipeline.process_frame(&pixel_array)
        } else {
            self.gate_pipeline.gate_matrix.force_all_active();
            256
        };

        // Step 3: Execute GPU reflex kernel
        let pipeline = self.wgpu_pipeline.as_ref().ok_or("WGPU pipeline not initialized")?;

        let gate_ref = if self.config.enable_epigenetic_gating {
            Some(&self.gate_pipeline.gate_matrix)
        } else {
            None
        };

        let intents = pipeline.execute_frame(&pixel_array, gate_ref).await;

        // Step 4: Compute motor intent from GPU output
        let motor_intent = pipeline.compute_motor_intent(
            &intents,
            &self.gate_pipeline.gate_matrix,
        );

        // Step 5: Execute HID output if enabled
        if self.config.enable_hid_output {
            self.hid_bridge.execute_intent(&motor_intent);
            self.motor_intent_count += 1;
        }

        // Update telemetry
        let compute_latency = frame_start.elapsed().as_micros() as f32;
        self.telemetry.frame_id += 1;
        self.telemetry.compute_latency_us = compute_latency;
        self.telemetry.active_sectors = active_sectors;
        self.telemetry.total_sectors = 256;
        self.telemetry.skip_ratio = self.gate_pipeline.gate_matrix.skip_ratio();
        self.telemetry.motor_intents_executed = self.motor_intent_count;
        self.telemetry.last_intent = Some(motor_intent);

        Ok(())
    }

    /// Stop the reflex loop
    pub fn stop(&mut self) {
        self.running = false;
        println!("[SpatialKineticEngine] Reflex loop stopped");
    }

    /// Get current telemetry snapshot
    pub fn get_telemetry(&self) -> EngineTelemetry {
        self.telemetry.clone()
    }

    /// Get frame history for graph rendering
    pub fn get_frame_history(&self) -> &[f32] {
        &self.frame_history
    }
}
