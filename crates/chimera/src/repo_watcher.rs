//! crates/chimera/src/repo_watcher.rs
//! Live Autonomous Repository Monitor & Background Self-Repair Watcher
//! Monitors workspace filesystem changes and triggers background verification and patch synthesis.

use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// Event produced when a source file is modified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceChangeEvent {
    pub path: PathBuf,
    pub extension: String,
    pub is_rust: bool,
    pub is_python: bool,
}

/// Autonomous Repository Watcher Engine
pub struct RepoWatcher {
    root_dir: PathBuf,
    _watcher: Option<RecommendedWatcher>,
    event_rx: Option<Receiver<notify::Result<Event>>>,
}

impl RepoWatcher {
    /// Creates a new RepoWatcher on a target directory
    pub fn new(root_dir: impl AsRef<Path>) -> Result<Self> {
        let root = root_dir.as_ref().to_path_buf();
        Ok(Self {
            root_dir: root,
            _watcher: None,
            event_rx: None,
        })
    }

    /// Initializes live background filesystem event subscription
    pub fn start_watching(&mut self) -> Result<()> {
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;

        watcher.watch(&self.root_dir, RecursiveMode::Recursive)?;
        self._watcher = Some(watcher);
        self.event_rx = Some(rx);
        Ok(())
    }

    /// Polls for the next source file modification (non-blocking)
    pub fn poll_next_change(&self, timeout: Duration) -> Option<SourceChangeEvent> {
        if let Some(rx) = &self.event_rx {
            if let Ok(Ok(event)) = rx.recv_timeout(timeout) {
                for path in event.paths {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        let is_rust = ext_lower == "rs";
                        let is_python = ext_lower == "py";

                        if is_rust || is_python || ext_lower == "c" || ext_lower == "cpp" || ext_lower == "ts" {
                            return Some(SourceChangeEvent {
                                path,
                                extension: ext_lower,
                                is_rust,
                                is_python,
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_repo_watcher_initialization() {
        let temp = tempdir().unwrap();
        let mut watcher = RepoWatcher::new(temp.path()).unwrap();
        assert!(watcher.start_watching().is_ok());

        // Non-blocking poll returns None when no files modified
        let event = watcher.poll_next_change(Duration::from_millis(50));
        assert!(event.is_none());
    }
}
