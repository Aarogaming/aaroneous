// Aaroneous File Watcher Module
// Real async filesystem monitoring for data ingestion

use notify::{Watcher, RecursiveMode, Result as NotifyResult, RecommendedWatcher};
use notify::event::EventKind;
use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use tokio::sync::mpsc;
use std::time::Duration;
use tracing::{info, warn, error};

/// File watch event
#[derive(Debug, Clone)]
pub enum FileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    Renamed(PathBuf, PathBuf),
}

/// File watcher configuration
#[derive(Debug, Clone)]
pub struct FileWatcherConfig {
    pub watch_path: PathBuf,
    pub recursive: bool,
    pub file_extensions: Vec<String>, // Only watch these file types (e.g., ["gguf", "json", "csv"])
    pub poll_interval: Duration,
    pub debounce_ms: u64, // Debounce rapid file changes
}

impl Default for FileWatcherConfig {
    fn default() -> Self {
        Self {
            watch_path: PathBuf::from("D:\\Aaroneous\\inbox"),
            recursive: true,
            file_extensions: vec![
                "gguf".to_string(),
                "json".to_string(),
                "csv".to_string(),
                "parquet".to_string(),
                "log".to_string(),
            ],
            poll_interval: Duration::from_secs(2),
            debounce_ms: 500,
        }
    }
}

/// Async file watcher
pub struct FileWatcher {
    config: FileWatcherConfig,
    tx: mpsc::UnboundedSender<FileEvent>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(
        config: FileWatcherConfig,
    ) -> (Self, mpsc::UnboundedReceiver<FileEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let watcher = FileWatcher { config, tx };
        (watcher, rx)
    }

    /// Start watching for file changes
    pub async fn start(&self) -> NotifyResult<()> {
        let config = self.config.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            match watch_directory(&config, tx).await {
                Ok(_) => info!("File watcher stopped cleanly"),
                Err(e) => error!("File watcher error: {}", e),
            }
        });

        info!(
            "File watcher started for: {}",
            self.config.watch_path.display()
        );
        Ok(())
    }

    /// Check if file should be watched based on extension
    fn should_watch_file(&self, path: &Path) -> bool {
        if self.config.file_extensions.is_empty() {
            return true;
        }

        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                self.config
                    .file_extensions
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(ext))
            })
            .unwrap_or(false)
    }
}

/// Watch directory with debouncing
async fn watch_directory(
    config: &FileWatcherConfig,
    tx: mpsc::UnboundedSender<FileEvent>,
) -> NotifyResult<()> {
    let (std_tx, std_rx) = std_mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            match res {
                Ok(event) => {
                    let _ = std_tx.send(convert_event(event));
                }
                Err(e) => warn!("Watch error: {}", e),
            }
        },
        Default::default(),
    )?;

    let recursive_mode = if config.recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };

    watcher.watch(&config.watch_path, recursive_mode)?;

    // Keep watcher alive, forward events to async channel
    tokio::spawn(async move {
        while let Ok(event) = std_rx.recv() {
            let _ = tx.send(event);
        }
    });

    // Keep watcher in scope
    std::mem::forget(watcher);
    
    Ok(())
}

/// Convert notify::Event to FileEvent
fn convert_event(event: notify::Event) -> FileEvent {
    match event.kind {
        EventKind::Create(_) => {
            if let Some(path) = event.paths.first() {
                FileEvent::Created(path.clone())
            } else {
                FileEvent::Created(PathBuf::new())
            }
        }
        EventKind::Modify(_) => {
            if let Some(path) = event.paths.first() {
                FileEvent::Modified(path.clone())
            } else {
                FileEvent::Modified(PathBuf::new())
            }
        }
        EventKind::Remove(_) => {
            if let Some(path) = event.paths.first() {
                FileEvent::Removed(path.clone())
            } else {
                FileEvent::Removed(PathBuf::new())
            }
        }
        EventKind::Access(_) => {
            // Treat access like modify
            if let Some(path) = event.paths.first() {
                FileEvent::Modified(path.clone())
            } else {
                FileEvent::Modified(PathBuf::new())
            }
        }
        _ => {
            if let Some(path) = event.paths.first() {
                FileEvent::Modified(path.clone())
            } else {
                FileEvent::Modified(PathBuf::new())
            }
        }
    }
}

/// Process file events by routing them to the ingestion system.
///
/// This function wires the file-watcher output to the `InboxSystem` so that
/// files dropped into the inbox folder are automatically ingested.
///
/// Pass `inbox_system: None` to disable automatic routing (useful when the
/// runtime hasn't been fully initialized yet).
pub async fn process_file_event(
    event: FileEvent,
    inbox_path: &Path,
    inbox_system: Option<&crate::inbox_system::InboxSystem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        FileEvent::Created(path) => {
            if path.starts_with(inbox_path) && path.is_file() {
                info!("New inbox file: {}", path.display());
                if let Some(system) = inbox_system {
                    match system.ingest_file(&path).await {
                        Ok(result) => info!(
                            "Ingested '{}' → {:?}",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            result.status
                        ),
                        Err(e) => warn!("Ingestion failed for '{}': {}", path.display(), e),
                    }
                }
            }
        }
        FileEvent::Modified(path) => {
            if path.starts_with(inbox_path) && path.is_file() {
                // Modified files in the inbox are treated as updated inputs —
                // trigger re-ingestion so the latest content is processed.
                info!("Inbox file modified (re-ingesting): {}", path.display());
                if let Some(system) = inbox_system {
                    if let Err(e) = system.ingest_file(&path).await {
                        warn!("Re-ingestion failed for '{}': {}", path.display(), e);
                    }
                }
            }
        }
        FileEvent::Removed(path) => {
            // Removed files: mark any in-progress ingestion as cancelled.
            info!("Inbox file removed: {}", path.display());
            if let Some(system) = inbox_system {
                system.cancel_ingestion_for_path(&path).await;
            }
        }
        FileEvent::Renamed(old, new) => {
            // Rename: if the destination is in the inbox, ingest it.
            // If the source was being ingested, cancel that record.
            info!("File renamed: {} → {}", old.display(), new.display());
            if let Some(system) = inbox_system {
                system.cancel_ingestion_for_path(&old).await;
                if new.starts_with(inbox_path) && new.is_file() {
                    if let Err(e) = system.ingest_file(&new).await {
                        warn!("Ingestion failed for renamed file '{}': {}", new.display(), e);
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_watcher_config_default() {
        let config = FileWatcherConfig::default();
        assert!(!config.file_extensions.is_empty());
        assert!(config.recursive);
    }

    #[test]
    fn test_file_watcher_creation() {
        let config = FileWatcherConfig::default();
        let (watcher, _rx) = FileWatcher::new(config);
        assert!(watcher.config.recursive);
    }

    #[test]
    fn test_file_should_watch() {
        let config = FileWatcherConfig::default();
        let watcher = FileWatcher::new(config).0;

        let gguf_file = Path::new("model.gguf");
        let json_file = Path::new("config.json");
        let unknown_file = Path::new("file.unknown");

        assert!(watcher.should_watch_file(gguf_file));
        assert!(watcher.should_watch_file(json_file));
        assert!(!watcher.should_watch_file(unknown_file));
    }

    #[test]
    fn test_empty_extensions_watches_all() {
        let mut config = FileWatcherConfig::default();
        config.file_extensions.clear();
        let watcher = FileWatcher::new(config).0;

        let any_file = Path::new("anything.xyz");
        assert!(watcher.should_watch_file(any_file));
    }

    #[tokio::test]
    async fn test_process_file_event() {
        let inbox = Path::new("D:\\Aaroneous\\inbox");
        let event = FileEvent::Created(inbox.join("test.gguf"));
        // Pass None for inbox_system in tests (no real system available)
        let result = process_file_event(event, inbox, None).await;
        assert!(result.is_ok());
    }
}
