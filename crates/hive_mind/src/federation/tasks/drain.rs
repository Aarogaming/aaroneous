/// Periodic drain tasks for Omnipresent and Symbiotic.
///
/// The recv tasks (OmnipresentRecvTask, SymbioticBleTask) fill the inbox
/// queues. These drain tasks periodically flush those queues into the
/// specialist's `drain_state`, keeping the state fresh even when
/// `propose()` or `execute()` aren't being called frequently.
///
/// # Why a separate drain task?
///
/// `drain_sync_inbox()` and `drain_bio_inbox()` require `&mut self` to
/// update the specialist's main state fields. Since the specialist lives
/// behind `Arc<Specialist>`, `&mut` is not directly available.
///
/// The solution used here: drain into `drain_state` (which is
/// `Arc<Mutex<*DrainState>>`) from `&self` via the `_shared` variants.
/// The drain tasks call `drain_sync_inbox_shared()` and
/// `drain_bio_inbox_shared()` which update the interior-mutable drain state.
///
/// Specialists read from `shared_current_state()` / `shared_sync_history_len()`
/// in their propose() path to always see the freshest data.

use super::BackgroundTaskHandle;
use crate::federation::specialists::{Omnipresent, Symbiotic};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tracing::{debug, info};

/// Periodic drain task for Omnipresent's sync inbox.
pub struct OmnipresentDrainTask {
    specialist: Arc<Omnipresent>,
    interval: Duration,
}

impl OmnipresentDrainTask {
    /// Create a drain task with the given drain interval.
    ///
    /// A shorter interval (e.g., 100ms) reduces latency between message
    /// receipt and state update. A longer interval (e.g., 1s) reduces
    /// CPU overhead. 500ms is a sensible default for most deployments.
    pub fn new(specialist: Arc<Omnipresent>, interval: Duration) -> Self {
        Self { specialist, interval }
    }

    /// Spawn the drain loop on a background tokio task.
    pub async fn spawn(&self, shutdown: Arc<Notify>) -> BackgroundTaskHandle {
        let specialist = self.specialist.clone();
        let interval = self.interval;
        let shutdown_for_task = shutdown.clone();
        let name = "OmnipresentDrainTask";

        let handle = tokio::spawn(async move {
            info!("{}: starting (interval: {}ms)", name, interval.as_millis());
            loop {
                tokio::select! {
                    _ = shutdown_for_task.notified() => {
                        info!("{}: shutdown signal received, exiting", name);
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        let n = specialist.drain_sync_inbox_shared();
                        if n > 0 {
                            debug!("{}: drained {} sync message(s)", name, n);
                        }
                    }
                }
            }
            info!("{}: exited", name);
        });

        BackgroundTaskHandle::new(name, shutdown, handle)
    }
}

/// Periodic drain task for Symbiotic's BLE inbox.
pub struct SymbioticDrainTask {
    specialist: Arc<Symbiotic>,
    interval: Duration,
}

impl SymbioticDrainTask {
    pub fn new(specialist: Arc<Symbiotic>, interval: Duration) -> Self {
        Self { specialist, interval }
    }

    pub async fn spawn(&self, shutdown: Arc<Notify>) -> BackgroundTaskHandle {
        let specialist = self.specialist.clone();
        let interval = self.interval;
        let shutdown_for_task = shutdown.clone();
        let name = "SymbioticDrainTask";

        let handle = tokio::spawn(async move {
            info!("{}: starting (interval: {}ms)", name, interval.as_millis());
            loop {
                tokio::select! {
                    _ = shutdown_for_task.notified() => {
                        info!("{}: shutdown signal received, exiting", name);
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        let n = specialist.drain_bio_inbox_shared();
                        if n > 0 {
                            debug!("{}: drained {} BLE sample(s)", name, n);
                        }
                    }
                }
            }
            info!("{}: exited", name);
        });

        BackgroundTaskHandle::new(name, shutdown, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::p2p::{P2pNodeId, SyncMessage};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_omnipresent_drain_task_processes_inbox() {
        let specialist = Arc::new(Omnipresent::new());
        let node_id = P2pNodeId::random();

        // Pre-fill inbox with 5 messages
        for _ in 0..5 {
            let msg = SyncMessage::heartbeat(node_id.clone(), 1);
            specialist.sync_inbox.lock().push_back(msg);
        }
        assert_eq!(specialist.sync_inbox.lock().len(), 5);

        // Spawn drain task with fast interval
        let task = OmnipresentDrainTask::new(specialist.clone(), Duration::from_millis(50));
        let shutdown = Arc::new(Notify::new());
        let handle = task.spawn(shutdown.clone()).await;

        // Wait for drain interval to fire
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Inbox should be empty and drain_state should have history
        assert_eq!(
            specialist.sync_inbox.lock().len(),
            0,
            "inbox should be empty after drain"
        );
        assert_eq!(
            specialist.shared_sync_history_len(),
            5,
            "drain_state.sync_history should have 5 entries"
        );

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_omnipresent_drain_full_state_updates_cached_intent() {
        let specialist = Arc::new(Omnipresent::new());
        let node_id = P2pNodeId::random();
        let msg = SyncMessage::full_state(node_id, 7, b"intent-v7".to_vec());
        specialist.sync_inbox.lock().push_back(msg);

        let task = OmnipresentDrainTask::new(specialist.clone(), Duration::from_millis(50));
        let shutdown = Arc::new(Notify::new());
        let handle = task.spawn(shutdown.clone()).await;

        tokio::time::sleep(Duration::from_millis(200)).await;

        let ci = specialist.cached_intent();
        assert!(ci.is_some(), "drain should update cached_intent");
        assert_eq!(ci.unwrap().content, "intent-v7", "cached intent content should match");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_symbiotic_drain_task_processes_inbox() {
        use crate::federation::biometric::BiometricSample;

        let specialist = Arc::new(Symbiotic::new());

        // Pre-fill inbox with 3 HR samples
        for bpm in [65u16, 70, 75] {
            let sample = BiometricSample::heart_rate("dev".to_string(), bpm);
            specialist.bio_inbox.lock().push_back(sample);
        }
        assert_eq!(specialist.bio_inbox.lock().len(), 3);

        let task = SymbioticDrainTask::new(specialist.clone(), Duration::from_millis(50));
        let shutdown = Arc::new(Notify::new());
        let handle = task.spawn(shutdown.clone()).await;

        tokio::time::sleep(Duration::from_millis(200)).await;

        assert_eq!(
            specialist.bio_inbox.lock().len(),
            0,
            "bio inbox should be empty after drain"
        );

        // drain_state.biometric_history should have 3 readings
        let drain = specialist.drain_state.lock();
        assert_eq!(
            drain.biometric_history.len(),
            3,
            "drain_state should have 3 readings"
        );

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_symbiotic_drain_updates_current_state() {
        use crate::federation::biometric::BiometricSample;

        let specialist = Arc::new(Symbiotic::new());

        // High-HRV sample → should result in low stress classification
        // HRV 90ms → (80 - 90) < 0, clamp to 0 stress
        let mut sample = BiometricSample::heart_rate("dev".to_string(), 60);
        // Craft a payload that includes RR intervals with high HRV
        // Flags=0x10 (RR present), HR=60, RR=[1024, 1024] = 1s intervals (no variation → low HRV)
        sample.raw_payload = Some(vec![0x10, 60, 0x00, 0x04, 0x00, 0x04]);
        specialist.bio_inbox.lock().push_back(sample);

        let task = SymbioticDrainTask::new(specialist.clone(), Duration::from_millis(50));
        let shutdown = Arc::new(Notify::new());
        let handle = task.spawn(shutdown.clone()).await;

        tokio::time::sleep(Duration::from_millis(200)).await;

        let state = specialist.shared_current_state();
        // State should have been classified from the new reading
        assert!(
            state.last_update > 0,
            "current_state.last_update should be set after drain"
        );

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_drain_task_stops_cleanly_on_shutdown() {
        let specialist = Arc::new(Omnipresent::new());
        let task = OmnipresentDrainTask::new(specialist, Duration::from_millis(50));
        let shutdown = Arc::new(Notify::new());
        let handle = task.spawn(shutdown).await;

        assert!(handle.is_running().await);
        handle.shutdown().await;
        assert!(!handle.is_running().await);
    }
}
