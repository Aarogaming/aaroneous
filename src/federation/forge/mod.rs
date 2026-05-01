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
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

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
    /// This is the native Rust equivalent of `tensor_forge::crystallize_hybrid()`.
    /// It runs on `tokio::task::spawn_blocking` since mmap + file I/O is blocking.
    ///
    /// # Output format
    ///
    /// The output is **not** a fully valid GGUF yet — it is a "bare spliced" file
    /// containing the magic bytes `GGUF` followed by raw tensor data in recipe order.
    /// A full GGUF header (metadata, tensor info table) would be needed for models
    /// that require it; for experimental inference via direct tensor access, the bare
    /// format is sufficient.
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
            let mut output = File::create(&output_path).map_err(|e| ForgeError::OutputCreate {
                path: output_path.clone(),
                source: e,
            })?;

            // GGUF magic header
            output.write_all(b"GGUF")?;

            let mut bytes_written: u64 = 4;
            let mut tensors_spliced: u64 = 0;
            let mut spliced_tensors: Vec<SplicedTensorInfo> = Vec::new();

            for segment in &recipe.segments {
                let gguf_meta = index.0.get(&segment.source_gguf)
                    .ok_or_else(|| ForgeError::GgufNotFound {
                        gguf: segment.source_gguf.clone(),
                    })?;

                let tensor_meta = gguf_meta.tensors.get(&segment.tensor_name)
                    .ok_or_else(|| ForgeError::TensorNotFound {
                        tensor: segment.tensor_name.clone(),
                        gguf: segment.source_gguf.clone(),
                    })?;

                debug!(
                    "Splicing tensor '{}' from {} (offset={}, size={}B)",
                    segment.tensor_name,
                    segment.source_gguf,
                    tensor_meta.offset,
                    tensor_meta.size
                );

                let source_file = File::open(&gguf_meta.path).map_err(|e| ForgeError::SourceOpen {
                    path: gguf_meta.path.clone(),
                    source: e,
                })?;

                let mmap = unsafe {
                    Mmap::map(&source_file).map_err(|e| ForgeError::MmapFailed {
                        path: gguf_meta.path.clone(),
                        source: e,
                    })?
                };

                let start = tensor_meta.offset as usize;
                let end = start + tensor_meta.size as usize;

                if end > mmap.len() {
                    return Err(ForgeError::OutOfBounds {
                        path: gguf_meta.path.clone(),
                        offset: tensor_meta.offset,
                        size: tensor_meta.size,
                        file_len: mmap.len(),
                    });
                }

                output.write_all(&mmap[start..end])?;
                bytes_written += tensor_meta.size;
                tensors_spliced += 1;
                spliced_tensors.push(SplicedTensorInfo {
                    source: segment.source_gguf.clone(),
                    name: segment.tensor_name.clone(),
                    size: tensor_meta.size,
                    kind: tensor_meta.kind.clone(),
                });
            }

            info!(
                "Crystallization complete: {} tensors, {}B → {}",
                tensors_spliced,
                bytes_written,
                output_path.display()
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
        });
        index.register("source.gguf", GgufMeta {
            path: file.path().to_path_buf(),
            tensors,
        });
        index
    }

    #[tokio::test]
    async fn test_crystallize_empty_recipe_errors() {
        let mut forge = Forge::new();
        let recipe = ForgeRecipe {
            recipe_id: "empty".to_string(),
            segments: vec![],
        };
        let index = GgufIndex::new();
        let result = forge.crystallize(&recipe, &index, "/tmp/nope.gguf").await;
        assert!(matches!(result, Err(ForgeError::EmptyRecipe)));
    }

    #[tokio::test]
    async fn test_crystallize_missing_gguf_errors() {
        let mut forge = Forge::new();
        let recipe = ForgeRecipe {
            recipe_id: "test".to_string(),
            segments: vec![SplicingSegment {
                source_gguf: "nonexistent.gguf".to_string(),
                tensor_name: "some.weight".to_string(),
            }],
        };
        let index = GgufIndex::new(); // empty
        let result = forge.crystallize(&recipe, &index, "/tmp/out.gguf").await;
        assert!(matches!(result, Err(ForgeError::GgufNotFound { .. })));
    }

    #[tokio::test]
    async fn test_crystallize_missing_tensor_errors() {
        let source = make_test_gguf(b"GGUF fake tensor data here");
        let mut forge = Forge::new();

        let recipe = ForgeRecipe {
            recipe_id: "test".to_string(),
            segments: vec![SplicingSegment {
                source_gguf: "source.gguf".to_string(),
                tensor_name: "nonexistent.weight".to_string(), // not in index
            }],
        };

        let mut index = GgufIndex::new();
        index.register("source.gguf", GgufMeta {
            path: source.path().to_path_buf(),
            tensors: HashMap::new(), // no tensors registered
        });

        let result = forge.crystallize(&recipe, &index, "/tmp/out.gguf").await;
        assert!(matches!(result, Err(ForgeError::TensorNotFound { .. })));
    }

    #[tokio::test]
    async fn test_crystallize_produces_output_with_gguf_header() {
        // Create a fake source "GGUF" file with known content
        let fake_model_data = b"GGUF_HEADER_HERE_tensor_data_payload_abc123";
        let source = make_test_gguf(fake_model_data);

        // "GGUF_HEADER_HERE" is 16 bytes, then "_tensor_data_payload_abc123"
        // offset 17 = "tensor_data_payload" (17 chars)
        let tensor_offset = 17u64;
        let tensor_size = 17u64;
        assert_eq!(&fake_model_data[tensor_offset as usize..(tensor_offset + tensor_size) as usize],
                   b"tensor_data_paylo");

        let index = make_index_with_file(&source, tensor_offset, tensor_size);

        let recipe = ForgeRecipe {
            recipe_id: "test-crystallize".to_string(),
            segments: vec![SplicingSegment {
                source_gguf: "source.gguf".to_string(),
                tensor_name: "test.weight".to_string(),
            }],
        };

        let output = NamedTempFile::new().unwrap();
        let output_path = output.path().to_path_buf();

        let mut forge = Forge::new();
        let result = forge.crystallize(&recipe, &index, &output_path).await.unwrap();

        // Verify output
        let written = std::fs::read(&output_path).unwrap();
        assert!(written.starts_with(b"GGUF"), "output should start with GGUF magic");
        assert_eq!(result.tensors_spliced, 1);
        assert_eq!(result.bytes_written, 4 + tensor_size); // magic + tensor data
        assert_eq!(result.spliced_tensors[0].kind, Some("attention".to_string()));

        // Verify stats
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
            offset: 8, size: 7, kind: Some("attention".to_string()),
        });
        let mut tensors_b = HashMap::new();
        tensors_b.insert("mlp.weight".to_string(), TensorMeta {
            offset: 8, size: 7, kind: Some("mlp".to_string()),
        });
        index.register("model_a.gguf", GgufMeta { path: src_a.path().to_path_buf(), tensors: tensors_a });
        index.register("model_b.gguf", GgufMeta { path: src_b.path().to_path_buf(), tensors: tensors_b });

        let recipe = ForgeRecipe {
            recipe_id: "hybrid-v1".to_string(),
            segments: vec![
                SplicingSegment { source_gguf: "model_a.gguf".to_string(), tensor_name: "attn.weight".to_string() },
                SplicingSegment { source_gguf: "model_b.gguf".to_string(), tensor_name: "mlp.weight".to_string() },
            ],
        };

        let output = NamedTempFile::new().unwrap();
        let mut forge = Forge::new();
        let result = forge.crystallize(&recipe, &index, output.path()).await.unwrap();

        assert_eq!(result.tensors_spliced, 2);
        assert_eq!(result.spliced_tensors[0].kind, Some("attention".to_string()));
        assert_eq!(result.spliced_tensors[1].kind, Some("mlp".to_string()));
        assert_eq!(forge.stats.tensors_spliced, 2);
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
}
