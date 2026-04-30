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

/// Process file events (route to appropriate specialists)
pub async fn process_file_event(
    event: FileEvent,
    inbox_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        FileEvent::Created(path) => {
            if path.starts_with(inbox_path) && path.is_file() {
                info!("New file detected: {}", path.display());
                // TODO: Route to ingestion system
            }
        }
        FileEvent::Modified(path) => {
            if path.starts_with(inbox_path) && path.is_file() {
                info!("File modified: {}", path.display());
                // TODO: Update ingestion status
            }
        }
        FileEvent::Removed(path) => {
            info!("File removed: {}", path.display());
            // TODO: Clean up ingestion records
        }
        FileEvent::Renamed(old, new) => {
            info!("File renamed: {} -> {}", old.display(), new.display());
            // TODO: Update ingestion tracking
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
        let result = process_file_event(event, inbox).await;
        assert!(result.is_ok());
    }
}
