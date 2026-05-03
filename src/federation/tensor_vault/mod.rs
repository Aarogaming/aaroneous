/// TensorVault — Cross-model tensor index and hybrid assembly engine.
///
/// # The real answer to "can we store weights categorically?"
///
/// GGUF files ARE the compressed storage. Q4_K_M stores weights as 4-bit
/// integers with f16 scale factors per 256-element block — this IS the
/// compressed form. Decompressing to F32 would expand 20GB to ~300GB for
/// zero benefit. The right architecture is:
///
/// 1. Keep the GGUF files on disk as the weight databank (they already are)
/// 2. Build a **cross-model tensor index** — a single in-memory registry
///    that knows "blk.14.attn_q.weight is at offset X in Mistral-7B.gguf
///    and at offset Y in Llama-3.1-8B.gguf with different dtype/shape"
/// 3. Use DNA comparison to drive block selection — `splice_boundary` from
///    `dna_compare` tells you the exact block where to switch from model A
///    to model B for maximum divergence
/// 4. Generate ForgeRecipe from DNA — one HTTP call: compare two models'
///    DNA → get splice point → crystallize a hybrid sovereign
///
/// # What this module provides
///
/// - `TensorVault`: the cross-model tensor index (scans all GGUFs on startup)
/// - `VaultEntry`: per-tensor record with source model, dtype, shape, offset
/// - `VaultQuery`: select tensors by block range, model preference, tensor kind
/// - `recipe_from_dna_compare()`: DNA-driven ForgeRecipe generation
/// - `vault_status()`: what's indexed, total unique tensors, per-model coverage

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

use crate::federation::forge::{
    read_gguf, GgufIndex, TensorMeta, ForgeRecipe, SplicingSegment,
    SovereignProfile, QuantizationPreference, RecommendedBase,
};
use crate::federation::dna::ModelDNA;

// ── Types ─────────────────────────────────────────────────────────────────────

/// A single tensor entry in the vault — where to find this tensor and what it is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    /// Canonical tensor name (e.g. "blk.14.attn_q.weight")
    pub tensor_name: String,
    /// Which model file this tensor comes from
    pub model_name: String,
    /// Absolute path to the source GGUF file
    pub model_path: PathBuf,
    /// Block index (None for non-block tensors like embeddings)
    pub block_idx: Option<usize>,
    /// Tensor kind: "attention", "mlp", "embedding", "norm", "other"
    pub kind: String,
    /// GGUF dtype code (0=F32, 1=F16, 8=Q8_0, 12=Q4_K_M, etc.)
    pub dtype: u32,
    /// Tensor shape dimensions
    pub shape: Vec<u64>,
    /// Byte offset within the source GGUF file (absolute)
    pub offset: u64,
    /// Byte size of tensor data
    pub size_bytes: u64,
    /// Number of parameters in this tensor
    pub param_count: u64,
    /// Architecture the source model uses (e.g. "llama", "qwen2", "phi3")
    pub architecture: String,
}

impl VaultEntry {
    /// Human-readable dtype label
    pub fn dtype_label(&self) -> &'static str {
        match self.dtype {
            0  => "F32",
            1  => "F16",
            2  => "Q4_0",
            8  => "Q8_0",
            12 => "Q4_K_M",
            14 => "Q6_K",
            15 => "Q8_K",
            30 => "BF16",
            _  => "IQ/other",
        }
    }

    /// Convert to a `SplicingSegment` for use in a `ForgeRecipe`
    pub fn to_splicing_segment(&self) -> SplicingSegment {
        SplicingSegment {
            source_gguf: self.model_path.to_string_lossy().to_string(),
            tensor_name: self.tensor_name.clone(),
        }
    }
}

/// A query for selecting tensors from the vault.
#[derive(Debug, Clone, Default)]
pub struct VaultQuery {
    /// Only return tensors from this model (None = any model)
    pub model_name: Option<String>,
    /// Only return tensors in blocks within this range (inclusive)
    pub block_range: Option<std::ops::RangeInclusive<usize>>,
    /// Only return these tensor kinds ("attention", "mlp", "embedding", "norm")
    pub kinds: Vec<String>,
    /// Prefer this dtype (if a tensor exists at multiple precisions, pick this one)
    pub preferred_dtype: Option<u32>,
    /// Maximum number of results
    pub limit: Option<usize>,
}

/// Vault index status summary
#[derive(Debug, Clone, Serialize)]
pub struct VaultStatus {
    pub indexed_models: Vec<VaultModelInfo>,
    pub total_unique_tensor_names: usize,
    pub total_vault_entries: usize,
    pub total_indexed_size_mb: f64,
    pub architectures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultModelInfo {
    pub model_name: String,
    pub architecture: String,
    pub tensor_count: usize,
    pub block_count: usize,
    pub file_size_mb: f64,
    pub dtype_summary: HashMap<String, usize>,
}

// ── TensorVault ───────────────────────────────────────────────────────────────

/// The cross-model tensor index.
///
/// Holds metadata for every tensor in every indexed GGUF model.
/// The actual weight bytes remain in the memory-mapped GGUF files on disk —
/// the vault stores only the addresses, not the data.
///
/// Usage:
/// ```no_run
/// let vault = TensorVault::new();
/// vault.index_model("D:\\Aaroneous\\models\\Mistral-7B-Instruct-v0.3-Q4_K_M.gguf").await?;
/// vault.index_model("D:\\Aaroneous\\models\\Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf").await?;
///
/// // Get all attention tensors from blocks 0-14 of Mistral
/// let tensors = vault.query(VaultQuery {
///     model_name: Some("Mistral-7B-Instruct-v0.3-Q4_K_M.gguf".into()),
///     block_range: Some(0..=14),
///     kinds: vec!["attention".into()],
///     ..Default::default()
/// });
/// ```
pub struct TensorVault {
    /// All entries, keyed by tensor_name → vec of entries (one per model)
    entries: HashMap<String, Vec<VaultEntry>>,
    /// Indexed model metadata
    models: HashMap<String, VaultModelInfo>,
    /// Architecture per model name
    architectures: HashMap<String, String>,
}

impl TensorVault {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            models: HashMap::new(),
            architectures: HashMap::new(),
        }
    }

    /// Index all GGUF files in the models directory.
    pub async fn index_all_models(&mut self, models_dir: &Path) -> anyhow::Result<()> {
        let Ok(dir) = std::fs::read_dir(models_dir) else {
            return Ok(());
        };
        let mut paths: Vec<PathBuf> = dir.flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("gguf"))
            .collect();
        paths.sort();

        let total = paths.len();
        for (i, path) in paths.iter().enumerate() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            info!("TensorVault: indexing {}/{} — {}", i+1, total, name);
            if let Err(e) = self.index_model(path).await {
                warn!("TensorVault: failed to index {}: {}", name, e);
            }
        }
        info!("TensorVault: indexed {} models, {} unique tensor names",
              self.models.len(), self.entries.len());
        Ok(())
    }

    /// Index a single GGUF model into the vault.
    pub async fn index_model(&mut self, model_path: &Path) -> anyhow::Result<()> {
        let model_name = model_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let file_size_mb = std::fs::metadata(model_path)
            .map(|m| m.len() as f64 / 1_048_576.0)
            .unwrap_or(0.0);

        let path_clone = model_path.to_path_buf();
        let (index, meta) = tokio::task::spawn_blocking(move || read_gguf(&path_clone))
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking panicked: {}", e))?
            .map_err(|e| anyhow::anyhow!("GGUF read error: {}", e))?;

        let arch = meta.architecture.clone();
        self.architectures.insert(model_name.clone(), arch.clone());

        // Extract all tensors from this model into vault entries
        let gguf_meta = index.0.into_values().next()
            .ok_or_else(|| anyhow::anyhow!("Empty GgufIndex"))?;

        let mut tensor_count = 0;
        let mut block_count = 0usize;
        let mut dtype_counts: HashMap<String, usize> = HashMap::new();

        for (tensor_name, tm) in &gguf_meta.tensors {
            let block_idx = parse_block_idx(tensor_name);
            let kind = classify_tensor_kind(tensor_name);
            let param_count: u64 = tm.shape.iter().product();

            if let Some(b) = block_idx {
                block_count = block_count.max(b + 1);
            }

            *dtype_counts.entry(dtype_label(tm.dtype).to_string()).or_default() += 1;

            let entry = VaultEntry {
                tensor_name: tensor_name.clone(),
                model_name: model_name.clone(),
                model_path: model_path.to_path_buf(),
                block_idx,
                kind,
                dtype: tm.dtype,
                shape: tm.shape.clone(),
                offset: tm.offset,
                size_bytes: tm.size,
                param_count,
                architecture: arch.clone(),
            };

            self.entries.entry(tensor_name.clone()).or_default().push(entry);
            tensor_count += 1;
        }

        self.models.insert(model_name.clone(), VaultModelInfo {
            model_name: model_name.clone(),
            architecture: arch,
            tensor_count,
            block_count,
            file_size_mb,
            dtype_summary: dtype_counts,
        });

        debug!("TensorVault: indexed {} — {} tensors, {} blocks",
               model_name, tensor_count, block_count);
        Ok(())
    }

    /// Query the vault for tensors matching the given criteria.
    pub fn query(&self, q: &VaultQuery) -> Vec<&VaultEntry> {
        let mut results: Vec<&VaultEntry> = self.entries.values()
            .flat_map(|entries| entries.iter())
            .filter(|e| {
                // Model filter
                if let Some(ref m) = q.model_name {
                    if &e.model_name != m { return false; }
                }
                // Block range filter
                if let Some(ref range) = q.block_range {
                    match e.block_idx {
                        None => return false,
                        Some(b) => if !range.contains(&b) { return false; }
                    }
                }
                // Kind filter
                if !q.kinds.is_empty() && !q.kinds.iter().any(|k| e.kind.contains(k.as_str())) {
                    return false;
                }
                // Dtype filter
                if let Some(dt) = q.preferred_dtype {
                    if e.dtype != dt { return false; }
                }
                true
            })
            .collect();

        results.sort_by_key(|e| (e.model_name.as_str(), e.block_idx.unwrap_or(usize::MAX), e.tensor_name.as_str()));

        if let Some(limit) = q.limit {
            results.truncate(limit);
        }
        results
    }

    /// Get all models that have a given tensor name.
    pub fn models_with_tensor(&self, tensor_name: &str) -> Vec<&VaultEntry> {
        self.entries.get(tensor_name).map(|v| v.iter().collect()).unwrap_or_default()
    }

    /// Get all unique tensor names in a specific block across all models.
    pub fn tensors_in_block(&self, block_idx: usize) -> Vec<&VaultEntry> {
        self.query(&VaultQuery {
            block_range: Some(block_idx..=block_idx),
            ..Default::default()
        })
    }

    /// Get vault status summary.
    pub fn status(&self) -> VaultStatus {
        let total_size: f64 = self.models.values().map(|m| m.file_size_mb).sum();
        let mut archs: Vec<String> = self.architectures.values().cloned().collect();
        archs.sort(); archs.dedup();

        VaultStatus {
            indexed_models: self.models.values().cloned().collect(),
            total_unique_tensor_names: self.entries.len(),
            total_vault_entries: self.entries.values().map(|v| v.len()).sum(),
            total_indexed_size_mb: total_size,
            architectures: archs,
        }
    }

    /// Find the best model for a given tensor based on quality preferences.
    ///
    /// Priority:
    /// 1. Highest-precision dtype (Q8_0 > Q4_K_M > Q3_K_M)
    /// 2. Largest model (more params = better quality weights)
    pub fn best_source_for_tensor(&self, tensor_name: &str) -> Option<&VaultEntry> {
        let entries = self.entries.get(tensor_name)?;
        entries.iter().max_by_key(|e| {
            let dtype_score = match e.dtype {
                0 => 100,   // F32
                30 => 90,   // BF16
                1 => 80,    // F16
                8 => 70,    // Q8_0
                15 => 60,   // Q8_K
                14 => 50,   // Q6_K
                13 => 40,   // Q5_K
                12 => 30,   // Q4_K_M
                _ => 10,
            };
            let size_score = (e.param_count / 1_000_000) as i64; // normalise to millions
            dtype_score * 1000 + size_score
        })
    }

    /// Check if a model is indexed
    pub fn is_indexed(&self, model_name: &str) -> bool {
        self.models.contains_key(model_name)
    }

    /// How many models are indexed
    pub fn model_count(&self) -> usize {
        self.models.len()
    }
}

// ── DNA-driven recipe generation ──────────────────────────────────────────────

/// Generate a ForgeRecipe from the DNA comparison of two models.
///
/// Uses `dna_a.splice_boundary` as the block cutoff:
/// - Blocks 0..splice_boundary come from model A
/// - Blocks splice_boundary..end come from model B
/// - Non-block tensors (embeddings, output) from model A unless specified
///
/// This is the "smart splice" — the splice boundary is the point of maximum
/// representation discontinuity between the two models, identified by the
/// cross-block Pearson correlation drop in the DNA dissection pipeline.
pub fn recipe_from_dna_compare(
    dna_a: &ModelDNA,
    dna_b: &ModelDNA,
    vault: &TensorVault,
    recipe_id: String,
    sovereign_name: &str,
) -> anyhow::Result<ForgeRecipe> {
    use crate::federation::forge::MetaValue;

    let path_a = PathBuf::from(&dna_a.model_path);
    let path_b = PathBuf::from(&dna_b.model_path);
    let splice = dna_a.splice_boundary;

    info!(
        "DNA-driven ForgeRecipe: {} (splice={}), A={} blocks 0-{}, B={} blocks {}-{}",
        sovereign_name, splice,
        dna_a.model_name, splice.saturating_sub(1),
        dna_b.model_name, splice, dna_b.num_blocks.saturating_sub(1),
    );

    // Collect all tensors from both models via the vault
    let model_name_a = &dna_a.model_name;
    let model_name_b = &dna_b.model_name;

    if !vault.is_indexed(model_name_a) {
        anyhow::bail!("Model {} not indexed in TensorVault. Call /vault/index first.", model_name_a);
    }
    if !vault.is_indexed(model_name_b) {
        anyhow::bail!("Model {} not indexed in TensorVault. Call /vault/index first.", model_name_b);
    }

    let mut segments: Vec<SplicingSegment> = Vec::new();

    // Non-block tensors (embeddings, output head) — always from model A
    // (embeddings must be consistent with the lower blocks)
    let non_block = vault.query(&VaultQuery {
        model_name: Some(model_name_a.clone()),
        block_range: None,
        kinds: vec!["embedding".into(), "output".into(), "norm".into()],
        ..Default::default()
    });
    // Filter to only truly non-block tensors
    for entry in non_block {
        if entry.block_idx.is_none() {
            segments.push(entry.to_splicing_segment());
        }
    }

    // Blocks 0..splice from model A
    if splice > 0 {
        let lower_blocks = vault.query(&VaultQuery {
            model_name: Some(model_name_a.clone()),
            block_range: Some(0..=splice.saturating_sub(1)),
            ..Default::default()
        });
        for entry in lower_blocks {
            segments.push(entry.to_splicing_segment());
        }
    }

    // Blocks splice..end from model B
    let b_blocks = dna_b.num_blocks;
    if splice < b_blocks {
        let upper_blocks = vault.query(&VaultQuery {
            model_name: Some(model_name_b.clone()),
            block_range: Some(splice..=b_blocks.saturating_sub(1)),
            ..Default::default()
        });
        for entry in upper_blocks {
            segments.push(entry.to_splicing_segment());
        }
    }

    if segments.is_empty() {
        anyhow::bail!("No tensors found for either model in vault — ensure both are indexed");
    }

    // Build metadata for the hybrid sovereign
    let mut metadata = HashMap::new();
    metadata.insert("general.name".to_string(),
        MetaValue::String(format!("aaroneous-{}-hybrid-v1", sovereign_name.to_lowercase())));
    metadata.insert("aaroneous.sovereign".to_string(),
        MetaValue::String(sovereign_name.to_string()));
    metadata.insert("aaroneous.source_a".to_string(),
        MetaValue::String(dna_a.model_name.clone()));
    metadata.insert("aaroneous.source_b".to_string(),
        MetaValue::String(dna_b.model_name.clone()));
    metadata.insert("aaroneous.splice_boundary".to_string(),
        MetaValue::Uint32(splice as u32));
    metadata.insert("aaroneous.assembly".to_string(),
        MetaValue::String("dna_splice".to_string()));
    metadata.insert("aaroneous.genetic_distance".to_string(),
        MetaValue::String(format!("{:.4}", dna_distance(dna_a, dna_b))));

    info!("ForgeRecipe '{}': {} segments from {} models, splice at block {}",
          recipe_id, segments.len(), 2, splice);

    Ok(ForgeRecipe {
        recipe_id,
        segments,
        metadata_overrides: metadata,
    })
}

/// Generate a "best-of-N" recipe by selecting each tensor from whichever
/// indexed model has the highest-precision/quality version of that tensor.
///
/// This is "pull the best of several models from the databank" — for each
/// tensor name that appears in the target sovereign's profile, pick the
/// highest-dtype version from any indexed model in the vault.
pub fn recipe_from_best_of_vault(
    vault: &TensorVault,
    target_tensor_names: &[String],
    recipe_id: String,
    sovereign_name: &str,
) -> anyhow::Result<ForgeRecipe> {
    use crate::federation::forge::MetaValue;

    let mut segments: Vec<SplicingSegment> = Vec::new();
    let mut sources: HashMap<String, usize> = HashMap::new();

    for name in target_tensor_names {
        if let Some(entry) = vault.best_source_for_tensor(name) {
            *sources.entry(entry.model_name.clone()).or_default() += 1;
            segments.push(entry.to_splicing_segment());
        } else {
            warn!("TensorVault: no source found for tensor '{}' — skipping", name);
        }
    }

    if segments.is_empty() {
        anyhow::bail!("No tensors found in vault for the requested tensor names");
    }

    let source_summary: Vec<String> = sources.iter()
        .map(|(m, n)| format!("{}:{}", m.replace(".gguf", ""), n))
        .collect();

    let mut metadata = HashMap::new();
    metadata.insert("general.name".to_string(),
        MetaValue::String(format!("aaroneous-{}-best-of-vault-v1", sovereign_name.to_lowercase())));
    metadata.insert("aaroneous.sovereign".to_string(),
        MetaValue::String(sovereign_name.to_string()));
    metadata.insert("aaroneous.assembly".to_string(),
        MetaValue::String("best_of_vault".to_string()));
    metadata.insert("aaroneous.sources".to_string(),
        MetaValue::String(source_summary.join(",")));

    Ok(ForgeRecipe {
        recipe_id,
        segments,
        metadata_overrides: metadata,
    })
}

/// Simple genetic distance proxy between two DNA records (correlation-based).
/// Full distance uses genetics.rs GeneticAnalyzer — this is a fast estimate
/// for metadata labelling.
fn dna_distance(a: &ModelDNA, b: &ModelDNA) -> f64 {
    let loci_a: HashMap<&str, f64> = a.genome_loci.iter()
        .map(|(k, &v)| (k.as_str(), v)).collect();
    let mut dist = 0.0f64;
    let mut n = 0usize;
    for (k, &v_b) in &b.genome_loci {
        if let Some(&v_a) = loci_a.get(k.as_str()) {
            dist += (v_a - v_b).abs();
            n += 1;
        }
    }
    if n == 0 { 1.0 } else { (dist / n as f64).clamp(0.0, 1.0) }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn parse_block_idx(name: &str) -> Option<usize> {
    let rest = name.strip_prefix("blk.")?;
    rest.split('.').next()?.parse().ok()
}

fn classify_tensor_kind(name: &str) -> String {
    if name.contains("attn") || name.contains("_q.") || name.contains("_k.") ||
       name.contains("_v.") || name.contains("_q_") || name.contains("query") {
        "attention".into()
    } else if name.contains("ffn") || name.contains("mlp") || name.contains("feed_forward") {
        "mlp".into()
    } else if name.contains("token_embd") || name.contains("embed_tokens") ||
              name.contains("embedding") {
        "embedding".into()
    } else if name.contains("norm") || name.contains("ln_") || name.contains("layer_norm") {
        "norm".into()
    } else if name.contains("output") || name.contains("lm_head") {
        "output".into()
    } else {
        "other".into()
    }
}

fn dtype_label(dtype: u32) -> &'static str {
    match dtype {
        0 => "F32", 1 => "F16", 2 => "Q4_0", 3 => "Q4_1",
        6 => "Q5_0", 7 => "Q5_1", 8 => "Q8_0", 10 => "Q2_K",
        11 => "Q3_K", 12 => "Q4_K_M", 13 => "Q5_K", 14 => "Q6_K",
        15 => "Q8_K", 30 => "BF16", _ => "IQ/other",
    }
}
