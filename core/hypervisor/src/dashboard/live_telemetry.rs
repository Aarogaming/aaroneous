// Live Telemetry Reader - Reads real-time data from Python telemetry bridge.
//
// Connects the egui dashboard to the Win32 intercept perimeter via shared memory,
// displaying live capture metrics, epigenetic gate states, and motor intents.

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Instant;

use crate::dashboard::spatial_kinetic::SpatialKineticTelemetry;

pub const TELEMETRY_MAGIC: [u8; 4] = *b"TEL1";
pub const TELEMETRY_SIZE: usize = 64 * 1024;

/// Telemetry data structure matching Python layout
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LiveTelemetryData {
    pub frame_id: u64,
    pub fps: f32,
    pub capture_latency_ms: f32,
    pub active_sectors: i32,
    pub total_sectors: i32,
    pub skip_ratio: f32,
    pub delta_mean: f32,
    pub delta_max: f32,
    pub intent_dx: f32,
    pub intent_dy: f32,
    pub intent_actions: u32,
    pub genome_voxels: u32,
    pub vram_mb: f32,
}

/// Reads live telemetry from the Python bridge shared memory
pub struct LiveTelemetryReader {
    path: PathBuf,
    file: Option<std::fs::File>,
    last_update: Instant,
    data: LiveTelemetryData,
    is_connected: bool,
}

impl LiveTelemetryReader {
    pub fn new(name: &str) -> Self {
        let path = PathBuf::from(
            std::env::var("LOCALAPPDATA").unwrap_or_default()
        )
        .join("Temp")
        .join(format!("{}.telemetry", name));

        Self {
            path,
            file: None,
            last_update: Instant::now(),
            data: LiveTelemetryData {
                frame_id: 0,
                fps: 0.0,
                capture_latency_ms: 0.0,
                active_sectors: 256,
                total_sectors: 256,
                skip_ratio: 0.0,
                delta_mean: 0.0,
                delta_max: 0.0,
                intent_dx: 0.0,
                intent_dy: 0.0,
                intent_actions: 0,
                genome_voxels: 0,
                vram_mb: 0.0,
            },
            is_connected: false,
        }
    }

    pub fn open(&mut self) -> Result<(), String> {
        if !self.path.exists() {
            return Err("Telemetry file not found. Run live_telemetry_bridge.py first.".to_string());
        }

        self.file = Some(
            OpenOptions::new()
                .read(true)
                .open(&self.path)
                .map_err(|e| format!("Failed to open telemetry: {}", e))?,
        );

        self.is_connected = true;
        Ok(())
    }

    /// Return the last time the telemetry was updated
    pub fn last_update(&self) -> Instant {
        self.last_update
    }

    pub fn read(&mut self) -> Result<LiveTelemetryData, String> {
        if !self.is_connected {
            self.open()?;
        }

        let file = self.file.as_mut().ok_or("Telemetry not opened")?;

        // Read magic
        let mut magic = [0u8; 4];
        file.seek(SeekFrom::Start(0))
            .map_err(|e| format!("Seek failed: {}", e))?;
        file.read_exact(&mut magic)
            .map_err(|e| format!("Read magic failed: {}", e))?;

        if magic != TELEMETRY_MAGIC {
            return Err("Invalid telemetry magic".to_string());
        }

        // Read frame ID
        let mut frame_id_bytes = [0u8; 8];
        file.read_exact(&mut frame_id_bytes)
            .map_err(|e| format!("Read frame_id failed: {}", e))?;
        let frame_id = u64::from_le_bytes(frame_id_bytes);

        // Skip if we've already read this frame
        if frame_id == self.data.frame_id {
            return Ok(self.data);
        }

        // Read telemetry data (48 bytes)
        let mut data_bytes = [0u8; 48];
        file.read_exact(&mut data_bytes)
            .map_err(|e| format!("Read data failed: {}", e))?;

        self.data = LiveTelemetryData {
            frame_id,
            fps: f32::from_le_bytes([data_bytes[0], data_bytes[1], data_bytes[2], data_bytes[3]]),
            capture_latency_ms: f32::from_le_bytes([data_bytes[4], data_bytes[5], data_bytes[6], data_bytes[7]]),
            active_sectors: i32::from_le_bytes([data_bytes[8], data_bytes[9], data_bytes[10], data_bytes[11]]),
            total_sectors: i32::from_le_bytes([data_bytes[12], data_bytes[13], data_bytes[14], data_bytes[15]]),
            skip_ratio: f32::from_le_bytes([data_bytes[16], data_bytes[17], data_bytes[18], data_bytes[19]]),
            delta_mean: f32::from_le_bytes([data_bytes[20], data_bytes[21], data_bytes[22], data_bytes[23]]),
            delta_max: f32::from_le_bytes([data_bytes[24], data_bytes[25], data_bytes[26], data_bytes[27]]),
            intent_dx: f32::from_le_bytes([data_bytes[28], data_bytes[29], data_bytes[30], data_bytes[31]]),
            intent_dy: f32::from_le_bytes([data_bytes[32], data_bytes[33], data_bytes[34], data_bytes[35]]),
            intent_actions: u32::from_le_bytes([data_bytes[36], data_bytes[37], data_bytes[38], data_bytes[39]]),
            genome_voxels: u32::from_le_bytes([data_bytes[40], data_bytes[41], data_bytes[42], data_bytes[43]]),
            vram_mb: f32::from_le_bytes([data_bytes[44], data_bytes[45], data_bytes[46], data_bytes[47]]),
        };

        Ok(self.data)
    }

    /// Update egui telemetry from live data
    pub fn update_telemetry(&mut self, telemetry: &mut SpatialKineticTelemetry) {
        if let Ok(data) = self.read() {
            telemetry.frame_fps = data.fps;
            telemetry.compute_latency_us = data.capture_latency_ms * 1000.0;
            telemetry.gate_matrix_active = data.active_sectors as u32;
            telemetry.gate_matrix_total = data.total_sectors as u32;
            telemetry.skip_ratio = data.skip_ratio;
            telemetry.genome_voxels = data.genome_voxels as u64;
            telemetry.vram_usage_mb = data.vram_mb;
        }
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    pub fn get_data(&self) -> LiveTelemetryData {
        self.data
    }
}
