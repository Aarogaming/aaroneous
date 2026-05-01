/// Background P2P receive task for the Omnipresent specialist.
///
/// When P2P is attached (`Omnipresent::with_p2p()`), this task calls
/// `P2pNode::recv()` in a loop, writing each incoming `SyncMessage`
/// into `Omnipresent::sync_inbox`. The specialist's `drain_sync_inbox()`
/// then processes the queued messages on its next tick.
///
/// When P2P is NOT attached, this task is a no-op: it registers as started
/// and immediately resolves when shutdown is signaled. This lets the host
/// always attach the task without conditional logic.

use super::BackgroundTaskHandle;
use crate::federation::p2p::P2pError;
use crate::federation::specialists::Omnipresent;
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::{debug, info, warn};

/// Task that drives the P2P receive loop for Omnipresent.
pub struct OmnipresentRecvTask {
    specialist: Arc<Omnipresent>,
}

impl OmnipresentRecvTask {
    pub fn new(specialist: Arc<Omnipresent>) -> Self {
        Self { specialist }
    }

    /// Spawn the receive loop on a background tokio task.
    ///
    /// If no P2P node is attached, the task starts, logs a note, and waits
    /// for shutdown. No messages will be received but the task is alive and
    /// can be cleanly shut down.
    pub async fn spawn(&self, shutdown: Arc<Notify>) -> BackgroundTaskHandle {
        let specialist = self.specialist.clone();
        let shutdown_for_task = shutdown.clone();
        let name = "OmnipresentRecvTask";

        let handle = tokio::spawn(async move {
            if specialist.p2p_node.is_none() {
                info!("{}: no P2P node attached, waiting for shutdown", name);
                shutdown_for_task.notified().await;
                info!("{}: shutdown received, exiting (no-op)", name);
                return;
            }

            let p2p_node = specialist.p2p_node.as_ref().unwrap();
            info!("{}: starting P2P receive loop (node: {})", name, p2p_node.endpoint_id().short());

            loop {
                tokio::select! {
                    // If shutdown is signaled, exit the loop
                    _ = shutdown_for_task.notified() => {
                        info!("{}: shutdown received, stopping recv loop", name);
                        break;
                    }
                    // Wait for the next incoming message
                    result = p2p_node.recv() => {
                        match result {
                            Ok(msg) => {
                                debug!(
                                    "{}: received {:?} from {}",
                                    name,
                                    msg.kind,
                                    msg.from.short()
                                );
                                // Push into the inbox; the specialist drains lazily
                                specialist.sync_inbox.lock().push_back(msg);
                            }
                            Err(P2pError::FeatureNotEnabled) => {
                                // Stub backend doesn't support recv; wait briefly
                                // then retry so we don't spin-loop at 100% CPU.
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            }
                            Err(e) => {
                                warn!("{}: recv error: {}, retrying in 1s", name, e);
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            }
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
    use std::sync::Arc;

    #[tokio::test]
    async fn test_task_no_p2p_node_starts_and_shuts_down() {
        let specialist = Arc::new(Omnipresent::new());
        // No P2P node attached
        let task = OmnipresentRecvTask::new(specialist);
        let shutdown = Arc::new(Notify::new());

        let handle = task.spawn(shutdown.clone()).await;
        assert!(handle.is_running().await);

        // Signal shutdown; task should exit quickly (no blocking recv)
        handle.shutdown().await;
        assert!(!handle.is_running().await);
    }

    #[tokio::test]
    async fn test_task_with_p2p_stub_queues_nothing() {
        // The stub P2P node's recv() returns FeatureNotEnabled immediately.
        // The task should not panic, just retry with backoff.
        let specialist = Omnipresent::new()
            .with_p2p()
            .await
            .expect("stub p2p spawn");
        let specialist = Arc::new(specialist);

        let task = OmnipresentRecvTask::new(specialist.clone());
        let shutdown = Arc::new(Notify::new());
        let handle = task.spawn(shutdown.clone()).await;

        // Let it run briefly
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;

        // Inbox should be empty (stub recv returns FeatureNotEnabled, not messages)
        let inbox_len = specialist.sync_inbox.lock().len();
        assert_eq!(inbox_len, 0, "stub recv should not produce messages");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_manual_inbox_delivery_and_drain() {
        // Simulate what the recv task does: push a message into the inbox,
        // then call drain_sync_inbox() and verify it's processed.
        let mut specialist = Omnipresent::new();
        let node_id = crate::federation::p2p::P2pNodeId::random();
        let msg = crate::federation::p2p::SyncMessage::heartbeat(node_id, 42);

        specialist.sync_inbox.lock().push_back(msg);
        assert_eq!(specialist.sync_inbox.lock().len(), 1);

        let drained = specialist.drain_sync_inbox();
        assert_eq!(drained, 1);
        assert_eq!(specialist.sync_inbox.lock().len(), 0);
        // The heartbeat should have been added to sync_history
        assert!(!specialist.sync_history.is_empty());
    }

    #[tokio::test]
    async fn test_apply_sync_message_full_state_updates_cache() {
        let mut specialist = Omnipresent::new();
        let node_id = crate::federation::p2p::P2pNodeId::random();
        let payload = b"current-intent-v7".to_vec();
        let msg = crate::federation::p2p::SyncMessage::full_state(node_id, 7, payload.clone());

        specialist.apply_sync_message(msg);

        assert_eq!(
            specialist.sync_state.cached_intent,
            Some("current-intent-v7".to_string())
        );
        assert!(!specialist.sync_history.is_empty());
    }

    #[tokio::test]
    async fn test_sync_history_capped_at_1000() {
        let mut specialist = Omnipresent::new();
        let node_id = crate::federation::p2p::P2pNodeId::random();

        // Push 1100 heartbeats - history should be capped at 1000
        for _ in 0..1100 {
            let msg = crate::federation::p2p::SyncMessage::heartbeat(node_id.clone(), 1);
            specialist.apply_sync_message(msg);
        }

        assert!(
            specialist.sync_history.len() <= 1000,
            "history should be capped at 1000, got {}",
            specialist.sync_history.len()
        );
    }
}
