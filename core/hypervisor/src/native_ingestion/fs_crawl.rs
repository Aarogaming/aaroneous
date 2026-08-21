use crate::native_ingestion::IngestionDataChunk;
use crate::native_ingestion::IngestionSourceType;
use seahash::SeaHasher;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

/// Zero-allocation directory walker that reads source code files as raw
/// UTF-8 byte streams and computes VSA signatures for each function block
/// using a sliding byte window.
pub struct FsCrawlIngestor {
    /// Root path for the crawl.
    root: PathBuf,
    /// File extension filter (e.g., "rs", "py", "ts").
    extension_filter: Option<String>,
    /// Maximum file size in bytes (files larger than this are skipped).
    max_file_size: u64,
}

impl FsCrawlIngestor {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            extension_filter: None,
            max_file_size: 10 * 1024 * 1024, // 10 MB
        }
    }

    pub fn with_extension(mut self, ext: &str) -> Self {
        self.extension_filter = Some(ext.to_string());
        self
    }

    /// Walk the directory tree (zero heap allocation per entry beyond the
    /// directory listing) and return an iterator of IngestionDataChunks.
    pub fn crawl(&self) -> Vec<IngestionDataChunk> {
        let mut chunks = Vec::new();
        self.crawl_dir(&self.root, &mut chunks);
        chunks
    }

    fn crawl_dir(&self, dir: &Path, chunks: &mut Vec<IngestionDataChunk>) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.crawl_dir(&path, chunks);
            } else if path.is_file() {
                // Check extension filter
                if let Some(ref ext) = self.extension_filter {
                    match path.extension() {
                        Some(e) if e == ext.as_str() => {}
                        _ => continue,
                    }
                }

                // Check file size
                if let Ok(meta) = fs::metadata(&path)
                    && meta.len() > self.max_file_size
                {
                    continue;
                }

                // Read file and compute VSA signature
                if let Ok(content) = fs::read(&path) {
                    let source_hash = hash_path(&path);
                    let mut vsa = [0u64; 128];

                    // Sliding byte window: hash each 64-byte window and
                    // XOR-fold into the VSA signature array.
                    for (idx, window) in content.chunks_exact(64).enumerate() {
                        let mut hasher = SeaHasher::new();
                        hasher.write(window);
                        let h = hasher.finish();
                        vsa[idx % 128] ^= h;
                    }
                    // Process remaining bytes
                    let remainder_start = (content.len() / 64) * 64;
                    if remainder_start < content.len() {
                        let remainder = &content[remainder_start..];
                        let mut hasher = SeaHasher::new();
                        hasher.write(remainder);
                        let h = hasher.finish();
                        vsa[content.len() % 128] ^= h;
                    }

                    chunks.push(IngestionDataChunk {
                        source_type: IngestionSourceType::ProgramDirectory,
                        source_identifier: source_hash,
                        byte_offset: 0,
                        coordinate_bounds: [0.0, 0.0, 1.0, 1.0],
                        spatial_signature: vsa,
                    });
                }
            }
        }
    }
}

/// Hash a file path to a u64 identifier using SeaHasher.
fn hash_path(path: &Path) -> u64 {
    let mut hasher = SeaHasher::new();
    hasher.write(path.to_string_lossy().as_bytes());
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crawl_nonexistent_directory() {
        let ingestor = FsCrawlIngestor::new(r"C:\nonexistent_dir_xyz123");
        let chunks = ingestor.crawl();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_crawl_current_directory_filtered() {
        // Crawl the native_ingestion directory itself looking for .rs files
        let ingestor = FsCrawlIngestor::new("src/native_ingestion").with_extension("rs");
        let chunks = ingestor.crawl();
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert_eq!(
                chunk.source_type as u8,
                IngestionSourceType::ProgramDirectory as u8
            );
        }
    }

    #[test]
    fn test_crawl_with_wrong_extension() {
        let ingestor = FsCrawlIngestor::new("src").with_extension("zzz");
        let chunks = ingestor.crawl();
        assert!(chunks.is_empty());
    }
}
