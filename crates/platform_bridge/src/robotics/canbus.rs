//! crates/platform_bridge/src/robotics/canbus.rs
//! Automotive CAN 2.0B / CAN-FD Bus Bridge and Protocol Entropy Analyzer.
//!
//! Provides zero-copy vehicular telemetry packetization and autonomous protocol reverse engineering.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Standard or Extended CAN / CAN-FD Frame Packet
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanFrame {
    pub arbitration_id: u32,
    pub is_extended: bool,
    pub is_fd: bool,
    pub dlc: u8,
    pub payload: Vec<u8>,
    pub timestamp_us: u64,
}

impl CanFrame {
    pub fn new_standard(arbitration_id: u16, data: &[u8], timestamp_us: u64) -> Self {
        Self {
            arbitration_id: arbitration_id as u32,
            is_extended: false,
            is_fd: false,
            dlc: data.len().min(8) as u8,
            payload: data[..data.len().min(8)].to_vec(),
            timestamp_us,
        }
    }

    pub fn new_fd(arbitration_id: u32, data: &[u8], timestamp_us: u64) -> Self {
        Self {
            arbitration_id,
            is_extended: true,
            is_fd: true,
            dlc: data.len().min(64) as u8,
            payload: data[..data.len().min(64)].to_vec(),
            timestamp_us,
        }
    }
}

/// Automotive Bus Bridge managing CAN 2.0B and CAN-FD channels
pub struct AutomotiveBusBridge {
    channel_name: String,
    baud_rate: u32,
    frame_history: Vec<CanFrame>,
}

impl AutomotiveBusBridge {
    pub fn new(channel_name: impl Into<String>, baud_rate: u32) -> Self {
        Self {
            channel_name: channel_name.into(),
            baud_rate,
            frame_history: Vec::new(),
        }
    }

    pub fn channel_name(&self) -> &str {
        &self.channel_name
    }

    pub fn baud_rate(&self) -> u32 {
        self.baud_rate
    }

    pub fn transmit_frame(&mut self, frame: CanFrame) -> Result<()> {
        if !frame.is_fd && frame.payload.len() > 8 {
            bail!("Standard CAN frames cannot exceed 8 bytes of payload");
        }
        self.frame_history.push(frame);
        Ok(())
    }

    pub fn frame_count(&self) -> usize {
        self.frame_history.len()
    }
}

/// Autonomous Shannon Entropy Analyzer for Unknown Protocol Ingestion
pub struct ProtocolEntropyAnalyzer;

impl ProtocolEntropyAnalyzer {
    /// Computes Shannon entropy per byte column across a collection of CAN payload slices
    pub fn compute_column_entropy(payloads: &[Vec<u8>], payload_len: usize) -> Vec<f32> {
        if payloads.is_empty() || payload_len == 0 {
            return Vec::new();
        }

        let mut entropies = Vec::with_capacity(payload_len);

        for col in 0..payload_len {
            let mut frequencies = [0usize; 256];
            let mut valid_samples = 0;

            for p in payloads {
                if col < p.len() {
                    frequencies[p[col] as usize] += 1;
                    valid_samples += 1;
                }
            }

            if valid_samples == 0 {
                entropies.push(0.0);
                continue;
            }

            let mut h = 0.0f32;
            for &count in &frequencies {
                if count > 0 {
                    let p_i = count as f32 / valid_samples as f32;
                    h -= p_i * p_i.log2();
                }
            }
            entropies.push(h);
        }

        entropies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_frame_and_bus_bridge() {
        let mut bridge = AutomotiveBusBridge::new("CAN0", 500_000);
        let frame = CanFrame::new_standard(0x1F0, &[0x01, 0x02, 0x03, 0x04], 1000);
        assert!(bridge.transmit_frame(frame).is_ok());
        assert_eq!(bridge.frame_count(), 1);
    }

    #[test]
    fn test_protocol_entropy_detection() {
        // Col 0 constant (H=0), Col 1 alternating (H=1)
        let payloads = vec![
            vec![0xAA, 0x00],
            vec![0xAA, 0x01],
            vec![0xAA, 0x00],
            vec![0xAA, 0x01],
        ];

        let h = ProtocolEntropyAnalyzer::compute_column_entropy(&payloads, 2);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0], 0.0);
        assert!((h[1] - 1.0).abs() < 1e-4);
    }
}
