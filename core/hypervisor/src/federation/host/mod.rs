/// SpecialistHost: lifecycle wrapper for a federation specialist
///
/// This module owns the full operational lifecycle of one federation
/// specialist (Visionary, Omnipresent, etc.):
///
/// - **Start**: Load any prior learning state from the persistence layer.
/// - **Run**: Optionally drive a periodic checkpoint loop that saves
///   learning state to SQLite at a configurable cadence.
/// - **Checkpoint**: Manual save trigger for callers that prefer event-driven
///   checkpointing (e.g., "save after every 100 executions").
/// - **Shutdown**: Stop the checkpoint loop, do one final save, return.
///
/// # Why a separate type instead of methods on Specialist?
///
/// The federation `Specialist` trait should describe what specialists *do*
/// (propose, execute, delegate, negotiate). Lifecycle and persistence
/// are operational concerns - they belong outside the trait.
///
/// `SpecialistHost` is also where future operational features will live:
/// - Health checks and watchdogs
/// - Crash recovery
/// - Metrics emission (Prometheus, OpenTelemetry)
/// - Resource quota enforcement
///
/// Keeping these out of the specialist itself makes specialists easy to
/// test in isolation and keeps the trait minimal.
///
/// # Usage
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use std::sync::Arc;
/// use a_run::federation::specialists::Visionary;
/// use a_run::federation::host::{SpecialistHost, HostConfig};
/// use a_run::persistence::PersistenceManager;
///
/// let pm = Arc::new(tokio::sync::Mutex::new(PersistenceManager::new("hive.db")?));
/// let visionary = Arc::new(Visionary::new());
///
/// let host = SpecialistHost::new(visionary.clone(), pm, HostConfig::default());
/// host.start().await?;                  // Loads prior state
/// host.spawn_checkpoint_loop();         // Auto-saves every interval
///
/// // ... use the specialist normally via the Arc<Visionary> ...
///
/// host.shutdown().await?;               // Final save + stop loop
/// # Ok(())
/// # }
/// ```

pub mod hostable;
pub mod config;

#[cfg(test)]
mod tests;

pub use config::HostConfig;
pub use hostable::HostableSpecialist;

use crate::federation::learn_persist::LearnPersistError;
use crate::persistence::PersistenceManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Thread-safe handle to a `PersistenceManager`.
///
/// `PersistenceManager` wraps a `rusqlite::Connection` which is `Send` but
/// not `Sync` (SQLite connections are inherently single-threaded - sharing
/// without serialization would be unsound). Wrapping in a `Mutex` makes the
/// connection safely shareable: each holder serializes its access.
///
/// SQLite handles serialized access efficiently, so this Mutex doesn't
/// become a real bottleneck for the kinds of operations specialists do
/// (small upserts on save, single SELECT on load).
pub type SharedPersistence = Arc<Mutex<PersistenceManager>>;

/// Convenience: wrap a `PersistenceManager` in `Arc<Mutex<>>` so it can
/// be shared across hosts and tasks.
pub fn shared(pm: PersistenceManager) -> SharedPersistence {
    Arc::new(Mutex::new(pm))
}

/// Errors that can occur during specialist hosting
#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("learning persistence error: {0}")]
    Persist(#[from] LearnPersistError),

    #[error("host already started")]
    AlreadyStarted,

    #[error("host not started")]
    NotStarted,

    #[error("host already shut down")]
    AlreadyShutDown,
}

/// Lifecycle state of a `SpecialistHost`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostState {
    /// Created but `start()` not yet called
    NotStarted,
    /// `start()` succeeded, specialist is operational
    Running,
    /// `shutdown()` called and completed
    ShutDown,
}

/// Lifecycle host for one specialist + its persistence.
///
/// The host holds an `Arc<S>` so the underlying specialist can be shared
/// freely. The host's contribution is the lifecycle wiring: load on start,
/// auto-save during run, final save on shutdown.
pub struct SpecialistHost<S: HostableSpecialist + 'static> {
    /// The specialist being hosted (kept as Arc so callers retain ownership)
    specialist: Arc<S>,
    /// Mutex-wrapped persistence manager. Shared across hosts in a federation.
    persistence: SharedPersistence,
    /// Host configuration
    config: HostConfig,
    /// Current lifecycle state
    state: Arc<RwLock<HostState>>,
    /// Signal used to stop the checkpoint loop
    shutdown_signal: Arc<Notify>,
    /// Handle to the checkpoint task (if spawned)
    checkpoint_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// Optional background receive task (e.g., P2P recv, BLE notifications).
    /// Started on `start()` if attached before the first call; stopped on
    /// `shutdown()`. Attach via `attach_recv_task()`.
    recv_task: Arc<RwLock<Option<crate::federation::tasks::BackgroundTaskHandle>>>,
}

impl<S: HostableSpecialist + 'static> SpecialistHost<S> {
    /// Create a new host. The specialist is *not* started yet.
    pub fn new(
        specialist: Arc<S>,
        persistence: SharedPersistence,
        config: HostConfig,
    ) -> Self {
        Self {
            specialist,
            persistence,
            config,
            state: Arc::new(RwLock::new(HostState::NotStarted)),
            shutdown_signal: Arc::new(Notify::new()),
            checkpoint_handle: Arc::new(RwLock::new(None::<JoinHandle<()>>)),
            recv_task: Arc::new(RwLock::new(None::<crate::federation::tasks::BackgroundTaskHandle>)),
        }
    }

    /// Attach a background receive task to this host.
    ///
    /// The task must already be running (returned by `*.spawn()`). It will
    /// be shut down as part of `shutdown()`. If called after `shutdown()`,
    /// the task is stored but will not be cleaned up by this host - the
    /// caller is responsible for shutting it down manually.
    ///
    /// Replace a previously-attached task by calling this again; the old
    /// task handle is dropped (NOT shut down) - shut it down first if needed.
    pub async fn attach_recv_task(
        &self,
        task: crate::federation::tasks::BackgroundTaskHandle,
    ) {
        *self.recv_task.write().await = Some(task);
    }

    /// Whether a receive task is currently attached and running.
    pub async fn has_recv_task(&self) -> bool {
        let guard: tokio::sync::RwLockReadGuard<'_, Option<crate::federation::tasks::BackgroundTaskHandle>> = self.recv_task.read().await;
        match guard.as_ref() {
            Some(task) => crate::federation::tasks::BackgroundTaskHandle::is_running(task),
            None => false,
        }
    }

    /// Get a clone of the inner specialist Arc
    pub fn specialist(&self) -> Arc<S> {
        self.specialist.clone()
    }

    /// Current lifecycle state
    pub async fn state(&self) -> HostState {
        *self.state.read().await
    }

    /// The persistence key under which this specialist's learning is stored
    pub fn persistence_key(&self) -> &'static str {
        S::persistence_key()
    }

    /// Start the host: load prior learning state from persistence.
    ///
    /// If no prior state exists, the specialist keeps its in-memory neutral
    /// state. The `Ok(true)` / `Ok(false)` distinction is logged but not
    /// returned to callers - either case is normal startup behavior.
    pub async fn start(&self) -> Result<(), HostError> {
        {
            let mut state = self.state.write().await;
            if *state == HostState::Running {
                return Err(HostError::AlreadyStarted);
            }
            if *state == HostState::ShutDown {
                return Err(HostError::AlreadyShutDown);
            }
            *state = HostState::Running;
        }

        let key = S::persistence_key();
        match self.specialist.load_learning(&self.persistence).await {
            Ok(true) => info!("Specialist {} loaded prior learning state", key),
            Ok(false) => info!("Specialist {} has no prior state, starting neutral", key),
            Err(e) => {
                error!("Failed to load learning state for {}: {}", key, e);
                return Err(HostError::Persist(e));
            }
        }

        Ok(())
    }

    /// Save the current learning state to persistence right now.
    ///
    /// Safe to call frequently - SQLite handles upserts cheaply.
    pub async fn checkpoint_now(&self) -> Result<(), HostError> {
        let state = *self.state.read().await;
        if state != HostState::Running {
            return Err(HostError::NotStarted);
        }
        self.specialist.save_learning(&self.persistence).await?;
        debug!(
            "Checkpointed learning state for {}",
            S::persistence_key()
        );
        Ok(())
    }

    /// Spawn the auto-checkpoint background task.
    ///
    /// The task wakes up every `config.checkpoint_interval`, calls
    /// `checkpoint_now()`, and exits when `shutdown()` is called.
    ///
    /// If `config.checkpoint_interval` is zero, no task is spawned.
    /// Calling this method twice replaces the prior handle (the prior
    /// task continues running but its handle is dropped, so it will
    /// keep running until explicit shutdown - prefer one call per host).
    pub async fn spawn_checkpoint_loop(&self) {
        if self.config.checkpoint_interval.is_zero() {
            debug!(
                "Checkpoint interval is zero for {}, not spawning loop",
                S::persistence_key()
            );
            return;
        }

        let specialist = self.specialist.clone();
        let persistence = self.persistence.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        let interval = self.config.checkpoint_interval;
        let key = S::persistence_key();

        let handle = tokio::spawn(async move {
            info!(
                "Checkpoint loop started for {} ({}ms interval)",
                key,
                interval.as_millis()
            );
            loop {
                tokio::select! {
                    _ = shutdown_signal.notified() => {
                        debug!("Checkpoint loop for {} received shutdown signal", key);
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        if let Err(e) = specialist.save_learning(&persistence).await {
                            warn!("Checkpoint failed for {}: {}", key, e);
                        }
                    }
                }
            }
            info!("Checkpoint loop ended for {}", key);
        });

        *self.checkpoint_handle.write().await = Some(handle);
    }

    /// Shut down the host: stop the checkpoint loop, do a final save, mark
    /// the host as shut down.
    ///
    /// Safe to call multiple times - subsequent calls return
    /// `Err(HostError::AlreadyShutDown)` without doing further work.
    pub async fn shutdown(&self) -> Result<(), HostError> {
        {
            let mut state = self.state.write().await;
            match *state {
                HostState::ShutDown => return Err(HostError::AlreadyShutDown),
                HostState::NotStarted => {
                    // Allow shutdown of never-started host as a no-op cleanup
                    *state = HostState::ShutDown;
                    return Ok(());
                }
                HostState::Running => {
                    *state = HostState::ShutDown;
                }
            }
        }

        // Stop the checkpoint loop
        self.shutdown_signal.notify_one();

        // Wait for the loop task to finish (with a generous timeout)
        if let Some(handle) = self.checkpoint_handle.write().await.take() {
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(_) => debug!("Checkpoint loop joined cleanly"),
                Err(_) => warn!("Checkpoint loop did not join within 5s, abandoning"),
            }
        }

        // Stop the recv task (if any)
        let mut recv_guard: tokio::sync::RwLockWriteGuard<'_, Option<crate::federation::tasks::BackgroundTaskHandle>> = self.recv_task.write().await;
        if let Some(ref mut task) = *recv_guard {
            crate::federation::tasks::BackgroundTaskHandle::shutdown(task).await;
            debug!("Recv task for {} stopped", S::persistence_key());
        }
        *recv_guard = None;

        // Final save
        if let Err(e) = self.specialist.save_learning(&self.persistence).await {
            warn!(
                "Final save during shutdown failed for {}: {}",
                S::persistence_key(),
                e
            );
            return Err(HostError::Persist(e));
        }

        info!("Host for {} shut down cleanly", S::persistence_key());
        Ok(())
    }
}
