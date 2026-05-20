/// Background receive tasks for federation specialists.
///
/// These tasks bridge real-world I/O (P2P network messages, BLE notifications)
/// into each specialist's inbox queue. They run for the lifetime of the
/// specialist host — started when the host starts, stopped when it shuts down.
///
/// # Architecture
///
/// ```text
/// P2P recv() ──► OmnipresentRecvTask ──► Omnipresent::sync_inbox
///                                               │
///                                         drain_sync_inbox()  ← called by apply/propose
///
/// BLE notify ──► SymbioticBleTask ──► Symbiotic::bio_inbox
///                                           │
///                                     drain_bio_inbox()  ← called by apply/propose
/// ```
///
/// The inbox-drain pattern keeps the specialist's main methods (`propose`,
/// `execute`) non-blocking: they lock the inbox briefly, drain any queued
/// messages, then continue. The background task fills the inbox asynchronously
/// without contending with the main logic.
///
/// # Lifecycle
///
/// Each task implements `BackgroundTask`:
/// - `spawn(shutdown)` → starts the task on a tokio background task
/// - `join()` → waits for the task to finish (after shutdown is signaled)
///
/// `BackgroundTaskHandle` wraps the `JoinHandle` and the `Notify`, providing
/// a clean lifecycle abstraction that `SpecialistHost` can hold.

pub mod omnipresent_recv;
pub mod symbiotic_ble;
pub mod drain;

#[cfg(test)]
mod tests;

pub use omnipresent_recv::OmnipresentRecvTask;
pub use symbiotic_ble::SymbioticBleTask;
pub use drain::{OmnipresentDrainTask, SymbioticDrainTask};

use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{debug, warn};

/// Handle to a running background task. Dropping does NOT stop the task -
/// call `shutdown()` explicitly.
pub struct BackgroundTaskHandle {
    name: &'static str,
    shutdown: Arc<Notify>,
    handle: tokio::sync::Mutex<Option<JoinHandle<()>>>,
}

impl BackgroundTaskHandle {
    pub(crate) fn new(
        name: &'static str,
        shutdown: Arc<Notify>,
        handle: JoinHandle<()>,
    ) -> Self {
        Self {
            name,
            shutdown,
            handle: tokio::sync::Mutex::new(Some(handle)),
        }
    }

    /// Signal the task to stop and wait up to 5 seconds for it to exit.
    ///
    /// Uses `notify_one()` (buffered) rather than `notify_waiters()` so the
    /// signal is received even if the task hasn't yet awaited the Notify.
    pub async fn shutdown(&self) {
        self.shutdown.notify_one();

        if let Some(handle) = self.handle.lock().await.take() {
            let timeout = std::time::Duration::from_secs(5);
            if tokio::time::timeout(timeout, handle).await.is_err() {
                warn!(
                    "Background task '{}' did not exit within {:?}, abandoning",
                    self.name, timeout
                );
            } else {
                debug!("Background task '{}' exited cleanly", self.name);
            }
        }
    }

    /// Whether the task is still running (handle not yet consumed by shutdown)
    pub async fn is_running(&self) -> bool {
        self.handle.lock().await.is_some()
    }
}
