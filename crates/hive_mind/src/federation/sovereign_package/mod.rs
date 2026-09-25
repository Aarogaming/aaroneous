/// SovereignPackage — portable specialist export/import format.
///
pub mod auto_fabricator;

use serde::{Deserialize, Serialize};
/// A `.sovereign` file is a zstd-compressed tar archive containing:
///
/// ```text
/// my-sovereign.sovereign
/// ├── manifest.json          — identity, version, capabilities, DNA hash
/// ├── model.gguf             — the crystallized weight file
/// ├── dna.json               — full ModelDNA genome sidecar
/// ├── system_prompt.txt      — the persona system prompt (plain text)
/// ├── learning_state.json    — confidence, execution history snapshot
/// └── specialist_config.json — domain, quantization, block selection
/// ```
///
/// This format enables:
/// - **Export**: `POST /specialists/export/Synthesizer` → `Synthesizer.sovereign`
/// - **Import**: `POST /specialists/import` with multipart file upload
/// - **Hive federation**: another Aaroneous instance imports the package and
///   immediately has a fully functional Synthesizer specialist with its learned
///   confidence scores and persona intact
/// - **Standalone operation**: the system_prompt.txt + model.gguf are enough
///   for llama.cpp to run the specialist without Aaroneous at all
///
/// # Wire format
///
/// The outer container is a zstd-compressed tar stream. Each entry is stored
/// uncompressed within the tar (zstd compresses the stream as a whole).
/// Typical sizes: Synthesizer.sovereign ≈ 2.9 GB (dominated by model.gguf).
///
/// The manifest is always the first entry in the archive so consumers can
/// read the identity without decompressing the full file.
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

// ── Manifest ──────────────────────────────────────────────────────────────────

/// The manifest.json entry — first file in every .sovereign archive.
/// Can be read by streaming only the first few kilobytes of the file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignManifest {
    /// Schema version of this manifest format
    pub schema_version: u32,
    /// Sovereign display name (e.g. "Synthesizer")
    pub sovereign_name: String,
    /// Internal domain key (e.g. "research")
    pub domain: String,
    /// Aaroneous version that created this package
    pub aaroneous_version: String,
    /// GGUF architecture (e.g. "llama", "qwen2")
    pub architecture: String,
    /// Approximate parameter count in millions
    pub parameter_count_m: f64,
    /// File size of the embedded model.gguf in bytes
    pub model_size_bytes: u64,
    /// DNA fingerprint (64-bit FNV-1a hash of block DNA signatures — non-cryptographic)
    pub dna_fingerprint: u64,
    /// SHA-256 hex digest of model.gguf (for integrity verification)
    pub model_sha256: String,
    /// Base model the sovereign was crystallized from
    pub base_model: String,
    /// Whether the base model was abliterated
    pub abliterated: bool,
    /// Quantization format (e.g. "Q4_K_M", "Q8_0")
    pub quantization: String,
    /// Block selection geometry (e.g. "0,2,4,6,8,10,12,14,16,18")
    pub block_selection: String,
    /// Blocks included count
    pub block_count: u32,
    /// Source hive endpoint (optional — where this sovereign came from)
    pub source_hive: Option<String>,
    /// Creation timestamp (Unix milliseconds)
    pub created_at: u64,
    /// Human-readable creation date
    pub created_at_human: String,
    /// Tags applied to this sovereign
    pub tags: Vec<String>,
    /// Capabilities list (from SovereignTaskSpec)
    pub capabilities: Vec<String>,
}

impl SovereignManifest {
    pub fn schema_version() -> u32 {
        1
    }
}

/// Learning state snapshot — included so imported specialists start with
/// their learned confidence rather than resetting to 0.5.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStateSnapshot {
    pub sovereign_name: String,
    pub confidence_score: f32,
    pub total_executions: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub execution_history: Vec<bool>,
    pub confidence_trend: Vec<(u64, f32)>,
    pub last_updated: u64,
}

// ── Export ────────────────────────────────────────────────────────────────────

/// Build options for creating a sovereign package.
pub struct PackageOptions {
    /// Include the learning state (confidence history). Default: true.
    pub include_learning_state: bool,
    /// Include the full DNA sidecar. Default: true.
    pub include_dna: bool,
    /// zstd compression level 1-22. Default: 3 (fast, good ratio).
    pub compression_level: i32,
    /// Source hive endpoint to embed in the manifest.
    pub source_hive: Option<String>,
    /// Tags to embed.
    pub tags: Vec<String>,
}

impl Default for PackageOptions {
    fn default() -> Self {
        Self {
            include_learning_state: true,
            include_dna: true,
            compression_level: 3,
            source_hive: None,
            tags: vec![],
        }
    }
}

/// Create a `.sovereign` package from a sovereign's GGUF file and associated data.
///
/// Returns the path to the created package file.
pub async fn export_sovereign(
    sovereign_name: &str,
    gguf_path: &Path,
    output_dir: &Path,
    learning_state: Option<LearningStateSnapshot>,
    opts: PackageOptions,
) -> anyhow::Result<PathBuf> {
    use anyhow::Context;

    let package_path = output_dir.join(format!("{}.sovereign", sovereign_name.to_lowercase()));
    std::fs::create_dir_all(output_dir).context("failed to create output dir")?;

    info!(
        "Exporting sovereign '{}' from {} → {}",
        sovereign_name,
        gguf_path.display(),
        package_path.display()
    );

    // Load DNA sidecar
    let dna = crate::federation::dna::load_dna_sidecar(gguf_path);

    // Read GGUF metadata for the manifest
    let (_, meta) = crate::federation::forge::read_gguf(gguf_path)
        .map_err(|e| anyhow::anyhow!("GGUF read error: {}", e))?;

    let model_size_bytes = std::fs::metadata(gguf_path)
        .map(|m| m.len())
        .context("cannot stat GGUF file")?;

    // Compute SHA-256 of the model file (streaming, no full-RAM load via mmap)
    let model_sha256 = compute_sha256_hex(gguf_path).unwrap_or_else(|_| "unavailable".to_string());

    // Extract aaroneous.* fields from GGUF KV
    let kv = &meta.kv;
    let domain = kv.get("aaroneous.domain").cloned().unwrap_or_default();
    let system_prompt = kv
        .get("aaroneous.system_prompt")
        .cloned()
        .unwrap_or_else(|| {
            crate::federation::specialists::system_prompt_for_domain(&domain, sovereign_name)
        });
    let quantization = kv
        .get("aaroneous.quantization")
        .cloned()
        .unwrap_or_else(|| "Q4_K_M".to_string());
    let block_sel = kv
        .get("aaroneous.block_selection")
        .cloned()
        .unwrap_or_default();
    let block_count = kv
        .get("aaroneous.block_count")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let base_model = kv
        .get("aaroneous.base_model")
        .cloned()
        .unwrap_or_else(|| "foundation_v1.gguf".to_string());
    let abliterated = kv
        .get("aaroneous.base_variant")
        .map(|v| v == "abliterated")
        .unwrap_or(false);

    // Capabilities from task spec
    let capabilities = crate::federation::graph::task_spec::spec_for(sovereign_name)
        .map(|s| {
            s.capabilities
                .iter()
                .map(|c| c.description.clone())
                .collect()
        })
        .unwrap_or_default();

    let dna_fingerprint = dna.as_ref().map(|d| d.dna_fingerprint).unwrap_or(0);
    let param_count_m = dna.as_ref().map(|d| d.parameter_count_m).unwrap_or(0.0);

    // Build manifest
    let now_ms = now_ms();
    let created_human = {
        let secs = now_ms / 1000;
        let dt =
            chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0).unwrap_or_default();
        dt.format("%Y-%m-%d %H:%M UTC").to_string()
    };

    let manifest = SovereignManifest {
        schema_version: SovereignManifest::schema_version(),
        sovereign_name: sovereign_name.to_string(),
        domain: domain.clone(),
        aaroneous_version: env!("CARGO_PKG_VERSION").to_string(),
        architecture: meta.architecture.clone(),
        parameter_count_m: param_count_m,
        model_size_bytes,
        dna_fingerprint,
        model_sha256,
        base_model,
        abliterated,
        quantization,
        block_selection: block_sel,
        block_count,
        source_hive: opts.source_hive,
        created_at: now_ms,
        created_at_human: created_human,
        tags: opts.tags,
        capabilities,
    };

    // ── Write the .sovereign archive ──────────────────────────────────────────
    // Structure: zstd(tar(manifest.json, model.gguf, dna.json, system_prompt.txt,
    //                      learning_state.json, specialist_config.json))
    let out_file = std::fs::File::create(&package_path).context("failed to create package file")?;

    // Wrap in BufWriter + zstd encoder
    let buf_writer = std::io::BufWriter::with_capacity(8 * 1024 * 1024, out_file);
    let zstd_encoder = zstd::stream::Encoder::new(buf_writer, opts.compression_level)
        .context("zstd encoder init failed")?;
    let mut tar = tar::Builder::new(zstd_encoder);

    // Entry 1: manifest.json (always first — allows streaming header read)
    let manifest_json = serde_json::to_vec_pretty(&manifest)?;
    add_bytes_to_tar(&mut tar, "manifest.json", &manifest_json)?;

    // Entry 2: system_prompt.txt (plain text for llama.cpp / standalone use)
    add_bytes_to_tar(&mut tar, "system_prompt.txt", system_prompt.as_bytes())?;

    // Entry 3: specialist_config.json
    let config = serde_json::json!({
        "sovereign_name": sovereign_name,
        "domain": domain,
        "quantization": manifest.quantization,
        "block_count": manifest.block_count,
        "block_selection": manifest.block_selection,
        "architecture": meta.architecture,
        "context_length": meta.context_length,
        "kv_metadata": kv,
    });
    add_bytes_to_tar(
        &mut tar,
        "specialist_config.json",
        &serde_json::to_vec_pretty(&config)?,
    )?;

    // Entry 4: learning_state.json
    if opts.include_learning_state {
        let ls = learning_state.unwrap_or_else(|| LearningStateSnapshot {
            sovereign_name: sovereign_name.to_string(),
            confidence_score: 0.5,
            total_executions: 0,
            success_count: 0,
            failure_count: 0,
            execution_history: vec![],
            confidence_trend: vec![],
            last_updated: now_ms,
        });
        add_bytes_to_tar(
            &mut tar,
            "learning_state.json",
            &serde_json::to_vec_pretty(&ls)?,
        )?;
    }

    // Entry 5: dna.json (full genome)
    if opts.include_dna
        && let Some(ref d) = dna
    {
        add_bytes_to_tar(&mut tar, "dna.json", &serde_json::to_vec_pretty(d)?)?;
    }

    // Entry 6: model.gguf — the largest entry, streamed via mmap
    // Add as a regular file entry using the GGUF's path
    add_file_to_tar(&mut tar, "model.gguf", gguf_path)?;

    // Finish the archive
    let zstd_encoder = tar.into_inner().context("tar finalize failed")?;
    zstd_encoder.finish().context("zstd finalize failed")?;

    let pkg_size = std::fs::metadata(&package_path)
        .map(|m| m.len() as f64 / 1_073_741_824.0)
        .unwrap_or(0.0);

    info!(
        "Sovereign package created: {} ({:.2}GB) — manifest+system_prompt+dna+learning+model",
        package_path.display(),
        pkg_size
    );

    Ok(package_path)
}

// ── Import ────────────────────────────────────────────────────────────────────

/// Result of importing a sovereign package.
pub struct ImportResult {
    pub manifest: SovereignManifest,
    pub gguf_path: PathBuf,
    pub learning_state: Option<LearningStateSnapshot>,
    pub dna_path: Option<PathBuf>,
}

/// Import a `.sovereign` package into the models directory.
///
/// Extracts the archive to `models_dir/` and registers the sovereign.
pub async fn import_sovereign(
    package_path: &Path,
    models_dir: &Path,
    register: bool,
) -> anyhow::Result<ImportResult> {
    use anyhow::Context;

    info!("Importing sovereign package: {}", package_path.display());
    std::fs::create_dir_all(models_dir).context("failed to create models dir")?;

    let file = std::fs::File::open(package_path).context("failed to open package")?;
    let buf_reader = std::io::BufReader::new(file);
    let zstd_decoder =
        zstd::stream::Decoder::new(buf_reader).context("zstd decoder init failed")?;
    let mut tar = tar::Archive::new(zstd_decoder);

    let mut manifest: Option<SovereignManifest> = None;
    let mut learning_state: Option<LearningStateSnapshot> = None;
    let mut gguf_path: Option<PathBuf> = None;
    let mut dna_path: Option<PathBuf> = None;

    for entry in tar.entries().context("tar entries failed")? {
        let mut entry = entry.context("tar entry read failed")?;
        let path = entry.path().context("tar entry path failed")?.to_path_buf();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        match name.as_str() {
            "manifest.json" => {
                let mut buf = String::new();
                entry
                    .read_to_string(&mut buf)
                    .context("manifest read failed")?;
                manifest = Some(serde_json::from_str(&buf).context("manifest parse failed")?);
            }
            "learning_state.json" => {
                let mut buf = String::new();
                entry
                    .read_to_string(&mut buf)
                    .context("learning_state read failed")?;
                learning_state = serde_json::from_str(&buf).ok();
            }
            "model.gguf" => {
                let sovereign_name = manifest
                    .as_ref()
                    .map(|m| m.sovereign_name.to_lowercase())
                    .unwrap_or_else(|| "imported".to_string());
                let dest = models_dir.join(format!("{}-imported.gguf", sovereign_name));
                let mut out =
                    std::fs::File::create(&dest).context("failed to create model file")?;
                std::io::copy(&mut entry, &mut out).context("failed to extract model.gguf")?;
                gguf_path = Some(dest);
            }
            "dna.json" => {
                if let Some(ref gguf) = gguf_path {
                    let sidecar = gguf.with_extension("gguf.dna.json");
                    let mut out =
                        std::fs::File::create(&sidecar).context("failed to create dna sidecar")?;
                    std::io::copy(&mut entry, &mut out).context("failed to extract dna.json")?;
                    dna_path = Some(sidecar);
                }
            }
            _ => {} // system_prompt.txt, specialist_config.json — skip to GGUF header
        }
    }

    let manifest = manifest.context("no manifest.json in package")?;
    let gguf_path = gguf_path.context("no model.gguf in package")?;

    // Verify integrity: check SHA-256 of extracted model against manifest
    if !manifest.model_sha256.is_empty() {
        let actual_hash = compute_sha256_hex(&gguf_path)?;
        if actual_hash != manifest.model_sha256 {
            anyhow::bail!(
                "Integrity check failed: model hash mismatch\n  Expected: {}\n  Actual:   {}",
                manifest.model_sha256,
                actual_hash
            );
        }
        info!("Integrity verified: SHA-256 matches manifest");
    } else {
        warn!("No model_sha256 in manifest — skipping integrity check");
    }

    info!(
        "Sovereign '{}' (domain={}) imported to {}",
        manifest.sovereign_name,
        manifest.domain,
        gguf_path.display()
    );

    // Register in specialist_registry.json
    if register {
        crate::federation::model_registry::register_as_specialist(&gguf_path, &manifest.tags).await;
    }

    // Re-dissect if no DNA sidecar was embedded
    if dna_path.is_none() {
        info!("No DNA in package — running dissection on imported model");
        if let Err(e) = crate::federation::dna::dissect_model(&gguf_path, None).await {
            warn!("Post-import DNA dissection failed: {}", e);
        }
    }

    Ok(ImportResult {
        manifest,
        gguf_path,
        learning_state,
        dna_path,
    })
}

/// Read ONLY the manifest from a .sovereign file (fast — decompresses just the first entry).
pub fn read_manifest(package_path: &Path) -> anyhow::Result<SovereignManifest> {
    use anyhow::Context;
    let file = std::fs::File::open(package_path).context("failed to open package")?;
    let buf = std::io::BufReader::new(file);
    let dec = zstd::stream::Decoder::new(buf).context("zstd decode failed")?;
    let mut tar = tar::Archive::new(dec);
    for entry in tar.entries().context("tar entries failed")? {
        let mut entry = entry.context("tar entry failed")?;
        let name = entry
            .path()?
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if name == "manifest.json" {
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            return Ok(serde_json::from_str(&buf)?);
        }
    }
    anyhow::bail!("no manifest.json in package")
}

/// Verify a `.sovereign` package without importing it.
///
/// Extracts to a temp directory, checks SHA-256 integrity, then cleans up.
/// Returns the manifest and verification status.
pub fn verify_sovereign(package_path: &Path) -> anyhow::Result<SovereignVerification> {
    use anyhow::Context;
    use std::io::Read;

    info!("Verifying sovereign package: {}", package_path.display());

    let manifest = read_manifest(package_path)?;

    let file = std::fs::File::open(package_path).context("failed to open package")?;
    let buf = std::io::BufReader::new(file);
    let dec = zstd::stream::Decoder::new(buf).context("zstd decode failed")?;
    let mut tar = tar::Archive::new(dec);

    let mut gguf_size: u64 = 0;
    let mut has_model = false;

    for entry in tar.entries().context("tar entries failed")? {
        let mut entry = entry.context("tar entry failed")?;
        let name = entry
            .path()?
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if name == "model.gguf" {
            has_model = true;
            // Read to a temp buffer to compute hash
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            gguf_size = buf.len() as u64;

            let hash = {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(&buf);
                hex::encode(hasher.finalize())
            };

            let hash_ok = if manifest.model_sha256.is_empty() {
                true // No hash to verify against
            } else {
                hash == manifest.model_sha256
            };

            return Ok(SovereignVerification {
                manifest,
                gguf_size,
                hash_ok,
                has_model,
                actual_hash: hash,
            });
        }
    }

    Ok(SovereignVerification {
        manifest,
        gguf_size,
        hash_ok: false,
        has_model,
        actual_hash: String::new(),
    })
}

/// Result of sovereign package verification.
#[derive(Debug, Clone)]
pub struct SovereignVerification {
    pub manifest: SovereignManifest,
    pub gguf_size: u64,
    pub hash_ok: bool,
    pub has_model: bool,
    pub actual_hash: String,
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn add_bytes_to_tar<W: Write>(
    tar: &mut tar::Builder<W>,
    name: &str,
    data: &[u8],
) -> anyhow::Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, name, data)?;
    Ok(())
}

fn add_file_to_tar<W: Write>(
    tar: &mut tar::Builder<W>,
    archive_name: &str,
    file_path: &Path,
) -> anyhow::Result<()> {
    let size = std::fs::metadata(file_path)?.len();
    let mut header = tar::Header::new_gnu();
    header.set_size(size);
    header.set_mode(0o644);
    header.set_cksum();
    let file = std::fs::File::open(file_path)?;
    let buf = std::io::BufReader::with_capacity(8 * 1024 * 1024, file);
    tar.append_data(&mut header, archive_name, buf)?;
    Ok(())
}

fn compute_sha256_hex(path: &Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::with_capacity(8 * 1024 * 1024, file);

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
