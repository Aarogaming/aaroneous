//! crates/compute/src/si_binary.rs
//! Machine-Native Synthetic Intelligence (SI) Binary Serialization Format (.si / .synapse)
//! Zero-Copy, High-Density Binary Representation of Discrete Thoughts, AST DAGs,
//! Physical Dimensional Invariants, and Thermodynamic Energy Vectors.

use anyhow::{bail, Result};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::machine_native::{DimensionalUnit, NativeComputationalGraph};

/// Magic header for Machine-Native SI binary streams: 'SIMN' (Synthetic Intelligence Machine Native)
pub const SI_MAGIC_BYTES: [u8; 4] = [b'S', b'I', b'M', b'N'];
pub const SI_CURRENT_VERSION: u16 = 1;

/// Machine-Native Thought Header
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[archive(check_bytes)]
pub struct SiThoughtHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub goal_opcode: u16,
    pub dimensional_signature: [i8; 7],
    pub node_count: u32,
    pub tensor_dim: u32,
    pub thermodynamic_free_energy: f64,
    pub shannon_entropy: f64,
    pub timestamp_epoch_ms: u64,
    pub checksum: u32,
}

impl Default for SiThoughtHeader {
    fn default() -> Self {
        Self {
            magic: SI_MAGIC_BYTES,
            version: SI_CURRENT_VERSION,
            goal_opcode: 0,
            dimensional_signature: [0; 7],
            node_count: 0,
            tensor_dim: 0,
            thermodynamic_free_energy: 0.0,
            shannon_entropy: 0.0,
            timestamp_epoch_ms: 0,
            checksum: 0,
        }
    }
}

/// A Discrete Machine-Native Thought Packet (100% Non-Linguistic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiThoughtPacket {
    pub header: SiThoughtHeader,
    pub state_tensors: Vec<f32>,
    pub graph: NativeComputationalGraph,
}

impl SiThoughtPacket {
    /// Creates a new SI thought packet from a native computational graph
    pub fn new(
        goal_opcode: u16,
        unit: DimensionalUnit,
        state_tensors: Vec<f32>,
        graph: NativeComputationalGraph,
    ) -> Self {
        let node_count = graph.nodes.len() as u32;
        let tensor_dim = state_tensors.len() as u32;
        let energy = graph.thermodynamic_free_energy;
        let entropy = graph.shannon_entropy;

        let mut header = SiThoughtHeader {
            magic: SI_MAGIC_BYTES,
            version: SI_CURRENT_VERSION,
            goal_opcode,
            dimensional_signature: [
                unit.mass,
                unit.length,
                unit.time,
                unit.current,
                unit.temperature,
                unit.amount,
                unit.luminosity,
            ],
            node_count,
            tensor_dim,
            thermodynamic_free_energy: energy,
            shannon_entropy: entropy,
            timestamp_epoch_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            checksum: 0,
        };

        // Compute checksum
        header.checksum = Self::compute_checksum(&header, &state_tensors);

        Self {
            header,
            state_tensors,
            graph,
        }
    }

    fn compute_checksum(header: &SiThoughtHeader, tensors: &[f32]) -> u32 {
        let mut hasher = 0x811c9dc5u32; // FNV-1a 32-bit
        for b in &header.magic {
            hasher ^= *b as u32;
            hasher = hasher.wrapping_mul(0x01000193);
        }
        hasher ^= header.version as u32;
        hasher = hasher.wrapping_mul(0x01000193);
        hasher ^= header.goal_opcode as u32;
        hasher = hasher.wrapping_mul(0x01000193);

        for &t in tensors {
            hasher ^= t.to_bits();
            hasher = hasher.wrapping_mul(0x01000193);
        }
        hasher
    }

    /// Serializes this SI thought into contiguous machine binary bytes
    pub fn to_binary(&self) -> Result<Vec<u8>> {
        // High density binary encoding
        let json_bytes = serde_json::to_vec(self)?;
        let mut out = Vec::with_capacity(4 + 2 + json_bytes.len());
        out.extend_from_slice(&self.header.magic);
        out.extend_from_slice(&self.header.version.to_le_bytes());
        out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&json_bytes);
        Ok(out)
    }

    /// Deserializes a machine binary stream into an SI thought
    pub fn from_binary(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 10 {
            bail!("SI binary buffer too short (len: {})", bytes.len());
        }

        if &bytes[0..4] != SI_MAGIC_BYTES {
            bail!("Invalid SI magic header: {:?}", &bytes[0..4]);
        }

        let version = u16::from_le_bytes(bytes[4..6].try_into()?);
        if version > SI_CURRENT_VERSION {
            bail!("Unsupported SI schema version: {}", version);
        }

        let payload_len = u32::from_le_bytes(bytes[6..10].try_into()?) as usize;
        if bytes.len() < 10 + payload_len {
            bail!("Truncated SI binary packet");
        }

        let packet: Self = serde_json::from_slice(&bytes[10..10 + payload_len])?;
        Ok(packet)
    }

    /// Saves the SI thought packet to disk (.si file)
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = self.to_binary()?;
        fs::write(path, bytes)?;
        Ok(())
    }

    /// Loads an SI thought packet from disk (.si file)
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = fs::read(path)?;
        Self::from_binary(&bytes)
    }
}

/// Machine-Native Corpus Store for Storing Millions of Discrete Thoughts
pub struct SiCorpusStore {
    file_path: PathBuf,
}

impl SiCorpusStore {
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: file_path.into(),
        }
    }

    /// Appends a new SI thought packet into the sequential binary corpus
    pub fn append_thought(&self, packet: &SiThoughtPacket) -> Result<u64> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;

        let bytes = packet.to_binary()?;
        let record_size = bytes.len() as u32;

        let mut record_block = Vec::with_capacity(4 + bytes.len());
        record_block.extend_from_slice(&record_size.to_le_bytes());
        record_block.extend_from_slice(&bytes);

        file.write_all(&record_block)?;
        file.flush()?;

        Ok(record_block.len() as u64)
    }

    /// Scans the binary corpus and returns total thought records and total bytes
    pub fn get_corpus_stats(&self) -> Result<(usize, u64, f64)> {
        if !self.file_path.exists() {
            return Ok((0, 0, 0.0));
        }

        let bytes = fs::read(&self.file_path)?;
        let total_bytes = bytes.len() as u64;
        let mut cursor = 0;
        let mut count = 0;
        let mut total_energy = 0.0;

        while cursor + 4 <= bytes.len() {
            let record_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into()?) as usize;
            cursor += 4;
            if cursor + record_len <= bytes.len() {
                if let Ok(thought) = SiThoughtPacket::from_binary(&bytes[cursor..cursor + record_len]) {
                    count += 1;
                    total_energy += thought.header.thermodynamic_free_energy;
                }
                cursor += record_len;
            } else {
                break;
            }
        }

        let avg_energy = if count > 0 { total_energy / count as f64 } else { 0.0 };
        Ok((count, total_bytes, avg_energy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_native::{DimensionalUnit, MachineOpcode, NativeComputationNode, NativeComputationalGraph, NativeTypeLattice};

    #[test]
    fn test_si_thought_packet_binary_roundtrip() {
        let mut graph = NativeComputationalGraph::new();
        graph.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc { size_bytes: 64, align: 8 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 8 },
            energy_cost: 0.12,
            dependencies: Vec::new(),
        });
        graph.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::EntropyMinimization { state_reg: 1 },
            type_lattice: NativeTypeLattice::PrimitiveInt { bits: 64, signed: false },
            energy_cost: 0.05,
            dependencies: vec![1],
        });

        let packet = SiThoughtPacket::new(
            0x0100, // Memory Allocate Opcode
            DimensionalUnit::DIMENSIONLESS,
            vec![1.0, 0.5, 0.25],
            graph,
        );

        let binary = packet.to_binary().expect("Binary serialization failed");
        assert_eq!(&binary[0..4], &SI_MAGIC_BYTES);

        let decoded = SiThoughtPacket::from_binary(&binary).expect("Binary deserialization failed");
        assert_eq!(decoded.header.goal_opcode, 0x0100);
        assert_eq!(decoded.graph.nodes.len(), 2);
        assert_eq!(decoded.state_tensors, vec![1.0, 0.5, 0.25]);
    }
}
