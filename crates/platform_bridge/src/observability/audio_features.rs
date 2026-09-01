//! crates/platform_bridge/src/observability/audio_features.rs
//! Real-Time Acoustic Feature Extractor, STFT Spectral Flux & 256-D Latent Projector.
//! Transforms raw PCM audio loopback frames into normalized 256-dimensional acoustic vectors.

use anyhow::{bail, Result};
use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const LATENT_DIM: usize = 256;
pub const FFT_SIZE: usize = 512;

/// A normalized 256-dimensional acoustic latent vector
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcousticLatent(pub [f32; LATENT_DIM]);

impl Serialize for AcousticLatent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AcousticLatent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<f32>::deserialize(deserializer)?;
        let mut arr = [0.0f32; LATENT_DIM];
        let copy_len = vec.len().min(LATENT_DIM);
        arr[..copy_len].copy_from_slice(&vec[..copy_len]);
        Ok(AcousticLatent(arr))
    }
}

impl Default for AcousticLatent {
    fn default() -> Self {
        Self([0.0; LATENT_DIM])
    }
}

impl AcousticLatent {
    pub fn new(vector: [f32; LATENT_DIM]) -> Self {
        Self(vector)
    }

    pub fn as_slice(&self) -> &[f32; LATENT_DIM] {
        &self.0
    }
}

/// Real-time Short-Time Fourier Transform (STFT) & Acoustic Feature Extractor
pub struct AcousticFeatureExtractor {
    sample_rate: u32,
    planner: FftPlanner<f32>,
    previous_spectrum: Vec<f32>,
    onset_threshold: f32,
    window: Vec<f32>,
}

impl Default for AcousticFeatureExtractor {
    fn default() -> Self {
        Self::new(48000)
    }
}

impl AcousticFeatureExtractor {
    pub fn new(sample_rate: u32) -> Self {
        // Pre-compute Hann window
        let mut window = Vec::with_capacity(FFT_SIZE);
        for i in 0..FFT_SIZE {
            let val = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos());
            window.push(val);
        }

        Self {
            sample_rate,
            planner: FftPlanner::new(),
            previous_spectrum: vec![0.0; LATENT_DIM],
            onset_threshold: 0.15,
            window,
        }
    }

    /// Processes PCM float audio samples, computes STFT, detects onsets, and produces a 256-D latent.
    pub fn process_frame(&mut self, pcm_samples: &[f32]) -> Result<(AcousticLatent, bool)> {
        if pcm_samples.is_empty() {
            bail!("Cannot process empty PCM frame");
        }

        // Prepare FFT input buffer with Hann windowing
        let mut buffer: Vec<Complex<f32>> = Vec::with_capacity(FFT_SIZE);
        for i in 0..FFT_SIZE {
            let sample = if i < pcm_samples.len() {
                pcm_samples[i] * self.window[i]
            } else {
                0.0
            };
            buffer.push(Complex { re: sample, im: 0.0 });
        }

        // Execute forward FFT
        let fft = self.planner.plan_fft_forward(FFT_SIZE);
        fft.process(&mut buffer);

        // Compute magnitude spectrum for the first 256 bins
        let mut current_spectrum = [0.0f32; LATENT_DIM];
        let mut spectral_flux = 0.0f32;
        let mut total_power = 0.0f32;

        for i in 0..LATENT_DIM {
            let mag = (buffer[i].re * buffer[i].re + buffer[i].im * buffer[i].im).sqrt();
            current_spectrum[i] = mag;
            total_power += mag * mag;

            let prev = self.previous_spectrum[i];
            let diff = mag - prev;
            if diff > 0.0 {
                spectral_flux += diff;
            }
            self.previous_spectrum[i] = mag;
        }

        // Onset Detection: Peak in positive spectral flux above threshold
        let is_onset = spectral_flux > self.onset_threshold && total_power > 1e-4;

        // Normalize 256-D spectrum into unit Euclidean sphere (L2 norm)
        let norm = (total_power + 1e-8).sqrt();
        let mut latent = [0.0f32; LATENT_DIM];
        for i in 0..LATENT_DIM {
            latent[i] = current_spectrum[i] / norm;
        }

        Ok((AcousticLatent(latent), is_onset))
    }

    /// Updates onset detection sensitivity threshold
    pub fn set_onset_threshold(&mut self, threshold: f32) {
        self.onset_threshold = threshold.max(0.01);
    }

    /// Returns the configured audio sample rate (Hz)
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acoustic_feature_extractor_stft_and_latent_projection() {
        let mut extractor = AcousticFeatureExtractor::new(48000);

        // Synthetic 440Hz Sine Wave PCM
        let mut pcm = Vec::with_capacity(FFT_SIZE);
        for i in 0..FFT_SIZE {
            let t = i as f32 / 48000.0;
            let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
            pcm.push(sample);
        }

        let (latent, _is_onset) = extractor.process_frame(&pcm).expect("Frame processing failed");
        assert_eq!(latent.0.len(), 256);

        // Verify L2 normalization
        let norm: f32 = latent.0.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-3, "Latent should be unit-normalized");
    }
}
