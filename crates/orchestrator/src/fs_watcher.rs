use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use tracing::{error, info};

use crate::memory_pipeline::EpisodicInsertionPipeline;

/// SEMANTIC-07: Background File System Watcher
/// Silently watches directories and automatically ingests modified files into the Vector DB.
pub struct DirectoryWatcher {
    pipeline: Arc<EpisodicInsertionPipeline>,
}

/// Alias for `DirectoryWatcher`
pub type FsWatcher = DirectoryWatcher;

impl DirectoryWatcher {
    pub fn new(pipeline: Arc<EpisodicInsertionPipeline>) -> Self {
        Self { pipeline }
    }

    /// Spawns a background thread that listens for OS file system events
    pub fn watch_directory(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        let pipeline = self.pipeline.clone();

        thread::spawn(move || {
            let (tx, rx) = channel();
            let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed to initialize file watcher: {}", e);
                    return;
                }
            };

            if let Err(e) = watcher.watch(&path, RecursiveMode::Recursive) {
                error!("Failed to watch directory {:?}: {}", path, e);
                return;
            }

            info!("Silently watching {:?} for RAG indexing...", path);

            for res in rx {
                match res {
                    Ok(event) => {
                        // We only care about file modifications or creations
                        if event.kind.is_modify() || event.kind.is_create() {
                            for file_path in event.paths {
                                // Skip massive files or binaries here in production
                                if let Ok(content) = std::fs::read_to_string(&file_path) {
                                    info!(
                                        "File changed: {:?}. Embedding into Episodic Memory...",
                                        file_path
                                    );
                                    let _ = pipeline
                                        .embed_and_insert(&content, "#file_system_event");
                                }
                            }
                        }
                    }
                    Err(e) => error!("File watch error: {}", e),
                }
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compute::episodic_memory::EpisodicMemoryFabric;

    #[test]
    fn test_fs_watcher_creation() {
        let fabric = Arc::new(EpisodicMemoryFabric::default());
        let pipeline = Arc::new(EpisodicInsertionPipeline::new(fabric));
        let watcher = FsWatcher::new(pipeline);
        let temp = tempfile::tempdir().unwrap();
        assert!(watcher.watch_directory(temp.path()).is_ok());
    }
}