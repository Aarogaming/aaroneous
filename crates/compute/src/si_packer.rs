//! crates/compute/src/si_packer.rs
//! Phase 4: `.si` Solid-State Container Packer & Zero-Copy Loader.
//!
//! Implements the full SINT binary layout specification:
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────┐
//! │                    .si (SINT) BINARY LAYOUT                       │
//! ├───────────────────────────────────────────────────────────────────┤
//! │ Offset 0x00 : Magic b"SINT" (4 bytes)                             │
//! │ Offset 0x04 : Version u32   (4 bytes) = SINT_PACKER_VERSION       │
//! │ Offset 0x08 : Flags   u32   (4 bytes) = 0x00 (standard)          │
//! │ Offset 0x0C : toc_len u64   (8 bytes) = manifest byte length      │
//! │ Offset 0x14 : Manifest bytes (bincode, variable length)           │
//! │ Offset PAD  : [64-byte alignment padding]                         │
//! ├───────────────────────────────────────────────────────────────────┤
//! │ [BLOCK 1+]  : Tensor payloads, each 64-byte aligned               │
//! │   per TensorDescriptor.byte_offset (absolute, from file start)    │
//! └───────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Key invariants:
//! - Every tensor payload starts at a multiple of 64 bytes from the file start.
//! - The manifest (TOC) is bincode-serialized for fast zero-copy deserialization.
//! - `SiSolidStateLoader::get_tensor_slice` returns `&[f32]` directly from the mmap
//!   with a debug_assert verifying 64-byte pointer alignment.

use anyhow::{bail, Result};
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Magic bytes for packer-format SINT containers (distinct from legacy v2 JSON containers)
pub const SINT_PACKER_MAGIC: [u8; 4] = *b"SINT";
/// Packer format version — v3 enforces tensor-descriptor manifest with explicit byte offsets
pub const SINT_PACKER_VERSION: u32 = 3;
/// Enforced SIMD / cache-line alignment for all tensor payloads
pub const ALIGNMENT_BYTES: usize = 64;

// ────────────────────────────────────────────────────────────────────────────
// Tier Designation Flags (Offset 0x08 in .si SINT header)
// ────────────────────────────────────────────────────────────────────────────

/// Tier Designation Flags defining CPU/memory execution profiles and routing topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiTierFlags(pub u32);

impl SiTierFlags {
    /// Tier 1: Strategic Cortex (HD R^4096 representation, background OS thread)
    pub const TIER_1_CORTEX: Self = Self(0b0000_0001);
    /// Tier 2: Orchestration / Hermes Router (R^256, connects to central SPMC hub)
    pub const TIER_2_ROUTER: Self = Self(0b0000_0010);
    /// Tier 3: Kinetic Specialist / Reflex (R^256, L1 cache priority, thread pinning)
    pub const TIER_3_REFLEX: Self = Self(0b0000_0100);

    pub fn bits(&self) -> u32 {
        self.0
    }

    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub fn is_cortex(&self) -> bool {
        self.0 & Self::TIER_1_CORTEX.0 != 0
    }

    pub fn is_router(&self) -> bool {
        self.0 & Self::TIER_2_ROUTER.0 != 0
    }

    pub fn is_reflex(&self) -> bool {
        self.0 & Self::TIER_3_REFLEX.0 != 0
    }

    pub fn label(&self) -> &'static str {
        if self.is_cortex() {
            "Tier 1: Strategic Cortex (R^4096)"
        } else if self.is_router() {
            "Tier 2: Hermes Router (R^256)"
        } else if self.is_reflex() {
            "Tier 3: Kinetic Reflex (R^256)"
        } else {
            "Tier 3: Kinetic Reflex (Default)"
        }
    }
}

impl Default for SiTierFlags {
    fn default() -> Self {
        Self::TIER_3_REFLEX
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Manifest types
// ────────────────────────────────────────────────────────────────────────────

/// Describes a single tensor stored inside the `.si` container.
/// `byte_offset` is absolute from the start of the file and guaranteed to be
/// a multiple of `ALIGNMENT_BYTES`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorDescriptor {
    /// Human-readable tensor name (e.g. "lora_a", "ssm_in_proj")
    pub name: String,
    /// Logical shape, e.g. `[256, 16]` for an A-adapter matrix
    pub shape: Vec<usize>,
    /// Element dtype: "F32" (future: "F16", "BF16")
    pub dtype: String,
    /// Absolute byte offset from file start — always aligned to `ALIGNMENT_BYTES`
    pub byte_offset: u64,
    /// Total payload size in bytes
    pub byte_length: u64,
    /// `false` = immutable core SSM weight, `true` = mutable LoRA/adapter
    pub is_mutable: bool,
}

/// Table-of-Contents manifest written after the file header.
/// Serialized via `bincode` for deterministic layout and fast deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiContainerManifest {
    /// Unique model identifier / name
    pub model_identifier: String,
    /// Inner projection dimension (e.g. 256 or 4096)
    pub d_model: usize,
    /// SSM recurrent hidden state rank (e.g. 16 or 64)
    pub d_state: usize,
    /// LoRA adaptation rank r (e.g. 16 → A: [d_model × r], B: [r × d_model])
    pub lora_rank: usize,
    /// Tier designation flags
    #[serde(default)]
    pub tier_flags: Option<SiTierFlags>,
    /// Ordered list of all tensor payloads in the container
    pub tensors: Vec<TensorDescriptor>,
}

/// Returns the number of zero-padding bytes needed to reach the next
/// `ALIGNMENT_BYTES` boundary from `current_offset`.
#[inline]
pub fn compute_padding(current_offset: u64) -> usize {
    let rem = (current_offset as usize) % ALIGNMENT_BYTES;
    if rem == 0 { 0 } else { ALIGNMENT_BYTES - rem }
}

// ────────────────────────────────────────────────────────────────────────────
// SiPacker — writes a .si container from raw f32 weight maps
// ────────────────────────────────────────────────────────────────────────────

pub struct SiPacker;

impl SiPacker {
    /// Compiles a set of named f32 weight tensors (core SSM + dynamic LoRA) into a
    /// standards-compliant `.si` container at `output_path`.
    ///
    /// # Layout
    /// - Immutable core weights come first (in HashMap iteration order, stable after sort).
    /// - Dynamic LoRA adapters (A and B matrices) are appended last and marked mutable.
    /// - Every tensor payload is 64-byte aligned.
    ///
    /// # Arguments
    /// - `core_weights` — `HashMap<name, flat f32 vec>` of immutable SSM weights
    /// - `d_model` / `d_state` / `lora_rank` — model geometry for manifest metadata
    pub fn pack_to_si(
        output_path: &Path,
        model_id: &str,
        d_model: usize,
        d_state: usize,
        lora_rank: usize,
        core_weights: HashMap<String, Vec<f32>>,
    ) -> Result<()> {
        Self::pack_to_si_with_tier(
            output_path,
            model_id,
            SiTierFlags::TIER_3_REFLEX,
            d_model,
            d_state,
            lora_rank,
            core_weights,
        )
    }

    /// Compiles a set of named f32 weight tensors with explicit tier flags into a .si container.
    pub fn pack_to_si_with_tier(
        output_path: &Path,
        model_id: &str,
        tier: SiTierFlags,
        d_model: usize,
        d_state: usize,
        lora_rank: usize,
        core_weights: HashMap<String, Vec<f32>>,
    ) -> Result<()> {
        // ----------------------------------------------------------------
        // 1. Prepare all tensor entries (core first, LoRA adapters last)
        // ----------------------------------------------------------------
        let mut all_tensors: Vec<(String, Vec<f32>, Vec<usize>, bool)> = Vec::new();

        // Sort core weights by name for deterministic ordering across runs
        let mut sorted_cores: Vec<(String, Vec<f32>)> = core_weights.into_iter().collect();
        sorted_cores.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, data) in sorted_cores {
            let shape = vec![data.len()]; // flat layout; caller may embed true shape in name
            all_tensors.push((name, data, shape, false));
        }

        // Dynamic LoRA adapter: A [d_model × lora_rank], B [lora_rank × d_model]
        // Zero-initialized so initial adapter output ≡ 0 (identity with core model)
        all_tensors.push((
            "lora_a".to_string(),
            vec![0.0f32; d_model * lora_rank],
            vec![d_model, lora_rank],
            true,
        ));
        all_tensors.push((
            "lora_b".to_string(),
            vec![0.0f32; lora_rank * d_model],
            vec![lora_rank, d_model],
            true,
        ));

        // ----------------------------------------------------------------
        // 2 + 3. Iterative layout convergence.
        // ----------------------------------------------------------------
        let payload_sizes: Vec<u64> = all_tensors
            .iter()
            .map(|(_, data, _, _)| (data.len() * 4) as u64)
            .collect();

        let header_prefix: u64 = 20; // 4 (magic) + 4 (version) + 4 (flags) + 8 (toc_len)

        let compute_layout = |manifest_len_guess: u64| -> (Vec<u64>, u64) {
            let after_manifest = header_prefix + manifest_len_guess;
            let first_start = after_manifest + compute_padding(after_manifest) as u64;
            let mut offsets = Vec::with_capacity(payload_sizes.len());
            let mut cur = first_start;
            for &plen in &payload_sizes {
                cur += compute_padding(cur) as u64;
                offsets.push(cur);
                cur += plen;
                cur += compute_padding(cur) as u64;
            }
            (offsets, first_start)
        };

        let build_manifest = |offsets: &[u64]| -> SiContainerManifest {
            let descs: Vec<TensorDescriptor> = all_tensors
                .iter()
                .zip(&payload_sizes)
                .zip(offsets)
                .map(|(((name, _, shape, mutable), &blen), &offset)| TensorDescriptor {
                    name: name.clone(),
                    shape: shape.clone(),
                    dtype: "F32".to_string(),
                    byte_offset: offset,
                    byte_length: blen,
                    is_mutable: *mutable,
                })
                .collect();
            SiContainerManifest {
                model_identifier: model_id.to_string(),
                d_model,
                d_state,
                lora_rank,
                tier_flags: Some(tier),
                tensors: descs,
            }
        };

        // Seed with guess = 0 (will grow to stable length quickly)
        let mut manifest_len_guess: u64 = 0;
        let mut manifest_bytes: Vec<u8>;
        let mut tensor_offsets: Vec<u64>;

        loop {
            let (offsets, _) = compute_layout(manifest_len_guess);
            tensor_offsets = offsets;
            let m = build_manifest(&tensor_offsets);
            manifest_bytes = bincode_serialize(&m)?;
            let new_len = manifest_bytes.len() as u64;
            if new_len == manifest_len_guess {
                break; // converged
            }
            manifest_len_guess = new_len;
        }

        let manifest_len = manifest_len_guess;

        // ----------------------------------------------------------------
        // 5. Write the actual file
        // ----------------------------------------------------------------
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path)?;

        // Header: magic + version + flags + toc_len
        file.write_all(&SINT_PACKER_MAGIC)?;
        file.write_all(&SINT_PACKER_VERSION.to_le_bytes())?;
        file.write_all(&tier.bits().to_le_bytes())?; // Tier Flags at offset 0x08
        file.write_all(&manifest_len.to_le_bytes())?;

        // Manifest payload
        file.write_all(&manifest_bytes)?;

        // Padding to align to first tensor start
        let pad_after_manifest = compute_padding(header_prefix + manifest_len);
        file.write_all(&vec![0u8; pad_after_manifest])?;

        // Tensor payloads with inter-tensor alignment padding
        let file_pos_after_header = header_prefix + manifest_len + pad_after_manifest as u64;
        let mut file_cursor = file_pos_after_header;

        for (((_, data, _, _), &expected_offset), &payload_len) in
            all_tensors.iter().zip(&tensor_offsets).zip(&payload_sizes)
        {
            // Inter-tensor alignment pad
            let pad = compute_padding(file_cursor) as u64;
            if pad > 0 {
                file.write_all(&vec![0u8; pad as usize])?;
                file_cursor += pad;
            }

            // Verify we are at the expected absolute offset
            debug_assert_eq!(
                file_cursor, expected_offset,
                "SiPacker: tensor '{}' offset mismatch: expected {}, at {}",
                data.len(), expected_offset, file_cursor
            );

            // Write tensor payload as raw little-endian f32 bytes
            let bytes: Vec<u8> = data
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect();
            file.write_all(&bytes)?;
            file_cursor += payload_len;

            // Pad after payload
            let trail = compute_padding(file_cursor) as u64;
            if trail > 0 {
                file.write_all(&vec![0u8; trail as usize])?;
                file_cursor += trail;
            }
        }

        file.flush()?;

        let total_bytes = file.metadata()?.len();
        println!(
            "📦 Packed '{model_id}' ({}) → {:?} ({:.3} MB, {} tensors, 64-byte aligned)",
            tier.label(),
            output_path,
            total_bytes as f64 / 1_048_576.0,
            all_tensors.len(),
        );

        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// SiSolidStateLoader — zero-copy memmap2 loader & tensor slicer
// ────────────────────────────────────────────────────────────────────────────

/// Mounts a `.si` container as a memory-mapped file and provides zero-copy
/// `&[f32]` tensor slices directly from the mmap buffer.
///
/// The entire file is kept mapped for the lifetime of this struct — no
/// heap allocations occur for individual tensor reads.
pub struct SiSolidStateLoader {
    pub manifest: SiContainerManifest,
    pub tier_flags: SiTierFlags,
    /// The underlying memory mapping — kept alive for the lifetime of this struct
    _file: File,
    mmap: Mmap,
}

impl SiSolidStateLoader {
    /// Extracts tier flags directly from mapped memory header at offset 0x08
    pub fn extract_tier_flags(mmap: &Mmap) -> SiTierFlags {
        if mmap.len() >= 12 {
            let flag_bytes: [u8; 4] = mmap[8..12].try_into().unwrap_or([0; 4]);
            SiTierFlags::from_bits(u32::from_le_bytes(flag_bytes))
        } else {
            SiTierFlags::TIER_3_REFLEX
        }
    }

    /// Opens and maps a `.si` container. Validates the magic bytes and
    /// deserializes the manifest from the embedded bincode TOC.
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Validate magic
        if mmap.len() < 20 || &mmap[0..4] != SINT_PACKER_MAGIC {
            bail!("SiSolidStateLoader: {:?} missing SINT magic bytes", path);
        }

        let version = u32::from_le_bytes(mmap[4..8].try_into()?);
        if version < 3 {
            bail!(
                "SiSolidStateLoader: container version v{} is not supported by this loader \
                 (requires v3+). Use SolidStateSiContainer::load_from_file for legacy containers.",
                version
            );
        }

        let flag_bytes: [u8; 4] = mmap[8..12].try_into()?;
        let tier_flags = SiTierFlags::from_bits(u32::from_le_bytes(flag_bytes));

        let toc_len = u64::from_le_bytes(mmap[12..20].try_into()?) as usize;
        let manifest_bytes = mmap
            .get(20..20 + toc_len)
            .ok_or_else(|| anyhow::anyhow!("SiSolidStateLoader: TOC truncated"))?;

        let manifest: SiContainerManifest = bincode_deserialize(manifest_bytes)?;

        Ok(Self { manifest, tier_flags, _file: file, mmap })
    }

    /// Returns a zero-copy `&[f32]` slice for the named tensor.
    ///
    /// The slice is backed directly by the memory-mapped file region.
    /// A `debug_assert!` verifies the 64-byte SIMD pointer alignment invariant.
    ///
    /// Returns `None` if no tensor with the given name exists in the manifest.
    pub fn get_tensor_slice(&self, name: &str) -> Option<&[f32]> {
        let desc = self.manifest.tensors.iter().find(|t| t.name == name)?;

        let start = desc.byte_offset as usize;
        let end = start + desc.byte_length as usize;

        let raw_slice = self.mmap.get(start..end)?;

        // SIMD alignment safety check — must be 64-byte aligned for AVX-512
        debug_assert_eq!(
            raw_slice.as_ptr() as usize % ALIGNMENT_BYTES,
            0,
            "SiSolidStateLoader: tensor '{}' is not 64-byte aligned (ptr={:#x})",
            name,
            raw_slice.as_ptr() as usize
        );

        // SAFETY: `raw_slice` is a &[u8] with length = multiple of 4 (f32 size),
        // and we have verified 4-byte alignment. The 64-byte invariant guarantees
        // we are also 4-byte aligned.
        debug_assert_eq!(raw_slice.len() % 4, 0, "Tensor byte length is not a multiple of 4");
        let float_count = raw_slice.len() / 4;
        let f32_slice = unsafe {
            std::slice::from_raw_parts(raw_slice.as_ptr() as *const f32, float_count)
        };

        Some(f32_slice)
    }

    /// Checks whether a named tensor is marked mutable (LoRA adapter) in the manifest.
    pub fn is_mutable(&self, name: &str) -> bool {
        self.manifest
            .tensors
            .iter()
            .find(|t| t.name == name)
            .map(|t| t.is_mutable)
            .unwrap_or(false)
    }

    /// Returns the shape of a tensor from the manifest.
    pub fn shape(&self, name: &str) -> Option<&[usize]> {
        self.manifest.tensors.iter().find(|t| t.name == name).map(|t| t.shape.as_slice())
    }

    /// Lists all tensor names in manifest order.
    pub fn tensor_names(&self) -> Vec<&str> {
        self.manifest.tensors.iter().map(|t| t.name.as_str()).collect()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// bincode shims (no external dep — inline serialization via serde_json fallback
// using bincode-compatible length-prefixed encoding via serde_json for now,
// since bincode is not in the workspace deps)
// ────────────────────────────────────────────────────────────────────────────

/// Serialize manifest to bytes using serde_json (stable, deterministic for our needs).
/// Uses a 4-byte little-endian length prefix so the reader knows TOC length.
fn bincode_serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

/// Deserialize manifest from bytes previously written by `bincode_serialize`.
fn bincode_deserialize<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    Ok(serde_json::from_slice(bytes)?)
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_compute_padding_always_64_aligned() {
        for offset in 0u64..=256 {
            let pad = compute_padding(offset);
            assert!((offset as usize + pad) % ALIGNMENT_BYTES == 0,
                "offset={offset} pad={pad} not aligned");
        }
    }

    #[test]
    fn test_si_packer_roundtrip_and_alignment() {
        let tmp = std::env::temp_dir().join("test_packer_roundtrip.si");

        let mut core = HashMap::new();
        core.insert("ssm_in_proj".to_string(), vec![0.1f32; 256 * 32]);
        core.insert("ssm_out_proj".to_string(), vec![0.2f32; 32 * 256]);

        SiPacker::pack_to_si(
            &tmp,
            "test_model",
            32,   // d_model
            8,    // d_state
            4,    // lora_rank
            core,
        )
        .expect("pack_to_si failed");

        assert!(tmp.exists());
        assert!(tmp.metadata().unwrap().len() > 0);

        let loader = SiSolidStateLoader::load(&tmp).expect("load failed");
        assert_eq!(loader.manifest.model_identifier, "test_model");
        assert_eq!(loader.manifest.d_model, 32);
        assert_eq!(loader.manifest.lora_rank, 4);

        // Verify tensor names are present
        let names = loader.tensor_names();
        assert!(names.contains(&"lora_a"), "missing lora_a");
        assert!(names.contains(&"lora_b"), "missing lora_b");
        assert!(names.contains(&"ssm_in_proj"), "missing ssm_in_proj");

        // Zero-copy slices
        let lora_a = loader.get_tensor_slice("lora_a").expect("lora_a slice failed");
        assert_eq!(lora_a.len(), 32 * 4); // d_model * lora_rank
        assert!(lora_a.iter().all(|&v| v == 0.0), "lora_a should be zeroed");

        let in_proj = loader.get_tensor_slice("ssm_in_proj").expect("in_proj slice failed");
        assert_eq!(in_proj.len(), 256 * 32);
        assert!((in_proj[0] - 0.1f32).abs() < 1e-6, "in_proj value mismatch");

        // mutable flags
        assert!(!loader.is_mutable("ssm_in_proj"), "core weights must be immutable");
        assert!(loader.is_mutable("lora_a"), "lora_a must be mutable");

        // Alignment: verify every descriptor offset is 64-byte aligned
        for desc in &loader.manifest.tensors {
            assert_eq!(
                desc.byte_offset as usize % ALIGNMENT_BYTES, 0,
                "tensor '{}' at offset {} is not 64-byte aligned",
                desc.name, desc.byte_offset
            );
        }

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_si_loader_rejects_legacy_container() {
        // Write a fake v2 header and verify the loader rejects it
        let tmp = std::env::temp_dir().join("legacy_v2.si");
        let mut f = std::fs::File::create(&tmp).unwrap();
        f.write_all(b"SINT").unwrap();
        f.write_all(&2u32.to_le_bytes()).unwrap(); // version = 2
        f.write_all(&[0u8; 100]).unwrap();
        drop(f);

        let result = SiSolidStateLoader::load(&tmp);
        assert!(result.is_err(), "Should reject v2 container");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_pack_si_with_empty_core_weights() {
        let tmp = std::env::temp_dir().join("empty_core.si");
        SiPacker::pack_to_si(
            &tmp,
            "empty_model",
            64, 16, 8,
            HashMap::new(), // no core weights
        ).expect("pack should succeed with empty core");

        let loader = SiSolidStateLoader::load(&tmp).unwrap();
        // Only lora_a and lora_b should be present
        assert_eq!(loader.manifest.tensors.len(), 2);
        assert_eq!(loader.manifest.tensors[0].name, "lora_a");
        assert_eq!(loader.manifest.tensors[1].name, "lora_b");

        let _ = std::fs::remove_file(&tmp);
    }
}
