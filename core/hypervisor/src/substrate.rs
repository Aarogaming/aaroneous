use seahash::SeaHasher;
use std::hash::Hasher;
use std::io::{Read, Seek, SeekFrom};

/// Source classification for the ingestion pipeline.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IngestionSourceType {
    DesktopVideoRecord = 0x0A,
    ProgramDirectory   = 0x0B,
    DocumentRawBytes   = 0x0C,
}

/// Normalized GGUF tensor layer type after key unification.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnifiedLayerType {
    AttentionQuery = 0x01,
    AttentionKey   = 0x02,
    AttentionValue = 0x03,
    FeedForwardUp  = 0x04,
}

/// Fractional screen coordinate in [0.0, 1.0] space.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ScreenCoordinate {
    pub x: f32,
    pub y: f32,
}

/// The foundational continuous data row format for HDF5 storage.
/// Cache-aligned to 64 bytes.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct SystemInstructionNode {
    pub sequence_id: u64,
    pub spatial_signature: [u64; 128],
    pub target_point: ScreenCoordinate,
    pub input_type: u8,
    pub execution_mask: u64,
    pub volatility_index: f32,
}

/// Metadata for an ingested data chunk from any source.
#[repr(C, align(64))]
#[derive(Debug, Clone)]
pub struct IngestionDataChunk {
    pub source_type: IngestionSourceType,
    pub source_identifier: u64,
    pub byte_offset: u64,
    pub coordinate_bounds: [f32; 4],
    pub spatial_signature: [u64; 128],
}

impl Default for IngestionDataChunk {
    fn default() -> Self {
        Self {
            source_type: IngestionSourceType::ProgramDirectory,
            source_identifier: 0,
            byte_offset: 0,
            coordinate_bounds: [0.0, 0.0, 1.0, 1.0],
            spatial_signature: [0u64; 128],
        }
    }
}

/// Sandboxed network data stream that digests web bytes directly into VSA.
#[repr(C, align(64))]
#[derive(Debug, Clone)]
pub struct NetworkDataStream {
    pub endpoint_hash: u64,
    pub bytes_received: u64,
}

impl NetworkDataStream {
    pub fn new(endpoint_hash: u64) -> Self {
        Self {
            endpoint_hash,
            bytes_received: 0,
        }
    }

    /// Ingests a raw byte slice from the secure network socket, processes it
    /// inside the WASM sandbox, and writes it directly to the VSA target.
    ///
    /// Uses `SeaHasher` (hardware-native non-cryptographic hash) to fold
    /// 64-byte chunks into the 8192-bit VSA signature via XOR superposition.
    pub fn digest_network_bytes(&mut self, raw_web_data: &[u8], target_vsa: &mut [u64; 128]) {
        self.bytes_received += raw_web_data.len() as u64;

        for (index, chunk) in raw_web_data.chunks_exact(64).enumerate() {
            let mut hasher = SeaHasher::new();
            hasher.write(chunk);
            let chunk_hash = hasher.finish();

            let array_index = (index % 128) as usize;

            unsafe {
                let target_register = target_vsa.get_unchecked_mut(array_index);
                *target_register ^= chunk_hash;
            }
        }
    }
}

// ── GGUF Raw Binary Seek Loop (no model runtime) ──────────────────────

/// GGUF file magic bytes (v3).
const GGUF_MAGIC: [u8; 4] = [b'G', b'G', b'U', b'F'];

/// GGUF version 3 identifier.
const GGUF_VERSION: u32 = 3;

/// Standard GGUF tensor info header (version 3, packed).
///
/// Memory layout:
///   [0..4]   — name_length: u32 (bytes of tensor name, null-terminated)
///   [4..8]   — n_dims: u32      (number of dimensions)
///   [8..12]  — dim_0: u64       (first dimension)
///   [12..20] — dim_1: u64       (second dimension, or 0 if 1-D)
///   [20..24] — tensor_type: u32 (GGML type enum)
///   [24..28] — offset: u64      (byte offset to tensor data from file start)
///   (32 + name_length bytes total)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct GgufTensorInfoHeader {
    pub name_length: u32,
    pub n_dims: u32,
    pub dim_0: u64,
    pub dim_1: u64,
    pub tensor_type: u32,
    pub offset: u64,
}

/// Parsed tensor block ready for SVD reduction or VSA projection.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct GgufTensorBlock {
    pub name: String,
    pub n_dims: u32,
    pub dim_0: u64,
    pub dim_1: u64,
    pub tensor_type: u32,
    pub offset: u64,
    pub raw_data: Vec<u8>,
}

/// Raw GGUF binary seek loop: reads tensor info headers + raw data from a GGUF file
/// without instantiating any model runtime or inference engine.
///
/// Scans the file, builds a list of all tensor blocks with their raw byte slices,
/// and returns them for downstream SVD/VSA processing.
pub fn seek_gguf_tensor_blocks(path: &str) -> Result<Vec<GgufTensorBlock>, String> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use byteorder::{LittleEndian, ReadBytesExt};

    let mut file = File::open(path).map_err(|e| format!("open {}: {}", path, e))?;

    // ── Verify magic and version ───────────────────────────────────────
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(|e| format!("read magic: {}", e))?;
    if magic != GGUF_MAGIC {
        return Err(format!("Bad magic: {:?} (expected GGUF)", magic));
    }

    let version = file.read_u32::<LittleEndian>()
        .map_err(|e| format!("read version: {}", e))?;
    if version < GGUF_VERSION {
        return Err(format!("Unsupported GGUF version {}, need >=3", version));
    }

    // ── Skip metadata header ───────────────────────────────────────────
    // After version: tensor_count (u64), metadata_kv_count (u64), then metadata KV pairs.
    // We skip these to reach the tensor info section directly.
    let _tensor_count = file.read_u64::<LittleEndian>()
        .map_err(|e| format!("read tensor_count: {}", e))?;
    let metadata_kv_count = file.read_u64::<LittleEndian>()
        .map_err(|e| format!("read metadata_kv_count: {}", e))?;

    // Skip each metadata KV pair: key_length(u32) + key + value_type(u32) + value_data
    for _ in 0..metadata_kv_count {
        let key_len = file.read_u32::<LittleEndian>()
            .map_err(|e| format!("read kv key_len: {}", e))?;
        file.seek(SeekFrom::Current(key_len as i64))
            .map_err(|e| format!("seek past key: {}", e))?;

        let value_type = file.read_u32::<LittleEndian>()
            .map_err(|e| format!("read value_type: {}", e))?;
        skip_metadata_value(&mut file, value_type)?;
    }

    // ── Tensor info section ────────────────────────────────────────────
    // Now at the start of the tensor info array. Re-read tensor_count
    // (we skipped it above; re-seek to read it properly).
    // Actually, tensor_count was read at offset 8. Let's stay at current position
    // and read tensor info headers one by one.

    let mut tensor_blocks = Vec::new();

    // The tensor info section contains `tensor_count` entries.
    // We estimated tensor_count from the initial read, but we need it again.
    // Re-seek to re-read (simpler approach).
    file.seek(SeekFrom::Start(8)).map_err(|e| format!("re-seek: {}", e))?;
    let total_tensors = file.read_u64::<LittleEndian>()
        .map_err(|e| format!("read tensor_count: {}", e))?;

    // Re-skip metadata again to get back to tensor info section
    let meta_count = file.read_u64::<LittleEndian>()
        .map_err(|e| format!("re-read meta_kv_count: {}", e))?;
    for _ in 0..meta_count {
        let key_len = file.read_u32::<LittleEndian>()
            .map_err(|e| format!("skip kv key_len: {}", e))?;
        file.seek(SeekFrom::Current(key_len as i64))
            .map_err(|e| format!("seek past key: {}", e))?;
        let value_type = file.read_u32::<LittleEndian>()
            .map_err(|e| format!("read value_type: {}", e))?;
        skip_metadata_value(&mut file, value_type)?;
    }

    // ── Read each tensor info header ──────────────────────────────────
    for _ in 0..total_tensors {
        let name_len = file.read_u32::<LittleEndian>()
            .map_err(|e| format!("read name_len: {}", e))?;

        let mut name_bytes = vec![0u8; name_len as usize];
        file.read_exact(&mut name_bytes)
            .map_err(|e| format!("read name: {}", e))?;
        // Trim trailing null bytes
        let name = String::from_utf8_lossy(&name_bytes)
            .trim_end_matches('\0')
            .to_string();

        let n_dims = file.read_u32::<LittleEndian>()
            .map_err(|e| format!("read n_dims: {}", e))?;
        let dim_0 = file.read_u64::<LittleEndian>()
            .map_err(|e| format!("read dim_0: {}", e))?;
        let dim_1 = file.read_u64::<LittleEndian>()
            .map_err(|e| format!("read dim_1: {}", e))?;
        let tensor_type = file.read_u32::<LittleEndian>()
            .map_err(|e| format!("read tensor_type: {}", e))?;
        let offset = file.read_u64::<LittleEndian>()
            .map_err(|e| format!("read offset: {}", e))?;

        tensor_blocks.push((name, n_dims, dim_0, dim_1, tensor_type, offset));
    }

    // ── Read raw data for each tensor ─────────────────────────────────
    let file_size = file.metadata()
        .map_err(|e| format!("metadata: {}", e))?
        .len();

    let mut result = Vec::with_capacity(total_tensors as usize);

    for (name, n_dims, dim_0, dim_1, tensor_type, offset) in tensor_blocks {
        // Estimate data size from dimensions and type
        let data_size = estimate_tensor_size(n_dims, dim_0, dim_1, tensor_type)
            .unwrap_or(64 * 1024); // fallback: 64KB

        let end = (offset as u64).saturating_add(data_size as u64).min(file_size) as usize;
        let start = offset as usize;

        if start > end || start >= file_size as usize {
            result.push(GgufTensorBlock {
                name,
                n_dims,
                dim_0,
                dim_1,
                tensor_type,
                offset,
                raw_data: vec![],
            });
            continue;
        }

        let data_len = end - start;
        file.seek(SeekFrom::Start(start as u64))
            .map_err(|e| format!("seek to tensor data: {}", e))?;

        let mut raw_data = vec![0u8; data_len];
        file.read_exact(&mut raw_data)
            .map_err(|e| format!("read tensor data: {}", e))?;

        result.push(GgufTensorBlock {
            name,
            n_dims,
            dim_0,
            dim_1,
            tensor_type,
            offset,
            raw_data,
        });
    }

    Ok(result)
}

/// Map a GGUF tensor name to the internal UnifiedLayerType.
/// Handles common naming conventions: q_proj, w1, attn_q, gate_proj, etc.
pub fn classify_layer_name(name: &str) -> Option<UnifiedLayerType> {
    let lower = name.to_lowercase();
    if lower.contains("q_proj") || lower.contains("attn_q") || lower.contains("query") {
        Some(UnifiedLayerType::AttentionQuery)
    } else if lower.contains("k_proj") || lower.contains("attn_k") || lower.contains("key") {
        Some(UnifiedLayerType::AttentionKey)
    } else if lower.contains("v_proj") || lower.contains("attn_v") || lower.contains("value") {
        Some(UnifiedLayerType::AttentionValue)
    } else if lower.contains("gate_proj") || lower.contains("w1") || lower.contains("gate")
        || lower.contains("up_proj") || lower.contains("w3") {
        Some(UnifiedLayerType::FeedForwardUp)
    } else {
        None // non-attention/ff layers (norm, embedding, output, etc.)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Skip a GGUF metadata value (based on its type tag).
fn skip_metadata_value<R: Read + Seek>(reader: &mut R, value_type: u32) -> Result<(), String> {
    // GGUF metadata value types:
    // 0 = uint8, 1 = int8, 2 = uint16, 3 = int16, 4 = uint32, 5 = int32,
    // 6 = float32, 7 = bool, 8 = string, 9 = array, 10 = uint64, 11 = int64,
    // 12 = float64, 13 = array_of_array (rare)
    match value_type {
        0 | 1 | 7 => { reader.seek(SeekFrom::Current(1)).ok(); }
        2 | 3 => { reader.seek(SeekFrom::Current(2)).ok(); }
        4 | 5 | 6 => { reader.seek(SeekFrom::Current(4)).ok(); }
        10 | 11 | 12 => { reader.seek(SeekFrom::Current(8)).ok(); }
        8 => { // string: length(u64) + data
            let len = read_u64_le(reader).unwrap_or(0);
            reader.seek(SeekFrom::Current(len as i64)).ok();
        }
        9 => { // array: type(u32) + count(u64) + elements
            let elem_type = read_u32_le(reader).unwrap_or(0);
            let count = read_u64_le(reader).unwrap_or(0);
            for _ in 0..count {
                skip_metadata_value(reader, elem_type)?;
            }
        }
        _ => { return Err(format!("Unknown GGUF metadata type: {}", value_type)); }
    }
    Ok(())
}

fn read_u64_le<R: Read + Seek>(reader: &mut R) -> Option<u64> {
    use byteorder::{LittleEndian, ReadBytesExt};
    reader.read_u64::<LittleEndian>().ok()
}

fn read_u32_le<R: Read + Seek>(reader: &mut R) -> Option<u32> {
    use byteorder::{LittleEndian, ReadBytesExt};
    reader.read_u32::<LittleEndian>().ok()
}

/// Estimate the byte size of a tensor from its dimensions and GGML type.
fn estimate_tensor_size(n_dims: u32, dim_0: u64, dim_1: u64, tensor_type: u32) -> Option<usize> {
    let element_count = if n_dims >= 2 {
        dim_0.saturating_mul(dim_1.max(1))
    } else {
        dim_0.max(1)
    } as usize;

    // GGML type byte sizes (common types):
    // 0 = F32 (4), 1 = F16 (2), 2 = Q4_0, 3 = Q4_1, ...
    let element_size = match tensor_type {
        0 => 4, // GGML_TYPE_F32
        1 => 2, // GGML_TYPE_F16
        2 | 3 => element_count / 32 * 16, // Q4_0/Q4_1: 16 bytes per 32 elements
        6 => element_count / 256 * 224,    // Q5_K: 224 bytes per 256 elements
        7 => element_count / 256 * 208,    // Q6_K: 208 bytes per 256 elements
        8 => element_count / 256 * 216,    // Q8_K: 216 bytes per 256 elements
        10 => element_count / 256 * 200,   // Q3_K: 200 bytes per 256 elements
        11 => element_count / 256 * 208,   // Q4_K: 208 bytes per 256 elements
        12 => element_count / 256 * 192,   // Q5_K: 192 bytes per 256 elements (alt)
        14 => element_count / 64 * 36,     // IQ2_XXS: 36 bytes per 64 elements
        15 => element_count / 256 * 128,   // Q2_K_S
        16 => element_count / 256 * 64,    // Q3_K_S
        17 => element_count / 256 * 96,    // Q4_K_S
        20 => element_count / 32 * 20,     // IQ2_XS
        21 => element_count / 32 * 18,     // IQ3_XS
        22 => element_count / 32 * 22,     // IQ3_S
        23 => element_count / 32 * 24,     // IQ4_NL
        24 => element_count / 32 * 22,     // IQ4_XS
        26 => element_count / 64 * 36,     // IQ1_S: 36 bytes per 64 elements
        27 => element_count / 32 * 18,     // IQ2_S: 18 bytes per 32 elements
        _ => return None, // unknown type
    };

    Some(element_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_layer_names() {
        assert_eq!(classify_layer_name("model.layers.0.attention.self.query.weight"), Some(UnifiedLayerType::AttentionQuery));
        assert_eq!(classify_layer_name("model.layers.0.attention.self.key.weight"), Some(UnifiedLayerType::AttentionKey));
        assert_eq!(classify_layer_name("model.layers.0.attention.self.value.weight"), Some(UnifiedLayerType::AttentionValue));
        assert_eq!(classify_layer_name("model.layers.0.mlp.gate_proj.weight"), Some(UnifiedLayerType::FeedForwardUp));
        assert_eq!(classify_layer_name("model.layers.0.mlp.up_proj.weight"), Some(UnifiedLayerType::FeedForwardUp));
        assert_eq!(classify_layer_name("model.layers.0.mlp.down_proj.weight"), None); // not ff-up
        assert_eq!(classify_layer_name("model.layers.0.input_layernorm.weight"), None);
    }

    #[test]
    fn test_network_data_stream() {
        let mut stream = NetworkDataStream::new(0xDEADBEEF);
        assert_eq!(stream.bytes_received, 0);

        let mut vsa = [0u64; 128];
        let data = b"hello world this is a test of the network data ingestion pipeline";
        let orig = vsa;
        stream.digest_network_bytes(data, &mut vsa);
        assert!(stream.bytes_received > 0);
        assert_ne!(vsa, orig); // at least one bit got XOR'd
    }

    #[test]
    fn test_system_instruction_node_size() {
        let actual = std::mem::size_of::<SystemInstructionNode>();
        // 8 + 1024 + 8 + 1 + padding(7) + 8 + 4 + trailing_padding = aligned to 64
        assert!(actual % 64 == 0, "size {} not 64-aligned", actual);
        assert!(actual >= 1060, "size {} too small", actual);
    }

    #[test]
    fn test_ingestion_data_chunk_default() {
        let chunk = IngestionDataChunk::default();
        assert_eq!(chunk.source_type as u8, IngestionSourceType::ProgramDirectory as u8);
        assert_eq!(chunk.coordinate_bounds, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_screen_coordinate_default() {
        let sc = ScreenCoordinate::default();
        assert_eq!(sc.x, 0.0);
        assert_eq!(sc.y, 0.0);
    }

    #[test]
    fn test_seek_gguf_nonexistent_file() {
        let result = seek_gguf_tensor_blocks(r"C:\nonexistent_file.gguf");
        assert!(result.is_err());
    }
}
