use anyhow::{Result, anyhow, bail};
use byteorder::{LittleEndian, ReadBytesExt};
/// GGUF Genome Compiler — extracts FFN decision geometries from GGUF models
/// and compiles them into the universal 2-bit genome format.
///
/// This is the Rust-native port of `GGUF_HARVESTER.py` + `HELIX_COMPILER.py`.
/// It reads GGUF files directly from disk, dequantizes FFN tensors, maps weights
/// to 2-bit genomic states (A/T/C/G), packs them into u32 voxels, and organizes
/// the result into 16 parallel tracks.
///
/// # Output Format
///
/// ```text
/// [5]   magic       = "AASv1"
/// [8]   voxel_count (u64 LE)
/// [8]   weight_count (u64 LE)
/// [4]   num_tracks  (u32 LE)
/// [8×N] track_sizes (u64 LE each)
/// [...] packed voxels (u32 LE, 16 x 2-bit per voxel)
/// ```
///
/// # Usage
///
/// ```rust,ignore
/// use std::path::PathBuf;
/// use a_run::genome_compiler::{GenomeCompiler, CompileConfig};
///
/// let config = CompileConfig {
///     input: PathBuf::from("models/my-model.gguf"),
///     output: PathBuf::from("chromosomes/my_genome.bin"),
///     num_tracks: 16,
///     ..Default::default()
/// };
/// let mut compiler = GenomeCompiler::new(config);
/// compiler.compile().unwrap();
/// ```
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

// ────────────────────────────────────────────────────────────────────
// GGUF format constants
// ────────────────────────────────────────────────────────────────────

const GGUF_MAGIC: &[u8; 4] = b"GGUF";
const _GGUF_VERSION: u32 = 3;

/// GGML tensor data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
#[allow(non_camel_case_types)]
pub enum GgmlType {
    F32 = 0,
    F16 = 1,
    Q4_0 = 2,
    Q4_1 = 3,
    Q5_0 = 6,
    Q5_1 = 7,
    Q8_0 = 8,
    Q8_1 = 9,
    Q2_K = 10,
    Q3_K = 11,
    Q4_K = 12,
    Q5_K = 13,
    Q6_K = 14,
    Q8_K = 15,
    IQ2_XXS = 16,
    IQ2_XS = 17,
    IQ3_XXS = 18,
    IQ1_S = 19,
    IQ4_NL = 20,
    IQ3_S = 21,
    IQ2_S = 22,
    IQ4_XS = 23,
    I8 = 24,
    I16 = 25,
    I32 = 26,
    I64 = 27,
    F64 = 28,
    IQ1_M = 29,
    BF16 = 30,
    MXFP4 = 39,
}

impl GgmlType {
    fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::F32,
            1 => Self::F16,
            2 => Self::Q4_0,
            3 => Self::Q4_1,
            6 => Self::Q5_0,
            7 => Self::Q5_1,
            8 => Self::Q8_0,
            9 => Self::Q8_1,
            10 => Self::Q2_K,
            11 => Self::Q3_K,
            12 => Self::Q4_K,
            13 => Self::Q5_K,
            14 => Self::Q6_K,
            15 => Self::Q8_K,
            24 => Self::I8,
            25 => Self::I16,
            26 => Self::I32,
            27 => Self::I64,
            28 => Self::F64,
            30 => Self::BF16,
            39 => Self::MXFP4,
            _ => Self::F32,
        }
    }

    fn bytes_per_element(self, n_elements: u64) -> u64 {
        match self {
            Self::F32 | Self::I32 => 4,
            Self::F16 | Self::BF16 | Self::I16 => 2,
            Self::F64 | Self::I64 => 8,
            Self::I8 => 1,
            Self::Q4_0 => n_elements / 2,
            Self::Q4_1 => (n_elements / 2) + (n_elements / 32) * 4,
            Self::Q8_0 | Self::Q8_1 => n_elements,
            Self::Q2_K => n_elements / 256 * 84,
            Self::Q3_K => n_elements / 32 * 30,
            Self::Q4_K => n_elements / 256 * 144,
            Self::Q5_K => n_elements / 256 * 176,
            Self::Q6_K => n_elements / 256 * 208,
            Self::MXFP4 => n_elements / 2,
            _ => n_elements,
        }
    }
}

/// GGUF metadata value types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GgufMetaType {
    Uint8 = 0,
    Int8 = 1,
    Uint16 = 2,
    Int16 = 3,
    Uint32 = 4,
    Int32 = 5,
    Float32 = 6,
    Bool = 7,
    String = 8,
    Array = 9,
    Uint64 = 10,
    Int64 = 11,
    Float64 = 12,
}

// ────────────────────────────────────────────────────────────────────
// 2-bit genome encoding
// ────────────────────────────────────────────────────────────────────

/// Quantization thresholds for mapping float weights to 2-bit states.
/// A=00 (-2.5), T=01 (-0.5), C=10 (0.5), G=11 (2.5)
const QUANT_THRESHOLDS: [f32; 3] = [-1.0, 0.0, 1.0];

/// Map a normalized float to a 2-bit genomic state.
#[inline]
fn float_to_2bit(v: f32) -> u8 {
    if v < QUANT_THRESHOLDS[0] {
        0b00 // A
    } else if v < QUANT_THRESHOLDS[1] {
        0b01 // T
    } else if v < QUANT_THRESHOLDS[2] {
        0b10 // C
    } else {
        0b11 // G
    }
}

/// Map a 2-bit genomic state back to a representative float (midpoint of range).
#[inline]
fn bit_to_float(bit: u8) -> f32 {
    match bit & 0x03 {
        0b00 => -1.5, // A: range [-2.5, -1.0)
        0b01 => -0.5, // T: range [-1.0,  0.0)
        0b10 => 0.5,  // C: range [ 0.0,  1.0)
        0b11 => 1.5,  // G: range [ 1.0,  2.5]
        _ => 0.0,
    }
}

/// Unpack u32 voxels back to 2-bit values (16 values per voxel).
fn unpack_2bit_array(voxels: &[u32], count: usize) -> Vec<u8> {
    let mut values = Vec::with_capacity(count);
    for &voxel in voxels {
        for j in 0..16 {
            if values.len() >= count {
                return values;
            }
            values.push(((voxel >> (j * 2)) & 0x03) as u8);
        }
    }
    values
}

/// Pack an array of 2-bit values into u32 voxels (16 values per u32).
fn pack_2bit_array(values: &[u8]) -> Vec<u32> {
    let remainder = values.len() % 16;
    let padded_len = if remainder != 0 {
        values.len() + 16 - remainder
    } else {
        values.len()
    };
    let mut padded = vec![0u8; padded_len];
    padded[..values.len()].copy_from_slice(values);

    let num_voxels = padded_len / 16;
    let mut voxels = vec![0u32; num_voxels];

    for (i, chunk) in padded.chunks(16).enumerate() {
        let mut voxel = 0u32;
        for (j, &val) in chunk.iter().enumerate() {
            voxel |= (val as u32) << (j * 2);
        }
        voxels[i] = voxel;
    }

    voxels
}

// ────────────────────────────────────────────────────────────────────
// GGUF structures
// ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GgufTensor {
    pub name: String,
    pub shape: Vec<u64>,
    pub ggml_type: GgmlType,
    pub n_elements: u64,
    pub n_bytes: u64,
    pub offset: u64,
}

#[derive(Debug)]
pub struct GgufHeader {
    pub version: u32,
    pub tensor_count: u64,
    pub metadata_kv_count: u64,
}

/// Result of genome binary verification.
#[derive(Debug, Clone)]
pub struct GenomeVerification {
    pub voxel_count: u64,
    pub weight_count: u64,
    pub num_tracks: usize,
    pub track_sizes: Vec<u64>,
    pub checksum: u32,
    pub bit_counts: [u64; 4],
    pub entropy: f64,
    pub file_size: u64,
    pub header_valid: bool,
    pub voxel_count_valid: bool,
    pub has_trailing_data: bool,
}

// ────────────────────────────────────────────────────────────────────
// Configuration
// ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CompileConfig {
    /// Path to input GGUF file or directory of GGUF files
    pub input: PathBuf,
    /// Path to output genome binary
    pub output: PathBuf,
    /// Number of parallel tracks (default: 16)
    pub num_tracks: usize,
    /// If true, only extract FFN tensors (ffn_gate, ffn_down, ffn_up, ffn_norm)
    pub ffn_only: bool,
    /// Maximum elements per tensor to process (skip larger tensors)
    pub max_elements: u64,
}

impl Default for CompileConfig {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output: PathBuf::from("chromosomes/universal_gaming_core.bin"),
            num_tracks: 16,
            ffn_only: true,
            max_elements: 100_000_000,
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Genome Compiler
// ────────────────────────────────────────────────────────────────────

pub struct GenomeCompiler {
    config: CompileConfig,
}

impl GenomeCompiler {
    pub fn new(config: CompileConfig) -> Self {
        Self { config }
    }

    /// Run the full compile pipeline.
    pub fn compile(&mut self) -> Result<()> {
        info!("GGUF Genome Compiler — starting pipeline");

        if self.config.input.is_dir() {
            self.compile_directory()
        } else {
            self.compile_single_file()
        }
    }

    /// Decompile a genome binary back to float weights.
    pub fn decompile(&self) -> Result<()> {
        let input = &self.config.input;
        let output = &self.config.output;

        info!("GGUF Genome Decompiler — reading {}", input.display());

        let file = File::open(input)?;
        let mut reader = BufReader::new(file);

        // Read header
        let mut magic = [0u8; 5];
        reader.read_exact(&mut magic)?;
        if &magic != b"AASv1" {
            bail!("Invalid genome magic: {:?} (expected AASv1)", &magic);
        }

        let voxel_count = reader.read_u64::<LittleEndian>()?;
        let weight_count = reader.read_u64::<LittleEndian>()?;
        let num_tracks = reader.read_u32::<LittleEndian>()? as usize;

        info!(
            "  Voxels: {}, Weights: {}, Tracks: {}",
            voxel_count, weight_count, num_tracks
        );

        // Read track sizes
        let mut track_sizes = Vec::with_capacity(num_tracks);
        for _ in 0..num_tracks {
            track_sizes.push(reader.read_u64::<LittleEndian>()?);
        }

        // Read voxels
        let mut voxels = vec![0u32; voxel_count as usize];
        for v in &mut voxels {
            *v = reader.read_u32::<LittleEndian>()?;
        }

        // Unpack 2-bit values
        let bits = unpack_2bit_array(&voxels, weight_count as usize);

        // Map back to floats
        let weights: Vec<f32> = bits.iter().map(|&b| bit_to_float(b)).collect();

        // Denormalize from [-2.5, 2.5] back to original range
        // We don't know the original abs_max, so we output normalized weights
        info!(
            "  Decompiled {} weights from {} voxels",
            weights.len(),
            voxels.len()
        );

        // Write output
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write as raw f32 binary
        let mut out_file = File::create(output)?;
        for &w in &weights {
            out_file.write_all(&w.to_le_bytes())?;
        }

        // Also write a human-readable summary
        let summary_path = output.with_extension("summary.txt");
        let mut summary = File::create(&summary_path)?;
        writeln!(summary, "Genome Decompile Summary")?;
        writeln!(summary, "========================")?;
        writeln!(summary, "Input: {}", input.display())?;
        writeln!(summary, "Voxels: {}", voxel_count)?;
        writeln!(summary, "Weights: {}", weight_count)?;
        writeln!(summary, "Tracks: {}", num_tracks)?;
        writeln!(summary, "Track sizes: {:?}", track_sizes)?;
        writeln!(summary)?;

        // Distribution of 2-bit values
        let mut counts = [0u64; 4];
        for &b in &bits {
            counts[(b & 0x03) as usize] += 1;
        }
        writeln!(summary, "2-bit Distribution:")?;
        writeln!(
            summary,
            "  A (00): {} ({:.1}%)",
            counts[0],
            counts[0] as f64 / bits.len() as f64 * 100.0
        )?;
        writeln!(
            summary,
            "  T (01): {} ({:.1}%)",
            counts[1],
            counts[1] as f64 / bits.len() as f64 * 100.0
        )?;
        writeln!(
            summary,
            "  C (10): {} ({:.1}%)",
            counts[2],
            counts[2] as f64 / bits.len() as f64 * 100.0
        )?;
        writeln!(
            summary,
            "  G (11): {} ({:.1}%)",
            counts[3],
            counts[3] as f64 / bits.len() as f64 * 100.0
        )?;
        writeln!(summary)?;

        // Weight statistics
        let min = weights.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = weights.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mean = weights.iter().sum::<f32>() / weights.len() as f32;
        writeln!(summary, "Weight Statistics (normalized):")?;
        writeln!(summary, "  Min: {:.4}", min)?;
        writeln!(summary, "  Max: {:.4}", max)?;
        writeln!(summary, "  Mean: {:.4}", mean)?;
        writeln!(summary)?;

        // First 64 weights as sample
        writeln!(summary, "First 64 weights (sample):")?;
        for (i, chunk) in weights.iter().take(64).enumerate() {
            if i > 0 && i % 8 == 0 {
                writeln!(summary)?;
            }
            write!(summary, "  {:8.4}", chunk)?;
        }
        writeln!(summary)?;

        let file_size = std::fs::metadata(output)?.len();
        let summary_size = std::fs::metadata(&summary_path)?.len();

        info!("Weights written to: {}", output.display());
        info!("Summary written to: {}", summary_path.display());
        info!(
            "  Weight file: {:.2} MB",
            file_size as f64 / (1024.0 * 1024.0)
        );
        info!("  Summary: {} bytes", summary_size);

        Ok(())
    }

    /// Verify a genome binary — validate header, check integrity, report stats.
    pub fn verify(&self) -> Result<GenomeVerification> {
        let input = &self.config.input;
        info!("Verifying genome binary: {}", input.display());

        let file = File::open(input)?;
        let mut reader = BufReader::new(file);

        // Read header
        let mut magic = [0u8; 5];
        reader.read_exact(&mut magic)?;
        if &magic != b"AASv1" {
            bail!("Invalid genome magic: {:?} (expected AASv1)", &magic);
        }

        let voxel_count = reader.read_u64::<LittleEndian>()?;
        let weight_count = reader.read_u64::<LittleEndian>()?;
        let num_tracks = reader.read_u32::<LittleEndian>()? as usize;

        // Validate track count
        if num_tracks == 0 || num_tracks > 256 {
            bail!("Invalid track count: {} (must be 1-256)", num_tracks);
        }

        // Read track sizes
        let mut track_sizes = Vec::with_capacity(num_tracks);
        let mut expected_voxels: u64 = 0;
        for _ in 0..num_tracks {
            let size = reader.read_u64::<LittleEndian>()?;
            track_sizes.push(size);
            expected_voxels += size;
        }

        // Validate voxel count matches sum of track sizes
        if expected_voxels != voxel_count {
            bail!(
                "Voxel count mismatch: header says {}, track sizes sum to {}",
                voxel_count,
                expected_voxels
            );
        }

        // Read voxels and compute checksum
        let mut voxels = vec![0u32; voxel_count as usize];
        for v in &mut voxels {
            *v = reader.read_u32::<LittleEndian>()?;
        }

        // Compute checksum (simple sum of all voxels as u32)
        let checksum: u32 = voxels.iter().fold(0u32, |acc, &v| acc.wrapping_add(v));

        // Analyze 2-bit distribution
        let mut bit_counts = [0u64; 4];
        for &voxel in &voxels {
            for j in 0..16 {
                let bits = (voxel >> (j * 2)) & 0x03;
                bit_counts[bits as usize] += 1;
            }
        }

        // Compute Shannon entropy
        let total_bits = (voxel_count as f64) * 16.0;
        let mut entropy = 0.0;
        for &count in &bit_counts {
            if count > 0 {
                let p = count as f64 / total_bits;
                entropy -= p * p.log2();
            }
        }

        // Check if file has trailing data
        let file_size = std::fs::metadata(input)?.len();
        let expected_header_size = 5 + 8 + 8 + 4 + (num_tracks as u64 * 8) + (voxel_count * 4);
        let has_trailing_data = file_size > expected_header_size;

        let verification = GenomeVerification {
            voxel_count,
            weight_count,
            num_tracks,
            track_sizes,
            checksum,
            bit_counts,
            entropy,
            file_size,
            header_valid: true,
            voxel_count_valid: expected_voxels == voxel_count,
            has_trailing_data,
        };

        info!("Verification complete for {}", input.display());
        info!(
            "  Voxels: {} (valid: {})",
            verification.voxel_count, verification.voxel_count_valid
        );
        info!("  Weights: {}", verification.weight_count);
        info!("  Tracks: {}", verification.num_tracks);
        info!("  Entropy: {:.3} bits (max 2.0)", verification.entropy);
        info!("  Checksum: {:08x}", verification.checksum);
        if verification.has_trailing_data {
            warn!("  Trailing data detected after voxel section");
        }

        Ok(verification)
    }

    /// Compile all GGUF files in a directory.
    fn compile_directory(&self) -> Result<()> {
        let dir = &self.config.input;
        if !dir.is_dir() {
            bail!("Not a directory: {}", dir.display());
        }

        let mut gguf_files: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "gguf"))
            .collect();

        if gguf_files.is_empty() {
            bail!("No .gguf files found in {}", dir.display());
        }

        gguf_files.sort();
        info!("Found {} GGUF file(s)", gguf_files.len());

        let mut all_voxels: Vec<u32> = Vec::new();
        let mut total_weights: u64 = 0;
        let mut source_models: Vec<String> = Vec::new();

        for gguf_path in &gguf_files {
            let model_name = gguf_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            info!("Processing: {}", gguf_path.display());
            let (voxels, weight_count) = self.harvest_gguf(gguf_path)?;
            all_voxels.extend_from_slice(&voxels);
            total_weights += weight_count;
            source_models.push(model_name);
        }

        info!(
            "Assembling {} voxels into {} tracks",
            all_voxels.len(),
            self.config.num_tracks
        );
        self.write_output(&all_voxels, total_weights, &source_models)
    }

    /// Compile a single GGUF file.
    fn compile_single_file(&self) -> Result<()> {
        let gguf_path = &self.config.input;
        if !gguf_path.exists() {
            bail!("GGUF file not found: {}", gguf_path.display());
        }

        let model_name = gguf_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        info!("Harvesting: {}", gguf_path.display());
        let (voxels, weight_count) = self.harvest_gguf(gguf_path)?;

        info!(
            "Compiling {} voxels into {} tracks",
            voxels.len(),
            self.config.num_tracks
        );
        self.write_output(&voxels, weight_count, &[model_name])
    }

    /// Harvest FFN tensors from a single GGUF file and compile to genome voxels.
    fn harvest_gguf(&self, path: &Path) -> Result<(Vec<u32>, u64)> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Parse header
        let header = self.read_header(&mut reader)?;
        info!(
            "  GGUF v{}: {} tensors, {} metadata KV pairs",
            header.version, header.tensor_count, header.metadata_kv_count
        );

        // Skip metadata KV pairs
        for _ in 0..header.metadata_kv_count {
            self.skip_metadata_value(&mut reader)?;
        }

        // Read tensor metadata
        let mut tensors = Vec::new();
        for _ in 0..header.tensor_count {
            let tensor = self.read_tensor_meta(&mut reader)?;
            tensors.push(tensor);
        }

        info!("  Parsed {} tensor metadata entries", tensors.len());

        // Filter to FFN tensors if requested
        let target_tensors: Vec<&GgufTensor> = if self.config.ffn_only {
            tensors
                .iter()
                .filter(|t| {
                    let n = t.name.to_lowercase();
                    n.contains("ffn_gate")
                        || n.contains("ffn_down")
                        || n.contains("ffn_up")
                        || n.contains("ffn_norm")
                })
                .collect()
        } else {
            tensors.iter().collect()
        };

        if target_tensors.is_empty() {
            warn!("  No FFN tensors found, falling back to all tensors");
        }

        let tensors_to_process: Vec<&GgufTensor> = if target_tensors.is_empty() {
            tensors.iter().collect()
        } else {
            target_tensors
        };

        // Extract and dequantize tensors
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut all_weights: Vec<f32> = Vec::new();

        for tensor in &tensors_to_process {
            if tensor.n_elements > self.config.max_elements {
                warn!(
                    "  Skipping {} ({} elements exceeds limit)",
                    tensor.name, tensor.n_elements
                );
                continue;
            }

            // Seek to tensor data offset
            reader.seek(SeekFrom::Start(tensor.offset))?;

            if let Some(data) = self.read_tensor_data(&mut reader, tensor)? {
                info!(
                    "  Extracted {} ({} elements, {:?})",
                    tensor.name, tensor.n_elements, tensor.ggml_type
                );
                all_weights.extend_from_slice(&data);
            }
        }

        if all_weights.is_empty() {
            warn!("  No weights extracted");
            return Ok((Vec::new(), 0));
        }

        let weight_count = all_weights.len() as u64;

        // Normalize to [-2.5, 2.5] range
        let abs_max = all_weights.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
        if abs_max > 0.0 {
            for w in &mut all_weights {
                *w = (*w / abs_max) * 2.5;
            }
        }

        // Map to 2-bit genomic states
        let genome_2bit: Vec<u8> = all_weights.iter().map(|&w| float_to_2bit(w)).collect();

        // Pack into u32 voxels
        let voxels = pack_2bit_array(&genome_2bit);

        info!(
            "  Compiled {} weights → {} voxels (ratio: {:.1}x)",
            weight_count,
            voxels.len(),
            weight_count as f32 / (voxels.len() as f32 * 16.0)
        );

        Ok((voxels, weight_count))
    }

    // ── GGUF Parsing ───────────────────────────────────────────────

    fn read_header(&self, reader: &mut BufReader<File>) -> Result<GgufHeader> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != GGUF_MAGIC {
            bail!(
                "Invalid GGUF magic: {:?} (expected {:?})",
                &magic,
                GGUF_MAGIC
            );
        }

        let version = reader.read_u32::<LittleEndian>()?;
        let tensor_count = reader.read_u64::<LittleEndian>()?;
        let metadata_kv_count = reader.read_u64::<LittleEndian>()?;

        Ok(GgufHeader {
            version,
            tensor_count,
            metadata_kv_count,
        })
    }

    fn read_string(&self, reader: &mut BufReader<File>) -> Result<String> {
        let len = reader.read_u64::<LittleEndian>()? as usize;
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;
        String::from_utf8(buf).map_err(|e| anyhow!("Invalid UTF-8 string: {}", e))
    }

    fn skip_metadata_value(&self, reader: &mut BufReader<File>) -> Result<()> {
        let type_code = reader.read_u32::<LittleEndian>()?;
        match type_code {
            0 => {
                reader.read_u8()?;
            } // Uint8
            1 => {
                reader.read_i8()?;
            } // Int8
            2 => {
                reader.read_u16::<LittleEndian>()?;
            } // Uint16
            3 => {
                reader.read_i16::<LittleEndian>()?;
            } // Int16
            4 => {
                reader.read_u32::<LittleEndian>()?;
            } // Uint32
            5 => {
                reader.read_i32::<LittleEndian>()?;
            } // Int32
            6 => {
                reader.read_f32::<LittleEndian>()?;
            } // Float32
            7 => {
                reader.read_u8()?;
            } // Bool
            8 => {
                self.read_string(reader)?;
            } // String
            10 => {
                reader.read_u64::<LittleEndian>()?;
            } // Uint64
            11 => {
                reader.read_i64::<LittleEndian>()?;
            } // Int64
            12 => {
                reader.read_f64::<LittleEndian>()?;
            } // Float64
            9 => {
                // Array — read element type and count, then skip each element
                let elem_type = reader.read_u32::<LittleEndian>()?;
                let count = reader.read_u64::<LittleEndian>()?;
                for _ in 0..count {
                    self.skip_metadata_value_by_type(reader, elem_type)?;
                }
            }
            _ => bail!("Unknown metadata value type: {}", type_code),
        }
        Ok(())
    }

    fn skip_metadata_value_by_type(
        &self,
        reader: &mut BufReader<File>,
        type_code: u32,
    ) -> Result<()> {
        match type_code {
            0 => {
                reader.read_u8()?;
            }
            1 => {
                reader.read_i8()?;
            }
            2 => {
                reader.read_u16::<LittleEndian>()?;
            }
            3 => {
                reader.read_i16::<LittleEndian>()?;
            }
            4 => {
                reader.read_u32::<LittleEndian>()?;
            }
            5 => {
                reader.read_i32::<LittleEndian>()?;
            }
            6 => {
                reader.read_f32::<LittleEndian>()?;
            }
            7 => {
                reader.read_u8()?;
            }
            8 => {
                self.read_string(reader)?;
            }
            10 => {
                reader.read_u64::<LittleEndian>()?;
            }
            11 => {
                reader.read_i64::<LittleEndian>()?;
            }
            12 => {
                reader.read_f64::<LittleEndian>()?;
            }
            9 => {
                let elem_type = reader.read_u32::<LittleEndian>()?;
                let count = reader.read_u64::<LittleEndian>()?;
                for _ in 0..count {
                    self.skip_metadata_value_by_type(reader, elem_type)?;
                }
            }
            _ => bail!("Unknown array element type: {}", type_code),
        }
        Ok(())
    }

    fn read_tensor_meta(&self, reader: &mut BufReader<File>) -> Result<GgufTensor> {
        let name = self.read_string(reader)?;
        let n_dims = reader.read_u32::<LittleEndian>()? as usize;

        let mut shape = Vec::with_capacity(n_dims);
        let mut n_elements: u64 = 1;
        for _ in 0..n_dims {
            let dim = reader.read_u64::<LittleEndian>()?;
            shape.push(dim);
            n_elements *= dim;
        }

        let ggml_type = GgmlType::from_u32(reader.read_u32::<LittleEndian>()?);
        let offset = reader.read_u64::<LittleEndian>()?;

        let n_bytes = ggml_type.bytes_per_element(n_elements);

        Ok(GgufTensor {
            name,
            shape,
            ggml_type,
            n_elements,
            n_bytes,
            offset,
        })
    }

    // ── Tensor Dequantization ──────────────────────────────────────

    fn read_tensor_data(
        &self,
        reader: &mut BufReader<File>,
        tensor: &GgufTensor,
    ) -> Result<Option<Vec<f32>>> {
        let mut raw_bytes = vec![0u8; tensor.n_bytes as usize];
        reader.read_exact(&mut raw_bytes)?;

        let data = match tensor.ggml_type {
            GgmlType::F32 => raw_bytes
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
            GgmlType::F16 => {
                raw_bytes
                    .chunks_exact(2)
                    .map(|c| {
                        let bits = u16::from_le_bytes([c[0], c[1]]);
                        // Simple f16 → f32 conversion
                        let sign = ((bits >> 15) as f32) * -2.0 + 1.0;
                        let exp = ((bits >> 10) & 0x1F) as i32 - 15;
                        let mantissa = (bits & 0x3FF) as f32 / 1024.0 + 1.0;
                        sign * mantissa * 2.0f32.powi(exp)
                    })
                    .collect()
            }
            GgmlType::Q4_0 => self.dequant_q4_0(&raw_bytes, tensor.n_elements)?,
            GgmlType::Q4_K => self.dequant_q4_k(&raw_bytes, tensor.n_elements)?,
            GgmlType::Q8_0 => self.dequant_q8_0(&raw_bytes, tensor.n_elements)?,
            GgmlType::Q8_1 => self.dequant_q8_0(&raw_bytes, tensor.n_elements)?,
            _ => {
                // Fallback: interpret as i8 and normalize
                let values: Vec<f32> = raw_bytes
                    .iter()
                    .map(|&b| (b as i8 as f32) / 128.0)
                    .collect();
                values
            }
        };

        Ok(Some(data))
    }

    fn dequant_q4_0(&self, raw: &[u8], n_elements: u64) -> Result<Vec<f32>> {
        let block_size = 32u64;
        let n_blocks = n_elements / block_size;
        let mut values = vec![0.0f32; n_elements as usize];
        let mut offset = 0usize;

        for i in 0..n_blocks {
            if offset + 18 > raw.len() {
                break;
            }

            // Read scale (f16)
            let scale_bits = u16::from_le_bytes([raw[offset], raw[offset + 1]]);
            let scale = self.f16_to_f32(scale_bits);
            offset += 2;

            // Read 16 bytes of 4-bit values (32 elements)
            for j in 0..16 {
                let byte = raw[offset];
                offset += 1;
                let idx = (i * block_size + j * 2) as usize;
                if idx + 1 < values.len() {
                    values[idx] = ((byte & 0x0F) as i8 - 8) as f32 * scale;
                    values[idx + 1] = ((byte >> 4) as i8 - 8) as f32 * scale;
                }
            }
        }

        Ok(values)
    }

    fn dequant_q8_0(&self, raw: &[u8], n_elements: u64) -> Result<Vec<f32>> {
        let block_size = 32u64;
        let n_blocks = n_elements / block_size;
        let mut values = vec![0.0f32; n_elements as usize];
        let mut offset = 0usize;

        for i in 0..n_blocks {
            if offset + 34 > raw.len() {
                break;
            }

            // Read scale (f16)
            let scale_bits = u16::from_le_bytes([raw[offset], raw[offset + 1]]);
            let scale = self.f16_to_f32(scale_bits);
            offset += 2;

            // Read 32 bytes of 8-bit values
            for j in 0..32 {
                let idx = (i * block_size + j) as usize;
                if idx < values.len() {
                    values[idx] = (raw[offset] as i8) as f32 * scale;
                }
                offset += 1;
            }
        }

        Ok(values)
    }

    fn dequant_q4_k(&self, raw: &[u8], n_elements: u64) -> Result<Vec<f32>> {
        let block_size = 256u64;
        let n_blocks = n_elements / block_size;
        let mut values = vec![0.0f32; n_elements as usize];
        let mut offset = 0usize;

        for i in 0..n_blocks {
            if offset + 144 > raw.len() {
                break;
            }

            // Read d and d_min (f16)
            let d_bits = u16::from_le_bytes([raw[offset], raw[offset + 1]]);
            let d = self.f16_to_f32(d_bits);
            offset += 2;
            let d_min_bits = u16::from_le_bytes([raw[offset], raw[offset + 1]]);
            let d_min = self.f16_to_f32(d_min_bits);
            offset += 2;

            // Read 12 scale bytes
            let scale_bytes: Vec<u8> = raw[offset..offset + 12].to_vec();
            offset += 12;

            // Read 12 min bytes
            let min_bytes: Vec<u8> = raw[offset..offset + 12].to_vec();
            offset += 12;

            // Compute scales and mins
            let mut scales = [0.0f32; 12];
            let mut mins = [0.0f32; 12];
            for j in 0..12 {
                scales[j] = d * scale_bytes[j] as f32;
                mins[j] = d_min * min_bytes[j] as f32;
            }

            // Read 128 bytes of 4-bit values (256 elements)
            for j in 0..128 {
                let byte = raw[offset];
                offset += 1;

                let idx = (i * block_size + j * 2) as usize;
                let block_idx = (j / 16 * 2 + if j % 16 >= 8 { 1 } else { 0 }) as usize;

                if idx + 1 < values.len() {
                    values[idx] = scales[block_idx] * ((byte & 0x0F) as f32) - mins[block_idx];
                    values[idx + 1] = scales[block_idx] * ((byte >> 4) as f32) - mins[block_idx];
                }
            }
        }

        Ok(values)
    }

    #[inline]
    fn f16_to_f32(&self, bits: u16) -> f32 {
        let sign = ((bits >> 15) as f32) * -2.0 + 1.0;
        let exp = ((bits >> 10) & 0x1F) as i32 - 15;
        let mantissa = (bits & 0x3FF) as f32 / 1024.0 + 1.0;
        sign * mantissa * 2.0f32.powi(exp)
    }

    // ── Output ─────────────────────────────────────────────────────

    fn write_output(
        &self,
        voxels: &[u32],
        weight_count: u64,
        source_models: &[String],
    ) -> Result<()> {
        let output_path = &self.config.output;

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = File::create(output_path)?;

        // Write AASv1 header
        file.write_all(b"AASv1")?;
        file.write_all(&(voxels.len() as u64).to_le_bytes())?;
        file.write_all(&weight_count.to_le_bytes())?;
        file.write_all(&(self.config.num_tracks as u32).to_le_bytes())?;

        // Compute track sizes
        let track_size = voxels.len() / self.config.num_tracks;
        let remainder = voxels.len() % self.config.num_tracks;

        for i in 0..self.config.num_tracks {
            let size = track_size + if i < remainder { 1 } else { 0 };
            file.write_all(&(size as u64).to_le_bytes())?;
        }

        // Write voxels
        for &voxel in voxels {
            file.write_all(&voxel.to_le_bytes())?;
        }

        let file_size = std::fs::metadata(output_path)?.len();
        info!("Genome written to: {}", output_path.display());
        info!(
            "  File size: {:.2} MB",
            file_size as f64 / (1024.0 * 1024.0)
        );
        info!("  Voxels: {}", voxels.len());
        info!("  Weight count: {}", weight_count);
        info!("  Tracks: {}", self.config.num_tracks);
        info!("  Sources: {}", source_models.join(", "));

        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────
// CLI entry point
// ────────────────────────────────────────────────────────────────────

pub fn run_cli(args: &[String]) -> Result<()> {
    use clap::Parser;

    #[derive(Parser)]
    #[command(
        name = "genome_compiler",
        about = "GGUF Genome Compiler — harvest FFN decision geometries into 2-bit genome format"
    )]
    struct Cli {
        /// Path to GGUF file (compile) or genome binary (decompile/verify)
        input: PathBuf,

        /// Output path for genome binary (compile) or weights file (decompile)
        #[arg(short, long, default_value = "chromosomes/universal_gaming_core.bin")]
        output: PathBuf,

        /// Number of tracks
        #[arg(short, long, default_value_t = 16)]
        tracks: usize,

        /// Extract all tensors, not just FFN
        #[arg(long)]
        all_tensors: bool,

        /// Decompile a genome binary back to float weights
        #[arg(long)]
        decompile: bool,

        /// Verify a genome binary (validate header, check integrity, report stats)
        #[arg(long)]
        verify: bool,
    }

    let cli = Cli::parse_from(args);

    let config = CompileConfig {
        input: cli.input,
        output: cli.output,
        num_tracks: cli.tracks,
        ffn_only: !cli.all_tensors,
        ..Default::default()
    };

    let compiler = GenomeCompiler::new(config.clone());

    if cli.decompile {
        compiler.decompile()?;
    } else if cli.verify {
        let verification = compiler.verify()?;
        println!("\nGenome Verification Report");
        println!("=========================");
        println!("File: {}", config.input.display());
        println!("Valid header: {}", verification.header_valid);
        println!("Valid voxel count: {}", verification.voxel_count_valid);
        println!("Voxels: {}", verification.voxel_count);
        println!("Weights: {}", verification.weight_count);
        println!("Tracks: {}", verification.num_tracks);
        println!("Track sizes: {:?}", verification.track_sizes);
        println!("Checksum: {:08x}", verification.checksum);
        println!("Entropy: {:.3} bits", verification.entropy);
        println!("2-bit distribution:");
        let total = verification.bit_counts.iter().sum::<u64>() as f64;
        println!(
            "  A (00): {} ({:.1}%)",
            verification.bit_counts[0],
            verification.bit_counts[0] as f64 / total * 100.0
        );
        println!(
            "  T (01): {} ({:.1}%)",
            verification.bit_counts[1],
            verification.bit_counts[1] as f64 / total * 100.0
        );
        println!(
            "  C (10): {} ({:.1}%)",
            verification.bit_counts[2],
            verification.bit_counts[2] as f64 / total * 100.0
        );
        println!(
            "  G (11): {} ({:.1}%)",
            verification.bit_counts[3],
            verification.bit_counts[3] as f64 / total * 100.0
        );
        println!("File size: {} bytes", verification.file_size);
        if verification.has_trailing_data {
            println!("WARNING: Trailing data detected after voxel section");
        }
    } else {
        let mut compiler = compiler;
        compiler.compile()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_to_2bit() {
        assert_eq!(float_to_2bit(-2.0), 0b00); // A
        assert_eq!(float_to_2bit(-0.5), 0b01); // T
        assert_eq!(float_to_2bit(0.5), 0b10); // C
        assert_eq!(float_to_2bit(2.0), 0b11); // G
    }

    #[test]
    fn test_pack_2bit_array() {
        // 16 values → 1 voxel
        let values = vec![
            0b00, 0b01, 0b10, 0b11, 0b00, 0b01, 0b10, 0b11, 0b00, 0b01, 0b10, 0b11, 0b00, 0b01,
            0b10, 0b11,
        ];
        let voxels = pack_2bit_array(&values);
        assert_eq!(voxels.len(), 1);
        // First value (0b00) in bits 0-1, second (0b01) in bits 2-3, etc.
        assert_eq!(voxels[0], 0b11_10_01_00_11_10_01_00_11_10_01_00_11_10_01_00);
    }

    #[test]
    fn test_pack_2bit_array_padding() {
        // 5 values → padded to 16 → 1 voxel
        let values = vec![0b00, 0b01, 0b10, 0b11, 0b00];
        let voxels = pack_2bit_array(&values);
        assert_eq!(voxels.len(), 1);
    }

    #[test]
    fn test_ggml_type_bytes_per_element() {
        assert_eq!(GgmlType::F32.bytes_per_element(100), 4);
        assert_eq!(GgmlType::F16.bytes_per_element(100), 2);
        assert_eq!(GgmlType::Q4_0.bytes_per_element(100), 50);
        assert_eq!(GgmlType::Q8_0.bytes_per_element(100), 100);
        // Q4_K: 256-element blocks, 144 bytes each → 144/256 = 0.5625 per element
        // But integer division: 100/256*144 = 0 for small inputs
        // For larger: 256 elements → 144 bytes
        assert_eq!(GgmlType::Q4_K.bytes_per_element(256), 144);
        assert_eq!(GgmlType::Q6_K.bytes_per_element(256), 208);
    }

    #[test]
    fn test_bit_to_float() {
        assert_eq!(bit_to_float(0b00), -1.5); // A
        assert_eq!(bit_to_float(0b01), -0.5); // T
        assert_eq!(bit_to_float(0b10), 0.5); // C
        assert_eq!(bit_to_float(0b11), 1.5); // G
    }

    #[test]
    fn test_unpack_2bit_array() {
        // Pack then unpack should round-trip
        let original = vec![
            0b00, 0b01, 0b10, 0b11, 0b00, 0b01, 0b10, 0b11, 0b00, 0b01, 0b10, 0b11, 0b00, 0b01,
            0b10, 0b11,
        ];
        let voxels = pack_2bit_array(&original);
        let unpacked = unpack_2bit_array(&voxels, original.len());
        assert_eq!(unpacked, original);
    }

    #[test]
    fn test_unpack_2bit_array_partial() {
        // Unpack fewer values than voxels contain
        let original = vec![0b00, 0b01, 0b10, 0b11, 0b00];
        let voxels = pack_2bit_array(&original);
        let unpacked = unpack_2bit_array(&voxels, 5);
        assert_eq!(unpacked, original);
    }

    #[test]
    fn test_compile_decompile_roundtrip() {
        // Verify bit_to_float(float_to_2bit(v)) maps to representative values
        assert_eq!(bit_to_float(float_to_2bit(-2.0)), -1.5);
        assert_eq!(bit_to_float(float_to_2bit(-0.5)), -0.5);
        assert_eq!(bit_to_float(float_to_2bit(0.5)), 0.5);
        assert_eq!(bit_to_float(float_to_2bit(2.0)), 1.5);
    }
}
