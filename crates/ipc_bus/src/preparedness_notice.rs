// Preparedness Notice System
// Coordinated Synchronized Stage Notice broadcast that tells all reader components
// to cleanly drop active references before memory swaps occur.

use tokio::sync::watch;

/// A preparedness notice - signals an imminent memory swap
#[derive(Debug, Clone)]
pub struct PreparednessNotice {
    /// Generation being swapped out
    pub generation: u64,
    /// Target field being mutated
    pub target_field: String,
    /// Nanosecond timestamp
    pub timestamp: u64,
}

impl PreparednessNotice {
    pub fn is_for_generation(&self, gen: u64) -> bool {
        self.generation == gen
    }
}

/// Broadcast channel for preparedness notices
pub struct NoticeBroadcast {
    tx: watch::Sender<Option<PreparednessNotice>>,
}

impl Default for NoticeBroadcast {
    fn default() -> Self {
        Self::new()
    }
}

impl NoticeBroadcast {
    pub fn new() -> Self {
        let (tx, _rx) = watch::channel(None);
        Self { tx }
    }

    /// Broadcast a preparedness notice to all readers
    pub fn broadcast(&self, notice: &PreparednessNotice) {
        let _ = self.tx.send(Some(notice.clone()));
    }

    /// Subscribe to preparedness notices
    pub fn subscribe(&self) -> watch::Receiver<Option<PreparednessNotice>> {
        self.tx.subscribe()
    }
}
