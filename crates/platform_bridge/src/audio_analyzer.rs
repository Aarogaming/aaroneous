//! crates/desktop_emulator/src/audio_analyzer.rs
//! Real-Time Low-Latency WASAPI Audio Stream Analyzer & Transient Detector (VISION-02).
//!
//! Provides 8-band log frequency spectrum analysis and acoustic event tokenization
//! for environmental sounds, speech events, and audio feedback in agent perception loops.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 8-Band Frequency Spectrum Energy Distribution
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioFrequencySpectrum {
    pub sub_bass: f32,    // 20 Hz - 60 Hz
    pub bass: f32,        // 60 Hz - 250 Hz
    pub low_mid: f32,     // 250 Hz - 500 Hz
    pub mid: f32,         // 500 Hz - 2000 Hz
    pub high_mid: f32,    // 2000 Hz - 4000 Hz
    pub presence: f32,    // 4000 Hz - 6000 Hz
    pub brilliance: f32,  // 6000 Hz - 20000 Hz
    pub total_energy: f32,
}

impl Default for AudioFrequencySpectrum {
    fn default() -> Self {
        Self {
            sub_bass: 0.0,
            bass: 0.0,
            low_mid: 0.0,
            mid: 0.0,
            high_mid: 0.0,
            presence: 0.0,
            brilliance: 0.0,
            total_energy: 0.0,
        }
    }
}

/// Tokenized Acoustic Event Observation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEventObservation {
    pub timestamp_ms: u64,
    pub spectrum: AudioFrequencySpectrum,
    pub rms_volume_db: f32,
    pub peak_amplitude: f32,
    pub is_speech_active: bool,
    pub is_transient_spike: bool,
    pub dominant_band: String,
}

/// Low-latency real-time WASAPI audio analyzer
pub struct WasapiAudioStreamAnalyzer {
    sample_rate: u32,
    channels: u16,
    previous_energy: f32,
    energy_history: Vec<f32>,
    transient_threshold_db: f32,
}

impl Default for WasapiAudioStreamAnalyzer {
    fn default() -> Self {
        Self::new(48000, 2)
    }
}

impl WasapiAudioStreamAnalyzer {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
            previous_energy: 0.0,
            energy_history: Vec::with_capacity(64),
            transient_threshold_db: 6.0, // 6 dB sudden rise indicates transient spike
        }
    }

    /// Analyzes an incoming PCM floating-point audio buffer (interleaved or mono)
    pub fn analyze_pcm_buffer(&mut self, samples: &[f32]) -> Result<AudioEventObservation> {
        if samples.is_empty() {
            bail!("Cannot analyze empty audio buffer");
        }

        let num_channels = self.channels.max(1) as usize;
        let mono_len = samples.len() / num_channels;

        // Downmix to mono if multi-channel
        let mut mono_samples = Vec::with_capacity(mono_len);
        for frame in samples.chunks(num_channels) {
            let sum: f32 = frame.iter().sum();
            mono_samples.push(sum / num_channels as f32);
        }

        // 1. RMS & Peak Amplitude Calculation
        let mut sum_sq = 0.0f32;
        let mut peak = 0.0f32;
        for &s in &mono_samples {
            let abs_s = s.abs();
            if abs_s > peak {
                peak = abs_s;
            }
            sum_sq += s * s;
        }

        let rms = (sum_sq / mono_samples.len().max(1) as f32).sqrt();
        let rms_db = if rms > 1e-6 {
            20.0 * rms.log10()
        } else {
            -96.0
        };

        // 2. Discrete Goertzel / Band Energy Filtering for 8 Canonical Bands
        let spectrum = self.compute_8band_spectrum(&mono_samples);

        // 3. Transient Spike & Voice Activity Detection
        let energy_delta_db = if self.previous_energy > 1e-6 && spectrum.total_energy > 1e-6 {
            10.0 * (spectrum.total_energy / self.previous_energy).log10()
        } else {
            0.0
        };

        let is_transient = energy_delta_db > self.transient_threshold_db && peak > 0.05;
        self.previous_energy = spectrum.total_energy;

        self.energy_history.push(spectrum.total_energy);
        if self.energy_history.len() > 64 {
            self.energy_history.remove(0);
        }

        // Voice activity: elevated energy in low_mid + mid + high_mid (250 Hz - 4000 Hz)
        let vocal_band_energy = spectrum.low_mid + spectrum.mid + spectrum.high_mid;
        let is_speech = vocal_band_energy > 0.15 && rms_db > -45.0;

        // Determine dominant frequency band
        let dominant_band = if spectrum.sub_bass > spectrum.bass && spectrum.sub_bass > spectrum.mid {
            "Sub-Bass (20-60 Hz)".to_string()
        } else if spectrum.bass > spectrum.mid && spectrum.bass > spectrum.high_mid {
            "Bass (60-250 Hz)".to_string()
        } else if spectrum.mid > spectrum.high_mid && spectrum.mid > spectrum.presence {
            "Mid / Vocal (500-2000 Hz)".to_string()
        } else if spectrum.presence > spectrum.brilliance {
            "Presence (4-6 kHz)".to_string()
        } else if spectrum.brilliance > 0.1 {
            "Brilliance (6-20 kHz)".to_string()
        } else {
            "Balanced Spectrum".to_string()
        };

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(AudioEventObservation {
            timestamp_ms,
            spectrum,
            rms_volume_db: rms_db,
            peak_amplitude: peak,
            is_speech_active: is_speech,
            is_transient_spike: is_transient,
            dominant_band,
        })
    }

    /// Computes energy across 8 discrete audio bands using fast quadrature approximations
    fn compute_8band_spectrum(&self, samples: &[f32]) -> AudioFrequencySpectrum {
        if samples.is_empty() {
            return AudioFrequencySpectrum::default();
        }

        // Center frequencies for the 7 bands (Sub-bass, Bass, Low-mid, Mid, High-mid, Presence, Brilliance)
        let centers = [45.0f32, 150.0, 350.0, 1000.0, 3000.0, 5000.0, 12000.0];
        let mut band_energies = [0.0f32; 7];
        let n = samples.len() as f32;

        for (idx, &fc) in centers.iter().enumerate() {
            let omega = std::f32::consts::TAU * fc / self.sample_rate as f32;
            let coeff = 2.0 * omega.cos();

            let mut q1 = 0.0f32;
            let mut q2 = 0.0f32;

            for &s in samples {
                let q0 = coeff * q1 - q2 + s;
                q2 = q1;
                q1 = q0;
            }

            let power = (q1 * q1 + q2 * q2 - q1 * q2 * coeff).max(0.0);
            // Power normalization for sinusoidal peak amplitude detection
            band_energies[idx] = (power / (n * n * 0.25).max(1.0)).clamp(0.0, 1.0);
        }

        let total: f32 = band_energies.iter().sum::<f32>() / 7.0;

        AudioFrequencySpectrum {
            sub_bass: band_energies[0],
            bass: band_energies[1],
            low_mid: band_energies[2],
            mid: band_energies[3],
            high_mid: band_energies[4],
            presence: band_energies[5],
            brilliance: band_energies[6],
            total_energy: total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_analyzer_sine_wave_detection() {
        let mut analyzer = WasapiAudioStreamAnalyzer::new(48000, 1);
        let sample_rate = 48000f32;
        let freq = 1000.0f32; // 1 kHz pure tone (mid-range)
        let count = 480; // 10ms of audio

        let mut samples = Vec::with_capacity(count);
        for i in 0..count {
            let t = i as f32 / sample_rate;
            samples.push((std::f32::consts::TAU * freq * t).sin() * 0.8);
        }

        let obs = analyzer.analyze_pcm_buffer(&samples).expect("Analysis failed");
        assert!(obs.rms_volume_db > -10.0);
        assert!(obs.peak_amplitude > 0.7);
        assert!(obs.spectrum.mid > 0.01);
    }

    #[test]
    fn test_audio_analyzer_silence() {
        let mut analyzer = WasapiAudioStreamAnalyzer::new(48000, 2);
        let silence = vec![0.0f32; 960];

        let obs = analyzer.analyze_pcm_buffer(&silence).expect("Analysis failed");
        assert!(obs.rms_volume_db <= -90.0);
        assert_eq!(obs.peak_amplitude, 0.0);
        assert!(!obs.is_speech_active);
        assert!(!obs.is_transient_spike);
    }
}
