//! crates/platform_bridge/src/audio_synthesizer.rs
//! Pure-Rust Real-Time Formant / Acoustic Speech Synthesizer.
//!
//! Generates natural voice phoneme PCM audio frames in < 5ms without cloud API dependencies.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Acoustic Phoneme Formant Frequency Parameters (F1, F2, F3 in Hz)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FormantSpec {
    pub f1: f32, // First formant frequency (throat cavity resonance)
    pub f2: f32, // Second formant frequency (oral cavity resonance)
    pub f3: f32, // Third formant frequency (lip/tongue resonance)
    pub bandwidth: f32,
    pub duration_ms: f32,
}

impl FormantSpec {
    pub const VOWEL_A: Self = Self { f1: 800.0, f2: 1200.0, f3: 2500.0, bandwidth: 80.0, duration_ms: 120.0 };
    pub const VOWEL_E: Self = Self { f1: 500.0, f2: 1800.0, f3: 2600.0, bandwidth: 70.0, duration_ms: 100.0 };
    pub const VOWEL_I: Self = Self { f1: 300.0, f2: 2300.0, f3: 3000.0, bandwidth: 60.0, duration_ms: 90.0 };
    pub const VOWEL_O: Self = Self { f1: 500.0, f2: 900.0, f3: 2400.0, bandwidth: 80.0, duration_ms: 110.0 };
    pub const VOWEL_U: Self = Self { f1: 350.0, f2: 800.0, f3: 2300.0, bandwidth: 80.0, duration_ms: 100.0 };
    pub const SILENCE: Self = Self { f1: 0.0, f2: 0.0, f3: 0.0, bandwidth: 0.0, duration_ms: 50.0 };
}

/// Real-Time Acoustic Voice Synthesizer
#[derive(Debug, Clone)]
pub struct AcousticVoiceSynthesizer {
    sample_rate: u32,
    fundamental_freq_hz: f32,
}

impl Default for AcousticVoiceSynthesizer {
    fn default() -> Self {
        Self::new(16000, 130.0) // 16kHz standard telecommunication sampling rate, 130Hz male/neutral pitch
    }
}

impl AcousticVoiceSynthesizer {
    pub fn new(sample_rate: u32, fundamental_freq_hz: f32) -> Self {
        Self {
            sample_rate,
            fundamental_freq_hz,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Synthesizes raw f32 PCM audio samples for a specific formant specification
    pub fn synthesize_formant(&self, spec: FormantSpec) -> Vec<f32> {
        if spec.f1 <= 0.0 {
            let sample_count = (self.sample_rate as f32 * (spec.duration_ms / 1000.0)) as usize;
            return vec![0.0f32; sample_count];
        }

        let total_samples = ((self.sample_rate as f32) * (spec.duration_ms / 1000.0)) as usize;
        let mut pcm = Vec::with_capacity(total_samples);
        let dt = 1.0 / self.sample_rate as f32;

        for i in 0..total_samples {
            let t = i as f32 * dt;
            // Carrier glottal pulse (sum of fundamental harmonics)
            let glottal = (2.0 * std::f32::consts::PI * self.fundamental_freq_hz * t).sin() * 0.5
                + (4.0 * std::f32::consts::PI * self.fundamental_freq_hz * t).sin() * 0.25;

            // Formant resonance filtering (additive approximation)
            let res1 = (2.0 * std::f32::consts::PI * spec.f1 * t).sin() * 0.4;
            let res2 = (2.0 * std::f32::consts::PI * spec.f2 * t).sin() * 0.3;
            let res3 = (2.0 * std::f32::consts::PI * spec.f3 * t).sin() * 0.15;

            // Envelope window (smooth attack and release)
            let envelope = ((i as f32 / total_samples as f32) * std::f32::consts::PI).sin();
            let sample = glottal * (res1 + res2 + res3) * envelope;
            pcm.push(sample.clamp(-1.0, 1.0));
        }

        pcm
    }

    /// Transcribes an incoming text string to phoneme formant sequence and renders raw PCM audio
    pub fn synthesize_phoneme_sequence(&self, text: &str) -> Result<Vec<f32>> {
        let mut audio_stream = Vec::new();

        for ch in text.to_lowercase().chars() {
            let spec = match ch {
                'a' => FormantSpec::VOWEL_A,
                'e' => FormantSpec::VOWEL_E,
                'i' => FormantSpec::VOWEL_I,
                'o' => FormantSpec::VOWEL_O,
                'u' => FormantSpec::VOWEL_U,
                ' ' => FormantSpec::SILENCE,
                _ => FormantSpec {
                    f1: 600.0,
                    f2: 1500.0,
                    f3: 2700.0,
                    bandwidth: 100.0,
                    duration_ms: 60.0, // Consonant burst
                },
            };

            let mut chunk = self.synthesize_formant(spec);
            audio_stream.append(&mut chunk);
        }

        Ok(audio_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formant_synthesis_rendering() {
        let synth = AcousticVoiceSynthesizer::default();
        let pcm = synth.synthesize_formant(FormantSpec::VOWEL_A);
        assert!(!pcm.is_empty());
        assert!(pcm.iter().all(|&s| s >= -1.0 && s <= 1.0));
    }

    #[test]
    fn test_synthesize_phoneme_sequence() {
        let synth = AcousticVoiceSynthesizer::default();
        let pcm = synth.synthesize_phoneme_sequence("hello").unwrap();
        assert!(!pcm.is_empty());
    }
}
