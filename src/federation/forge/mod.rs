/// Synth DNA Forge — tensor surgery and agent crystallization.
///
/// This module is the Rust-native implementation of the "Synth DNA" concept:
/// instead of training new agents from scratch, we surgically splice weight
/// tensors from existing GGUF models to produce hybrid agents in milliseconds.
///
/// The forge logic was originally prototyped as a native enzyme (`tensor_forge.dll`)
/// called via the legacy FFI path in `main.rs`. This module re-implements the
/// same logic as safe, async Rust directly callable from the `Federation`.
///
/// # Output format
///
/// `crystallize()` writes a **valid GGUF v3 file** readable by llama.cpp,
/// gguf-py, and any conforming GGUF reader:
///
/// ```text
/// [4]  magic       = "GGUF"
/// [4]  version     = 3  (u32 LE)
/// [8]  tensor_count        (u64 LE)
/// [8]  metadata_kv_count   (u64 LE)
/// [...] metadata KV pairs  (from recipe.metadata_overrides + Aaroneous defaults)
/// [...] tensor info table  (per tensor: name, n_dims, dims[], dtype, data_offset)
/// [...] alignment padding  to GGUF_ALIGNMENT (32 bytes)
/// [...] tensor data blobs  (raw weights from source models)
/// ```
///
/// # Architecture
///
/// ```text
/// ┌─ GGUF Model A ─────────────┐    ┌─ GGUF Model B ─────────────┐
/// │  attention_weights[0..N]   │    │  mlp_weights[0..M]          │
/// │  mlp_weights[0..M]         │    │  embedding[0..E]            │
/// └────────────────────────────┘    └─────────────────────────────┘
///                                                │
///                  ForgeRecipe describes which tensors to take from where
///                                                │
///                                                ▼
///                          ┌─────────────────────────────────────┐
///                          │  Hybrid GGUF ("Crystallized Agent") │
///                          │  attention from A + mlp from B       │
///                          └─────────────────────────────────────┘
/// ```
///
/// # Usage
///
/// ```no_run
/// use a_run::federation::forge::{Forge, ForgeRecipe, SplicingSegment, GgufIndex, GgufMeta, TensorMeta};
/// use std::collections::HashMap;
///
/// let recipe = ForgeRecipe {
///     recipe_id: "design-v1".to_string(),
///     segments: vec![
///         SplicingSegment {
///             source_gguf: "visionary.gguf".to_string(),
///             tensor_name: "blk.0.attn_q.weight".to_string(),
///         },
///     ],
///     metadata_overrides: HashMap::new(),
/// };
///
/// let mut index = GgufIndex(HashMap::new());
/// // ... populate index ...
///
/// let forge = Forge::new();
/// forge.crystallize(&recipe, &index, "/output/hybrid.gguf").await.unwrap();
/// ```

use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

// ────────────────────────────────────────────────────────────────────
// GGUF v3 constants
// ────────────────────────────────────────────────────────────────────

/// Output file alignment: tensor data section must start at a multiple of this.
const GGUF_ALIGNMENT: u64 = 32;
const GGUF_VERSION: u32 = 3;

/// GGUF metadata value type codes (spec §2.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GgufMetaType {
    Uint8   = 0,
    Int8    = 1,
    Uint16  = 2,
    Int16   = 3,
    Uint32  = 4,
    Int32   = 5,
    Float32 = 6,
    Bool    = 7,
    String  = 8,
    Array   = 9,
    Uint64  = 10,
    Int64   = 11,
    Float64 = 12,
}

/// A typed metadata value for the GGUF header KV store.
/// Only the variants used by Qwen/common architectures are modeled.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum MetaValue {
    Uint32(u32),
    Int32(i32),
    Float32(f32),
    Uint64(u64),
    Bool(bool),
    String(String),
    /// Array of strings (e.g., tokenizer.ggml.tokens)
    StringArray(Vec<String>),
}

// ────────────────────────────────────────────────────────────────────
// Wire types (same schema as the tensor_forge enzyme)
// ────────────────────────────────────────────────────────────────────

/// Location of one tensor within a GGUF file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TensorMeta {
    /// Byte offset of the tensor data within the GGUF file.
    pub offset: u64,
    /// Byte length of the tensor data.
    pub size: u64,
    /// Optional: tensor kind label (e.g., "attention", "mlp", "embedding")
    pub kind: Option<String>,
    /// Tensor shape dimensions (outermost first), e.g. [4096, 4096].
    /// Required to write a valid GGUF tensor info table.
    #[serde(default)]
    pub shape: Vec<u64>,
    /// GGUF data type code (spec §2.3): 0=F32, 1=F16, 12=Q4_K, 14=Q6_K, etc.
    /// Defaults to 0 (F32) when absent.
    #[serde(default)]
    pub dtype: u32,
}

/// Metadata about one source GGUF file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufMeta {
    /// Absolute path to the GGUF file on disk.
    pub path: PathBuf,
    /// Map from tensor name → offset/size within this file.
    pub tensors: HashMap<String, TensorMeta>,
}

/// The index of all available source GGUF files.
///
/// Key: logical name (e.g., `"visionary.gguf"`) used in `SplicingSegment`.
/// Value: physical file metadata + tensor map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufIndex(pub HashMap<String, GgufMeta>);

impl GgufIndex {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Register a GGUF file in the index.
    pub fn register(&mut self, name: impl Into<String>, meta: GgufMeta) {
        self.0.insert(name.into(), meta);
    }

    /// Number of registered GGUF files.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for GgufIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// One tensor to be taken from one source model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SplicingSegment {
    /// Logical name of the source GGUF (key in `GgufIndex`).
    pub source_gguf: String,
    /// Name of the tensor within that GGUF (must exist in `GgufMeta::tensors`).
    pub tensor_name: String,
}

/// A complete recipe for creating a hybrid GGUF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRecipe {
    /// Stable identifier for this recipe (used for caching/deduplication).
    pub recipe_id: String,
    /// Ordered list of tensor splices. Order determines layout in the output file.
    pub segments: Vec<SplicingSegment>,
    /// Metadata KV pairs written into the GGUF header.
    ///
    /// Common keys for Qwen-based specialists:
    /// - `"general.architecture"` → `String("qwen2")`
    /// - `"general.name"`         → `String("aaroneous-visionary-v1")`
    /// - `"llama.context_length"` → `Uint32(4096)`
    /// - `"llama.rope.freq_base"` → `Float32(1000000.0)` (Qwen uses 1M)
    /// - `"tokenizer.ggml.model"` → `String("gpt2")`
    ///
    /// Any keys not provided here receive Aaroneous defaults in `crystallize()`.
    #[serde(default)]
    pub metadata_overrides: HashMap<String, MetaValue>,
}

// ────────────────────────────────────────────────────────────────────
// GGUF v3 reader — builds GgufIndex from a real model file
// ────────────────────────────────────────────────────────────────────

/// Error from the GGUF reader/parser.
#[derive(Debug, thiserror::Error)]
pub enum GgufReadError {
    #[error("I/O error reading '{path}': {source}")]
    Io { path: PathBuf, #[source] source: std::io::Error },
    #[error("not a GGUF file (bad magic at '{path}')")]
    BadMagic { path: PathBuf },
    #[error("unsupported GGUF version {version} in '{path}' (expected 1–3)")]
    BadVersion { path: PathBuf, version: u32 },
    #[error("unsupported metadata value type {type_code} in '{path}'")]
    UnsupportedMetaType { path: PathBuf, type_code: u32 },
    #[error("tensor name too long ({len} bytes) in '{path}'")]
    NameTooLong { path: PathBuf, len: u64 },
    #[error("too many dimensions ({n}) in tensor '{name}' in '{path}'")]
    TooManyDims { path: PathBuf, name: String, n: u32 },
}

/// Parsed metadata from a GGUF file header.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GgufParsedMeta {
    /// GGUF format version (1, 2, or 3)
    pub version: u32,
    /// All metadata KV pairs, values as strings for portability
    pub kv: HashMap<String, String>,
    /// Architecture (e.g., "qwen2", "llama", "mistral")
    pub architecture: String,
    /// Model name from `general.name`
    pub model_name: String,
    /// Context length from `llama.context_length`
    pub context_length: Option<u32>,
    /// Number of tensors in this file
    pub tensor_count: u64,
}

/// Read a GGUF file and return both a `GgufIndex` (for forging) and parsed metadata.
///
/// This is the **missing link** between "I have a Qwen GGUF on disk" and
/// "I can forge a hybrid from it." It parses the full GGUF v3 (and v1/v2
/// for backwards compatibility) tensor info table and metadata KV section.
///
/// # Usage
///
/// ```no_run
/// use a_run::federation::forge::read_gguf;
///
/// # async fn example() -> anyhow::Result<()> {
/// let (index, meta) = read_gguf("models/qwen2.5-1.5b.gguf")?;
/// println!("Model: {} — {} tensors", meta.model_name, meta.tensor_count);
/// for (name, tm) in &index.0["models/qwen2.5-1.5b.gguf"].tensors {
///     println!("  {} offset={} size={} shape={:?} dtype={}", name, tm.offset, tm.size, tm.shape, tm.dtype);
/// }
/// # Ok(())
/// # }
/// ```
pub fn read_gguf(
    path: impl AsRef<Path>,
) -> Result<(GgufIndex, GgufParsedMeta), GgufReadError> {
    let path = path.as_ref().to_path_buf();
    let data = std::fs::read(&path).map_err(|e| GgufReadError::Io { path: path.clone(), source: e })?;
    let mut r = Cursor::new(&data[..]);

    // ── magic ──────────────────────────────────────────────────────────
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).map_err(|e| GgufReadError::Io { path: path.clone(), source: e })?;
    if &magic != b"GGUF" {
        return Err(GgufReadError::BadMagic { path });
    }

    // ── version ────────────────────────────────────────────────────────
    let version = read_u32_le(&mut r, &path)?;
    if version == 0 || version > 3 {
        return Err(GgufReadError::BadVersion { path, version });
    }

    // ── counts ─────────────────────────────────────────────────────────
    let tensor_count = read_u64_le(&mut r, &path)?;
    let kv_count = read_u64_le(&mut r, &path)?;

    // ── metadata KV ────────────────────────────────────────────────────
    let mut kv_map: HashMap<String, String> = HashMap::new();
    let mut alignment: u64 = 32;

    for _ in 0..kv_count {
        let key = read_gguf_string_r(&mut r, &path)?;
        let type_code = read_u32_le(&mut r, &path)?;
        let value_str = skip_or_read_meta_value(&mut r, &path, type_code, version)?;

        if key == "general.alignment" {
            if let Ok(v) = value_str.parse::<u64>() { alignment = v; }
        }
        kv_map.insert(key, value_str);
    }

    // ── tensor info table ──────────────────────────────────────────────
    let mut tensors: HashMap<String, TensorMeta> = HashMap::new();

    for _ in 0..tensor_count {
        let name = read_gguf_string_r(&mut r, &path)?;
        if name.len() > 256 {
            return Err(GgufReadError::NameTooLong { path, len: name.len() as u64 });
        }

        let n_dims = read_u32_le(&mut r, &path)?;
        if n_dims > 8 {
            return Err(GgufReadError::TooManyDims { path, name, n: n_dims });
        }

        let mut shape = Vec::with_capacity(n_dims as usize);
        for _ in 0..n_dims {
            // v1 uses u32, v2/v3 use u64
            let dim = if version == 1 {
                read_u32_le(&mut r, &path)? as u64
            } else {
                read_u64_le(&mut r, &path)?
            };
            shape.push(dim);
        }

        let dtype = read_u32_le(&mut r, &path)?;
        let data_offset = read_u64_le(&mut r, &path)?; // relative to tensor data section start

        tensors.insert(name, TensorMeta {
            offset: data_offset, // will be fixed up below once we know section start
            size: gguf_tensor_nbytes(&shape, dtype),
            kind: None,
            shape,
            dtype,
        });
    }

    // ── compute absolute offsets ───────────────────────────────────────
    // Tensor data section starts at the first alignment boundary after
    // the end of the tensor info table.
    let header_end = r.position();
    let pad = pad_to_alignment(header_end, alignment);
    let tensor_data_start = header_end + pad;

    for tm in tensors.values_mut() {
        let rel = tm.offset; // was data_offset (relative)
        tm.offset = tensor_data_start + rel;
    }

    // ── build index + metadata ─────────────────────────────────────────
    let architecture = kv_map.get("general.architecture").cloned().unwrap_or_default();
    let model_name   = kv_map.get("general.name").cloned().unwrap_or_default();
    let context_length = kv_map.get("llama.context_length")
        .or_else(|| kv_map.get("llm.context_length"))
        .and_then(|s| s.parse().ok());

    let mut index = GgufIndex::new();
    index.register(
        path.to_string_lossy().to_string(),
        GgufMeta { path: path.clone(), tensors },
    );

    let meta = GgufParsedMeta {
        version,
        kv: kv_map,
        architecture,
        model_name,
        context_length,
        tensor_count,
    };

    info!(
        "read_gguf: '{}' v{} — {} tensors, arch='{}'",
        path.display(), version, tensor_count, meta.architecture
    );

    Ok((index, meta))
}

// ── GGUF binary reading helpers ──────────────────────────────────────────────

fn read_u8(r: &mut Cursor<&[u8]>, path: &Path) -> Result<u8, GgufReadError> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b).map_err(|e| GgufReadError::Io { path: path.to_path_buf(), source: e })?;
    Ok(b[0])
}

fn read_u16_le(r: &mut Cursor<&[u8]>, path: &Path) -> Result<u16, GgufReadError> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b).map_err(|e| GgufReadError::Io { path: path.to_path_buf(), source: e })?;
    Ok(u16::from_le_bytes(b))
}

fn read_u32_le(r: &mut Cursor<&[u8]>, path: &Path) -> Result<u32, GgufReadError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|e| GgufReadError::Io { path: path.to_path_buf(), source: e })?;
    Ok(u32::from_le_bytes(b))
}

fn read_i32_le(r: &mut Cursor<&[u8]>, path: &Path) -> Result<i32, GgufReadError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|e| GgufReadError::Io { path: path.to_path_buf(), source: e })?;
    Ok(i32::from_le_bytes(b))
}

fn read_u64_le(r: &mut Cursor<&[u8]>, path: &Path) -> Result<u64, GgufReadError> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).map_err(|e| GgufReadError::Io { path: path.to_path_buf(), source: e })?;
    Ok(u64::from_le_bytes(b))
}

fn read_i64_le(r: &mut Cursor<&[u8]>, path: &Path) -> Result<i64, GgufReadError> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).map_err(|e| GgufReadError::Io { path: path.to_path_buf(), source: e })?;
    Ok(i64::from_le_bytes(b))
}

fn read_f32_le(r: &mut Cursor<&[u8]>, path: &Path) -> Result<f32, GgufReadError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|e| GgufReadError::Io { path: path.to_path_buf(), source: e })?;
    Ok(f32::from_le_bytes(b))
}

fn read_f64_le(r: &mut Cursor<&[u8]>, path: &Path) -> Result<f64, GgufReadError> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).map_err(|e| GgufReadError::Io { path: path.to_path_buf(), source: e })?;
    Ok(f64::from_le_bytes(b))
}

/// Read a GGUF length-prefixed UTF-8 string from a reader (u64 len + bytes, no NUL).
fn read_gguf_string_r(r: &mut Cursor<&[u8]>, path: &Path) -> Result<String, GgufReadError> {
    let len = read_u64_le(r, path)?;
    if len > 1024 * 1024 {
        return Err(GgufReadError::NameTooLong { path: path.to_path_buf(), len });
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf).map_err(|e| GgufReadError::Io { path: path.to_path_buf(), source: e })?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

/// Read one GGUF metadata value and return it as a display string.
/// Arrays are shown as comma-separated lists of their element values.
fn skip_or_read_meta_value(
    r: &mut Cursor<&[u8]>,
    path: &Path,
    type_code: u32,
    _version: u32,
) -> Result<String, GgufReadError> {
    match type_code {
        0  => Ok(read_u8(r, path)?.to_string()),           // UINT8
        1  => Ok((read_u8(r, path)? as i8).to_string()),   // INT8
        2  => Ok(read_u16_le(r, path)?.to_string()),       // UINT16
        3  => Ok((read_u16_le(r, path)? as i16).to_string()), // INT16
        4  => Ok(read_u32_le(r, path)?.to_string()),       // UINT32
        5  => Ok(read_i32_le(r, path)?.to_string()),       // INT32
        6  => Ok(read_f32_le(r, path)?.to_string()),       // FLOAT32
        7  => Ok(if read_u8(r, path)? != 0 { "true" } else { "false" }.to_string()), // BOOL
        8  => read_gguf_string_r(r, path),                 // STRING
        9  => {
            // ARRAY: array_type u32, count u64, items
            let item_type = read_u32_le(r, path)?;
            let count = read_u64_le(r, path)?;
            let cap = count.min(64) as usize; // only materialise first 64 items
            let mut parts = Vec::with_capacity(cap);
            for i in 0..count {
                let s = skip_or_read_meta_value(r, path, item_type, _version)?;
                if (i as usize) < cap { parts.push(s); }
            }
            Ok(parts.join(","))
        }
        10 => Ok(read_u64_le(r, path)?.to_string()),       // UINT64
        11 => Ok(read_i64_le(r, path)?.to_string()),       // INT64
        12 => Ok(read_f64_le(r, path)?.to_string()),       // FLOAT64
        t  => Err(GgufReadError::UnsupportedMetaType { path: path.to_path_buf(), type_code: t }),
    }
}

/// Return the number of bytes occupied by a tensor with the given shape and dtype.
///
/// Based on the GGUF spec quantization table.  Returns 0 for unknown dtypes
/// rather than panicking — the forge can still copy bytes if the offset and
/// source size are known from the index.
fn gguf_tensor_nbytes(shape: &[u64], dtype: u32) -> u64 {
    let n_elements: u64 = shape.iter().product();
    if n_elements == 0 { return 0; }

    // bytes_per_element × n_elements for floating-point types
    // For quantized types: bytes_per_block × n_blocks
    match dtype {
        0  => n_elements * 4,           // F32
        1  => n_elements * 2,           // F16
        2  => {                         // Q4_0: 18 bytes per 32-element block
            let blocks = (n_elements + 31) / 32;
            blocks * 18
        }
        3  => {                         // Q4_1: 20 bytes per 32-element block
            let blocks = (n_elements + 31) / 32;
            blocks * 20
        }
        6  => {                         // Q5_0: 22 bytes per 32-element block
            let blocks = (n_elements + 31) / 32;
            blocks * 22
        }
        7  => {                         // Q5_1: 24 bytes per 32-element block
            let blocks = (n_elements + 31) / 32;
            blocks * 24
        }
        8  => {                         // Q8_0: 34 bytes per 32-element block
            let blocks = (n_elements + 31) / 32;
            blocks * 34
        }
        10 => {                         // Q2_K: 84 bytes per 256-element block
            let blocks = (n_elements + 255) / 256;
            blocks * 84
        }
        11 => {                         // Q3_K: 110 bytes per 256-element block
            let blocks = (n_elements + 255) / 256;
            blocks * 110
        }
        12 => {                         // Q4_K: 144 bytes per 256-element block (Q4_K_M)
            let blocks = (n_elements + 255) / 256;
            blocks * 144
        }
        13 => {                         // Q5_K: 176 bytes per 256-element block
            let blocks = (n_elements + 255) / 256;
            blocks * 176
        }
        14 => {                         // Q6_K: 210 bytes per 256-element block
            let blocks = (n_elements + 255) / 256;
            blocks * 210
        }
        15 => {                         // Q8_K: 292 bytes per 256-element block
            let blocks = (n_elements + 255) / 256;
            blocks * 292
        }
        16 | 17 | 18 => n_elements * 2, // IQ types (approximate, 2 bytes)
        30 => n_elements * 2,           // BF16
        _  => 0,                        // Unknown — caller should use file size instead
    }
}

// ────────────────────────────────────────────────────────────────────
// Errors
// ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    #[error("source GGUF '{gguf}' not in index")]
    GgufNotFound { gguf: String },

    #[error("tensor '{tensor}' not found in '{gguf}'")]
    TensorNotFound { tensor: String, gguf: String },

    #[error("source file '{path}' could not be opened: {source}")]
    SourceOpen {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("mmap failed on '{path}': {source}")]
    MmapFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("tensor data out of bounds in '{path}' (offset={offset}, size={size}, file_len={file_len})")]
    OutOfBounds {
        path: PathBuf,
        offset: u64,
        size: u64,
        file_len: usize,
    },

    #[error("output file '{path}' could not be created: {source}")]
    OutputCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("write error: {0}")]
    WriteError(#[from] std::io::Error),

    #[error("empty recipe — no segments to splice")]
    EmptyRecipe,
}

// ────────────────────────────────────────────────────────────────────
// Forge
// ────────────────────────────────────────────────────────────────────

/// Tensor forge: produces hybrid GGUF files from splicing recipes.
pub struct Forge {
    /// Statistics for observability.
    pub stats: ForgeStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForgeStats {
    pub crystallizations_attempted: u64,
    pub crystallizations_succeeded: u64,
    pub tensors_spliced: u64,
    pub bytes_written: u64,
}

impl Forge {
    pub fn new() -> Self {
        Self {
            stats: ForgeStats::default(),
        }
    }

    /// Crystallize a hybrid GGUF by splicing tensors according to the recipe.
    ///
    /// Writes a **valid GGUF v3 file** readable by llama.cpp, gguf-py, and any
    /// conforming GGUF reader.  The header contains:
    ///   magic (4) · version u32 · tensor_count u64 · kv_count u64 ·
    ///   KV metadata pairs · tensor info table · alignment padding ·
    ///   tensor data blobs (mmapped from source models)
    ///
    /// Runs on `tokio::task::spawn_blocking` since mmap + file I/O is blocking.
    pub async fn crystallize(
        &mut self,
        recipe: &ForgeRecipe,
        index: &GgufIndex,
        output_path: impl AsRef<Path>,
    ) -> Result<CrystallizationResult, ForgeError> {
        if recipe.segments.is_empty() {
            return Err(ForgeError::EmptyRecipe);
        }

        self.stats.crystallizations_attempted += 1;

        let recipe = recipe.clone();
        let index = index.clone();
        let output_path = output_path.as_ref().to_path_buf();

        let result = tokio::task::spawn_blocking(move || -> Result<CrystallizationResult, ForgeError> {
            // ── Phase 1: resolve every segment → (meta, mmap) ──────────────
            struct ResolvedSegment {
                segment_name: String,
                source_gguf:  String,
                meta:         TensorMeta,
                source_path:  PathBuf,
            }
            let mut resolved: Vec<ResolvedSegment> = Vec::with_capacity(recipe.segments.len());

            for seg in &recipe.segments {
                let gguf_meta = index.0.get(&seg.source_gguf)
                    .ok_or_else(|| ForgeError::GgufNotFound { gguf: seg.source_gguf.clone() })?;
                let tensor_meta = gguf_meta.tensors.get(&seg.tensor_name)
                    .ok_or_else(|| ForgeError::TensorNotFound {
                        tensor: seg.tensor_name.clone(),
                        gguf: seg.source_gguf.clone(),
                    })?;
                resolved.push(ResolvedSegment {
                    segment_name: seg.tensor_name.clone(),
                    source_gguf:  seg.source_gguf.clone(),
                    meta:         tensor_meta.clone(),
                    source_path:  gguf_meta.path.clone(),
                });
            }

            // ── Phase 2: build metadata KV list ────────────────────────────
            let mut kv: Vec<(String, MetaValue)> = Vec::new();

            // Aaroneous-specific provenance key — always present
            kv.push(("general.source".to_string(),
                MetaValue::String("aaroneous-forge".to_string())));
            kv.push(("general.recipe_id".to_string(),
                MetaValue::String(recipe.recipe_id.clone())));

            // Architecture defaults (overridden by recipe)
            let defaults: &[(&str, MetaValue)] = &[
                ("general.architecture",    MetaValue::String("qwen2".to_string())),
                ("general.name",            MetaValue::String(format!("aaroneous-{}", recipe.recipe_id))),
                ("llama.context_length",    MetaValue::Uint32(4096)),
                ("llama.embedding_length",  MetaValue::Uint32(4096)),
                ("llama.feed_forward_length", MetaValue::Uint32(11008)),
                ("llama.attention.head_count",    MetaValue::Uint32(32)),
                ("llama.attention.head_count_kv", MetaValue::Uint32(32)),
                ("llama.rope.freq_base",    MetaValue::Float32(1_000_000.0)),
                ("llama.rope.dimension_count", MetaValue::Uint32(128)),
                ("tokenizer.ggml.model",    MetaValue::String("gpt2".to_string())),
            ];
            for (key, val) in defaults {
                if !recipe.metadata_overrides.contains_key(*key) {
                    kv.push((key.to_string(), val.clone()));
                }
            }
            // Apply recipe overrides last
            for (key, val) in &recipe.metadata_overrides {
                kv.push((key.clone(), val.clone()));
            }

            // ── Phase 3: write header ───────────────────────────────────────
            let mut output = File::create(&output_path).map_err(|e| ForgeError::OutputCreate {
                path: output_path.clone(),
                source: e,
            })?;

            // magic
            output.write_all(b"GGUF")?;
            // version (u32 LE)
            output.write_all(&GGUF_VERSION.to_le_bytes())?;
            // tensor_count (u64 LE)
            output.write_all(&(resolved.len() as u64).to_le_bytes())?;
            // metadata_kv_count (u64 LE)
            output.write_all(&(kv.len() as u64).to_le_bytes())?;

            // KV pairs
            for (key, val) in &kv {
                write_gguf_string(&mut output, key)?;
                write_gguf_meta_value(&mut output, val)?;
            }

            // ── Phase 4: tensor info table ─────────────────────────────────
            // data_offset for each tensor is relative to the start of the
            // tensor data section (after all padding).  We compute them now
            // as running cumulative sums of tensor sizes.
            let mut data_offset: u64 = 0;
            let tensor_offsets: Vec<u64> = resolved.iter().map(|r| {
                let off = data_offset;
                data_offset += r.meta.size;
                off
            }).collect();

            for (r, &data_off) in resolved.iter().zip(tensor_offsets.iter()) {
                // tensor name
                write_gguf_string(&mut output, &r.segment_name)?;
                // n_dims (u32 LE)
                let n_dims = r.meta.shape.len() as u32;
                output.write_all(&n_dims.to_le_bytes())?;
                // dims[n_dims] (u64 LE each)
                for &dim in &r.meta.shape {
                    output.write_all(&dim.to_le_bytes())?;
                }
                // type (u32 LE) — GGUF dtype code
                output.write_all(&r.meta.dtype.to_le_bytes())?;
                // data_offset (u64 LE) — relative to tensor data section start
                output.write_all(&data_off.to_le_bytes())?;
            }

            // ── Phase 5: alignment padding ─────────────────────────────────
            let header_end = output.seek(SeekFrom::Current(0))
                .map_err(ForgeError::WriteError)?;
            let pad_len = pad_to_alignment(header_end, GGUF_ALIGNMENT);
            if pad_len > 0 {
                output.write_all(&vec![0u8; pad_len as usize])?;
            }

            // ── Phase 6: tensor data ───────────────────────────────────────
            let mut bytes_written: u64 = output.seek(SeekFrom::Current(0))
                .map_err(ForgeError::WriteError)?;
            let mut tensors_spliced: u64 = 0;
            let mut spliced_tensors: Vec<SplicedTensorInfo> = Vec::new();

            for r in &resolved {
                debug!(
                    "Splicing tensor '{}' from {} (offset={}, size={}B)",
                    r.segment_name, r.source_gguf, r.meta.offset, r.meta.size
                );

                let source_file = File::open(&r.source_path).map_err(|e| ForgeError::SourceOpen {
                    path: r.source_path.clone(),
                    source: e,
                })?;
                let mmap = unsafe {
                    Mmap::map(&source_file).map_err(|e| ForgeError::MmapFailed {
                        path: r.source_path.clone(),
                        source: e,
                    })?
                };

                let start = r.meta.offset as usize;
                let end   = start + r.meta.size as usize;
                if end > mmap.len() {
                    return Err(ForgeError::OutOfBounds {
                        path: r.source_path.clone(),
                        offset: r.meta.offset,
                        size: r.meta.size,
                        file_len: mmap.len(),
                    });
                }

                output.write_all(&mmap[start..end])?;
                bytes_written += r.meta.size;
                tensors_spliced += 1;
                spliced_tensors.push(SplicedTensorInfo {
                    source: r.source_gguf.clone(),
                    name:   r.segment_name.clone(),
                    size:   r.meta.size,
                    kind:   r.meta.kind.clone(),
                });
            }

            info!(
                "Crystallization complete: {} tensors, {}B → {}",
                tensors_spliced, bytes_written, output_path.display()
            );

            Ok(CrystallizationResult {
                recipe_id: recipe.recipe_id.clone(),
                output_path,
                tensors_spliced,
                bytes_written,
                spliced_tensors,
            })
        })
        .await
        .map_err(|e| ForgeError::WriteError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("spawn_blocking panicked: {}", e),
        )))??;

        self.stats.crystallizations_succeeded += 1;
        self.stats.tensors_spliced += result.tensors_spliced;
        self.stats.bytes_written += result.bytes_written;

        Ok(result)
    }
}

// ────────────────────────────────────────────────────────────────────
// GGUF v3 binary serialization helpers
// ────────────────────────────────────────────────────────────────────

/// Write a GGUF length-prefixed UTF-8 string: u64 LE length + bytes (no NUL).
fn write_gguf_string(w: &mut impl Write, s: &str) -> Result<(), ForgeError> {
    let bytes = s.as_bytes();
    w.write_all(&(bytes.len() as u64).to_le_bytes())?;
    w.write_all(bytes)?;
    Ok(())
}

/// Write a GGUF metadata value: u32 LE type tag + payload.
fn write_gguf_meta_value(w: &mut impl Write, val: &MetaValue) -> Result<(), ForgeError> {
    match val {
        MetaValue::Uint32(v) => {
            w.write_all(&(GgufMetaType::Uint32 as u32).to_le_bytes())?;
            w.write_all(&v.to_le_bytes())?;
        }
        MetaValue::Int32(v) => {
            w.write_all(&(GgufMetaType::Int32 as u32).to_le_bytes())?;
            w.write_all(&v.to_le_bytes())?;
        }
        MetaValue::Float32(v) => {
            w.write_all(&(GgufMetaType::Float32 as u32).to_le_bytes())?;
            w.write_all(&v.to_le_bytes())?;
        }
        MetaValue::Uint64(v) => {
            w.write_all(&(GgufMetaType::Uint64 as u32).to_le_bytes())?;
            w.write_all(&v.to_le_bytes())?;
        }
        MetaValue::Bool(v) => {
            w.write_all(&(GgufMetaType::Bool as u32).to_le_bytes())?;
            w.write_all(&[if *v { 1u8 } else { 0u8 }])?;
        }
        MetaValue::String(s) => {
            w.write_all(&(GgufMetaType::String as u32).to_le_bytes())?;
            write_gguf_string(w, s)?;
        }
        MetaValue::StringArray(arr) => {
            // Array of strings: type=Array(9), array_type=String(8), count u64, then strings
            w.write_all(&(GgufMetaType::Array as u32).to_le_bytes())?;
            w.write_all(&(GgufMetaType::String as u32).to_le_bytes())?; // item type
            w.write_all(&(arr.len() as u64).to_le_bytes())?;
            for s in arr {
                write_gguf_string(w, s)?;
            }
        }
    }
    Ok(())
}

/// Return the number of padding bytes needed to align `pos` to `alignment`.
fn pad_to_alignment(pos: u64, alignment: u64) -> u64 {
    let rem = pos % alignment;
    if rem == 0 { 0 } else { alignment - rem }
}

impl Default for Forge {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of one completed crystallization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizationResult {
    pub recipe_id: String,
    pub output_path: PathBuf,
    pub tensors_spliced: u64,
    pub bytes_written: u64,
    pub spliced_tensors: Vec<SplicedTensorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplicedTensorInfo {
    pub source: String,
    pub name: String,
    pub size: u64,
    pub kind: Option<String>,
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_test_gguf(data: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(data).unwrap();
        f
    }

    fn make_index_with_file(file: &NamedTempFile, tensor_offset: u64, tensor_size: u64) -> GgufIndex {
        let mut index = GgufIndex::new();
        let mut tensors = HashMap::new();
        tensors.insert("test.weight".to_string(), TensorMeta {
            offset: tensor_offset,
            size: tensor_size,
            kind: Some("attention".to_string()),
            shape: vec![4, 4],
            dtype: 0, // F32
        });
        index.register("source.gguf", GgufMeta {
            path: file.path().to_path_buf(),
            tensors,
        });
        index
    }

    fn basic_recipe(segments: Vec<SplicingSegment>) -> ForgeRecipe {
        ForgeRecipe {
            recipe_id: "test".to_string(),
            segments,
            metadata_overrides: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_crystallize_empty_recipe_errors() {
        let mut forge = Forge::new();
        let recipe = ForgeRecipe {
            recipe_id: "empty".to_string(),
            segments: vec![],
            metadata_overrides: HashMap::new(),
        };
        let index = GgufIndex::new();
        let result = forge.crystallize(&recipe, &index, "C:\\Temp\\nope.gguf").await;
        assert!(matches!(result, Err(ForgeError::EmptyRecipe)));
    }

    #[tokio::test]
    async fn test_crystallize_missing_gguf_errors() {
        let mut forge = Forge::new();
        let recipe = basic_recipe(vec![SplicingSegment {
            source_gguf: "nonexistent.gguf".to_string(),
            tensor_name: "some.weight".to_string(),
        }]);
        let index = GgufIndex::new();
        let result = forge.crystallize(&recipe, &index, "C:\\Temp\\out.gguf").await;
        assert!(matches!(result, Err(ForgeError::GgufNotFound { .. })));
    }

    #[tokio::test]
    async fn test_crystallize_missing_tensor_errors() {
        let source = make_test_gguf(b"GGUF fake tensor data here");
        let mut forge = Forge::new();
        let recipe = basic_recipe(vec![SplicingSegment {
            source_gguf: "source.gguf".to_string(),
            tensor_name: "nonexistent.weight".to_string(),
        }]);
        let mut index = GgufIndex::new();
        index.register("source.gguf", GgufMeta {
            path: source.path().to_path_buf(),
            tensors: HashMap::new(),
        });
        let result = forge.crystallize(&recipe, &index, "C:\\Temp\\out.gguf").await;
        assert!(matches!(result, Err(ForgeError::TensorNotFound { .. })));
    }

    #[tokio::test]
    async fn test_crystallize_produces_valid_gguf_v3_output() {
        // Source GGUF: 43 bytes of fake data; tensor data starts at offset 17
        let fake_model_data = b"GGUF_HEADER_HERE_tensor_data_paylo_abc123";
        let source = make_test_gguf(fake_model_data);

        let tensor_offset = 17u64;
        let tensor_size   = 17u64;

        let index = make_index_with_file(&source, tensor_offset, tensor_size);
        let recipe = ForgeRecipe {
            recipe_id: "test-crystallize".to_string(),
            segments: vec![SplicingSegment {
                source_gguf: "source.gguf".to_string(),
                tensor_name: "test.weight".to_string(),
            }],
            metadata_overrides: HashMap::new(),
        };

        let output = NamedTempFile::new().unwrap();
        let output_path = output.path().to_path_buf();

        let mut forge = Forge::new();
        let result = forge.crystallize(&recipe, &index, &output_path).await.unwrap();

        let written = std::fs::read(&output_path).unwrap();

        // ── GGUF v3 header validation ──────────────────────────────────
        assert!(written.starts_with(b"GGUF"), "magic must be GGUF");

        // version = 3 at offset 4
        let version = u32::from_le_bytes(written[4..8].try_into().unwrap());
        assert_eq!(version, 3, "version must be 3");

        // tensor_count = 1 at offset 8
        let tensor_count = u64::from_le_bytes(written[8..16].try_into().unwrap());
        assert_eq!(tensor_count, 1, "tensor_count must be 1");

        // kv_count at offset 16 — must be > 0 (Aaroneous provenance keys present)
        let kv_count = u64::from_le_bytes(written[16..24].try_into().unwrap());
        assert!(kv_count >= 2, "must have at least 2 metadata KV pairs");

        // Total file size > header (data section follows alignment padding)
        assert!(written.len() > 24, "file must be larger than fixed header");

        // Result fields
        assert_eq!(result.tensors_spliced, 1);
        assert!(result.bytes_written > tensor_size, "bytes_written must include header overhead");
        assert_eq!(result.spliced_tensors[0].kind, Some("attention".to_string()));

        // Stats
        assert_eq!(forge.stats.crystallizations_succeeded, 1);
        assert_eq!(forge.stats.tensors_spliced, 1);
    }

    #[tokio::test]
    async fn test_crystallize_multiple_segments() {
        let data_a = b"MODEL_A_weights_payload_here";
        let data_b = b"MODEL_B_weights_payload_here";
        let src_a = make_test_gguf(data_a);
        let src_b = make_test_gguf(data_b);

        let mut index = GgufIndex::new();
        let mut tensors_a = HashMap::new();
        tensors_a.insert("attn.weight".to_string(), TensorMeta {
            offset: 8, size: 7,
            kind: Some("attention".to_string()),
            shape: vec![7],
            dtype: 0,
        });
        let mut tensors_b = HashMap::new();
        tensors_b.insert("mlp.weight".to_string(), TensorMeta {
            offset: 8, size: 7,
            kind: Some("mlp".to_string()),
            shape: vec![7],
            dtype: 0,
        });
        index.register("model_a.gguf", GgufMeta { path: src_a.path().to_path_buf(), tensors: tensors_a });
        index.register("model_b.gguf", GgufMeta { path: src_b.path().to_path_buf(), tensors: tensors_b });

        let recipe = ForgeRecipe {
            recipe_id: "hybrid-v1".to_string(),
            segments: vec![
                SplicingSegment { source_gguf: "model_a.gguf".to_string(), tensor_name: "attn.weight".to_string() },
                SplicingSegment { source_gguf: "model_b.gguf".to_string(), tensor_name: "mlp.weight".to_string() },
            ],
            metadata_overrides: HashMap::new(),
        };

        let output = NamedTempFile::new().unwrap();
        let mut forge = Forge::new();
        let result = forge.crystallize(&recipe, &index, output.path()).await.unwrap();

        assert_eq!(result.tensors_spliced, 2);
        assert_eq!(result.spliced_tensors[0].kind, Some("attention".to_string()));
        assert_eq!(result.spliced_tensors[1].kind, Some("mlp".to_string()));
        assert_eq!(forge.stats.tensors_spliced, 2);
    }

    #[tokio::test]
    async fn test_metadata_overrides_applied() {
        let data = b"QWEN_MODEL_tensor_payload_data_";
        let src = make_test_gguf(data);
        let mut tensors = HashMap::new();
        tensors.insert("tok_embd.weight".to_string(), TensorMeta {
            offset: 11, size: 7, kind: Some("embedding".to_string()),
            shape: vec![7], dtype: 1,
        });
        let mut index = GgufIndex::new();
        index.register("qwen.gguf", GgufMeta { path: src.path().to_path_buf(), tensors });

        let mut overrides = HashMap::new();
        overrides.insert("general.name".to_string(), MetaValue::String("my-qwen-v1".to_string()));
        overrides.insert("llama.context_length".to_string(), MetaValue::Uint32(8192));

        let recipe = ForgeRecipe {
            recipe_id: "qwen-test".to_string(),
            segments: vec![SplicingSegment {
                source_gguf: "qwen.gguf".to_string(),
                tensor_name: "tok_embd.weight".to_string(),
            }],
            metadata_overrides: overrides,
        };

        let output = NamedTempFile::new().unwrap();
        let mut forge = Forge::new();
        let result = forge.crystallize(&recipe, &index, output.path()).await.unwrap();

        // File should be valid GGUF v3
        let written = std::fs::read(output.path()).unwrap();
        assert!(written.starts_with(b"GGUF"));
        let version = u32::from_le_bytes(written[4..8].try_into().unwrap());
        assert_eq!(version, 3);
        assert_eq!(result.recipe_id, "qwen-test");
        assert_eq!(result.tensors_spliced, 1);
    }

    #[test]
    fn test_gguf_index_register_and_len() {
        let mut index = GgufIndex::new();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());

        index.register("a.gguf", GgufMeta { path: PathBuf::from("a.gguf"), tensors: HashMap::new() });
        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
    }

    // ── read_gguf tests ──────────────────────────────────────────────────────

    /// Build a minimal but valid GGUF v3 file in memory for testing the reader.
    fn make_valid_gguf_v3(tensors: &[(&str, &[u64], u32)]) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();

        // magic + version
        buf.extend_from_slice(b"GGUF");
        buf.extend_from_slice(&3u32.to_le_bytes());

        // tensor_count + kv_count (2 KV entries: general.architecture + general.name)
        buf.extend_from_slice(&(tensors.len() as u64).to_le_bytes());
        buf.extend_from_slice(&2u64.to_le_bytes());

        // KV: general.architecture = "qwen2"
        let arch = b"general.architecture";
        buf.extend_from_slice(&(arch.len() as u64).to_le_bytes());
        buf.extend_from_slice(arch);
        buf.extend_from_slice(&8u32.to_le_bytes()); // STRING type
        let arch_val = b"qwen2";
        buf.extend_from_slice(&(arch_val.len() as u64).to_le_bytes());
        buf.extend_from_slice(arch_val);

        // KV: general.name = "test-model"
        let name_key = b"general.name";
        buf.extend_from_slice(&(name_key.len() as u64).to_le_bytes());
        buf.extend_from_slice(name_key);
        buf.extend_from_slice(&8u32.to_le_bytes()); // STRING type
        let name_val = b"test-model";
        buf.extend_from_slice(&(name_val.len() as u64).to_le_bytes());
        buf.extend_from_slice(name_val);

        // Tensor info table: data_offset is relative to tensor data section start
        let mut running_offset = 0u64;
        let mut tensor_sizes: Vec<u64> = Vec::new();
        for (tname, shape, dtype) in tensors {
            let name_bytes = tname.as_bytes();
            buf.extend_from_slice(&(name_bytes.len() as u64).to_le_bytes());
            buf.extend_from_slice(name_bytes);
            buf.extend_from_slice(&(shape.len() as u32).to_le_bytes());
            for &d in *shape {
                buf.extend_from_slice(&d.to_le_bytes());
            }
            buf.extend_from_slice(&dtype.to_le_bytes());
            buf.extend_from_slice(&running_offset.to_le_bytes());
            let sz = gguf_tensor_nbytes(shape, *dtype);
            tensor_sizes.push(sz);
            running_offset += sz;
        }

        // Alignment padding (32-byte boundary)
        let header_end = buf.len() as u64;
        let pad = pad_to_alignment(header_end, 32);
        buf.extend(std::iter::repeat(0u8).take(pad as usize));

        // Tensor data
        for &sz in &tensor_sizes {
            buf.extend(std::iter::repeat(0xABu8).take(sz as usize));
        }

        buf
    }

    #[test]
    fn test_read_gguf_basic() {
        let tensors = &[
            ("token_embd.weight", &[32000u64, 2048][..], 1u32),  // F16
            ("blk.0.attn_q.weight", &[2048u64, 2048][..], 0u32), // F32
        ];
        let data = make_valid_gguf_v3(tensors);

        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &data).unwrap();

        let (index, meta) = read_gguf(tmp.path()).unwrap();

        assert_eq!(meta.version, 3);
        assert_eq!(meta.tensor_count, 2);
        assert_eq!(meta.architecture, "qwen2");
        assert_eq!(meta.model_name, "test-model");

        // Index should have one entry (the file path)
        assert_eq!(index.len(), 1);
        let gguf_meta = index.0.values().next().unwrap();
        assert_eq!(gguf_meta.tensors.len(), 2);

        // token_embd.weight: F16, shape [32000, 2048]
        let embd = &gguf_meta.tensors["token_embd.weight"];
        assert_eq!(embd.dtype, 1);
        assert_eq!(embd.shape, vec![32000, 2048]);
        assert_eq!(embd.size, 32000 * 2048 * 2); // F16 = 2 bytes/element

        // attn_q.weight: F32, shape [2048, 2048]
        let attn = &gguf_meta.tensors["blk.0.attn_q.weight"];
        assert_eq!(attn.dtype, 0);
        assert_eq!(attn.shape, vec![2048, 2048]);
        assert_eq!(attn.size, 2048 * 2048 * 4); // F32 = 4 bytes/element
    }

    #[test]
    fn test_read_gguf_bad_magic() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"NOT_A_GGUF_FILE").unwrap();
        let result = read_gguf(tmp.path());
        assert!(matches!(result, Err(GgufReadError::BadMagic { .. })));
    }

    #[test]
    fn test_read_gguf_offsets_are_absolute() {
        // Single F32 tensor with shape [4], data is 16 bytes
        let tensors = &[("test.weight", &[4u64][..], 0u32)];
        let data = make_valid_gguf_v3(tensors);

        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &data).unwrap();

        let (index, _meta) = read_gguf(tmp.path()).unwrap();
        let gguf_meta = index.0.values().next().unwrap();
        let tm = &gguf_meta.tensors["test.weight"];

        // Offset must point into the data section, past the header
        // The header is at minimum 4+4+8+8 = 24 bytes, plus KV data, plus alignment
        assert!(tm.offset >= 24, "offset {} must be past fixed header", tm.offset);

        // Verify: reading size bytes at offset from the raw file gives 0xAB pattern
        let file_data = std::fs::read(tmp.path()).unwrap();
        let start = tm.offset as usize;
        let end = start + tm.size as usize;
        assert!(end <= file_data.len(), "tensor data out of file bounds");
        assert_eq!(file_data[start], 0xAB, "expected fill byte at tensor start");
    }

    #[tokio::test]
    async fn test_roundtrip_read_then_crystallize() {
        // 1. Forge a valid GGUF v3 with one tensor
        let tensors = &[("blk.0.attn_q.weight", &[8u64, 8][..], 0u32)];
        let src_data = make_valid_gguf_v3(tensors);
        let src = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(src.path(), &src_data).unwrap();

        // 2. Parse it with read_gguf → GgufIndex
        let (index, meta) = read_gguf(src.path()).unwrap();
        assert_eq!(meta.tensor_count, 1);

        // 3. Build a recipe referencing that tensor by name
        let src_logical_name = src.path().to_string_lossy().to_string();
        let recipe = ForgeRecipe {
            recipe_id: "roundtrip-test".to_string(),
            segments: vec![SplicingSegment {
                source_gguf: src_logical_name,
                tensor_name: "blk.0.attn_q.weight".to_string(),
            }],
            metadata_overrides: HashMap::new(),
        };

        // 4. Crystallize
        let out = tempfile::NamedTempFile::new().unwrap();
        let mut forge = Forge::new();
        let result = forge.crystallize(&recipe, &index, out.path()).await.unwrap();

        assert_eq!(result.tensors_spliced, 1);

        // 5. The output must itself be a valid GGUF v3
        let (out_index, out_meta) = read_gguf(out.path()).unwrap();
        assert_eq!(out_meta.version, 3);
        assert_eq!(out_meta.tensor_count, 1);
        let out_tm = &out_index.0.values().next().unwrap().tensors["blk.0.attn_q.weight"];
        assert_eq!(out_tm.shape, vec![8, 8]);
        assert_eq!(out_tm.dtype, 0); // F32
    }
}
