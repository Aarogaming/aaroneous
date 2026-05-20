use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::workspace::WorkspacePaths;

/// Represents a data source entry point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    /// File uploaded to inbox folder
    InboxFile { path: PathBuf, timestamp: DateTime<Utc> },
    /// Direct API call with payload
    DirectPayload { data: String, media_type: String },
    /// Database query result
    DatabaseQuery { query: String, source: String },
    /// Stream endpoint
    StreamEndpoint { url: String, protocol: String },
}

/// Supported file formats for ingestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileFormat {
    // Text formats
    Json,
    Jsonl,
    Csv,
    Tsv,
    Txt,
    Markdown,
    Xml,
    Yaml,
    Log,
    // Binary formats
    Gguf,
    Parquet,
    Sqlite,
    // Archives
    Zip,
    TarGz,
}

impl FileFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "json" => Some(FileFormat::Json),
            "jsonl" | "ndjson" => Some(FileFormat::Jsonl),
            "csv" => Some(FileFormat::Csv),
            "tsv" => Some(FileFormat::Tsv),
            "txt" => Some(FileFormat::Txt),
            "md" | "markdown" => Some(FileFormat::Markdown),
            "xml" => Some(FileFormat::Xml),
            "yaml" | "yml" => Some(FileFormat::Yaml),
            "log" => Some(FileFormat::Log),
            "gguf" => Some(FileFormat::Gguf),
            "parquet" => Some(FileFormat::Parquet),
            "sqlite" | "db" => Some(FileFormat::Sqlite),
            "zip" => Some(FileFormat::Zip),
            "tar.gz" | "tgz" => Some(FileFormat::TarGz),
            _ => None,
        }
    }

    pub fn is_binary(&self) -> bool {
        matches!(self, FileFormat::Gguf | FileFormat::Parquet | FileFormat::Sqlite | FileFormat::Zip | FileFormat::TarGz)
    }

    pub fn is_text(&self) -> bool {
        !self.is_binary()
    }
}

/// Represents ingested data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestibleData {
    /// Unique identifier for this ingestion
    pub id: String,
    /// Source of the data
    pub source: DataSource,
    /// Detected file format
    pub format: Option<FileFormat>,
    /// Raw content (for text formats)
    pub content: Option<String>,
    /// File path (for binary formats)
    pub file_path: Option<PathBuf>,
    /// Metadata extracted from content
    pub metadata: DataMetadata,
    /// Ingestion timestamp
    pub ingested_at: DateTime<Utc>,
    /// Processing status
    pub status: IngestionStatus,
}

/// Status of ingestion process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IngestionStatus {
    Received,
    Copied,
    Classified,
    Distilled,
    EventGenerated,
    Completed,
    Failed,
}

/// Metadata extracted during ingestion
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataMetadata {
    /// File name (if from file)
    pub filename: Option<String>,
    /// File size in bytes
    pub size_bytes: Option<u64>,
    /// Detected MIME type
    pub mime_type: Option<String>,
    /// Number of records/lines
    pub record_count: Option<usize>,
    /// Key-value pairs extracted from content
    pub extracted_fields: HashMap<String, String>,
    /// Detected domains/topics
    pub detected_domains: Vec<String>,
    /// Confidence scores for domain detection
    pub domain_confidence: HashMap<String, f32>,
    /// Raw sample of content (first N bytes)
    pub content_sample: Option<String>,
    /// Checksum for integrity verification
    pub checksum: Option<String>,
}

impl IngestibleData {
    /// Create a new ingestible data from a file
    pub fn from_file(path: PathBuf) -> Self {
        let id = Uuid::new_v4().to_string();
        let filename = path.file_name().map(|n| n.to_string_lossy().to_string());
        let format = path
            .extension()
            .and_then(|ext| FileFormat::from_extension(&ext.to_string_lossy()));

        Self {
            id,
            source: DataSource::InboxFile {
                path: path.clone(),
                timestamp: Utc::now(),
            },
            format,
            content: None,
            file_path: Some(path),
            metadata: DataMetadata {
                filename,
                ..Default::default()
            },
            ingested_at: Utc::now(),
            status: IngestionStatus::Received,
        }
    }

    /// Create a new ingestible data from direct payload
    pub fn from_payload(data: String, media_type: String) -> Self {
        let id = Uuid::new_v4().to_string();

        // Try to detect format from media type
        let format = match media_type.as_str() {
            "application/json" => Some(FileFormat::Json),
            "text/csv" => Some(FileFormat::Csv),
            "text/plain" => Some(FileFormat::Txt),
            "text/markdown" => Some(FileFormat::Markdown),
            _ => None,
        };

        Self {
            id,
            source: DataSource::DirectPayload { data: data.clone(), media_type: media_type.clone() },
            format,
            content: Some(data),
            file_path: None,
            metadata: DataMetadata {
                mime_type: Some(media_type),
                ..Default::default()
            },
            ingested_at: Utc::now(),
            status: IngestionStatus::Received,
        }
    }

    /// Calculate checksum for integrity verification
    pub fn calculate_checksum(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        if let Some(content) = &self.content {
            content.hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }

    /// Update ingestion status
    pub fn set_status(&mut self, status: IngestionStatus) {
        self.status = status;
    }

    /// Add detected domain
    pub fn add_domain(&mut self, domain: String, confidence: f32) {
        self.metadata.detected_domains.push(domain.clone());
        self.metadata.domain_confidence.insert(domain, confidence);
    }

    /// Get top-N detected domains sorted by confidence
    pub fn top_domains(&self, n: usize) -> Vec<(String, f32)> {
        let mut domains: Vec<_> = self
            .metadata
            .domain_confidence
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        domains.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        domains.into_iter().take(n).collect()
    }
}

/// Configuration for data ingestion system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    pub inbox_path: PathBuf,
    pub processing_path: PathBuf,
    pub processed_path: PathBuf,
    pub failed_path: PathBuf,
    pub analytics_path: PathBuf,
    pub max_file_size_mb: u64,
    pub max_concurrent_ingestions: usize,
    pub content_sample_size_bytes: usize,
    pub file_watcher_enabled: bool,
    pub scan_interval_ms: u64,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        let paths = WorkspacePaths::discover();
        let data = paths.data();
        Self {
            inbox_path: paths.inbox(),
            processing_path: data.join("processing"),
            processed_path: data.join("processed"),
            failed_path: data.join("failed"),
            analytics_path: data.join("analytics"),
            max_file_size_mb: 512,
            max_concurrent_ingestions: 4,
            content_sample_size_bytes: 51200,
            file_watcher_enabled: true,
            scan_interval_ms: 2000,
        }
    }
}

impl IngestionConfig {
    /// Initialize all required directories
    pub async fn init_directories(&self) -> Result<(), std::io::Error> {
        async_fs::create_dir_all(&self.inbox_path).await?;
        async_fs::create_dir_all(&self.processing_path).await?;
        async_fs::create_dir_all(&self.processed_path).await?;
        async_fs::create_dir_all(&self.failed_path).await?;
        async_fs::create_dir_all(&self.analytics_path).await?;
        Ok(())
    }
}

/// Result of non-destructive file copy
#[derive(Debug, Clone)]
pub struct CopyResult {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub bytes_copied: u64,
    pub timestamp: DateTime<Utc>,
}

/// Non-destructive copy operation from inbox to processing
pub async fn copy_file_non_destructive(
    source: &Path,
    dest_dir: &Path,
) -> Result<CopyResult, std::io::Error> {
    // Ensure destination directory exists
    async_fs::create_dir_all(dest_dir).await?;

    // Create subdirectory with timestamp for organization
    let now = Utc::now();
    let timestamp_dir = dest_dir.join(now.format("%Y%m%d_%H%M%S").to_string());
    async_fs::create_dir_all(&timestamp_dir).await?;

    // Copy file to timestamped location
    let dest = timestamp_dir.join(
        source
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "No filename"))?,
    );

    let bytes_copied = async_fs::copy(source, &dest).await?;

    Ok(CopyResult {
        source: source.to_path_buf(),
        destination: dest,
        bytes_copied,
        timestamp: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_format_detection() {
        assert_eq!(FileFormat::from_extension("json"), Some(FileFormat::Json));
        assert_eq!(FileFormat::from_extension("csv"), Some(FileFormat::Csv));
        assert_eq!(FileFormat::from_extension("gguf"), Some(FileFormat::Gguf));
        assert_eq!(FileFormat::from_extension("unknown"), None);
    }

    #[test]
    fn test_file_format_binary_text() {
        assert!(!FileFormat::Json.is_binary());
        assert!(FileFormat::Gguf.is_binary());
        assert!(FileFormat::Json.is_text());
        assert!(!FileFormat::Gguf.is_text());
    }

    #[test]
    fn test_ingestible_data_creation() {
        let path = PathBuf::from("test_data.json");
        let data = IngestibleData::from_file(path);

        assert_eq!(data.format, Some(FileFormat::Json));
        assert_eq!(data.status, IngestionStatus::Received);
        assert!(data.metadata.filename.is_some());
    }

    #[test]
    fn test_ingestible_data_from_payload() {
        let payload = r#"{"test": "data"}"#.to_string();
        let data = IngestibleData::from_payload(payload, "application/json".to_string());

        assert_eq!(data.format, Some(FileFormat::Json));
        assert!(data.content.is_some());
    }

    #[test]
    fn test_domain_tracking() {
        let mut data = IngestibleData::from_payload("test".to_string(), "text/plain".to_string());

        data.add_domain("database".to_string(), 0.95);
        data.add_domain("networking".to_string(), 0.75);
        data.add_domain("security".to_string(), 0.85);

        let top = data.top_domains(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "database"); // Highest confidence first
    }

    #[test]
    fn test_checksum_calculation() {
        let data1 = IngestibleData::from_payload("test data".to_string(), "text/plain".to_string());
        let data2 = IngestibleData::from_payload("test data".to_string(), "text/plain".to_string());
        let data3 = IngestibleData::from_payload("different data".to_string(), "text/plain".to_string());

        let cs1 = data1.calculate_checksum();
        let cs2 = data2.calculate_checksum();
        let cs3 = data3.calculate_checksum();

        assert_eq!(cs1, cs2);
        assert_ne!(cs1, cs3);
    }
}
