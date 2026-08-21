/// Standard BLE GATT service and characteristic UUIDs
///
/// These are the assigned UUIDs from the Bluetooth SIG. Many wearables
/// (Polar, Wahoo, Garmin, Apple Watch in HR-broadcast mode) implement
/// these standard services, making integration straightforward.
///
/// References:
/// - https://www.bluetooth.com/specifications/assigned-numbers/
use uuid::Uuid;

/// Construct a 128-bit UUID from a 16-bit short ID per the BT SIG base UUID
/// `0000XXXX-0000-1000-8000-00805f9b34fb`
const fn from_u16(short: u16) -> Uuid {
    Uuid::from_u128(0x0000_0000_0000_1000_8000_0080_5f9b_34fb_u128 | ((short as u128) << 96))
}

/// Standard BLE GATT services and characteristics
pub struct StandardServices;

impl StandardServices {
    // === Heart Rate Service (0x180D) ===

    /// Heart Rate Service UUID
    pub fn heart_rate_service() -> Uuid {
        from_u16(0x180D)
    }

    /// Heart Rate Measurement characteristic (notify)
    /// Format: 1 byte flags + 1-2 bytes HR + optional RR intervals
    pub fn heart_rate_measurement() -> Uuid {
        from_u16(0x2A37)
    }

    /// Body Sensor Location (read)
    pub fn body_sensor_location() -> Uuid {
        from_u16(0x2A38)
    }

    // === Battery Service (0x180F) ===

    /// Battery Service UUID
    pub fn battery_service() -> Uuid {
        from_u16(0x180F)
    }

    /// Battery Level characteristic (read, notify)
    /// Format: 1 byte 0-100%
    pub fn battery_level() -> Uuid {
        from_u16(0x2A19)
    }

    // === Device Information Service (0x180A) ===

    /// Device Information Service UUID
    pub fn device_information_service() -> Uuid {
        from_u16(0x180A)
    }

    /// Manufacturer Name String
    pub fn manufacturer_name() -> Uuid {
        from_u16(0x2A29)
    }

    /// Model Number String
    pub fn model_number() -> Uuid {
        from_u16(0x2A24)
    }

    /// Firmware Revision String
    pub fn firmware_revision() -> Uuid {
        from_u16(0x2A26)
    }

    // === Pulse Oximeter Service (0x1822) ===

    /// Pulse Oximeter Service
    pub fn pulse_oximeter_service() -> Uuid {
        from_u16(0x1822)
    }

    /// Pulse Oximeter Spot-Check Measurement
    pub fn plx_spot_check_measurement() -> Uuid {
        from_u16(0x2A5E)
    }

    /// Pulse Oximeter Continuous Measurement
    pub fn plx_continuous_measurement() -> Uuid {
        from_u16(0x2A5F)
    }
}

/// Parse a Heart Rate Measurement notification payload per BT spec
///
/// Format (per Bluetooth GATT Specification Supplement):
/// - Byte 0: Flags
///   - Bit 0: HR Value Format (0 = uint8, 1 = uint16)
///   - Bit 1-2: Sensor Contact Status
///   - Bit 3: Energy Expended Present
///   - Bit 4: RR-Interval Present
/// - Bytes 1-2: Heart rate (uint8 or uint16 depending on flag)
/// - Variable trailing: energy expended (uint16) and/or RR intervals (uint16 list)
pub fn parse_heart_rate_measurement(data: &[u8]) -> Result<HeartRateData, ParseError> {
    if data.is_empty() {
        return Err(ParseError::Empty);
    }

    let flags = data[0];
    let hr_format_uint16 = (flags & 0x01) != 0;
    let energy_present = (flags & 0x08) != 0;
    let rr_present = (flags & 0x10) != 0;

    let mut idx = 1;
    let bpm = if hr_format_uint16 {
        if data.len() < 3 {
            return Err(ParseError::Truncated);
        }
        let v = u16::from_le_bytes([data[idx], data[idx + 1]]);
        idx += 2;
        v
    } else {
        if data.len() < 2 {
            return Err(ParseError::Truncated);
        }
        let v = data[idx] as u16;
        idx += 1;
        v
    };

    let energy_expended = if energy_present {
        if data.len() < idx + 2 {
            return Err(ParseError::Truncated);
        }
        let v = u16::from_le_bytes([data[idx], data[idx + 1]]);
        idx += 2;
        Some(v)
    } else {
        None
    };

    let mut rr_intervals = Vec::new();
    if rr_present {
        // Each RR interval is 2 bytes, units of 1/1024 second
        while idx + 1 < data.len() {
            let v = u16::from_le_bytes([data[idx], data[idx + 1]]);
            rr_intervals.push(v);
            idx += 2;
        }
    }

    Ok(HeartRateData {
        bpm,
        energy_expended,
        rr_intervals,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeartRateData {
    /// Heart rate in beats per minute
    pub bpm: u16,
    /// Energy expended in kilojoules (if present)
    pub energy_expended: Option<u16>,
    /// RR intervals in units of 1/1024 second (used for HRV calculation)
    pub rr_intervals: Vec<u16>,
}

impl HeartRateData {
    /// Calculate RMSSD-based HRV from RR intervals
    /// (Root Mean Square of Successive Differences)
    /// Returns HRV in milliseconds, or None if fewer than 2 RR intervals.
    pub fn rmssd_ms(&self) -> Option<f64> {
        if self.rr_intervals.len() < 2 {
            return None;
        }

        // Convert from 1/1024 second units to milliseconds: rr * 1000.0 / 1024.0
        let rr_ms: Vec<f64> = self
            .rr_intervals
            .iter()
            .map(|&r| (r as f64) * 1000.0 / 1024.0)
            .collect();

        let sum_sq_diff: f64 = rr_ms.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();

        let n = (rr_ms.len() - 1) as f64;
        Some((sum_sq_diff / n).sqrt())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("Empty payload")]
    Empty,
    #[error("Payload truncated")]
    Truncated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_construction() {
        let hr = StandardServices::heart_rate_service();
        assert_eq!(hr.to_string(), "0000180d-0000-1000-8000-00805f9b34fb");

        let battery = StandardServices::battery_service();
        assert_eq!(battery.to_string(), "0000180f-0000-1000-8000-00805f9b34fb");

        let hr_meas = StandardServices::heart_rate_measurement();
        assert_eq!(hr_meas.to_string(), "00002a37-0000-1000-8000-00805f9b34fb");
    }

    #[test]
    fn test_parse_heart_rate_uint8() {
        // Flags = 0x00 (uint8 format), HR = 72
        let data = vec![0x00, 72];
        let parsed = parse_heart_rate_measurement(&data).unwrap();
        assert_eq!(parsed.bpm, 72);
        assert!(parsed.energy_expended.is_none());
        assert!(parsed.rr_intervals.is_empty());
    }

    #[test]
    fn test_parse_heart_rate_uint16() {
        // Flags = 0x01 (uint16 format), HR = 0x0078 = 120
        let data = vec![0x01, 0x78, 0x00];
        let parsed = parse_heart_rate_measurement(&data).unwrap();
        assert_eq!(parsed.bpm, 120);
    }

    #[test]
    fn test_parse_heart_rate_with_rr() {
        // Flags = 0x10 (RR present), HR = 60, RR = [800, 850]
        // Note: RR is little-endian uint16, units of 1/1024 sec
        let data = vec![0x10, 60, 0x20, 0x03, 0x52, 0x03];
        let parsed = parse_heart_rate_measurement(&data).unwrap();
        assert_eq!(parsed.bpm, 60);
        assert_eq!(parsed.rr_intervals.len(), 2);
        assert_eq!(parsed.rr_intervals[0], 0x0320); // 800
        assert_eq!(parsed.rr_intervals[1], 0x0352); // 850
    }

    #[test]
    fn test_parse_heart_rate_with_energy() {
        // Flags = 0x08 (energy present), HR = 80, Energy = 0x0064 = 100
        let data = vec![0x08, 80, 0x64, 0x00];
        let parsed = parse_heart_rate_measurement(&data).unwrap();
        assert_eq!(parsed.bpm, 80);
        assert_eq!(parsed.energy_expended, Some(100));
    }

    #[test]
    fn test_parse_heart_rate_empty() {
        let result = parse_heart_rate_measurement(&[]);
        assert!(matches!(result, Err(ParseError::Empty)));
    }

    #[test]
    fn test_parse_heart_rate_truncated() {
        // uint16 format flag but only 1 byte
        let result = parse_heart_rate_measurement(&[0x01, 0x78]);
        assert!(matches!(result, Err(ParseError::Truncated)));
    }

    #[test]
    fn test_rmssd_calculation() {
        // RR intervals in 1/1024 sec: 1024, 1024 = 1000ms each, no variability
        let data = HeartRateData {
            bpm: 60,
            energy_expended: None,
            rr_intervals: vec![1024, 1024, 1024],
        };
        let hrv = data.rmssd_ms().unwrap();
        assert!(hrv.abs() < 0.01, "expected ~0, got {}", hrv);

        // With variability
        let data = HeartRateData {
            bpm: 60,
            energy_expended: None,
            rr_intervals: vec![1024, 1126], // ~1000ms then ~1100ms
        };
        let hrv = data.rmssd_ms().unwrap();
        // Difference is (1126-1024)*1000/1024 ≈ 99.6ms, sqrt(99.6^2 / 1) ≈ 99.6
        assert!((99.0..101.0).contains(&hrv), "expected ~99.6, got {}", hrv);
    }

    #[test]
    fn test_rmssd_insufficient_data() {
        let data = HeartRateData {
            bpm: 60,
            energy_expended: None,
            rr_intervals: vec![1024],
        };
        assert!(data.rmssd_ms().is_none());
    }
}
