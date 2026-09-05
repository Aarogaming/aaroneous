use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::thread;
use anyhow::Result;
use tracing::{info, error};
use crate::memory_pipeline::EpisodicInsertionPipeline;
use std::sync::Arc;

/// SEMANTIC-07: Background File System Watcher
/// Silently watches directories and automatically ingests modified files into the Vector DB.
pub struct DirectoryWatcher {
    pipeline: Arc<EpisodicInsertionPipeline>,
}

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
                                    info!("File changed: {:?}. Embedding into Episodic Memory...", file_path);
                                    let _ = pipeline.embed_and_insert(&content, "#file_system_event");
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