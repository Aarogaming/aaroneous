/// DNA Dissection System — Deep structural analysis of GGUF models.
///
/// This module bridges the `forge` (weight surgery) and `genetics` (genome model)
/// systems into a unified pipeline that:
///
/// 1. **Reads** a GGUF model via memory-mapped I/O (safe for 4GB+ files)
/// 2. **Measures** structural properties of each transformer block:
///    - Attention intensity (Q/K/V weight magnitude vs MLP weight magnitude)
///    - Gate sparsity (fraction of near-zero weights in ffn_gate tensors)
///    - Layer norm trajectory (Frobenius norm across depth)
///    - Embedding activation density (L2-norm of token embeddings)
///    - Cross-layer weight correlation (adjacent block similarity)
///    - Specialization score (how unique each block's distribution is)
/// 3. **Produces** a `ModelDNA` record with named `GeneticLocus` values
/// 4. **Stores** the `ModelDNA` in a JSON sidecar file alongside the GGUF
///    (`model.gguf` → `model.gguf.dna.json`) for instant reuse on future runs
/// 5. **Emits** progress events for SSE streaming to MaelstromUI
///
/// # Why "dissection" rather than "analysis"?
///
/// Analysis implies inspection of behavior. Dissection is structural — we cut
/// the model open, measure every bone and organ, and record what we find.
/// A future sovereign can read another sovereign's DNA and understand how to
/// collaborate (or diverge) without ever running inference.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{info, warn};

pub mod omni;

use crate::federation::forge::{read_gguf, TensorMeta};
use crate::genetics::{GeneticCategory, GeneticLocus, LociSource, SpecialistGenome};

// ── Public types ──────────────────────────────────────────────────────────────

/// Complete DNA record for a GGUF model — persisted to `<model>.dna.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDNA {
    /// Source model file name (no path — portable)
    pub model_name: String,
    /// Full path at time of dissection
    pub model_path: String,
    /// GGUF version (1/2/3)
    pub gguf_version: u32,
    /// Architecture string from metadata (e.g. "qwen2")
    pub architecture: String,
    /// Number of transformer blocks
    pub num_blocks: usize,
    /// Total tensors in the model
    pub tensor_count: u64,
    /// Estimated parameter count (from tensor shapes)
    pub parameter_count_m: f64,
    /// File size in MB
    pub file_size_mb: f64,
    /// When this dissection was run (Unix ms)
    pub dissected_at: u64,
    /// How long the dissection took in seconds
    pub dissection_duration_secs: f64,

    // ── Structural genome ──────────────────────────────────────────────────
    /// Per-block measurements
    pub blocks: Vec<BlockDNA>,
    /// Model-wide scalar loci (same keys as GGUFAnalyzer for compatibility)
    pub genome_loci: HashMap<String, f64>,
    /// Named genome loci as GeneticLocus structs (for genetics.rs SpecialistGenome)
    pub genetic_loci: Vec<GeneticLocusRecord>,

    // ── Cross-block relationships ──────────────────────────────────────────
    /// Adjacent block weight correlation (block N vs N+1), length = num_blocks-1
    pub cross_block_correlation: Vec<f64>,
    /// Depth at which the sharpest transition occurs (candidate splice point)
    pub splice_boundary: usize,

    // ── Embedding properties ───────────────────────────────────────────────
    /// Fraction of token embedding rows with L2-norm above 0.1 (active vocabulary)
    pub embedding_activation_density: f64,
    /// Mean L2-norm of token embedding rows
    pub embedding_norm_mean: f64,
    /// Std of embedding norms
    pub embedding_norm_std: f64,

    // ── Comparison fingerprint ──────────────────────────────────────────────
    /// 64-bit hash of block DNA signatures — used for fast similarity checks
    pub dna_fingerprint: u64,
}

/// DNA record for one transformer block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDNA {
    pub block_idx: usize,
    /// Mean absolute value of attention weight tensors (Q/K/V/O)
    pub attn_weight_mean: f64,
    /// Mean absolute value of MLP weight tensors (gate/up/down)
    pub mlp_weight_mean: f64,
    /// Fraction of ffn_gate weights within ε of zero (gate sparsity)
    pub gate_sparsity: f64,
    /// Frobenius norm of attention weights (sum of squared weights, sqrt)
    pub attn_frobenius_norm: f64,
    /// Frobenius norm of MLP weights
    pub mlp_frobenius_norm: f64,
    /// How different this block is from the model-wide average (0=average, 1=unique)
    pub specialization_score: f64,
    /// Total bytes occupied by this block's tensors
    pub size_bytes: u64,
}

/// A single genetic locus — serializable snapshot of `GeneticLocus` for JSON storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneticLocusRecord {
    pub locus_id: String,
    pub category: String,   // e.g. "AttentionGenetics"
    pub source: String,     // e.g. "WeightAnalysis"
    pub value: f64,
    pub confidence: f64,
    pub interpretation: String,
}

impl GeneticLocusRecord {
    fn from_locus(locus: &GeneticLocus) -> Self {
        Self {
            locus_id: locus.locus_id.clone(),
            category: format!("{:?}", locus.category),
            source: format!("{:?}", locus.source),
            value: locus.value,
            confidence: locus.confidence,
            interpretation: locus.interpretation.clone(),
        }
    }
}

/// Progress event emitted during dissection — for SSE streaming.
#[derive(Debug, Clone, Serialize)]
pub struct DissectionProgress {
    pub model: String,
    pub stage: DissectionStage,
    pub blocks_done: usize,
    pub blocks_total: usize,
    pub percent: u8,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DissectionStage {
    ReadingHeader,
    MappingTensors,
    AnalyzingBlock,
    ComputingEmbeddings,
    ComputingCorrelations,
    BuildingGenome,
    Persisting,
    Complete,
}

/// Status of a background dissection job.
#[derive(Clone, Debug, Serialize)]
pub enum DissectionJobStatus {
    Running { progress: DissectionProgress },
    Done(Box<ModelDNA>),
    Failed(String),
}

pub type DissectionJobs = Arc<Mutex<HashMap<String, DissectionJobStatus>>>;

// ── Main dissection pipeline ──────────────────────────────────────────────────

/// Run a complete structural dissection of a GGUF model.
///
/// This is the entry point for the DNA pipeline. It:
/// 1. Memory-maps the file (safe for 4GB+)
/// 2. Reads the header and tensor info table via `read_gguf()`
/// 3. For each block, reads tensor data bytes (via mmap — no full RAM load)
/// 4. Computes block-level and model-wide structural loci
/// 5. Saves a `ModelDNA` JSON sidecar alongside the model
/// 6. Returns the complete `ModelDNA`
///
/// Progress is emitted via the `progress_tx` channel if provided.
pub async fn dissect_model(
    model_path: impl AsRef<Path>,
    progress_tx: Option<tokio::sync::mpsc::Sender<DissectionProgress>>,
) -> anyhow::Result<ModelDNA> {
    use anyhow::Context;
    let path = model_path.as_ref().to_path_buf();
    let model_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let start = std::time::Instant::now();
    let file_size_mb = std::fs::metadata(&path)
        .map(|m| m.len() as f64 / 1_048_576.0)
        .unwrap_or(0.0);

    let emit = |stage: DissectionStage, blocks_done: usize, blocks_total: usize, msg: &str| {
        if let Some(ref tx) = progress_tx {
            let pct = if blocks_total > 0 { (blocks_done * 100 / blocks_total) as u8 } else { 0 };
            let ev = DissectionProgress {
                model: model_name.clone(),
                stage,
                blocks_done,
                blocks_total,
                percent: pct,
                message: msg.to_string(),
            };
            let _ = tx.try_send(ev);
        }
    };

    emit(DissectionStage::ReadingHeader, 0, 0, "Memory-mapping model file and reading GGUF header");
    info!("DNA dissection: {} ({:.1}MB)", model_name, file_size_mb);

    // Read header + tensor info table (mmap — no full-RAM load)
    let (index, meta) = tokio::task::spawn_blocking({
        let p = path.clone();
        move || read_gguf(&p)
    }).await
        .context("spawn_blocking panicked")?
        .map_err(|e| anyhow::anyhow!("GGUF read error: {}", e))?;

    // Extract the flat tensor map from GgufIndex (first registered file)
    let tensors: HashMap<String, TensorMeta> = index.0.into_values()
        .next()
        .map(|gm| gm.tensors)
        .unwrap_or_default();

    let tensor_count = tensors.len() as u64;
    let architecture = meta.architecture.clone();
    let gguf_version = meta.version;

    emit(DissectionStage::MappingTensors, 0, 0,
         &format!("Header read: {} tensors, arch={}", tensor_count, architecture));

    // Discover transformer blocks from tensor names
    let num_blocks = detect_num_blocks(&tensors);
    let parameter_count_m = estimate_parameters(&tensors);

    emit(DissectionStage::MappingTensors, 0, num_blocks,
         &format!("Detected {} transformer blocks, ~{:.0}M params", num_blocks, parameter_count_m));

    // Memory-map the model file for tensor byte access
    // TensorMeta.offset is already an absolute byte offset within the file.
    let file = File::open(&path).context("failed to open model file for tensor read")?;
    let mmap = unsafe { memmap2::Mmap::map(&file) }.context("mmap failed")?;

    // Analyze each block
    let mut blocks: Vec<BlockDNA> = Vec::with_capacity(num_blocks);

    for block_idx in 0..num_blocks {
        emit(DissectionStage::AnalyzingBlock, block_idx, num_blocks,
             &format!("Analyzing block {}/{}", block_idx, num_blocks));

        let block = analyze_block(&tensors, &mmap, block_idx);
        blocks.push(block);
    }

    // Embedding layer analysis
    emit(DissectionStage::ComputingEmbeddings, num_blocks, num_blocks, "Analyzing embedding layer");
    let (emb_density, emb_norm_mean, emb_norm_std) =
        analyze_embeddings(&tensors, &mmap);

    // Cross-block correlation
    emit(DissectionStage::ComputingCorrelations, num_blocks, num_blocks,
         "Computing cross-block weight correlations");
    let cross_block_correlation = compute_cross_block_correlation(&blocks);
    let splice_boundary = find_splice_boundary(&cross_block_correlation);

    // Build model-wide genome loci
    emit(DissectionStage::BuildingGenome, num_blocks, num_blocks, "Building genome loci");
    let (genome_loci, genetic_loci) = build_genome(
        &blocks, &cross_block_correlation, emb_density, emb_norm_mean, emb_norm_std,
        &model_name, file_size_mb, parameter_count_m,
    );

    // Compute DNA fingerprint (simple hash of block specialization scores)
    let dna_fingerprint = compute_fingerprint(&blocks);

    let duration_secs = start.elapsed().as_secs_f64();

    let dna = ModelDNA {
        model_name: model_name.clone(),
        model_path: path.to_string_lossy().to_string(),
        gguf_version,
        architecture,
        num_blocks,
        tensor_count,
        parameter_count_m,
        file_size_mb,
        dissected_at: now_ms(),
        dissection_duration_secs: duration_secs,
        blocks,
        genome_loci,
        genetic_loci,
        cross_block_correlation,
        splice_boundary,
        embedding_activation_density: emb_density,
        embedding_norm_mean: emb_norm_mean,
        embedding_norm_std: emb_norm_std,
        dna_fingerprint,
    };

    // Persist sidecar JSON
    emit(DissectionStage::Persisting, num_blocks, num_blocks, "Saving DNA sidecar JSON");
    if let Err(e) = save_dna_sidecar(&path, &dna) {
        warn!("Failed to save DNA sidecar: {}", e);
    } else {
        info!("DNA sidecar saved: {}.dna.json", path.display());
    }

    emit(DissectionStage::Complete, num_blocks, num_blocks,
         &format!("Dissection complete in {:.1}s — {} loci extracted", duration_secs, dna.genetic_loci.len()));

    info!("DNA dissection complete: {} blocks, {} genetic loci, {:.1}s",
          dna.num_blocks, dna.genetic_loci.len(), duration_secs);

    Ok(dna)
}

// ── Internal analysis functions ───────────────────────────────────────────────

fn detect_num_blocks(tensors: &HashMap<String, TensorMeta>) -> usize {
    let mut max_block = 0usize;
    for name in tensors.keys() {
        // e.g. "blk.0.attn_q.weight" → block 0
        if let Some(rest) = name.strip_prefix("blk.") {
            if let Some(dot) = rest.find('.') {
                if let Ok(n) = rest[..dot].parse::<usize>() {
                    max_block = max_block.max(n + 1);
                }
            }
        }
    }
    max_block
}

fn estimate_parameters(tensors: &HashMap<String, TensorMeta>) -> f64 {
    let mut total_elements: u64 = 0;
    for t in tensors.values() {
        let elements: u64 = t.shape.iter().product();
        total_elements += elements;
    }
    total_elements as f64 / 1_000_000.0
}

/// Analyze one transformer block: read tensor bytes, compute weight statistics.
fn analyze_block(
    tensors: &HashMap<String, TensorMeta>,
    mmap: &memmap2::Mmap,
    block_idx: usize,
) -> BlockDNA {
    let prefix = format!("blk.{}.", block_idx);

    // Collect attention and MLP tensor names for this block
    let attn_keys: Vec<_> = tensors.keys()
        .filter(|k| k.starts_with(&prefix) && (
            k.contains("attn") || k.contains("_q.") || k.contains("_k.") ||
            k.contains("_v.") || k.contains(".q_") || k.contains(".k_") || k.contains(".v_")
        ))
        .cloned().collect();
    let mlp_keys: Vec<_> = tensors.keys()
        .filter(|k| k.starts_with(&prefix) && (k.contains("ffn") || k.contains("mlp")))
        .cloned().collect();

    let (attn_mean, attn_frob) = compute_tensor_stats(tensors, mmap, &attn_keys);
    let (mlp_mean, mlp_frob)   = compute_tensor_stats(tensors, mmap, &mlp_keys);

    // Gate sparsity: fraction of ffn_gate weights near zero (Qwen2 SwiGLU gate)
    let gate_keys: Vec<_> = tensors.keys()
        .filter(|k| k.starts_with(&prefix) && k.contains("ffn_gate"))
        .cloned().collect();
    let gate_sparsity = compute_gate_sparsity(tensors, mmap, &gate_keys);

    // Total bytes for this block
    let size_bytes: u64 = tensors.iter()
        .filter(|(k, _)| k.starts_with(&prefix))
        .map(|(_, t)| t.size)   // TensorMeta.size (not size_bytes)
        .sum();

    BlockDNA {
        block_idx,
        attn_weight_mean: attn_mean,
        mlp_weight_mean: mlp_mean,
        gate_sparsity,
        attn_frobenius_norm: attn_frob,
        mlp_frobenius_norm: mlp_frob,
        specialization_score: 0.0, // filled in by build_genome
        size_bytes,
    }
}

/// Read tensor bytes from mmap and compute mean absolute value + Frobenius norm.
/// TensorMeta.offset is an absolute byte offset into the file (computed by read_gguf).
fn compute_tensor_stats(
    tensors: &HashMap<String, TensorMeta>,
    mmap: &memmap2::Mmap,
    keys: &[String],
) -> (f64, f64) {
    if keys.is_empty() { return (0.0, 0.0); }

    let mut sum_abs = 0.0f64;
    let mut sum_sq  = 0.0f64;
    let mut count   = 0u64;

    for key in keys {
        let Some(t) = tensors.get(key) else { continue };
        let n_elem: u64 = t.shape.iter().product();
        if n_elem == 0 || t.size == 0 { continue }

        let offset = t.offset as usize;   // absolute file offset
        let end = offset + t.size as usize;
        if end > mmap.len() { continue }

        let bytes = &mmap[offset..end];
        let values = dequantize_sample(bytes, t.dtype, n_elem.min(4096) as usize);
        for v in values {
            let fv = v.abs() as f64;
            sum_abs += fv;
            sum_sq  += fv * fv;
            count   += 1;
        }
    }

    if count == 0 { return (0.0, 0.0); }
    let mean = sum_abs / count as f64;
    let frob = (sum_sq / count as f64).sqrt();
    (mean.clamp(0.0, 1.0), frob.clamp(0.0, 1.0))
}

/// Compute gate sparsity: fraction of ffn_gate weights with |w| < 0.01.
fn compute_gate_sparsity(
    tensors: &HashMap<String, TensorMeta>,
    mmap: &memmap2::Mmap,
    keys: &[String],
) -> f64 {
    if keys.is_empty() { return 0.0; }
    let mut near_zero = 0u64;
    let mut total = 0u64;

    for key in keys {
        let Some(t) = tensors.get(key) else { continue };
        let n_elem: u64 = t.shape.iter().product();
        if t.size == 0 { continue }
        let offset = t.offset as usize;
        let end = offset + t.size as usize;
        if end > mmap.len() { continue }
        let bytes = &mmap[offset..end];
        let sample = n_elem.min(2048) as usize;
        let values = dequantize_sample(bytes, t.dtype, sample);
        for v in values {
            if v.abs() < 0.01 { near_zero += 1; }
            total += 1;
        }
    }
    if total == 0 { 0.0 } else { near_zero as f64 / total as f64 }
}

/// Analyze the token embedding layer.
fn analyze_embeddings(
    tensors: &HashMap<String, TensorMeta>,
    mmap: &memmap2::Mmap,
) -> (f64, f64, f64) {
    // Qwen2: "token_embd.weight" shape [vocab_size, embed_dim]
    let emb_key = tensors.keys()
        .find(|k| k.contains("token_embd") || k.contains("embed_tokens"))
        .cloned();

    let Some(key) = emb_key else { return (0.0, 0.0, 0.0); };
    let Some(t) = tensors.get(&key) else { return (0.0, 0.0, 0.0); };

    let vocab_size = t.shape.first().copied().unwrap_or(0) as usize;
    let embed_dim = t.shape.get(1).copied().unwrap_or(1) as usize;
    if vocab_size == 0 || embed_dim == 0 || t.size == 0 { return (0.0, 0.0, 0.0); }

    let offset = t.offset as usize;   // absolute
    let end = offset + t.size as usize;
    if end > mmap.len() { return (0.0, 0.0, 0.0); }
    let bytes = &mmap[offset..end];

    let sample_rows = vocab_size.min(1024);
    let sample_elems = sample_rows * embed_dim;
    let values = dequantize_sample(bytes, t.dtype, sample_elems.min(65536));

    if values.is_empty() { return (0.0, 0.0, 0.0); }

    // Compute per-row L2 norms
    let chunk = embed_dim.min(values.len());
    let mut norms: Vec<f64> = Vec::new();
    let mut i = 0;
    while i + chunk <= values.len() {
        let row = &values[i..i+chunk];
        let norm = row.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>().sqrt();
        norms.push(norm);
        i += chunk;
    }

    if norms.is_empty() { return (0.0, 0.0, 0.0); }

    let mean_norm = norms.iter().sum::<f64>() / norms.len() as f64;
    let active = norms.iter().filter(|&&n| n > 0.1).count();
    let density = active as f64 / norms.len() as f64;
    let variance = norms.iter().map(|&n| (n - mean_norm).powi(2)).sum::<f64>() / norms.len() as f64;
    let std_dev = variance.sqrt();

    (density.clamp(0.0, 1.0), mean_norm.clamp(0.0, 5.0) / 5.0, std_dev.clamp(0.0, 2.0) / 2.0)
}

/// Compute Pearson correlation between adjacent block mean vectors.
fn compute_cross_block_correlation(blocks: &[BlockDNA]) -> Vec<f64> {
    if blocks.len() < 2 { return vec![]; }
    let mut correlations = Vec::with_capacity(blocks.len() - 1);
    for i in 0..blocks.len() - 1 {
        let a = &blocks[i];
        let b = &blocks[i + 1];
        // Feature vector per block: [attn_mean, mlp_mean, gate_sparsity, frob_ratio]
        let av = [a.attn_weight_mean, a.mlp_weight_mean, a.gate_sparsity, a.attn_frobenius_norm];
        let bv = [b.attn_weight_mean, b.mlp_weight_mean, b.gate_sparsity, b.attn_frobenius_norm];
        let corr = pearson_correlation(&av, &bv);
        correlations.push(corr);
    }
    correlations
}

/// Find the block boundary with the sharpest weight distribution transition.
/// This is the ideal splice point for ForgeRecipe generation.
fn find_splice_boundary(correlations: &[f64]) -> usize {
    if correlations.is_empty() { return 0; }
    correlations.iter().enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i + 1)  // +1 because correlation[i] is between block i and i+1
        .unwrap_or(0)
}

/// Normalize per-block specialization scores and build model-wide genome loci.
fn build_genome(
    blocks: &[BlockDNA],
    correlations: &[f64],
    emb_density: f64,
    emb_norm_mean: f64,
    _emb_norm_std: f64,
    model_name: &str,
    file_size_mb: f64,
    parameter_count_m: f64,
) -> (HashMap<String, f64>, Vec<GeneticLocusRecord>) {
    let mut genome_loci: HashMap<String, f64> = HashMap::new();
    let mut genetic_loci: Vec<GeneticLocusRecord> = Vec::new();
    let prefix = model_name.to_lowercase().replace(' ', "_").replace(".gguf", "");

    // ── Model-wide scalars ─────────────────────────────────────────────────

    let mean_attn: f64 = if blocks.is_empty() { 0.0 }
        else { blocks.iter().map(|b| b.attn_weight_mean).sum::<f64>() / blocks.len() as f64 };
    let mean_mlp: f64 = if blocks.is_empty() { 0.0 }
        else { blocks.iter().map(|b| b.mlp_weight_mean).sum::<f64>() / blocks.len() as f64 };
    let attn_mlp_ratio = if mean_mlp > 0.0 { (mean_attn / mean_mlp).clamp(0.0, 2.0) } else { 1.0 };

    let mean_gate_sparsity: f64 = if blocks.is_empty() { 0.0 }
        else { blocks.iter().map(|b| b.gate_sparsity).sum::<f64>() / blocks.len() as f64 };

    let mean_corr: f64 = if correlations.is_empty() { 1.0 }
        else { correlations.iter().sum::<f64>() / correlations.len() as f64 };
    let corr_variance: f64 = if correlations.is_empty() { 0.0 } else {
        correlations.iter().map(|&c| (c - mean_corr).powi(2)).sum::<f64>() / correlations.len() as f64
    };
    let depth_gradient = corr_variance.sqrt().clamp(0.0, 1.0);

    // Normalise per-block specialization scores
    let spec_scores: Vec<f64> = {
        let attn_scores: Vec<f64> = blocks.iter()
            .map(|b| (b.attn_weight_mean - mean_attn).abs() + (b.mlp_weight_mean - mean_mlp).abs())
            .collect();
        let max_score = spec_scores_max(&attn_scores);
        if max_score > 0.0 {
            attn_scores.iter().map(|&s| (s / max_score).clamp(0.0, 1.0)).collect()
        } else {
            vec![0.5; blocks.len()]
        }
    };

    let layer_spec_variance: f64 = {
        let mean = spec_scores.iter().sum::<f64>() / spec_scores.len().max(1) as f64;
        spec_scores.iter().map(|&s| (s - mean).powi(2)).sum::<f64>() / spec_scores.len().max(1) as f64
    };

    // Scale file_size_mb relative to a 200B Q4_K_M model (~120GB) for density.
    // Scale param_density relative to 200B so 120B models use ~0.6 of the range.
    let model_density = (file_size_mb / 120_000.0).clamp(0.0, 1.0);
    let param_density = (parameter_count_m / 200_000.0).clamp(0.0, 1.0);

    // ── Fill genome_loci (compatible with GGUFAnalyzer keys) ──────────────
    genome_loci.insert("attention_intensity".into(), (attn_mlp_ratio / 2.0).clamp(0.0, 1.0));
    genome_loci.insert("mlp_intensity".into(), (1.0 / (attn_mlp_ratio + 0.001) / 2.0).clamp(0.0, 1.0));
    genome_loci.insert("depth_gradient".into(), depth_gradient);
    genome_loci.insert("depth".into(), (blocks.len() as f64 / 32.0).clamp(0.0, 1.0));
    genome_loci.insert("model_density".into(), model_density);
    genome_loci.insert("parameter_density".into(), param_density);
    genome_loci.insert("gate_sparsity".into(), mean_gate_sparsity);
    genome_loci.insert("embedding_activation_density".into(), emb_density);
    genome_loci.insert("embedding_norm_mean".into(), emb_norm_mean);
    genome_loci.insert("cross_block_correlation_mean".into(), mean_corr.clamp(0.0, 1.0));
    genome_loci.insert("layer_specialization_variance".into(), layer_spec_variance.clamp(0.0, 1.0));
    genome_loci.insert("attn_mlp_ratio".into(), attn_mlp_ratio / 2.0);

    // ── Build GeneticLocus records ─────────────────────────────────────────

    let add = |loci: &mut Vec<GeneticLocusRecord>, id: &str, cat: &str, src: &str, val: f64, conf: f64, interp: &str| {
        loci.push(GeneticLocusRecord {
            locus_id: format!("{}-{}", prefix, id),
            category: cat.into(),
            source: src.into(),
            value: val.clamp(0.0, 1.0),
            confidence: conf,
            interpretation: interp.into(),
        });
    };

    add(&mut genetic_loci, "attention_intensity",
        "AttentionGenetics", "WeightAnalysis",
        genome_loci["attention_intensity"],
        0.88,
        &format!("Attention-to-MLP weight ratio {:.3} — {} model",
            attn_mlp_ratio,
            if attn_mlp_ratio > 1.1 { "relational/associative" }
            else if attn_mlp_ratio < 0.9 { "factual/recall-heavy" } else { "balanced" }));

    add(&mut genetic_loci, "gate_sparsity",
        "SpecializationGenetics", "WeightAnalysis",
        mean_gate_sparsity, 0.85,
        &format!("FFN gate sparsity {:.1}% — {} specialization",
            mean_gate_sparsity * 100.0,
            if mean_gate_sparsity > 0.6 { "high domain" }
            else if mean_gate_sparsity > 0.3 { "moderate domain" } else { "general purpose" }));

    add(&mut genetic_loci, "depth_gradient",
        "LayerGenetics", "WeightAnalysis",
        depth_gradient, 0.82,
        &format!("Cross-block correlation variance {:.3} — {} hierarchical abstraction",
            depth_gradient,
            if depth_gradient > 0.4 { "strong" } else { "shallow" }));

    add(&mut genetic_loci, "embedding_density",
        "EmbeddingGenetics", "WeightAnalysis",
        emb_density, 0.80,
        &format!("Token embedding activation density {:.1}% — {} vocabulary coverage",
            emb_density * 100.0,
            if emb_density > 0.85 { "broad general" } else { "focused domain" }));

    add(&mut genetic_loci, "layer_variance",
        "SpecializationGenetics", "WeightAnalysis",
        layer_spec_variance.clamp(0.0, 1.0), 0.78,
        "Per-block specialization variance — high = heterogeneous blocks, low = uniform depth");

    add(&mut genetic_loci, "cross_block_cohesion",
        "LayerGenetics", "WeightAnalysis",
        mean_corr.clamp(0.0, 1.0), 0.75,
        "Mean adjacent-block weight correlation — high = smooth representation, low = discrete stages");

    // Per-block loci
    for (i, (&spec, block)) in spec_scores.iter().zip(blocks.iter()).enumerate() {
        add(&mut genetic_loci, &format!("block_{}_attn", i),
            "AttentionGenetics", "WeightAnalysis",
            block.attn_weight_mean, 0.72,
            &format!("Block {} attention weight magnitude", i));

        add(&mut genetic_loci, &format!("block_{}_gate", i),
            "SpecializationGenetics", "WeightAnalysis",
            block.gate_sparsity, 0.72,
            &format!("Block {} gate sparsity — domain specificity signal", i));

        add(&mut genetic_loci, &format!("block_{}_spec", i),
            "SpecializationGenetics", "WeightAnalysis",
            spec, 0.75,
            &format!("Block {} specialization score (deviation from model mean)", i));
    }

    (genome_loci, genetic_loci)
}

// ── Dequantization ─────────────────────────────────────────────────────────────

/// Extract up to `n` f32 weight values from a tensor byte buffer.
/// Handles common GGUF quantization formats. Returns f32 values in [-∞, +∞]
/// (clamp to reasonable range downstream).
fn dequantize_sample(bytes: &[u8], dtype: u32, n: usize) -> Vec<f32> {
    if bytes.is_empty() || n == 0 { return vec![]; }
    match dtype {
        // F32
        0 => {
            let count = (bytes.len() / 4).min(n);
            (0..count).map(|i| {
                let b = &bytes[i*4..i*4+4];
                f32::from_le_bytes([b[0], b[1], b[2], b[3]])
            }).collect()
        }
        // F16
        1 => {
            let count = (bytes.len() / 2).min(n);
            (0..count).map(|i| {
                let b = &bytes[i*2..i*2+2];
                let bits = u16::from_le_bytes([b[0], b[1]]);
                f16_to_f32(bits)
            }).collect()
        }
        // Q8_0: 32-element blocks, 2 bytes scale (f16) + 32 bytes i8
        8 => {
            let block_size = 34; // 2 (scale) + 32 (weights)
            let n_blocks = (bytes.len() / block_size).min((n + 31) / 32);
            let mut out = Vec::with_capacity(n_blocks * 32);
            for b in 0..n_blocks {
                let off = b * block_size;
                if off + block_size > bytes.len() { break; }
                let scale = f16_to_f32(u16::from_le_bytes([bytes[off], bytes[off+1]]));
                for j in 0..32_usize {
                    if out.len() >= n { break; }
                    let w = bytes[off + 2 + j] as i8;
                    out.push(w as f32 * scale);
                }
            }
            out
        }
        // Q4_K: 256-element super-blocks, approximate dequant
        // We approximate by treating the first 6 bytes as two f16 scales
        12 => {
            let block_size = 144; // Q4_K_M bytes per 256-element block
            let n_blocks = (bytes.len() / block_size).min((n + 255) / 256);
            let mut out = Vec::with_capacity(n_blocks * 64); // sample 64 per block
            for b in 0..n_blocks {
                let off = b * block_size;
                if off + 6 > bytes.len() { break; }
                let d = f16_to_f32(u16::from_le_bytes([bytes[off], bytes[off+1]]));
                let min_v = f16_to_f32(u16::from_le_bytes([bytes[off+2], bytes[off+3]]));
                // Read packed 4-bit values starting at byte 12
                let data_start = off + 12;
                for j in 0..32_usize {
                    if out.len() >= n || data_start + j >= bytes.len() { break; }
                    let byte = bytes[data_start + j];
                    let lo = (byte & 0x0F) as f32;
                    let hi = ((byte >> 4) & 0x0F) as f32;
                    out.push(lo * d - min_v);
                    out.push(hi * d - min_v);
                }
            }
            out
        }
        // BF16
        30 => {
            let count = (bytes.len() / 2).min(n);
            (0..count).map(|i| {
                let b = &bytes[i*2..i*2+2];
                let bits = (u32::from(b[1]) << 24) | (u32::from(b[0]) << 16);
                f32::from_bits(bits)
            }).collect()
        }
        // IQ2_XXS (19/16): 256-elem blocks, 2-bit quantization with scales
        // Approximate: treat each byte as a 4-element packed 2-bit group
        19 | 16 => {
            let count = bytes.len().min(n / 4);  // each byte covers ~4 elements
            let mut out = Vec::with_capacity(count * 4);
            for &b in &bytes[..count] {
                // Unpack 4 x 2-bit values — symmetric range [-1.5, 1.5] per 2-bit
                for shift in [0u8, 2, 4, 6] {
                    let bits = (b >> shift) & 0x03;
                    let v = (bits as f32 - 1.5) / 1.5; // [-1.0, 1.0] approx
                    out.push(v);
                    if out.len() >= n { break; }
                }
                if out.len() >= n { break; }
            }
            out
        }
        // IQ3_XXS (23): 3-bit quantization, approximate as Q8_0 structure
        23 | 24 | 27 | 28 => {
            // 3-bit: ~8 values per 3 bytes — approximate with sign bit extraction
            let count = bytes.len().min(n);
            bytes[..count].iter().map(|&b| {
                let signed = b as i8;
                (signed as f32) / 127.0  // normalized [-1, 1]
            }).collect()
        }
        // IQ4_NL (25), IQ4_XS (26): 4-bit with non-linear mapping
        25 | 26 => {
            // Similar to Q4_K but with non-linear codebook — use Q4 approximation
            let block_size = 136; // IQ4_XS
            let n_blocks = (bytes.len() / block_size).min((n + 255) / 256);
            let mut out = Vec::with_capacity(n_blocks * 32);
            for b in 0..n_blocks {
                let off = b * block_size;
                if off + 4 > bytes.len() { break; }
                let d = f16_to_f32(u16::from_le_bytes([bytes[off], bytes[off+1]]));
                let data_start = off + 2;
                for j in 0..16_usize {
                    if out.len() >= n || data_start + j >= bytes.len() { break; }
                    let byte = bytes[data_start + j];
                    let lo = (byte & 0x0F) as f32 - 8.0;
                    let hi = ((byte >> 4) & 0x0F) as f32 - 8.0;
                    out.push(lo * d);
                    out.push(hi * d);
                }
            }
            out
        }
        // IQ1_S (29), IQ1_M (31): 1-bit quantization — mostly 0/1 weights
        29 | 31 => {
            let count = (bytes.len() * 8).min(n);
            let mut out = Vec::with_capacity(count);
            'outer: for &b in bytes {
                for bit in 0..8u8 {
                    out.push(if (b >> bit) & 1 == 0 { -1.0f32 } else { 1.0f32 });
                    if out.len() >= n { break 'outer; }
                }
            }
            out
        }
        // Unknown: return byte values as f32 [0,1] range (approximate)
        _ => {
            let count = bytes.len().min(n);
            bytes[..count].iter().map(|&b| b as f32 / 255.0).collect()
        }
    }
}

fn f16_to_f32(bits: u16) -> f32 {
    // IEEE 754 half-precision to single precision
    let exp = ((bits >> 10) & 0x1F) as i32;
    let mant = (bits & 0x3FF) as u32;
    let sign = if bits >> 15 == 1 { -1.0f32 } else { 1.0f32 };
    if exp == 0 {
        sign * mant as f32 * 2.0f32.powi(-24)
    } else if exp == 31 {
        if mant == 0 { sign * f32::INFINITY } else { f32::NAN }
    } else {
        sign * (1 << 10 | mant) as f32 * 2.0f32.powi(exp - 25)
    }
}

// ── Statistical helpers ───────────────────────────────────────────────────────

fn pearson_correlation(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let n = a.len() as f64;
    let mean_a = a.iter().sum::<f64>() / n;
    let mean_b = b.iter().sum::<f64>() / n;
    let num: f64 = a.iter().zip(b.iter()).map(|(&x, &y)| (x - mean_a) * (y - mean_b)).sum();
    let den_a: f64 = a.iter().map(|&x| (x - mean_a).powi(2)).sum::<f64>().sqrt();
    let den_b: f64 = b.iter().map(|&y| (y - mean_b).powi(2)).sum::<f64>().sqrt();
    let den = den_a * den_b;
    if den < 1e-12 { 0.0 } else { (num / den).clamp(-1.0, 1.0) }
}

fn spec_scores_max(scores: &[f64]) -> f64 {
    scores.iter().cloned().fold(0.0f64, f64::max)
}

fn compute_fingerprint(blocks: &[BlockDNA]) -> u64 {
    // Simple rolling hash of block specialization and attention values
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for b in blocks {
        let a = (b.attn_weight_mean * 1000.0) as u64;
        let g = (b.gate_sparsity * 1000.0) as u64;
        let v = a ^ g.wrapping_shl(16);
        h = h.wrapping_mul(0x100000001b3).wrapping_add(v);
    }
    h
}

// ── Persistence ───────────────────────────────────────────────────────────────

/// Save a `ModelDNA` as a JSON sidecar next to the model file.
pub fn save_dna_sidecar(model_path: &Path, dna: &ModelDNA) -> anyhow::Result<()> {
    let sidecar_path = model_path.with_extension("gguf.dna.json");
    let json = serde_json::to_string_pretty(dna)?;
    std::fs::write(&sidecar_path, json)?;
    Ok(())
}

/// Load an existing DNA sidecar if present (avoids re-dissection).
pub fn load_dna_sidecar(model_path: &Path) -> Option<ModelDNA> {
    let sidecar_path = model_path.with_extension("gguf.dna.json");
    if !sidecar_path.exists() { return None; }
    let data = std::fs::read_to_string(&sidecar_path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Convert a `ModelDNA` into a `SpecialistGenome` for use with `genetics.rs`.
pub fn dna_to_genome(dna: &ModelDNA) -> SpecialistGenome {
    use crate::genetics::EpigeneticState;

    let mut genome = SpecialistGenome::new(
        dna.model_name.clone(),
        dna.model_name.replace(".gguf", "").replace('-', " "),
        dna.model_path.clone(),
    );

    for record in &dna.genetic_loci {
        let category = parse_category(&record.category);
        let source = parse_source(&record.source);
        let locus = GeneticLocus::new(
            record.locus_id.clone(),
            category,
            record.value.clamp(0.0, 1.0),
            source,
        )
        .with_interpretation(record.interpretation.clone())
        .with_confidence(record.confidence);
        genome.add_locus(locus);
    }

    // Set genome-level summary fields from DNA
    genome.specialization_score = dna.genome_loci.get("gate_sparsity").copied().unwrap_or(0.0);
    genome.genetic_distance_to_base = 1.0 - dna.genome_loci.get("cross_block_correlation_mean").copied().unwrap_or(1.0);

    // Epigenetic modulation: models with high gate sparsity are more "methylated"
    // (specialized genes suppressed in favour of domain focus)
    let sparsity = dna.genome_loci.get("gate_sparsity").copied().unwrap_or(0.5);
    genome.epigenetic_state = EpigeneticState {
        methylation: sparsity,
        chromatin_accessibility: 1.0 - sparsity,
        histone_modification: (dna.genome_loci.get("depth_gradient").copied().unwrap_or(0.0) - 0.5),
    };

    genome
}

pub fn parse_category_pub(s: &str) -> GeneticCategory {
    parse_category(s)
}

pub fn parse_source_pub(s: &str) -> LociSource {
    parse_source(s)
}

fn parse_category(s: &str) -> GeneticCategory {
    match s {
        "AttentionGenetics"      => GeneticCategory::AttentionGenetics,
        "LayerGenetics"          => GeneticCategory::LayerGenetics,
        "EmbeddingGenetics"      => GeneticCategory::EmbeddingGenetics,
        "BiasGenetics"           => GeneticCategory::BiasGenetics,
        "DAGGenetics"            => GeneticCategory::DAGGenetics,
        "RAGGenetics"            => GeneticCategory::RAGGenetics,
        "PersonalityGenetics"    => GeneticCategory::PersonalityGenetics,
        _                        => GeneticCategory::SpecializationGenetics,
    }
}

fn parse_source(s: &str) -> LociSource {
    match s {
        "WeightAnalysis"           => LociSource::WeightAnalysis,
        "AttentionPatternAnalysis" => LociSource::AttentionPatternAnalysis,
        "BehavioralProfiling"      => LociSource::BehavioralProfiling,
        "DAGAnalysis"              => LociSource::DAGAnalysis,
        "RAGAnalysis"              => LociSource::RAGAnalysis,
        _                          => LociSource::Inferred,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
