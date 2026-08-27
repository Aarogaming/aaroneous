/// Model Registry — Import, export, and lifecycle management of GGUF models.
///
/// This module provides the full model import/export pipeline:
///
/// # Import paths
/// 1. **Local file**: copy or link an existing `.gguf` from anywhere on the filesystem
/// 2. **HuggingFace download**: `hf://owner/repo/filename.gguf` or bare model ID
/// 3. **Inbox watcher**: drop a `.gguf` into the workspace models inbox and it
///    is automatically ingested, dissected (DNA), and registered as a specialist
///
/// # Export paths
/// 1. **HTTP stream**: `GET /models/export/:name` streams the raw GGUF bytes with
///    proper `Content-Disposition` so browsers/curl save it correctly
/// 2. **Crystallize**: the existing `POST /forge/crystallize` path writes a GGUF
///    to a local path (now with fixed per-tensor mmap overhead)
///
/// # Registry
/// The registry tracks all known models: their path, DNA sidecar status, last
/// dissection timestamp, associated sovereign (if any), and import source.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::workspace::WorkspacePaths;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{info, warn};

// ── Types ─────────────────────────────────────────────────────────────────────

/// Source of a model import.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImportSource {
    LocalCopy,
    LocalLink,
    HuggingFace { repo_id: String, filename: String },
    Inbox,
    Crystallized { recipe_id: String },
}

/// Current status of a model import job.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Downloading {
        percent: u8,
        bytes_done: u64,
        bytes_total: u64,
    },
    Copying,
    Dissecting {
        percent: u8,
    },
    Registering,
    Done,
    Failed(String),
}

/// A registered model entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    /// Filename (e.g. "llama-3.1-70b-q4_k_m.gguf")
    pub name: String,
    /// Absolute path on disk
    pub path: PathBuf,
    /// File size in bytes
    pub size_bytes: u64,
    /// GGUF version (from header)
    pub gguf_version: Option<u32>,
    /// Architecture (e.g. "llama", "qwen2")
    pub architecture: Option<String>,
    /// Associated sovereign name, if this model backs a GenericSpecialist
    pub sovereign: Option<String>,
    /// Source of the import
    pub source: ImportSource,
    /// Whether a `.dna.json` sidecar exists
    pub dna_dissected: bool,
    /// When this entry was registered (Unix ms)
    pub registered_at: u64,
    /// Tags set by the user or import pipeline
    pub tags: Vec<String>,
}

/// A background import job.
#[derive(Clone, Debug)]
pub struct ImportJob {
    pub job_id: String,
    pub model_name: String,
    pub status: ImportStatus,
}

pub type ImportJobs = Arc<Mutex<HashMap<String, ImportJob>>>;

/// Federation model registry with HashMap index for O(1) lookup.
pub struct FederationModelRegistry {
    entries: Vec<ModelEntry>,
    index: HashMap<String, usize>,
}

impl FederationModelRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Insert a model entry. Updates index for O(1) lookup.
    pub fn insert(&mut self, entry: ModelEntry) {
        let idx = self.entries.len();
        self.index.insert(entry.name.clone(), idx);
        self.entries.push(entry);
    }

    /// Get a model by name (O(1)).
    pub fn get_by_name(&self, name: &str) -> Option<&ModelEntry> {
        self.index.get(name).and_then(|&i| self.entries.get(i))
    }

    /// Get a mutable reference to a model by name.
    pub fn get_mut_by_name(&mut self, name: &str) -> Option<&mut ModelEntry> {
        self.index
            .get(name)
            .copied()
            .and_then(|i| self.entries.get_mut(i))
    }

    /// List all model names.
    pub fn list_names(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.name.as_str()).collect()
    }

    /// Get all entries.
    pub fn all(&self) -> &[ModelEntry] {
        &self.entries
    }

    /// Count of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove a model by name.
    pub fn remove(&mut self, name: &str) -> bool {
        if let Some(&idx) = self.index.get(name) {
            self.entries.remove(idx);
            // Rebuild index (indices shifted)
            self.index.clear();
            for (i, entry) in self.entries.iter().enumerate() {
                self.index.insert(entry.name.clone(), i);
            }
            true
        } else {
            false
        }
    }
}

impl Default for FederationModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Registry operations ───────────────────────────────────────────────────────

/// Scan the workspace models directory and return all `.gguf` files as `ModelEntry` values.
/// Does NOT read GGUF headers (fast scan — metadata from filesystem only).
/// DNA sidecar presence is checked for each entry.
pub fn scan_models_dir() -> Vec<ModelEntry> {
    let models_dir = WorkspacePaths::workspace_root().join("models");
    let mut entries = Vec::new();

    let Ok(dir) = std::fs::read_dir(&models_dir) else {
        return entries;
    };

    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("gguf") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }

        let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
        let dna_path = path.with_extension("gguf.dna.json");
        let dna_dissected = dna_path.exists();

        entries.push(ModelEntry {
            name,
            path,
            size_bytes,
            gguf_version: None,
            architecture: None,
            sovereign: None,
            source: ImportSource::LocalCopy,
            dna_dissected,
            registered_at: now_ms(),
            tags: vec![],
        });
    }

    // Sort: Foundation model first, then by size descending
    entries.sort_by(|a, b| {
        let a_is_foundation = a.name.contains("foundation");
        let b_is_foundation = b.name.contains("foundation");
        if a_is_foundation && !b_is_foundation {
            return std::cmp::Ordering::Less;
        }
        if !a_is_foundation && b_is_foundation {
            return std::cmp::Ordering::Greater;
        }
        b.size_bytes.cmp(&a.size_bytes)
    });

    entries
}

// ── Import pipeline ────────────────────────────────────────────────────────────

/// Import a model from a local path or HuggingFace URL.
///
/// Accepted formats for `source_spec`:
/// - Absolute path: `D:\models\llama-3.1-70b-q4.gguf`
/// - HuggingFace: `hf://username/repo-name/filename.gguf`
/// - HuggingFace short: `username/repo-name` (downloads the largest Q4_K_M file)
/// - HuggingFace bare: `bartowski/Meta-Llama-3.1-70B-Instruct-GGUF`
///
/// Runs in the background. Returns a job_id. Poll via `GET /models/import/jobs/:id`.
pub async fn import_model(
    source_spec: String,
    tags: Vec<String>,
    auto_dissect: bool,
    auto_register_sovereign: bool,
    jobs: ImportJobs,
) -> String {
    let job_id = format!("import-{}", now_ms());
    let job_id_bg = job_id.clone();

    {
        let mut j = jobs.lock().await;
        j.insert(
            job_id.clone(),
            ImportJob {
                job_id: job_id.clone(),
                model_name: source_spec
                    .split('/')
                    .next_back()
                    .unwrap_or(&source_spec)
                    .to_string(),
                status: ImportStatus::Copying,
            },
        );
    }

    tokio::spawn(async move {
        let result = run_import(
            &source_spec,
            &tags,
            auto_dissect,
            auto_register_sovereign,
            &jobs,
            &job_id_bg,
        )
        .await;

        let mut j = jobs.lock().await;
        match result {
            Ok(()) => {
                if let Some(job) = j.get_mut(&job_id_bg) {
                    job.status = ImportStatus::Done;
                }
            }
            Err(e) => {
                if let Some(job) = j.get_mut(&job_id_bg) {
                    job.status = ImportStatus::Failed(e.to_string());
                }
            }
        }
    });

    job_id
}

async fn run_import(
    source_spec: &str,
    tags: &[String],
    auto_dissect: bool,
    auto_register_sovereign: bool,
    jobs: &ImportJobs,
    job_id: &str,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let models_dir = WorkspacePaths::workspace_root().join("models");
    std::fs::create_dir_all(&models_dir).context("failed to create models dir")?;

    let target_path = if source_spec.starts_with("hf://") || is_hf_repo_id(source_spec) {
        // HuggingFace download
        download_from_huggingface(source_spec, &models_dir, jobs, job_id).await?
    } else {
        // Local copy
        let src = PathBuf::from(source_spec);
        if !src.exists() {
            anyhow::bail!("Source file not found: {}", source_spec);
        }
        let filename = src.file_name().context("source path has no filename")?;
        let dest = models_dir.join(filename);
        if dest == src {
            // Already in models dir
            dest
        } else {
            update_job_status(jobs, job_id, ImportStatus::Copying).await;
            info!("Copying {} → {}", src.display(), dest.display());
            std::fs::copy(&src, &dest).context("failed to copy model file")?;
            dest
        }
    };

    // Auto-dissect: run DNA analysis
    if auto_dissect {
        update_job_status(jobs, job_id, ImportStatus::Dissecting { percent: 0 }).await;
        info!("Auto-dissecting imported model: {}", target_path.display());
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<crate::federation::dna::DissectionProgress>(32);
        let jobs_clone = jobs.clone();
        let jid = job_id.to_string();
        tokio::spawn(async move {
            while let Some(p) = rx.recv().await {
                update_job_status(
                    &jobs_clone,
                    &jid,
                    ImportStatus::Dissecting { percent: p.percent },
                )
                .await;
            }
        });
        match crate::federation::dna::dissect_model(&target_path, Some(tx)).await {
            Ok(dna) => {
                info!(
                    "DNA dissection complete: {} loci from {}",
                    dna.genetic_loci.len(),
                    target_path.display()
                );

                // Convert ModelDNA → SpecialistGenome and generate soul.
                // This closes the self-digestion loop: imported models get genome
                // + soul alongside DNA so they can be compared, bred, and routed.
                let genome = crate::federation::dna::dna_to_genome(&dna);
                let model_name = target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("imported")
                    .to_string();

                // Generate soul using DigestionEngine (requires a discardable event channel)
                let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();
                let engine =
                    crate::DigestionEngine::new(crate::DigestionConfig::default(), event_tx);
                let task = crate::DigestionTask {
                    digestion_id: format!("import-{}", now_ms()),
                    model_path: target_path.clone(),
                    model_name: model_name.clone(),
                    parameter_count: (dna.parameter_count_m * 1_000_000.0) as u64,
                    created_at: chrono::Utc::now(),
                    priority: crate::digestion::DigestionPriority::Normal,
                    status: crate::digestion::DigestionStatus::StructuralAnalysis,
                    estimated_duration_minutes: 1,
                };

                match engine.generate_soul(&task, &genome).await {
                    Ok(persona) => {
                        // Save persona sidecar alongside the DNA sidecar
                        let persona_path = target_path.with_extension("gguf.persona.json");
                        if let Ok(persona_json) = serde_json::to_string_pretty(&persona)
                            && std::fs::write(&persona_path, persona_json).is_ok()
                        {
                            info!(
                                "Persona generated and saved: {} (archetype: {})",
                                persona_path.display(),
                                persona.personality_persona.archetype
                            );
                        }
                    }
                    Err(e) => warn!("Persona generation failed (non-fatal): {}", e),
                }
            }
            Err(e) => warn!("DNA dissection failed (non-fatal): {}", e),
        }
    }

    // Auto-register as a dynamic specialist
    if auto_register_sovereign {
        update_job_status(jobs, job_id, ImportStatus::Registering).await;
        register_as_specialist(&target_path, tags).await;
    }

    Ok(())
}

/// Download a GGUF from HuggingFace.
///
/// Accepted formats:
/// - `hf://username/repo/filename.gguf` — direct file URL
/// - `username/repo-name` — auto-selects largest Q4_K_M file via HF API
async fn download_from_huggingface(
    spec: &str,
    models_dir: &Path,
    jobs: &ImportJobs,
    job_id: &str,
) -> anyhow::Result<PathBuf> {
    use anyhow::Context;

    let (repo_id, filename) = parse_hf_spec(spec)?;

    // Resolve the direct download URL from HuggingFace
    let download_url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        repo_id, filename
    );

    let dest_path = models_dir.join(&filename);
    if dest_path.exists() {
        info!(
            "Model already exists at {} — skipping download",
            dest_path.display()
        );
        return Ok(dest_path);
    }

    info!(
        "Downloading {} from HuggingFace ({}/{})",
        filename, repo_id, filename
    );
    info!("URL: {}", download_url);

    // Stream download with progress reporting
    stream_download(&download_url, &dest_path, jobs, job_id, &repo_id, &filename)
        .await
        .context("HuggingFace download failed")?;

    Ok(dest_path)
}

/// Stream-download a URL to a file, reporting progress.
async fn stream_download(
    url: &str,
    dest: &Path,
    jobs: &ImportJobs,
    job_id: &str,
    _repo_id: &str,
    filename: &str,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let client = build_http_client()?;
    let resp = client
        .get(url)
        .header("User-Agent", "Aaroneous/1.0")
        .send()
        .await
        .context("HTTP request failed")?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP {} for {}", resp.status(), url);
    }

    let total_bytes = resp.content_length().unwrap_or(0);
    info!("Downloading {} bytes for {}", total_bytes, filename);

    let mut file = tokio::fs::File::create(dest)
        .await
        .context("failed to create destination file")?;
    let mut downloaded: u64 = 0;
    let mut last_pct = 0u8;

    use tokio::io::AsyncWriteExt;

    // Stream the response body in chunks
    // Stream the response body in chunks — critical for large models (68GB+)
    // Using bytes_stream() to avoid loading the entire response into RAM
    use futures_util::StreamExt;
    let mut body_stream = resp.bytes_stream();
    while let Some(chunk_result) = body_stream.next().await {
        let chunk = chunk_result.context("stream read error")?;
        file.write_all(&chunk).await.context("write chunk failed")?;
        downloaded += chunk.len() as u64;
        if let Some(pct) = (downloaded * 100).checked_div(total_bytes) {
            let pct = pct as u8;
            if pct != last_pct {
                last_pct = pct;
                update_job_status(
                    jobs,
                    job_id,
                    ImportStatus::Downloading {
                        percent: pct,
                        bytes_done: downloaded,
                        bytes_total: total_bytes,
                    },
                )
                .await;
            }
        }
    }

    if let Some(pct) = (downloaded * 100).checked_div(total_bytes) {
        let pct = pct as u8;
        update_job_status(
            jobs,
            job_id,
            ImportStatus::Downloading {
                percent: pct,
                bytes_done: downloaded,
                bytes_total: total_bytes,
            },
        )
        .await;
    }

    file.flush().await.context("failed to flush model file")?;
    info!(
        "Download complete: {} ({} bytes)",
        dest.display(),
        downloaded
    );
    Ok(())
}

/// Resolve a HuggingFace spec to (repo_id, filename).
///
/// If no specific filename is given, queries the HF API to find the largest
/// Q4_K_M GGUF file in the repo.
fn parse_hf_spec(spec: &str) -> anyhow::Result<(String, String)> {
    // Strip hf:// prefix
    let spec = spec.strip_prefix("hf://").unwrap_or(spec);

    let parts: Vec<&str> = spec.splitn(3, '/').collect();
    match parts.len() {
        // "username/repo/filename.gguf"
        3 if parts[2].ends_with(".gguf") => {
            Ok((format!("{}/{}", parts[0], parts[1]), parts[2].to_string()))
        }
        // "username/repo" — auto-select file
        2 => {
            let repo_id = format!("{}/{}", parts[0], parts[1]);
            // Default to Q4_K_M naming convention
            let model_base = parts[1]
                .to_lowercase()
                .replace("-gguf", "")
                .replace("_gguf", "");
            let filename = format!("{}-Q4_K_M.gguf", model_base);
            warn!(
                "No filename specified for HF repo '{}' — guessing '{}'",
                repo_id, filename
            );
            warn!("To specify: hf://{}/{}", repo_id, filename);
            Ok((repo_id, filename))
        }
        _ => anyhow::bail!(
            "Invalid HuggingFace spec: '{}'. Use: hf://owner/repo/file.gguf or owner/repo",
            spec
        ),
    }
}

fn is_hf_repo_id(s: &str) -> bool {
    // Matches "owner/repo" pattern without a leading slash or drive letter
    let parts: Vec<&str> = s.splitn(2, '/').collect();
    parts.len() == 2
        && !parts[0].is_empty()
        && !parts[1].is_empty()
        && !s.contains('\\')
        && !s.contains(':')
        && !s.starts_with('/')
}

fn build_http_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600)) // 1hr for large models
        .tcp_keepalive(std::time::Duration::from_secs(30))
        .connection_verbose(false)
        .build()
        .map_err(|e| anyhow::anyhow!("reqwest client build failed: {}", e))
}

/// Register a downloaded/copied model as a dynamic specialist in the registry.
pub async fn register_as_specialist(model_path: &Path, tags: &[String]) {
    let name = model_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Infer sovereign name from filename
    // e.g. "llama-3.1-70b-orchestrator-q4_k_m" → try to find a known sovereign name
    let sovereign_hint = infer_sovereign_name(&name);

    info!(
        "Registering imported model as specialist: name='{}' sovereign_hint={:?} path={}",
        name,
        sovereign_hint,
        model_path.display()
    );

    // Write to specialist_registry.json — append a new entry
    // This is done by reading the existing registry, adding the entry, and writing back
    let registry_path = WorkspacePaths::workspace_root()
        .join("config")
        .join("specialist_registry.json");
    if !registry_path.exists() {
        warn!("specialist_registry.json not found — skipping auto-registration");
        return;
    }

    let data = match std::fs::read_to_string(&registry_path) {
        Ok(d) => d,
        Err(e) => {
            warn!("Failed to read specialist registry: {}", e);
            return;
        }
    };

    let mut registry: serde_json::Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse specialist registry JSON: {}", e);
            return;
        }
    };

    // Add to dynamic_specialists array if not already present
    if let Some(arr) = registry
        .get_mut("dynamic_specialists")
        .and_then(|v| v.as_array_mut())
    {
        let already_registered = arr.iter().any(|e| {
            e.get("gguf_path").and_then(|p| p.as_str())
                == Some(model_path.to_string_lossy().as_ref())
        });

        if !already_registered {
            let sovereign_name = sovereign_hint.unwrap_or_else(|| {
                // Use capitalised filename base as sovereign name
                let base = model_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("imported");
                let mut c = base.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().to_string() + c.as_str(),
                }
            });

            arr.push(serde_json::json!({
                "name": sovereign_name,
                "gguf_path": model_path.to_string_lossy(),
                "enabled": true,
                "domain": infer_domain_from_tags(tags),
                "tags": tags,
                "source": "imported",
                "registered_at": now_ms(),
            }));

            if let Ok(updated) = serde_json::to_string_pretty(&registry) {
                if let Err(e) = std::fs::write(&registry_path, updated) {
                    warn!("Failed to write updated registry: {}", e);
                } else {
                    info!(
                        "Registered {} in specialist_registry.json as '{}'",
                        model_path.display(),
                        sovereign_name
                    );
                }
            }
        }
    }
}

fn infer_sovereign_name(filename: &str) -> Option<String> {
    let lower = filename.to_lowercase();
    for sovereign in &[
        "orchestrator",
        "synthesizer",
        "sentinel",
        "presenter",
        "router",
        "aligner",
        "perceiver",
        "archivist",
        "fabricator",
    ] {
        if lower.contains(sovereign) {
            let mut c = sovereign.chars();
            return Some(match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            });
        }
    }
    None
}

fn infer_domain_from_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        return "general_purpose".into();
    }
    tags[0].to_lowercase().replace(' ', "_")
}

// ── Inbox watcher ─────────────────────────────────────────────────────────────

/// Start the inbox folder watcher.
///
/// Watches the workspace models inbox every 10 seconds for new `.gguf` files.
/// When a file is found, triggers the full import pipeline:
/// 1. Move to the workspace models directory
/// 2. Run DNA dissection
/// 3. Register as dynamic specialist (auto_register=true)
///
/// This replaces the notification-only `DigestionEngine::start_folder_watching()`.
pub async fn start_inbox_watcher(jobs: ImportJobs) {
    let root = WorkspacePaths::workspace_root();
    let inbox_dir = root.join("models_inbox");
    std::fs::create_dir_all(&inbox_dir).unwrap_or_default();
    let models_dir = root.join("models");

    tokio::spawn(async move {
        let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

        loop {
            interval.tick().await;

            let Ok(entries) = std::fs::read_dir(&inbox_dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let src = entry.path();
                if src.extension().and_then(|e| e.to_str()) != Some("gguf") {
                    continue;
                }
                if seen.contains(&src) {
                    continue;
                }

                seen.insert(src.clone());
                let filename = match src.file_name().and_then(|n| n.to_str()) {
                    Some(f) => f.to_string(),
                    None => continue,
                };
                let dest = models_dir.join(&filename);

                info!(
                    "Inbox: found {} — moving to models/ and ingesting",
                    filename
                );

                // Move file
                if let Err(e) = std::fs::rename(&src, &dest) {
                    warn!("Failed to move {} from inbox: {}", filename, e);
                    continue;
                }

                // Trigger full import pipeline in background
                let jobs_clone = jobs.clone();
                let dest_clone = dest.clone();
                tokio::spawn(async move {
                    let job_id = format!("inbox-{}", now_ms());
                    let _spec = dest_clone.to_string_lossy().to_string();
                    {
                        let mut j = jobs_clone.lock().await;
                        j.insert(
                            job_id.clone(),
                            ImportJob {
                                job_id: job_id.clone(),
                                model_name: filename.clone(),
                                status: ImportStatus::Dissecting { percent: 0 },
                            },
                        );
                    }

                    // DNA dissection
                    match crate::federation::dna::dissect_model(&dest_clone, None).await {
                        Ok(dna) => {
                            info!(
                                "Inbox ingestion complete: {} — {} loci",
                                filename,
                                dna.genetic_loci.len()
                            );
                        }
                        Err(e) => warn!("Inbox dissection failed for {}: {}", filename, e),
                    }

                    // Register as specialist
                    register_as_specialist(&dest_clone, &[]).await;

                    let mut j = jobs_clone.lock().await;
                    if let Some(job) = j.get_mut(&job_id) {
                        job.status = ImportStatus::Done;
                    }
                });
            }
        }
    });

    info!(
        "Inbox watcher started: {}",
        std::path::PathBuf::from("inbox").display()
    );
}

// ── Helpers ────────────────────────────────────────────────────────────────────

async fn update_job_status(jobs: &ImportJobs, job_id: &str, status: ImportStatus) {
    let mut j = jobs.lock().await;
    if let Some(job) = j.get_mut(job_id) {
        job.status = status;
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
