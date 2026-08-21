// Metadata Ingestor
// Watches files, git, system metrics, and feeds them to the compute engine

use crate::workspace::WorkspacePaths;
use compute::ComputeEngine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Types of metadata sources
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MetadataSource {
    FileSystem(PathBuf),
    GitRepository(PathBuf),
    SystemMetrics,
    SabStore,
    Constellation,
}

/// A single metadata event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataEvent {
    pub source: String,
    pub event_type: String,
    pub timestamp: f64,
    pub data: serde_json::Value,
    pub raw_bytes: Option<Vec<u8>>,
}

/// Configuration for the metadata ingestor
#[derive(Debug, Clone)]
pub struct MetadataIngestorConfig {
    pub watch_paths: Vec<PathBuf>,
    pub poll_interval: Duration,
    pub max_event_queue: usize,
    pub compute_entropy: bool,
    pub compute_complexity: bool,
}

impl Default for MetadataIngestorConfig {
    fn default() -> Self {
        Self {
            watch_paths: WorkspacePaths::discover(),
            poll_interval: Duration::from_secs(5),
            max_event_queue: 1000,
            compute_entropy: true,
            compute_complexity: true,
        }
    }
}

use notify::{Event as NotifyEvent, RecursiveMode, Watcher};
use tokio::sync::mpsc;

/// Metadata Ingestor - collects and analyzes metadata from various sources
pub struct MetadataIngestor {
    pub config: MetadataIngestorConfig,
    pub compute: ComputeEngine,
    pub event_queue: Vec<MetadataEvent>,
    pub file_hashes: std::collections::HashMap<PathBuf, String>,
    pub last_system_metrics: Option<SystemMetrics>,
    // Event channel
    event_rx: mpsc::Receiver<MetadataEvent>,
    _watcher: notify::RecommendedWatcher,
}

/// System metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub timestamp: f64,
}

impl MetadataIngestor {
    pub fn new(config: MetadataIngestorConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.max_event_queue);

        // Define watcher
        let event_handler = move |res: notify::Result<NotifyEvent>| {
            if let Ok(event) = res {
                // Simplified: just wrap as MetadataEvent
                let _ = tx.blocking_send(MetadataEvent {
                    source: "FileSystem".to_string(),
                    event_type: format!("{:?}", event.kind),
                    timestamp: chrono::Utc::now().timestamp() as f64,
                    data: serde_json::json!({"paths": event.paths}),
                    raw_bytes: None,
                });
            }
        };

        let mut watcher =
            notify::RecommendedWatcher::new(event_handler, notify::Config::default()).unwrap();

        for path in &config.watch_paths {
            if path.exists() {
                let _ = watcher.watch(path, RecursiveMode::Recursive);
            }
        }

        Self {
            config,
            compute: ComputeEngine::new(),
            event_queue: Vec::new(),
            file_hashes: std::collections::HashMap::new(),
            last_system_metrics: None,
            event_rx: rx,
            _watcher: watcher,
        }
    }
    /// Scan watched paths for filesystem changes.
    fn scan_filesystem(&mut self) -> Vec<MetadataEvent> {
        let mut events = Vec::new();
        let watch_paths = self.config.watch_paths.clone();

        for path in &watch_paths {
            self.scan_path_recursive(path, &mut events);
        }

        events
    }

    fn scan_path_recursive(&mut self, path: &Path, events: &mut Vec<MetadataEvent>) {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => return,
        };

        if metadata.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let entry_path = entry.path();

                    // Skip hidden files/directories and target trees.
                    if let Some(name) = entry_path.file_name().and_then(|n| n.to_str())
                        && (name.starts_with('.') || name == "target")
                    {
                        continue;
                    }

                    self.scan_path_recursive(&entry_path, events);
                }
            }
            return;
        }

        if !metadata.is_file() {
            return;
        }

        let file_hash = self.compute_file_hash(path);
        let is_new = !self.file_hashes.contains_key(path);
        let is_changed = self.file_hashes.get(path) != Some(&file_hash);

        if is_new || is_changed {
            let event = MetadataEvent {
                source: format!("fs:{}", path.display()),
                event_type: if is_new {
                    "file_created".to_string()
                } else {
                    "file_modified".to_string()
                },
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
                data: serde_json::json!({
                    "path": path.to_string_lossy(),
                    "size": metadata.len(),
                    "is_file": metadata.is_file(),
                    "modified": metadata.modified().ok().map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()
                    }),
                }),
                raw_bytes: if metadata.len() < 1_000_000 {
                    fs::read(path).ok()
                } else {
                    None
                },
            };

            events.push(event);
            self.file_hashes.insert(path.to_path_buf(), file_hash);
        }
    }

    /// Collect system metrics
    pub fn collect_system_metrics(&mut self) -> MetadataEvent {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();

        let cpu_usage =
            sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;
        let memory_usage = sys.used_memory() as f32 / sys.total_memory() as f32 * 100.0;
        let mut disk_usage = 0.0;
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let mut total_space = 0;
        let mut total_used = 0;
        for disk in disks.list() {
            total_space += disk.total_space();
            total_used += disk.total_space() - disk.available_space();
        }
        if total_space > 0 {
            disk_usage = (total_used as f64 / total_space as f64 * 100.0) as f32;
        }

        let metrics = SystemMetrics {
            cpu_usage,
            memory_usage,
            disk_usage,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        };

        self.last_system_metrics = Some(metrics.clone());

        MetadataEvent {
            source: "system:metrics".to_string(),
            event_type: "metrics_update".to_string(),
            timestamp: metrics.timestamp,
            data: serde_json::json!({
                "cpu_usage": cpu_usage,
                "memory_usage": memory_usage,
                "disk_usage": disk_usage,
            }),
            raw_bytes: None,
        }
    }

    /// Analyze a metadata event using compute engine
    pub fn analyze_event(&mut self, event: &MetadataEvent) -> MetadataAnalysis {
        let mut analysis = MetadataAnalysis::default();

        // Compute entropy if raw bytes available
        if self.config.compute_entropy
            && let Some(ref bytes) = event.raw_bytes
        {
            let byte_values: Vec<f64> = bytes.iter().map(|&b| b as f64 / 255.0).collect();
            if let Ok(result) = self.compute.execute("entropy", &byte_values) {
                analysis.entropy = result.first().copied().unwrap_or(0.0);
            }
        }

        // Compute complexity based on event data
        if self.config.compute_complexity {
            let complexity_input = match event.event_type.as_str() {
                "file_modified" => vec![0.5, 0.3],
                "file_created" => vec![0.7, 0.2],
                "metrics_update" => vec![0.3, 0.8],
                _ => vec![0.5, 0.5],
            };

            if let Ok(result) = self.compute.execute("monte_carlo", &complexity_input) {
                analysis.predicted_complexity = result.first().copied().unwrap_or(0.5);
            }
        }

        analysis
    }

    /// Process all pending events and return analyses
    pub fn process_pending_events(&mut self) -> Vec<(MetadataEvent, MetadataAnalysis)> {
        let mut results = Vec::new();

        while let Ok(event) = self.event_rx.try_recv() {
            if self.event_queue.len() < self.config.max_event_queue {
                self.event_queue.push(event);
            }
        }

        // Scan for new events
        let fs_events = self.scan_filesystem();
        let metrics_event = self.collect_system_metrics();

        // Add to queue
        for event in fs_events {
            if self.event_queue.len() < self.config.max_event_queue {
                self.event_queue.push(event);
            }
        }
        if self.event_queue.len() < self.config.max_event_queue {
            self.event_queue.push(metrics_event);
        }

        // Analyze all queued events
        let events: Vec<MetadataEvent> = self.event_queue.drain(..).collect();
        for event in events {
            let analysis = self.analyze_event(&event);
            results.push((event, analysis));
        }

        results
    }

    /// Compute a simple hash for a file (for change detection)
    fn compute_file_hash(&self, path: &Path) -> String {
        if let Ok(metadata) = fs::metadata(path)
            && let Ok(modified) = metadata.modified()
            && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            return format!("{}:{}", metadata.len(), duration.as_secs());
        }
        "unknown".to_string()
    }
}

/// Analysis result for a metadata event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataAnalysis {
    pub entropy: f64,
    pub predicted_complexity: f64,
    pub risk_score: f64,
    pub recommended_action: String,
}

impl Default for MetadataAnalysis {
    fn default() -> Self {
        Self {
            entropy: 0.0,
            predicted_complexity: 0.5,
            risk_score: 0.0,
            recommended_action: "monitor".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_ingestor_creation() {
        let config = MetadataIngestorConfig::default();
        let ingestor = MetadataIngestor::new(config);
        assert!(ingestor.file_hashes.is_empty());
        assert!(ingestor.event_queue.is_empty());
    }

    #[test]
    fn test_system_metrics_collection() {
        let config = MetadataIngestorConfig::default();
        let mut ingestor = MetadataIngestor::new(config);
        let event = ingestor.collect_system_metrics();

        assert_eq!(event.source, "system:metrics");
        assert_eq!(event.event_type, "metrics_update");
        assert!(ingestor.last_system_metrics.is_some());
    }

    #[test]
    fn test_event_analysis() {
        let config = MetadataIngestorConfig::default();
        let mut ingestor = MetadataIngestor::new(config);

        let event = MetadataEvent {
            source: "test".to_string(),
            event_type: "file_modified".to_string(),
            timestamp: 0.0,
            data: serde_json::json!({"path": "test.rs"}),
            raw_bytes: Some(b"fn main() {}".to_vec()),
        };

        let analysis = ingestor.analyze_event(&event);
        assert!(analysis.entropy >= 0.0);
        assert!(analysis.predicted_complexity >= 0.0 && analysis.predicted_complexity <= 1.0);
    }

    #[test]
    fn test_recursive_filesystem_scan() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested").join("deeper");
        fs::create_dir_all(&nested).unwrap();
        let file_path = nested.join("sample.rs");
        fs::write(&file_path, b"fn main() {}\n").unwrap();

        let config = MetadataIngestorConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            poll_interval: Duration::from_secs(1),
            max_event_queue: 16,
            compute_entropy: false,
            compute_complexity: false,
        };
        let mut ingestor = MetadataIngestor::new(config);
        let events = ingestor.scan_filesystem();
        let expected_path = file_path.to_string_lossy().to_string();

        assert!(
            events
                .iter()
                .any(|event| event.data.get("path").and_then(|p| p.as_str())
                    == Some(expected_path.as_str()))
        );
    }
}
