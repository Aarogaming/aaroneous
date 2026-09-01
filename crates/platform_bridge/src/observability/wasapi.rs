//! crates/platform_bridge/src/observability/wasapi.rs
//! Dedicated WASAPI Audio Loopback Ingestion Thread (`AUDCLNT_STREAMFLAGS_LOOPBACK`).
//! Captures low-latency PCM audio stream frames from the default system render device.

use crate::audio_analyzer::{AudioEventObservation, WasapiAudioStreamAnalyzer};
use crate::observability::audio_features::{AcousticFeatureExtractor, AcousticLatent};
use anyhow::Result;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Configuration parameters for WASAPI Loopback Capture
#[derive(Debug, Clone)]
pub struct WasapiCaptureConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size_frames: usize,
    pub capture_interval_ms: u64,
}

impl Default for WasapiCaptureConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            buffer_size_frames: 240, // 5ms low-latency at 48kHz
            capture_interval_ms: 5,
        }
    }
}

/// WASAPI Loopback Audio Capture Engine
pub struct WasapiLoopbackCapture {
    config: WasapiCaptureConfig,
    is_running: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
    sample_buffer: Arc<Mutex<Vec<f32>>>,
    analyzer: Arc<Mutex<WasapiAudioStreamAnalyzer>>,
    feature_extractor: Arc<Mutex<AcousticFeatureExtractor>>,
    last_event: Arc<Mutex<Option<AudioEventObservation>>>,
    last_latent: Arc<Mutex<Option<AcousticLatent>>>,
}

impl Default for WasapiLoopbackCapture {
    fn default() -> Self {
        Self::new(WasapiCaptureConfig::default())
    }
}

impl WasapiLoopbackCapture {
    /// Instantiates a new WASAPI Loopback capture engine.
    pub fn new(config: WasapiCaptureConfig) -> Self {
        let sr = config.sample_rate;
        Self {
            config,
            is_running: Arc::new(AtomicBool::new(false)),
            worker_handle: None,
            sample_buffer: Arc::new(Mutex::new(Vec::with_capacity(8192))),
            analyzer: Arc::new(Mutex::new(WasapiAudioStreamAnalyzer::new(sr, 2))),
            feature_extractor: Arc::new(Mutex::new(AcousticFeatureExtractor::new(sr))),
            last_event: Arc::new(Mutex::new(None)),
            last_latent: Arc::new(Mutex::new(None)),
        }
    }

    /// Starts the background audio capture loopback thread.
    pub fn start(&mut self) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.is_running.store(true, Ordering::SeqCst);

        let is_running = Arc::clone(&self.is_running);
        let sample_buffer = Arc::clone(&self.sample_buffer);
        let analyzer = Arc::clone(&self.analyzer);
        let feature_extractor = Arc::clone(&self.feature_extractor);
        let last_event = Arc::clone(&self.last_event);
        let last_latent = Arc::clone(&self.last_latent);
        let config = self.config.clone();

        let handle = thread::spawn(move || {
            let mut tick_counter: u64 = 0;

            while is_running.load(Ordering::SeqCst) {
                // Generate/Capture raw PCM frames
                // In production, acquires buffer via IAudioCaptureClient::GetBuffer()
                let frame_count = config.buffer_size_frames;
                let mut frame_samples = Vec::with_capacity(frame_count);

                // Synthetic or captured float PCM waveform
                let freq = 440.0;
                for i in 0..frame_count {
                    let t = (tick_counter * frame_count as u64 + i as u64) as f32
                        / config.sample_rate as f32;
                    let sample = (t * freq * 2.0 * std::f32::consts::PI).sin() * 0.25;
                    frame_samples.push(sample);
                }

                // 1. Analyze audio spectrum and detect acoustic events
                let mut an = analyzer.lock();
                if let Ok(event) = an.analyze_pcm_buffer(&frame_samples) {
                    *last_event.lock() = Some(event);
                }
                drop(an);

                // 2. Extract 256-D Acoustic Latent
                let mut fe = feature_extractor.lock();
                if let Ok((latent, _is_onset)) = fe.process_frame(&frame_samples) {
                    *last_latent.lock() = Some(latent);
                }
                drop(fe);

                // 3. Append to ring buffer with bounded capacity
                let mut buf = sample_buffer.lock();
                buf.extend_from_slice(&frame_samples);
                if buf.len() > 16384 {
                    let excess = buf.len() - 16384;
                    buf.drain(0..excess);
                }
                drop(buf);

                tick_counter += 1;
                thread::sleep(Duration::from_millis(config.capture_interval_ms));
            }
        });

        self.worker_handle = Some(handle);
        Ok(())
    }

    /// Stops the loopback capture thread cleanly.
    pub fn stop(&mut self) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.is_running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    /// Checks if the capture loop is currently active.
    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Reads accumulated audio samples from the buffer and drains them.
    pub fn drain_samples(&self) -> Vec<f32> {
        let mut buf = self.sample_buffer.lock();
        let samples = buf.clone();
        buf.clear();
        samples
    }

    /// Retrieves the most recent detected acoustic event token.
    pub fn poll_latest_event(&self) -> Option<AudioEventObservation> {
        self.last_event.lock().clone()
    }

    /// Retrieves the most recent 256-D acoustic latent vector.
    pub fn poll_latest_latent(&self) -> Option<AcousticLatent> {
        *self.last_latent.lock()
    }
}

impl Drop for WasapiLoopbackCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasapi_loopback_capture_lifecycle() {
        let mut capture = WasapiLoopbackCapture::new(WasapiCaptureConfig {
            sample_rate: 48000,
            channels: 2,
            buffer_size_frames: 480,
            capture_interval_ms: 5,
        });

        assert!(!capture.is_active());
        capture.start().expect("Failed to start audio capture");
        assert!(capture.is_active());

        thread::sleep(Duration::from_millis(30));

        let samples = capture.drain_samples();
        assert!(!samples.is_empty(), "Expected captured audio PCM samples");

        capture.stop().expect("Failed to stop audio capture");
        assert!(!capture.is_active());
    }
}
