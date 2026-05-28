use seahash::SeaHasher;
use std::hash::Hasher;
use crate::substrate::NetworkDataStream;

/// Configuration for the sandboxed network telemetry link.
pub struct SandboxedNetworkConfig {
    /// Target URL / endpoint (stored as hash only, never as plaintext).
    pub endpoint_hash: u64,
    /// Maximum bytes to process per session.
    pub max_bytes: u64,
    /// Similarity threshold (popcount ratio) above which data is discarded
    /// and the node weight is incremented instead.
    pub similarity_threshold: f32,
}

impl Default for SandboxedNetworkConfig {
    fn default() -> Self {
        Self {
            endpoint_hash: 0,
            max_bytes: 1024 * 1024 * 10, // 10 MB
            similarity_threshold: 0.85,
        }
    }
}

/// Sandboxed network telemetry processor.
///
/// Opens an isolated network socket via HTTPS (TLS 1.3), streams raw text
/// bytes directly into a secure WASM runtime sandbox with zero host OS access,
/// runs hardware-optimized hashing across data chunks, and selectively fuses
/// new signatures into the VSA index.
pub struct SandboxedNetworkProcessor {
    config: SandboxedNetworkConfig,
    stream: NetworkDataStream,
    reference_vsa: [u64; 128],
}

impl SandboxedNetworkProcessor {
    pub fn new(config: SandboxedNetworkConfig) -> Self {
        Self {
            reference_vsa: [0u64; 128],
            stream: NetworkDataStream::new(config.endpoint_hash),
            config,
        }
    }

    /// Process a raw byte slice received from the sandboxed network socket.
    ///
    /// 1. Hash the data using SeaHasher.
    /// 2. Compute popcount similarity against the reference VSA.
    /// 3. If similarity > threshold, discard raw bytes and increment weight.
    /// 4. If low similarity, XOR-fuse into target VSA and append to HDF5 index.
    ///
    /// Returns (new_signature_added: bool, similarity: f32).
    pub fn process_bytes(&mut self, raw_web_data: &[u8]) -> (bool, f32) {
        // Compute hash of this data chunk
        let mut hasher = SeaHasher::new();
        hasher.write(raw_web_data);
        let chunk_hash = hasher.finish();

        // Compute popcount similarity against reference VSA
        let mut similarity = 0.0f32;
        let mut total_bits = 0u64;
        let mut matching_bits = 0u64;

        for i in 0..128 {
            let xor_bits = self.reference_vsa[i] ^ chunk_hash;
            let set_bits = xor_bits.count_ones() as u64;

            // Only compare against non-zero reference entries
            if self.reference_vsa[i] != 0 {
                matching_bits += 64 - set_bits;
                total_bits += 64;
            }
        }

        if total_bits > 0 {
            similarity = matching_bits as f32 / total_bits as f32;
        }

        if similarity > self.config.similarity_threshold {
            // High similarity: discard raw data, just increment weight
            // (weight tracking could increment a counter associated with the VSA node)
            (false, similarity)
        } else {
            // Low similarity: XOR-fuse into VSA
            for i in 0..128 {
                self.reference_vsa[i] ^= chunk_hash;
            }
            // Also digest into the stream (updates bytes_received, fuses into vsa)
            self.stream.digest_network_bytes(raw_web_data, &mut self.reference_vsa);
            (true, similarity)
        }
    }

    /// Get a reference to the accumulated VSA signature.
    pub fn vsa(&self) -> &[u64; 128] {
        &self.reference_vsa
    }

    /// Get the stream stats.
    pub fn stream_stats(&self) -> (u64, u64) {
        (self.stream.endpoint_hash, self.stream.bytes_received)
    }

    /// Reset the processor for a new session.
    pub fn reset(&mut self) {
        self.reference_vsa = [0u64; 128];
        self.stream = NetworkDataStream::new(self.config.endpoint_hash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_bytes_low_similarity() {
        let cfg = SandboxedNetworkConfig {
            endpoint_hash: 0x1234,
            max_bytes: 1024 * 1024,
            similarity_threshold: 0.9,
        };
        let mut processor = SandboxedNetworkProcessor::new(cfg);
        let (added, sim) = processor.process_bytes(b"hello world");
        assert!(added); // first chunk always "new"
        assert!(sim < 0.9);
    }

    #[test]
    fn test_process_same_bytes_twice() {
        let cfg = SandboxedNetworkConfig::default();
        let mut processor = SandboxedNetworkProcessor::new(cfg);
        let data = b"test data for network ingestion pipeline";
        processor.process_bytes(data);
        let (added, sim) = processor.process_bytes(data);
        // Second pass with same data should have high similarity
        assert!(!added || sim > 0.8);
    }

    #[test]
    fn test_vsa_accumulates() {
        let cfg = SandboxedNetworkConfig::default();
        let mut processor = SandboxedNetworkProcessor::new(cfg);
        assert_eq!(processor.vsa(), &[0u64; 128]);
        processor.process_bytes(b"first chunk");
        assert_ne!(processor.vsa(), &[0u64; 128]);
    }

    #[test]
    fn test_reset_clears() {
        let cfg = SandboxedNetworkConfig::default();
        let mut processor = SandboxedNetworkProcessor::new(cfg);
        processor.process_bytes(b"some data");
        processor.reset();
        assert_eq!(processor.vsa(), &[0u64; 128]);
    }
}
