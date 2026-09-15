//! # Aaroneous Wire Protocol
//! Universal `#![no_std]` compatible wire protocol, COBS framing, and telemetry serialization
//! for bare-metal MCUs (RP2040/ESP32), Android Head Units, and PC Host Hypervisors.

#![cfg_attr(not(feature = "std"), no_std)]

use serde::{Deserialize, Serialize};

/// Maximum packet payload size in bytes.
pub const MAX_PAYLOAD_SIZE: usize = 512;
/// Maximum framed buffer size (accounting for COBS overhead + delimiter + CRC16).
pub const MAX_FRAMED_SIZE: usize = MAX_PAYLOAD_SIZE + (MAX_PAYLOAD_SIZE / 254) + 1 + 2 + 1;

pub const CRC_ALGO: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

/// Telemetry channel identifier for mapped registers / sensors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelKind {
    DigitalInput,
    DigitalOutput,
    AnalogInput,
    PwmOutput,
    CanMessage,
    EncoderTicks,
    SystemHealth,
    TemperatureCelsius,
    PressurePascal,
    CurrentAmperes,
    VoltageVolts,
    ImuAcceleration,
    ImuGyroscope,
    OpticalLux,
    AcousticDecibels,
    GpsCoordinates,
    CustomSensor(u16),
}

impl ChannelKind {
    /// Encodes channel kind to a compact 1-byte representation for columnar batches.
    #[inline]
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::DigitalInput => 0,
            Self::DigitalOutput => 1,
            Self::AnalogInput => 2,
            Self::PwmOutput => 3,
            Self::CanMessage => 4,
            Self::EncoderTicks => 5,
            Self::SystemHealth => 6,
            Self::TemperatureCelsius => 7,
            Self::PressurePascal => 8,
            Self::CurrentAmperes => 9,
            Self::VoltageVolts => 10,
            Self::ImuAcceleration => 11,
            Self::ImuGyroscope => 12,
            Self::OpticalLux => 13,
            Self::AcousticDecibels => 14,
            Self::GpsCoordinates => 15,
            Self::CustomSensor(_) => 16,
        }
    }

    /// Decodes channel kind from a 1-byte representation.
    #[inline]
    pub const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::DigitalInput),
            1 => Some(Self::DigitalOutput),
            2 => Some(Self::AnalogInput),
            3 => Some(Self::PwmOutput),
            4 => Some(Self::CanMessage),
            5 => Some(Self::EncoderTicks),
            6 => Some(Self::SystemHealth),
            7 => Some(Self::TemperatureCelsius),
            8 => Some(Self::PressurePascal),
            9 => Some(Self::CurrentAmperes),
            10 => Some(Self::VoltageVolts),
            11 => Some(Self::ImuAcceleration),
            12 => Some(Self::ImuGyroscope),
            13 => Some(Self::OpticalLux),
            14 => Some(Self::AcousticDecibels),
            15 => Some(Self::GpsCoordinates),
            16 => Some(Self::CustomSensor(0)),
            _ => None,
        }
    }
}

/// A single telemetry datum or register reading.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChannelValue {
    pub channel_id: u8,
    pub kind: ChannelKind,
    pub raw_value: u32,
    pub calibrated_f32: f32,
}

/// Telemetry state packet broadcast from the edge MCU / Android / PC.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TelemetryPacket {
    pub sequence: u32,
    pub uptime_ms: u64,
    pub channels: [Option<ChannelValue>; 8],
    pub error_flags: u16,
}

impl TelemetryPacket {
    /// Inserts or updates a channel reading at the given slot (0..8)
    pub fn set_channel(&mut self, slot: usize, value: ChannelValue) -> bool {
        if slot < self.channels.len() {
            self.channels[slot] = Some(value);
            true
        } else {
            false
        }
    }

    /// Retrieves an active channel reading from the given slot
    pub fn get_channel(&self, slot: usize) -> Option<&ChannelValue> {
        self.channels.get(slot).and_then(|opt| opt.as_ref())
    }

    /// Checks whether an error flag mask is set
    pub fn has_error(&self, mask: u16) -> bool {
        (self.error_flags & mask) != 0
    }
}

/// High-throughput Structure-of-Arrays (SoA) SIMD record batch for telemetry channels.
///
/// Designed with 64-byte cacheline alignment and zero uninitialized padding bytes,
/// matching Apache Arrow columnar mechanics to enable AVX-512 autovectorization.
#[repr(C, align(64))]
#[derive(
    Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize,
)]
pub struct TelemetryBatchPod {
    /// Bitmask indicating which channel slots (0..15) contain active readings.
    pub valid_mask: u16,
    /// Channel kind discriminants corresponding to `ChannelKind::to_u8()`.
    pub channel_kinds: [u8; 16],
    /// Explicit padding to ensure 4-byte natural alignment for subsequent `u32` array.
    pub _pad0: [u8; 2],
    /// Raw sensor register readings (16 channels).
    pub raw_values: [u32; 16],
    /// Calibrated IEEE 754 float values (16 channels).
    pub calibrated_f32: [f32; 16],
    /// Trailing padding chunk 1 (32 bytes).
    pub _pad1: [u8; 32],
    /// Trailing padding chunk 2 (12 bytes, totaling 44 bytes to reach 192 bytes / 3 cachelines).
    pub _pad2: [u8; 12],
}

const _: () = {
    assert!(core::mem::size_of::<TelemetryBatchPod>() == 192);
    assert!(core::mem::align_of::<TelemetryBatchPod>() == 64);
    assert!(core::mem::offset_of!(TelemetryBatchPod, valid_mask) == 0);
    assert!(core::mem::offset_of!(TelemetryBatchPod, channel_kinds) == 2);
    assert!(core::mem::offset_of!(TelemetryBatchPod, _pad0) == 18);
    assert!(core::mem::offset_of!(TelemetryBatchPod, raw_values) == 20);
    assert!(core::mem::offset_of!(TelemetryBatchPod, calibrated_f32) == 84);
    assert!(core::mem::offset_of!(TelemetryBatchPod, _pad1) == 148);
    assert!(core::mem::offset_of!(TelemetryBatchPod, _pad2) == 180);
};

impl TelemetryBatchPod {
    /// Constructs an empty batch with zero valid channels.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            valid_mask: 0,
            channel_kinds: [0u8; 16],
            _pad0: [0u8; 2],
            raw_values: [0u32; 16],
            calibrated_f32: [0.0f32; 16],
            _pad1: [0u8; 32],
            _pad2: [0u8; 12],
        }
    }

    /// Checks whether a channel slot (0..15) is marked valid.
    #[inline(always)]
    pub const fn is_valid(&self, slot: usize) -> bool {
        slot < 16 && (self.valid_mask & (1 << slot)) != 0
    }

    /// Sets a channel reading in slot 0..15.
    #[inline(always)]
    pub fn set_channel(&mut self, slot: usize, kind: u8, raw: u32, calibrated: f32) -> bool {
        if slot >= 16 {
            return false;
        }
        self.valid_mask |= 1 << slot;
        self.channel_kinds[slot] = kind;
        self.raw_values[slot] = raw;
        self.calibrated_f32[slot] = calibrated;
        true
    }

    /// Gets a channel reading if valid.
    #[inline(always)]
    pub fn get_channel(&self, slot: usize) -> Option<(u8, u32, f32)> {
        if self.is_valid(slot) {
            Some((
                self.channel_kinds[slot],
                self.raw_values[slot],
                self.calibrated_f32[slot],
            ))
        } else {
            None
        }
    }

    /// Clears a channel slot.
    #[inline(always)]
    pub fn clear_channel(&mut self, slot: usize) -> bool {
        if slot >= 16 {
            return false;
        }
        self.valid_mask &= !(1 << slot);
        self.channel_kinds[slot] = 0;
        self.raw_values[slot] = 0;
        self.calibrated_f32[slot] = 0.0;
        true
    }

    /// Returns the number of active channels.
    #[inline(always)]
    pub const fn active_channel_count(&self) -> usize {
        self.valid_mask.count_ones() as usize
    }

    /// Sums all calibrated values for valid channels (vectorizable).
    pub fn sum_calibrated_f32(&self) -> f32 {
        let mut sum = 0.0f32;
        let mut i = 0;
        while i < 16 {
            if self.is_valid(i) {
                sum += self.calibrated_f32[i];
            }
            i += 1;
        }
        sum
    }
}

impl Default for TelemetryBatchPod {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<&TelemetryPacket> for TelemetryBatchPod {
    fn from(packet: &TelemetryPacket) -> Self {
        let mut pod = Self::empty();
        for (i, channel_opt) in packet.channels.iter().enumerate() {
            if let Some(cv) = channel_opt
                && i < 16
            {
                pod.set_channel(i, cv.kind.to_u8(), cv.raw_value, cv.calibrated_f32);
            }
        }
        pod
    }
}

impl From<TelemetryPacket> for TelemetryBatchPod {
    fn from(packet: TelemetryPacket) -> Self {
        Self::from(&packet)
    }
}

impl From<&TelemetryBatchPod> for TelemetryPacket {
    fn from(pod: &TelemetryBatchPod) -> Self {
        let mut channels = [None; 8];
        for (i, channel) in channels.iter_mut().enumerate() {
            if pod.is_valid(i) {
                let kind =
                    ChannelKind::from_u8(pod.channel_kinds[i]).unwrap_or(ChannelKind::DigitalInput);
                *channel = Some(ChannelValue {
                    channel_id: i as u8,
                    kind,
                    raw_value: pod.raw_values[i],
                    calibrated_f32: pod.calibrated_f32[i],
                });
            }
        }
        TelemetryPacket {
            sequence: 0,
            uptime_ms: 0,
            channels,
            error_flags: 0,
        }
    }
}

impl From<TelemetryBatchPod> for TelemetryPacket {
    fn from(pod: TelemetryBatchPod) -> Self {
        Self::from(&pod)
    }
}

/// Extended command packet sent from host/HMI to edge MCU/PLC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommandPacket {
    Heartbeat {
        sequence: u32,
    },
    SetDigitalOut {
        pin: u8,
        state: bool,
    },
    SetPwm {
        channel: u8,
        duty_cycle: u16,
    },
    SetRegister {
        address: u16,
        value: u16,
    },
    EmergencyStop,
    ResetDevice,
    SyncTime {
        epoch_timestamp_ms: u64,
    },
    SetBaudRate {
        port_id: u8,
        baud_rate: u32,
    },
    ReadRegister {
        address: u16,
    },
    ConfigureChannel {
        channel_id: u8,
        kind: ChannelKind,
        sample_rate_hz: u16,
    },
    CalibrateSensor {
        channel_id: u8,
        zero_offset: f32,
        scale_multiplier: f32,
    },
    CanTransmit {
        id: u32,
        is_extended: bool,
        data: [u8; 8],
        dlc: u8,
    },
}

/// Top-level wire message enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WireMessage {
    Telemetry(TelemetryPacket),
    TelemetryBatch(TelemetryBatchPod),
    Command(CommandPacket),
    Ack {
        sequence: u32,
    },
    Nack {
        sequence: u32,
        error_code: u8,
    },
    RpcRequest {
        request_id: u32,
        method_id: u16,
        payload: [u8; 32],
        payload_len: u8,
    },
    RpcResponse {
        request_id: u32,
        status_code: u16,
        payload: [u8; 32],
        payload_len: u8,
    },
}

/// Errors during encoding, decoding, or framing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    BufferTooSmall,
    SerializationFailed,
    DeserializationFailed,
    FramingError,
    ChecksumMismatch,
    EmptyFrame,
}

/// Encodes a slice into Consistent Overhead Byte Stuffing (COBS).
pub fn cobs_encode(input: &[u8], output: &mut [u8]) -> Result<usize, WireError> {
    let mut read_idx = 0;
    let mut write_idx = 1;
    let mut code_idx = 0;
    let mut code: u8 = 1;

    if output.len() < input.len() + (input.len() / 254) + 1 {
        return Err(WireError::BufferTooSmall);
    }

    while read_idx < input.len() {
        if input[read_idx] == 0 {
            output[code_idx] = code;
            code_idx = write_idx;
            write_idx += 1;
            code = 1;
            read_idx += 1;
        } else {
            output[write_idx] = input[read_idx];
            write_idx += 1;
            read_idx += 1;
            code += 1;
            if code == 0xFF {
                output[code_idx] = code;
                code_idx = write_idx;
                write_idx += 1;
                code = 1;
            }
        }
    }
    output[code_idx] = code;
    Ok(write_idx)
}

/// Decodes a COBS encoded slice.
pub fn cobs_decode(input: &[u8], output: &mut [u8]) -> Result<usize, WireError> {
    if input.is_empty() {
        return Err(WireError::EmptyFrame);
    }

    let mut read_idx = 0;
    let mut write_idx = 0;

    while read_idx < input.len() {
        let code = input[read_idx];
        if code == 0 {
            return Err(WireError::FramingError);
        }
        read_idx += 1;

        let end = read_idx + (code as usize) - 1;
        if end > input.len() {
            return Err(WireError::FramingError);
        }

        while read_idx < end {
            if write_idx >= output.len() {
                return Err(WireError::BufferTooSmall);
            }
            output[write_idx] = input[read_idx];
            write_idx += 1;
            read_idx += 1;
        }

        if code < 0xFF && read_idx < input.len() {
            if write_idx >= output.len() {
                return Err(WireError::BufferTooSmall);
            }
            output[write_idx] = 0;
            write_idx += 1;
        }
    }

    Ok(write_idx)
}

/// Encodes a `WireMessage` into a framed buffer with CRC16 and zero delimiter.
pub fn encode_frame<'a>(msg: &WireMessage, out_buf: &'a mut [u8]) -> Result<&'a [u8], WireError> {
    let mut raw_buf = [0u8; MAX_PAYLOAD_SIZE];
    let serialized =
        postcard::to_slice(msg, &mut raw_buf).map_err(|_| WireError::SerializationFailed)?;

    // Append CRC16
    let checksum = CRC_ALGO.checksum(serialized);
    let mut payload_with_crc = [0u8; MAX_PAYLOAD_SIZE + 2];
    payload_with_crc[..serialized.len()].copy_from_slice(serialized);
    payload_with_crc[serialized.len()..serialized.len() + 2]
        .copy_from_slice(&checksum.to_le_bytes());

    let payload_len = serialized.len() + 2;
    let cobs_len = cobs_encode(&payload_with_crc[..payload_len], out_buf)?;

    if cobs_len >= out_buf.len() {
        return Err(WireError::BufferTooSmall);
    }

    // Trailing 0x00 delimiter
    out_buf[cobs_len] = 0x00;
    Ok(&out_buf[..cobs_len + 1])
}

/// Decodes a raw framed slice (terminated by 0x00) into a `WireMessage`.
pub fn decode_frame(framed: &[u8]) -> Result<WireMessage, WireError> {
    // Strip trailing delimiter if present
    let raw_frame = if let Some((&0x00, rest)) = framed.split_last() {
        rest
    } else {
        framed
    };

    let mut decoded = [0u8; MAX_PAYLOAD_SIZE + 2];
    let len = cobs_decode(raw_frame, &mut decoded)?;
    if len < 2 {
        return Err(WireError::ChecksumMismatch);
    }

    let payload_len = len - 2;
    let expected_crc = u16::from_le_bytes([decoded[payload_len], decoded[payload_len + 1]]);
    let computed_crc = CRC_ALGO.checksum(&decoded[..payload_len]);

    if expected_crc != computed_crc {
        return Err(WireError::ChecksumMismatch);
    }

    postcard::from_bytes(&decoded[..payload_len]).map_err(|_| WireError::DeserializationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cobs_roundtrip() {
        let input = [0x01, 0x00, 0x02, 0x03, 0x00, 0x04];
        let mut encoded = [0u8; 32];
        let enc_len = cobs_encode(&input, &mut encoded).unwrap();

        let mut decoded = [0u8; 32];
        let dec_len = cobs_decode(&encoded[..enc_len], &mut decoded).unwrap();

        assert_eq!(&decoded[..dec_len], &input);
    }

    #[test]
    fn test_wire_message_roundtrip() {
        let mut pkt = TelemetryPacket {
            sequence: 42,
            uptime_ms: 12345,
            ..Default::default()
        };
        pkt.channels[0] = Some(ChannelValue {
            channel_id: 1,
            kind: ChannelKind::AnalogInput,
            raw_value: 1023,
            calibrated_f32: 3.3,
        });

        let msg = WireMessage::Telemetry(pkt.clone());
        let mut frame_buf = [0u8; MAX_FRAMED_SIZE];
        let frame = encode_frame(&msg, &mut frame_buf).unwrap();

        assert_eq!(*frame.last().unwrap(), 0x00);

        let decoded = decode_frame(frame).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_crc_corruption_detection() {
        let msg = WireMessage::Command(CommandPacket::EmergencyStop);
        let mut frame_buf = [0u8; MAX_FRAMED_SIZE];
        let frame = encode_frame(&msg, &mut frame_buf).unwrap();

        let mut corrupted = [0u8; MAX_FRAMED_SIZE];
        corrupted[..frame.len()].copy_from_slice(frame);
        // Corrupt a byte before delimiter
        corrupted[1] ^= 0xFF;

        let result = decode_frame(&corrupted[..frame.len()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_rpc_and_calibration_command_roundtrip() {
        let cmd = WireMessage::Command(CommandPacket::CalibrateSensor {
            channel_id: 3,
            zero_offset: 0.05,
            scale_multiplier: 1.02,
        });
        let mut frame_buf = [0u8; MAX_FRAMED_SIZE];
        let frame = encode_frame(&cmd, &mut frame_buf).unwrap();
        let decoded = decode_frame(frame).unwrap();
        assert_eq!(decoded, cmd);

        let rpc = WireMessage::RpcRequest {
            request_id: 101,
            method_id: 5,
            payload: [0xAA; 32],
            payload_len: 16,
        };
        let frame2 = encode_frame(&rpc, &mut frame_buf).unwrap();
        let decoded2 = decode_frame(frame2).unwrap();
        assert_eq!(decoded2, rpc);
    }

    #[test]
    fn test_telemetry_packet_helpers() {
        let mut pkt = TelemetryPacket::default();
        assert_eq!(pkt.get_channel(0), None);

        let val = ChannelValue {
            channel_id: 2,
            kind: ChannelKind::TemperatureCelsius,
            raw_value: 300,
            calibrated_f32: 24.5,
        };
        assert!(pkt.set_channel(0, val));
        assert_eq!(pkt.get_channel(0), Some(&val));
        assert!(!pkt.set_channel(10, val)); // Out of bounds

        pkt.error_flags = 0x0004;
        assert!(pkt.has_error(0x0004));
        assert!(!pkt.has_error(0x0001));
    }

    #[test]
    fn test_sync_time_and_baud_rate_commands() {
        let cmd = WireMessage::Command(CommandPacket::SyncTime {
            epoch_timestamp_ms: 1725372000000,
        });
        let mut frame_buf = [0u8; MAX_FRAMED_SIZE];
        let frame = encode_frame(&cmd, &mut frame_buf).unwrap();
        let decoded = decode_frame(frame).unwrap();
        assert_eq!(decoded, cmd);

        let baud = WireMessage::Command(CommandPacket::SetBaudRate {
            port_id: 1,
            baud_rate: 115200,
        });
        let frame_baud = encode_frame(&baud, &mut frame_buf).unwrap();
        let decoded_baud = decode_frame(frame_baud).unwrap();
        assert_eq!(decoded_baud, baud);
    }

    #[test]
    fn test_telemetry_batch_pod_geometry() {
        assert_eq!(core::mem::size_of::<TelemetryBatchPod>(), 192);
        assert_eq!(core::mem::align_of::<TelemetryBatchPod>(), 64);
        assert_eq!(core::mem::offset_of!(TelemetryBatchPod, valid_mask), 0);
        assert_eq!(core::mem::offset_of!(TelemetryBatchPod, channel_kinds), 2);
        assert_eq!(core::mem::offset_of!(TelemetryBatchPod, _pad0), 18);
        assert_eq!(core::mem::offset_of!(TelemetryBatchPod, raw_values), 20);
        assert_eq!(core::mem::offset_of!(TelemetryBatchPod, calibrated_f32), 84);
        assert_eq!(core::mem::offset_of!(TelemetryBatchPod, _pad1), 148);
        assert_eq!(core::mem::offset_of!(TelemetryBatchPod, _pad2), 180);
    }

    #[test]
    fn test_telemetry_batch_pod_bytemuck() {
        let mut batch = TelemetryBatchPod::empty();
        batch.set_channel(0, ChannelKind::AnalogInput.to_u8(), 1023, 3.3);
        batch.set_channel(7, ChannelKind::TemperatureCelsius.to_u8(), 250, 25.0);

        let bytes = bytemuck::bytes_of(&batch);
        assert_eq!(bytes.len(), 192);

        let roundtrip: &TelemetryBatchPod = bytemuck::from_bytes(bytes);
        assert_eq!(*roundtrip, batch);
        assert_eq!(
            roundtrip.get_channel(0),
            Some((ChannelKind::AnalogInput.to_u8(), 1023, 3.3))
        );
        assert_eq!(
            roundtrip.get_channel(7),
            Some((ChannelKind::TemperatureCelsius.to_u8(), 250, 25.0))
        );
    }

    #[test]
    fn test_telemetry_batch_pod_slots() {
        let mut batch = TelemetryBatchPod::default();
        assert_eq!(batch.active_channel_count(), 0);
        assert!(!batch.is_valid(0));

        assert!(batch.set_channel(3, ChannelKind::VoltageVolts.to_u8(), 1200, 12.0));
        assert!(batch.set_channel(15, ChannelKind::CurrentAmperes.to_u8(), 500, 5.0));
        assert!(!batch.set_channel(16, 0, 0, 0.0)); // Out of bounds

        assert_eq!(batch.active_channel_count(), 2);
        assert!(batch.is_valid(3));
        assert!(batch.is_valid(15));
        assert!(!batch.is_valid(4));

        assert_eq!(
            batch.get_channel(3),
            Some((ChannelKind::VoltageVolts.to_u8(), 1200, 12.0))
        );
        assert_eq!(batch.sum_calibrated_f32(), 17.0);

        assert!(batch.clear_channel(3));
        assert_eq!(batch.active_channel_count(), 1);
        assert!(!batch.is_valid(3));
    }

    #[test]
    fn test_telemetry_batch_conversion() {
        let mut pkt = TelemetryPacket::default();
        pkt.set_channel(
            0,
            ChannelValue {
                channel_id: 0,
                kind: ChannelKind::TemperatureCelsius,
                raw_value: 300,
                calibrated_f32: 30.0,
            },
        );
        pkt.set_channel(
            4,
            ChannelValue {
                channel_id: 4,
                kind: ChannelKind::PressurePascal,
                raw_value: 101325,
                calibrated_f32: 101.325,
            },
        );

        let batch = TelemetryBatchPod::from(&pkt);
        assert_eq!(batch.active_channel_count(), 2);
        assert!(batch.is_valid(0));
        assert!(batch.is_valid(4));

        let recovered = TelemetryPacket::from(&batch);
        assert_eq!(recovered.get_channel(0), pkt.get_channel(0));
        assert_eq!(recovered.get_channel(4), pkt.get_channel(4));
        assert_eq!(recovered.get_channel(1), None);
    }

    #[test]
    fn test_wire_message_batch_roundtrip() {
        let mut batch = TelemetryBatchPod::empty();
        batch.set_channel(1, ChannelKind::ImuAcceleration.to_u8(), 980, 9.8);
        let msg = WireMessage::TelemetryBatch(batch);

        let mut frame_buf = [0u8; MAX_FRAMED_SIZE];
        let frame = encode_frame(&msg, &mut frame_buf).unwrap();
        assert_eq!(*frame.last().unwrap(), 0x00);

        let decoded = decode_frame(frame).unwrap();
        assert_eq!(decoded, msg);
    }
}
