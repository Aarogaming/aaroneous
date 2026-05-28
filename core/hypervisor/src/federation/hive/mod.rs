/// Federation: aggregator for all specialist hosts in one process.
///
/// Where `SpecialistHost<S>` manages one specialist's lifecycle, `Federation`
/// composes hosts for any subset of the 5 federation specialists into a
/// single start/shutdown surface. This is the type that an application's
/// `main` typically constructs and drives.
///
/// # Why fields instead of `Vec<dyn HostableSpecialist>`?
///
/// The 5 specialists have different concrete types and specialist-specific
/// methods (e.g., `Omnipresent::register_device_with_endpoint()`). A trait
/// object would erase those, which is the wrong trade-off here. Instead,
/// each specialist gets an `Option<Arc<SpecialistHost<S>>>` field so:
///
/// - Partial deployments work (some apps only need Visionary + Symbiotic)
/// - Typed accessors return the concrete `Arc<S>` for direct use
/// - Lifecycle iterates the present hosts and applies start/checkpoint/shutdown
///
/// # Usage
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use std::time::Duration;
/// use a_run::federation::hive::{Federation, FederationConfig};
/// use a_run::persistence::PersistenceManager;
///
/// let pm = PersistenceManager::new("hive.db")?;
/// let fed = Federation::builder(pm)
///     .with_visionary()
///     .with_omnipresent()
///     .with_symbiotic()
///     .checkpoint_every(Duration::from_secs(30))
///     .build();
///
/// fed.start_all().await?;             // Each host loads its prior state
/// fed.spawn_checkpoint_loops().await; // Each host's auto-save loop spins up
///
/// // ... use specialists via fed.visionary().unwrap() etc ...
///
/// fed.shutdown_all().await?;          // All hosts: stop loop + final save
/// # Ok(())
/// # }
/// ```

pub mod builder;
pub mod config;
pub mod scheduler;

#[cfg(test)]
mod tests;

pub use builder::FederationBuilder;
pub use config::FederationConfig;

use crate::federation::host::{HostError, SharedPersistence, SpecialistHost};
use crate::federation::specialists::{Archivist, Omnipresent, Phygital, Symbiotic, Visionary};
use crate::federation::intent::Intent;
use crate::federation::session::SessionManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

/// Aggregated lifecycle errors from a federation start/shutdown cycle.
///
/// Unlike `HostError`, which represents a single host's failure, this type
/// can hold multiple errors (one per failing specialist) so the caller sees
/// the full picture even when some hosts succeed.
#[derive(Debug)]
pub struct FederationErrors {
    pub errors: Vec<(String, HostError)>,
}

impl std::fmt::Display for FederationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.errors.len() == 1 {
            write!(f, "federation error: [{}] {}", self.errors[0].0, self.errors[0].1)
        } else {
            write!(f, "federation errors ({}):", self.errors.len())?;
            for (kind, err) in &self.errors {
                write!(f, "\n  [{}] {}", kind, err)?;
            }
            Ok(())
        }
    }
}

impl std::error::Error for FederationErrors {}

/// All five specialists, each optional. A `Federation` is constructed via
/// `Federation::builder()`.
pub struct Federation {
    persistence: SharedPersistence,
    config: FederationConfig,
    visionary: Option<Arc<SpecialistHost<Visionary>>>,
    omnipresent: Option<Arc<SpecialistHost<Omnipresent>>>,
    symbiotic: Option<Arc<SpecialistHost<Symbiotic>>>,
    phygital: Option<Arc<SpecialistHost<Phygital>>>,
    archivist: Option<Arc<SpecialistHost<Archivist>>>,

    /// Runtime-spawned generic specialists, each backed by their own GGUF model.
    /// These sovereigns are added via `FederationBuilder::with_gguf_specialist()`
    /// or `Federation::add_generic_specialist()` at runtime without recompilation.
    pub dynamic: Arc<RwLock<Vec<Arc<crate::federation::specialists::GenericSpecialist>>>>,

    /// The active user intent — what the user is trying to accomplish right now.
    /// `None` until the first `submit_intent()` call.
    /// Shared with all specialists and the HTTP `/intent` endpoint.
    pub active_intent: Arc<RwLock<Option<Intent>>>,

    /// Queue of completed execution results waiting to be read.
    /// Populated by `record_result()`, consumed by the HTTP `/results` endpoint.
    pub results: Arc<Mutex<Vec<crate::federation::specialist::ExecutionResult>>>,

    /// Sentinel for proposal arbitration.
    /// Instantiated when the federation starts; `None` until `start_all()`.
    pub sentinel: Arc<RwLock<Option<crate::federation::sentinel::Sentinel>>>,

    /// Shutdown signal for the Sentinel arbitration loop
    sentinel_shutdown: Arc<tokio::sync::Notify>,

    /// Session registry: user identity and per-session history.
    pub sessions: Arc<RwLock<SessionManager>>,

    /// Optional multi-hive federation context.
    /// When enabled, this hive can coordinate with other Aaroneous instances.
    pub multi_hive: Arc<RwLock<Option<crate::federation::multi_hive::MultihiveFederation>>>,

    /// NATS client for broadcasting events and intents
    pub nats: Arc<crate::nats_client::NatsPublisher>,

    /// Audit log for compliance and observability.
    /// Records every federation decision, intent submission, and session event.
    pub audit_log: Arc<Mutex<crate::federation::enterprise::AuditLog>>,

    /// Broadcast channel for specialist state events.
    ///
    /// Every time a specialist executes, proposes, or its learning state
    /// changes, a `SpecialistEvent` JSON value is sent on this channel.
    /// Consumers (SSE handlers, real-time dashboard clients, etc.) subscribe via
    /// `Federation::subscribe_specialist_events()`.
    ///
    /// Capacity: 256 events buffered. Slow consumers miss events rather
    /// than blocking the federation.
    pub specialist_events: Arc<tokio::sync::broadcast::Sender<serde_json::Value>>,

    /// Cross-sovereign federation memory (RAG).
    ///
    /// Every sovereign execution result is stored here with intent + output text.
    /// Future intent submissions query it to retrieve the most relevant past
    /// context across ALL sovereigns, which is prepended to Odin's decomposition.
    pub federation_memory: Arc<Mutex<crate::federation::graph::EmbeddingStore>>,

    /// Autonomous Task Scheduler
    pub scheduler: Arc<RwLock<crate::federation::hive::scheduler::AutonomousScheduler>>,

    /// System Biology: token-bucket expression rate governance
    pub biology: Arc<RwLock<crate::SystemBiology>>,
}

impl Federation {
    /// Begin building a `Federation`. The persistence manager is wrapped in
    /// the federation's `SharedPersistence` so all hosts share one connection.
    pub fn builder(pm: crate::persistence::PersistenceManager) -> FederationBuilder {
        FederationBuilder::new(pm)
    }

    /// Get the shared persistence handle (if callers need direct DB access)
    pub fn persistence(&self) -> SharedPersistence {
        self.persistence.clone()
    }

    /// The configuration this federation was built with
    pub fn config(&self) -> &FederationConfig {
        &self.config
    }

    // ------- Typed specialist accessors -------

    pub fn visionary(&self) -> Option<Arc<Visionary>> {
        self.visionary.as_ref().map(|h| h.specialist())
    }

    pub fn omnipresent(&self) -> Option<Arc<Omnipresent>> {
        self.omnipresent.as_ref().map(|h| h.specialist())
    }

    pub fn symbiotic(&self) -> Option<Arc<Symbiotic>> {
        self.symbiotic.as_ref().map(|h| h.specialist())
    }

    pub fn phygital(&self) -> Option<Arc<Phygital>> {
        self.phygital.as_ref().map(|h| h.specialist())
    }

    pub fn archivist(&self) -> Option<Arc<Archivist>> {
        self.archivist.as_ref().map(|h| h.specialist())
    }

    /// How many specialists are configured in this federation (1..=5 core + N dynamic)
    pub fn enabled_count(&self) -> usize {
        let mut n = 0;
        if self.visionary.is_some() { n += 1; }
        if self.omnipresent.is_some() { n += 1; }
        if self.symbiotic.is_some() { n += 1; }
        if self.phygital.is_some() { n += 1; }
        if self.archivist.is_some() { n += 1; }
        // Dynamic specialists are counted synchronously via try_read
        if let Ok(dyn_guard) = self.dynamic.try_read() {
            n += dyn_guard.len();
        }
        n
    }

    /// Add a runtime-spawned `GenericSpecialist` to this federation.
    ///
    /// The specialist participates in `collect_proposals()` and `run_decision()`
    /// immediately after being added.  Its learning state is persisted to
    /// SQLite using its `persistence_key`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example(fed: &a_run::federation::hive::Federation) {
    /// use a_run::federation::specialists::GenericSpecialist;
    ///
    /// let s = GenericSpecialist::new("LegalAnalyst", "legal")
    ///     .with_gguf_path("models/qwen-legal-1.8b.gguf").await;
    /// fed.add_generic_specialist(std::sync::Arc::new(s)).await;
    /// # }
    /// ```
    pub async fn add_generic_specialist(
        &self,
        specialist: Arc<crate::federation::specialists::GenericSpecialist>,
    ) {
        // Load prior learning from DB if available (best-effort)
        {
            let pm = self.persistence.lock().await;
            let _ = specialist.load_learning_from(&*pm);
        }
        // Restore sovereign-local RAG memory from disk
        specialist.load_memory_from_disk();
        let name = specialist.name.clone();
        // Dedup: skip if a sovereign with the same name is already loaded
        {
            let guard = self.dynamic.read().await;
            if guard.iter().any(|s| s.name == name) {
                info!("Sovereign '{}' already loaded — skipping duplicate", name);
                return;
            }
        }
        self.dynamic.write().await.push(specialist);
        info!("Sovereign '{}' added to federation (dynamic slot)", name);
    }

    /// List all dynamic specialists currently in this federation.
    pub async fn dynamic_specialists(
        &self,
    ) -> Vec<Arc<crate::federation::specialists::GenericSpecialist>> {
        self.dynamic.read().await.clone()
    }

    /// Subscribe to the specialist event broadcast channel.
    ///
    /// Returns a `Receiver` that receives `serde_json::Value` payloads
    /// describing specialist state changes (execution results, proposals,
    /// intent updates, confidence changes).
    ///
    /// Used by `GET /specialists/stream` and can be used by any other
    /// real-time consumer (SSE clients, monitoring tools, etc.).
    pub fn subscribe_specialist_events(&self) -> tokio::sync::broadcast::Receiver<serde_json::Value> {
        self.specialist_events.subscribe()
    }

    /// Broadcast a specialist state event to all active SSE/WebSocket subscribers.
    ///
    /// Called internally after every execution, proposal collection, and
    /// intent submission. Silently no-ops if there are no subscribers.
    pub fn broadcast_specialist_event(&self, event: serde_json::Value) {
        // send() returns Err only when there are no receivers — that's fine
        let _ = self.specialist_events.send(event);
    }

    /// Build and broadcast a complete specialist snapshot event.
    /// Called after each execution so SSE clients always have current state.
    pub async fn broadcast_specialist_snapshot(&self, specialist_name: &str) {
        let summary = self.learning_summary();
        let intent = self.active_intent.read().await
            .as_ref().map(|i| i.content.clone())
            .unwrap_or_default();

        let find_summary = |name: &str| -> Option<serde_json::Value> {
            macro_rules! check {
                ($field:expr, $label:literal) => {
                    if name.to_lowercase() == $label {
                        return $field.as_ref().map(|s| serde_json::json!({
                            "confidence": s.confidence_score,
                            "total_executions": s.total_executions,
                            "success_rate": s.success_rate_percent(),
                        }));
                    }
                };
            }
            check!(summary.visionary,   "visionary");
            check!(summary.omnipresent, "omnipresent");
            check!(summary.symbiotic,   "symbiotic");
            check!(summary.phygital,    "phygital");
            check!(summary.archivist,   "archivist");
            None
        };

        let learning = find_summary(specialist_name);

        let event = serde_json::json!({
            "type": "specialist_update",
            "specialist": specialist_name,
            "active_intent": intent,
            "learning": learning,
            "timestamp_ms": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        });
        self.broadcast_specialist_event(event);
    }

    // ------- Lifecycle -------

    /// Start every configured host. Each host loads its prior learning state
    /// from persistence as part of its own `start()`.
    ///
    /// If any host fails to start, all errors are collected and returned in
    /// a `FederationErrors`. Hosts that succeeded remain in `Running` state -
    /// callers can decide whether to abort (call `shutdown_all`) or proceed.
    pub async fn start_all(&self) -> Result<(), FederationErrors> {
        info!("Starting federation with {} specialist(s)", self.enabled_count());
        let mut errors = Vec::new();

        // Restore federation RAG memory from previous session.
        // Skipped in test builds to prevent persistent disk state from
        // interfering with hermetic test assertions.
        #[cfg(not(test))]
        {
            let restored = crate::federation::graph::EmbeddingStore::load_from_disk(256);
            let count = restored.total_count();
            *self.federation_memory.lock().await = restored;
            if count > 0 {
                info!("Federation memory: restored {} cross-sovereign memories", count);
            }
        }

        // Reload sessions that survived the last process run
        self.load_sessions_from_db().await;
        // Restore the active intent from the most recently used session so
        // specialists resume proposing on in-flight work after a restart.
        self.restore_active_intent_from_sessions().await;

        if let Some(h) = &self.visionary {
            if let Err(e) = h.start().await {
                errors.push((Visionary::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.omnipresent {
            if let Err(e) = h.start().await {
                errors.push((Omnipresent::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.symbiotic {
            if let Err(e) = h.start().await {
                errors.push((Symbiotic::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.phygital {
            if let Err(e) = h.start().await {
                errors.push((Phygital::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.archivist {
            if let Err(e) = h.start().await {
                errors.push((Archivist::PERSISTENCE_KEY.to_string(), e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(FederationErrors { errors })
        }
    }

    /// Spawn auto-checkpoint loops for every configured host.
    ///
    /// Hosts that are configured with `HostConfig::manual_only()` (zero
    /// checkpoint interval) will skip spawning their loop - calling this
    /// method is still safe.
    pub async fn spawn_checkpoint_loops(&self) {
        if let Some(h) = &self.visionary { h.spawn_checkpoint_loop().await; }
        if let Some(h) = &self.omnipresent { h.spawn_checkpoint_loop().await; }
        if let Some(h) = &self.symbiotic { h.spawn_checkpoint_loop().await; }
        if let Some(h) = &self.phygital { h.spawn_checkpoint_loop().await; }
        if let Some(h) = &self.archivist { h.spawn_checkpoint_loop().await; }

        // Spawn checkpoint loops for dynamic (generic) specialists.
        // Each saves its learning state every 30 seconds independently.
        let interval = self.config.default_checkpoint_interval;
        if interval.is_zero() { return; }

        let dynamic = self.dynamic.read().await.clone();
        let persistence = self.persistence.clone();
        let shutdown = self.sentinel_shutdown.clone();

        if dynamic.is_empty() { return; }

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = tokio::time::sleep(interval) => {
                        let pm = persistence.lock().await;
                        for s in &dynamic {
                            if let Err(e) = s.save_learning_to(&*pm) {
                                warn!("GenericSpecialist '{}' checkpoint failed: {}", s.name, e);
                            }
                        }
                    }
                }
            }
        });
    }

    /// Trigger a manual checkpoint on every configured host.
    pub async fn checkpoint_all(&self) -> Result<(), FederationErrors> {
        let mut errors = Vec::new();

        if let Some(h) = &self.visionary {
            if let Err(e) = h.checkpoint_now().await {
                errors.push((Visionary::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.omnipresent {
            if let Err(e) = h.checkpoint_now().await {
                errors.push((Omnipresent::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.symbiotic {
            if let Err(e) = h.checkpoint_now().await {
                errors.push((Symbiotic::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.phygital {
            if let Err(e) = h.checkpoint_now().await {
                errors.push((Phygital::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.archivist {
            if let Err(e) = h.checkpoint_now().await {
                errors.push((Archivist::PERSISTENCE_KEY.to_string(), e));
            }
        }

        if errors.is_empty() { Ok(()) } else { Err(FederationErrors { errors }) }
    }

    /// Shut down every configured host: stop loops, final save, mark shut down.
    ///
    /// Errors are collected; every host gets a chance to shut down even if
    /// one fails. Use this for graceful application exit.
    pub async fn shutdown_all(&self) -> Result<(), FederationErrors> {
        info!("Shutting down federation");
        let mut errors = Vec::new();

        if let Some(h) = &self.visionary {
            if let Err(e) = h.shutdown().await {
                errors.push((Visionary::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.omnipresent {
            if let Err(e) = h.shutdown().await {
                errors.push((Omnipresent::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.symbiotic {
            if let Err(e) = h.shutdown().await {
                errors.push((Symbiotic::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.phygital {
            if let Err(e) = h.shutdown().await {
                errors.push((Phygital::PERSISTENCE_KEY.to_string(), e));
            }
        }
        if let Some(h) = &self.archivist {
            if let Err(e) = h.shutdown().await {
                errors.push((Archivist::PERSISTENCE_KEY.to_string(), e));
            }
        }

        // Final save for all dynamic (generic) specialists
        {
            let pm = self.persistence.lock().await;
            let dynamic = self.dynamic.read().await;
            for s in dynamic.iter() {
                if let Err(e) = s.save_learning_to(&*pm) {
                    warn!("GenericSpecialist '{}' final save failed: {}", s.name, e);
                } else {
                    info!("GenericSpecialist '{}' learning saved on shutdown", s.name);
                }
                // Persist each sovereign's local RAG memory
                if let Err(e) = s.save_memory_to_disk() {
                    warn!("GenericSpecialist '{}' memory save failed: {}", s.name, e);
                }
            }
        }

        // Persist federation RAG memory for next session
        {
            let mem = self.federation_memory.lock().await;
            match mem.save_to_disk() {
                Ok(()) => info!("Federation memory: {} memories persisted to disk",
                                mem.total_count()),
                Err(e) => warn!("Federation memory: save failed: {}", e),
            }
        }

        if errors.is_empty() { Ok(()) } else { Err(FederationErrors { errors }) }
    }

    // --------------------------------------------------------------
    // Convenience: full lifecycle in one call
    // --------------------------------------------------------------

    /// Run the federation until a shutdown signal is received.
    ///
    /// This is the canonical pattern for `main.rs`:
    ///
    /// 1. `start_all()` (load prior state)
    /// 2. `spawn_checkpoint_loops()` (auto-save in background)
    /// 3. Wait for `ctrl_c()`
    /// 4. `shutdown_all()` (stop loops + final save)
    ///
    /// Returns the result of `shutdown_all()`. Errors during start_all are
    /// returned immediately without proceeding.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use a_run::federation::hive::Federation;
    /// use a_run::persistence::PersistenceManager;
    ///
    /// let pm = PersistenceManager::new("hive.db")?;
    /// let fed = Federation::builder(pm).with_all().build();
    /// fed.run_until_signal().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_until_signal(&self) -> Result<(), FederationErrors> {
        self.run_until(async {
            // Best-effort ctrl_c handler; log if installation fails but
            // don't crash the process.
            if let Err(e) = tokio::signal::ctrl_c().await {
                warn!("Failed to wait for ctrl_c signal: {}", e);
            }
            info!("Shutdown signal received");
        })
        .await
    }

    /// Run the federation until the given future resolves.
    ///
    /// Like `run_until_signal()` but with a custom termination condition,
    /// useful for tests (e.g., resolve after a `tokio::time::sleep`) and
    /// for applications with custom shutdown logic (e.g., HTTP signal,
    /// admin command).
    ///
    /// The provided future is awaited *after* `start_all` and
    /// `spawn_checkpoint_loops`, and *before* `shutdown_all`. Any panic in
    /// the future propagates to the caller.
    pub async fn run_until<F>(&self, terminator: F) -> Result<(), FederationErrors>
    where
        F: std::future::Future<Output = ()>,
    {
        self.start_all().await?;
        self.spawn_checkpoint_loops().await;
        info!("Federation running ({} specialist(s))", self.enabled_count());
        terminator.await;
        self.shutdown_all().await
    }

    // --------------------------------------------------------------
    // Diagnostics
    // --------------------------------------------------------------

    /// Take a read-only snapshot of every configured specialist's learning
    /// state.
    ///
    /// This locks each specialist's `learning` Mutex briefly to copy the
    /// counters out, then releases. No SQL I/O happens - this reads from
    /// in-memory state only. Cheap to call frequently for status/HTTP/CLI
    /// endpoints.
    ///
    /// Specialists not configured in this federation appear as `None` in
    /// the corresponding field. Specialists that are configured but never
    /// loaded/trained (e.g., right after `Federation::builder().with_*()`
    /// before `start_all()`) appear as `Some(neutral_summary)`.
    pub fn learning_summary(&self) -> LearningSummary {
        // Helper closure: bind the Arc<Specialist> first so the lock guard's
        // referent (the Mutex) outlives the borrow. `h.specialist()` returns
        // an Arc by value; binding it to `arc` keeps it alive across the lock.
        LearningSummary {
            visionary: self.visionary.as_ref().map(|h| {
                let arc = h.specialist();
                let l = arc.learning.lock();
                SpecialistLearningSummary::from_data(
                    l.success_count,
                    l.failure_count,
                    l.total_executions,
                    l.confidence_score,
                    l.execution_history.len(),
                    l.last_updated,
                )
            }),
            omnipresent: self.omnipresent.as_ref().map(|h| {
                let arc = h.specialist();
                let l = arc.learning.lock();
                SpecialistLearningSummary::from_data(
                    l.success_count,
                    l.failure_count,
                    l.total_executions,
                    l.confidence_score,
                    l.execution_history.len(),
                    l.last_updated,
                )
            }),
            symbiotic: self.symbiotic.as_ref().map(|h| {
                let arc = h.specialist();
                let l = arc.learning.lock();
                SpecialistLearningSummary::from_data(
                    l.success_count,
                    l.failure_count,
                    l.total_executions,
                    l.confidence_score,
                    l.execution_history.len(),
                    l.last_updated,
                )
            }),
            phygital: self.phygital.as_ref().map(|h| {
                let arc = h.specialist();
                let l = arc.learning.lock();
                SpecialistLearningSummary::from_data(
                    l.success_count,
                    l.failure_count,
                    l.total_executions,
                    l.confidence_score,
                    l.execution_history.len(),
                    l.last_updated,
                )
            }),
            archivist: self.archivist.as_ref().map(|h| {
                let arc = h.specialist();
                let l = arc.learning.lock();
                SpecialistLearningSummary::from_data(
                    l.success_count,
                    l.failure_count,
                    l.total_executions,
                    l.confidence_score,
                    l.execution_history.len(),
                    l.last_updated,
                )
            }),
        }
    }

    /// Return the confidence trend (time-series) for all configured specialists.
    ///
    /// Each entry is `(unix_seconds, confidence_score)`. The trend is populated
    /// by `record_result()` on each `*LearningData` and persisted across restarts
    /// via the v2 `execution_history_json` envelope.
    pub fn learning_trends(&self) -> LearningTrends {
        macro_rules! trend_for {
            ($host_opt:expr) => {
                $host_opt.as_ref().map(|h| {
                    let arc = h.specialist();
                    let l = arc.learning.lock();
                    l.confidence_trend.clone()
                })
            };
        }

        // Dynamic specialist trends (sync try_read — safe since not awaiting)
        let dynamic_trends: std::collections::HashMap<String, Vec<(u64, f32)>> = self
            .dynamic
            .try_read()
            .map(|guard| {
                guard.iter()
                    .map(|s| {
                        let l = s.learning.lock();
                        (s.name.clone(), l.confidence_trend.clone())
                    })
                    .collect()
            })
            .unwrap_or_default();

        LearningTrends {
            visionary:    trend_for!(self.visionary),
            omnipresent:  trend_for!(self.omnipresent),
            symbiotic:    trend_for!(self.symbiotic),
            phygital:     trend_for!(self.phygital),
            archivist:    trend_for!(self.archivist),
            dynamic:      dynamic_trends,
        }
    }
}

/// Read-only summary of one specialist's learning state.
///
/// Returned as part of `LearningSummary` from `Federation::learning_summary()`.
/// Cheap to construct (just integers) and `serde`-friendly so it can be
/// emitted as JSON for HTTP status endpoints.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpecialistLearningSummary {
    pub success_count: u32,
    pub failure_count: u32,
    pub total_executions: u32,
    pub confidence_score: f32,
    /// How many recent outcomes are in the rolling history (always <= 20)
    pub history_len: usize,
    /// Unix seconds of last record_result, or 0 if never recorded
    pub last_updated: u64,
}

impl SpecialistLearningSummary {
    fn from_data(
        success_count: u32,
        failure_count: u32,
        total_executions: u32,
        confidence_score: f32,
        history_len: usize,
        last_updated: u64,
    ) -> Self {
        Self {
            success_count,
            failure_count,
            total_executions,
            confidence_score,
            history_len,
            last_updated,
        }
    }

    /// Success rate as a percentage 0.0..=100.0. Returns 0.0 if no executions.
    pub fn success_rate_percent(&self) -> f32 {
        if self.total_executions == 0 {
            0.0
        } else {
            (self.success_count as f32 / self.total_executions as f32) * 100.0
        }
    }
}

/// Time-series confidence trends for all configured specialists.
///
/// Each value is `Vec<(unix_seconds, confidence_score)>` — `None` when the
/// specialist is not configured.  Returned by `Federation::learning_trends()`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LearningTrends {
    pub visionary:   Option<Vec<(u64, f32)>>,
    pub omnipresent: Option<Vec<(u64, f32)>>,
    pub symbiotic:   Option<Vec<(u64, f32)>>,
    pub phygital:    Option<Vec<(u64, f32)>>,
    pub archivist:   Option<Vec<(u64, f32)>>,
    /// Dynamic (GenericSpecialist) trends keyed by specialist name.
    /// Empty map when no dynamic specialists are configured.
    #[serde(default)]
    pub dynamic: std::collections::HashMap<String, Vec<(u64, f32)>>,
}

/// Snapshot of every configured specialist's learning state.
///
/// Returned by `Federation::learning_summary()`. Specialists not configured
/// in the federation appear as `None`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LearningSummary {
    pub visionary: Option<SpecialistLearningSummary>,
    pub omnipresent: Option<SpecialistLearningSummary>,
    pub symbiotic: Option<SpecialistLearningSummary>,
    pub phygital: Option<SpecialistLearningSummary>,
    pub archivist: Option<SpecialistLearningSummary>,
}

impl LearningSummary {
    /// Iterate over (specialist_name, summary) pairs for every present specialist.
    /// Useful for printing in tabular form.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &SpecialistLearningSummary)> {
        [
            ("Visionary", self.visionary.as_ref()),
            ("Omnipresent", self.omnipresent.as_ref()),
            ("Symbiotic", self.symbiotic.as_ref()),
            ("Phygital", self.phygital.as_ref()),
            ("Archivist", self.archivist.as_ref()),
        ]
        .into_iter()
        .filter_map(|(name, opt)| opt.map(|s| (name, s)))
    }

    /// Sum total_executions across all configured specialists
    pub fn total_executions(&self) -> u32 {
        self.iter().map(|(_, s)| s.total_executions).sum()
    }

    /// Sum success_count across all configured specialists
    pub fn total_successes(&self) -> u32 {
        self.iter().map(|(_, s)| s.success_count).sum()
    }
}

// Internal constructor used by the builder
impl Federation {
    pub(crate) fn from_parts(
        persistence: SharedPersistence,
        config: FederationConfig,
        visionary: Option<Arc<SpecialistHost<Visionary>>>,
        omnipresent: Option<Arc<SpecialistHost<Omnipresent>>>,
        symbiotic: Option<Arc<SpecialistHost<Symbiotic>>>,
        phygital: Option<Arc<SpecialistHost<Phygital>>>,
        archivist: Option<Arc<SpecialistHost<Archivist>>>,
    ) -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(256);
        let nats_config = crate::nats_client::NatsClientConfig::default();
        let nats = Arc::new(
            crate::nats_client::NatsPublisher::new(&nats_config.server_url)
        );
        Self {
            persistence,
            config,
            visionary,
            omnipresent,
            symbiotic,
            phygital,
            archivist,
            dynamic: Arc::new(RwLock::new(Vec::new())),
            active_intent: Arc::new(RwLock::new(None)),
            results: Arc::new(Mutex::new(Vec::new())),
            sentinel: Arc::new(RwLock::new(None)),
            sentinel_shutdown: Arc::new(tokio::sync::Notify::new()),
            sessions: Arc::new(RwLock::new(SessionManager::new())),
            multi_hive: Arc::new(RwLock::new(None)),
            nats,
            audit_log: Arc::new(Mutex::new(crate::federation::enterprise::AuditLog::new())),
            specialist_events: Arc::new(event_tx),
            federation_memory: Arc::new(Mutex::new(crate::federation::graph::EmbeddingStore::new(256))),
            scheduler: Arc::new(RwLock::new(crate::federation::hive::scheduler::AutonomousScheduler::new())),
            biology: Arc::new(RwLock::new(crate::SystemBiology::new())),
        }
    }
}

// Multi-hive federation management
impl Federation {
    /// Enable multi-hive coordination with the given cluster configuration.
    ///
    /// After this call, this hive can join a distributed cluster, participate
    /// in federated learning, and coordinate specialist work across multiple
    /// Aaroneous instances on the same network.
    pub async fn enable_multi_hive(
        &self,
        config: crate::federation::multi_hive::ClusterConfig,
    ) {
        use crate::federation::multi_hive::MultihiveFederation;
        let mh = MultihiveFederation::new(config);
        let node_id = mh.cluster_status().first().map(|(id, _)| id.clone()).unwrap_or_default();
        info!(
            "Multi-hive federation enabled: node_id={}",
            node_id
        );
        *self.multi_hive.write().await = Some(mh);
    }

    /// Get the multi-hive cluster status.
    pub async fn cluster_status(
        &self,
    ) -> Vec<(String, crate::federation::multi_hive::HiveNodeStatus)> {
        if let Some(mh) = self.multi_hive.read().await.as_ref() {
            mh.cluster_status()
        } else {
            vec![]
        }
    }

    /// Join a remote hive node to the cluster.
    pub async fn join_hive(
        &self,
        node: crate::federation::multi_hive::HiveNode,
    ) -> Result<(), String> {
        let mut guard = self.multi_hive.write().await;
        if let Some(mh) = guard.as_mut() {
            mh.join_hive(node)
        } else {
            Err("Multi-hive not enabled. Call enable_multi_hive() first.".to_string())
        }
    }

    /// Whether multi-hive federation is enabled.
    pub async fn has_multi_hive(&self) -> bool {
        self.multi_hive.read().await.is_some()
    }
}

// ─── Decision execution helper ───────────────────────────────────────────────

/// Execute one specialist decision and record the result.
///
/// This is the **canonical** execution path, shared by:
/// - `Federation::execute_decision()` (public, on-demand)
/// - The Sentinel arbitration loop (background task)
///
/// Callers pass the specialist Arcs they already hold, the shared state Arcs,
/// and the decision to execute.  Returns `true` if an `ExecutionResult` was
/// produced (success or failure).
#[allow(clippy::too_many_arguments)]
async fn run_decision(
    decision: crate::federation::specialist::Decision,
    vis: &Option<Arc<crate::federation::specialists::Visionary>>,
    omni: &Option<Arc<crate::federation::specialists::Omnipresent>>,
    symb: &Option<Arc<crate::federation::specialists::Symbiotic>>,
    phyg: &Option<Arc<crate::federation::specialists::Phygital>>,
    arch: &Option<Arc<crate::federation::specialists::Archivist>>,
    dyn_specialists: &[Arc<crate::federation::specialists::GenericSpecialist>],
    persistence: &SharedPersistence,
    event_tx: &Arc<tokio::sync::broadcast::Sender<serde_json::Value>>,
    results_store: &Arc<tokio::sync::Mutex<Vec<crate::federation::specialist::ExecutionResult>>>,
    active_intent_arc: &Arc<RwLock<Option<Intent>>>,
    sessions_arc: &Arc<RwLock<crate::federation::session::SessionManager>>,
    audit_log_arc: &Arc<tokio::sync::Mutex<crate::federation::enterprise::AuditLog>>,
    federation_memory: &Arc<tokio::sync::Mutex<crate::federation::graph::EmbeddingStore>>,
) -> bool {
    use crate::federation::specialist::{Specialist, SpecialistId};

    // Check if this decision was issued for a dynamic specialist by name
    let dynamic_specialist_name = decision.context.get("dynamic_specialist").cloned();

    let exec_result = if let Some(ref ds_name) = dynamic_specialist_name {
        // Route to the named dynamic specialist
        let ds = dyn_specialists.iter().find(|s| &s.name == ds_name);
        if let Some(s) = ds {
            s.execute(&decision).await.ok()
        } else {
            None
        }
    } else {
        match decision.specialist {
            SpecialistId::Visionary => {
                if let Some(s) = vis { s.execute(&decision).await.ok() } else { None }
            }
            SpecialistId::Omnipresent => {
                if let Some(s) = omni { s.execute(&decision).await.ok() } else { None }
            }
            SpecialistId::Symbiotic => {
                if let Some(s) = symb { s.execute(&decision).await.ok() } else { None }
            }
            SpecialistId::Phygital => {
                if let Some(s) = phyg { s.execute(&decision).await.ok() } else { None }
            }
            SpecialistId::Archivist => {
                if let Some(s) = arch { s.execute(&decision).await.ok() } else { None }
            }
            _ => None,
        }
    };

    if let Some(result) = exec_result {
        use crate::federation::enterprise::{AuditEvent, AuditLevel, AuditResult};

        info!(
            "Executed: {:?} '{}' → {:?}",
            result.specialist, decision.action, result.status
        );

        // Audit log
        {
            let audit_result = match result.status {
                crate::federation::specialist::ExecutionStatus::Success => AuditResult::Success,
                crate::federation::specialist::ExecutionStatus::Failed => AuditResult::Failure,
                _ => AuditResult::PartialSuccess,
            };
            let event = AuditEvent::new(
                format!("{:?}", result.specialist),
                format!("executed:{}", decision.action),
                AuditLevel::Info,
            )
            .with_resource(decision.proposal_id.clone())
            .with_result(audit_result)
            .with_details(result.output.chars().take(200).collect::<String>());
            let _ = audit_log_arc.lock().await.record(event);
        }

        // If Symbiotic executed a scale_intent_* action, apply the scaling
        // recommendation to the active intent.  Symbiotic emits a JSON object
        // with "action":"apply_scaling" and fields: delay_seconds,
        // max_duration_minutes, allow_interruption, adjusted_priority, reason,
        // defer.  We parse and apply them here so execute() stays &self.
        if result.specialist == crate::federation::specialist::SpecialistId::Symbiotic
            && decision.action.starts_with("scale_intent")
        {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&result.output) {
                if v.get("action").and_then(|a| a.as_str()) == Some("apply_scaling") {
                    use crate::federation::intent::{IntentPriority, IntentScaling, IntentStatus};
                    let mut intent_guard = active_intent_arc.write().await;
                    if let Some(intent) = intent_guard.as_mut() {
                        // Apply scaling object
                        intent.scaling = Some(IntentScaling {
                            delay_seconds: v.get("delay_seconds")
                                .and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                            max_duration_minutes: v.get("max_duration_minutes")
                                .and_then(|x| x.as_u64()).unwrap_or(30) as u32,
                            allow_interruption: v.get("allow_interruption")
                                .and_then(|x| x.as_bool()).unwrap_or(true),
                            reason: v.get("reason")
                                .and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        });
                        // Adjust priority
                        if let Some(p) = v.get("adjusted_priority").and_then(|x| x.as_str()) {
                            intent.priority = match p {
                                "Background" => IntentPriority::Background,
                                "High"       => IntentPriority::High,
                                "Critical"   => IntentPriority::Critical,
                                _            => IntentPriority::Normal,
                            };
                        }
                        // Defer if recovery mode
                        if v.get("defer").and_then(|x| x.as_bool()).unwrap_or(false) {
                            intent.status = IntentStatus::Deferred;
                        }
                        intent.version += 1;
                        info!(
                            "Symbiotic scaling applied to intent '{}' (v{}): priority={:?}, defer={}",
                            intent.id, intent.version, intent.priority,
                            matches!(intent.status, IntentStatus::Deferred)
                        );
                    }
                }
            }
        }

        // Route to originating session.
        // Read the session_id under the read guard, then release the guard
        // BEFORE acquiring the sessions write lock — avoids potential deadlock
        // when both locks are on the same async executor thread.
        let session_id_opt = {
            let intent_guard = active_intent_arc.read().await;
            intent_guard.as_ref()
                .and_then(|i| i.context.get("session_id").cloned())
            // intent_guard drops here, releasing the read lock
        };
        if let Some(session_id) = session_id_opt {
            let mut sessions = sessions_arc.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.add_result(result.clone());
            }
        }

        // Global ring buffer
        {
            let mut store = results_store.lock().await;
            store.push(result.clone());
            if store.len() > 100 {
                let excess = store.len() - 100;
                store.drain(0..excess);
            }
        }

        // Federation RAG memory — store intent+output pair for cross-sovereign recall
        {
            let intent_text = active_intent_arc.read().await
                .as_ref().map(|i| i.content.clone())
                .unwrap_or_else(|| decision.action.clone());
            let sovereign = result.specialist_name.as_deref()
                .unwrap_or_else(|| result.specialist.name());
            let memory_text = format!(
                "intent:{} | output:{}",
                intent_text.chars().take(120).collect::<String>(),
                result.output.chars().take(300).collect::<String>()
            );
            let memory_id = format!("fed-exec-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0));
            let mut mem = federation_memory.lock().await;
            mem.store_text(&memory_id, sovereign, &memory_text, "execution");
        }

        // Broadcast specialist state event for SSE / real-time clients
        {
            let specialist_display = result.specialist_name.as_deref()
                .unwrap_or_else(|| result.specialist.name());
            let intent = active_intent_arc.read().await
                .as_ref().map(|i| i.content.clone())
                .unwrap_or_default();
            let event = serde_json::json!({
                "type": "execution_complete",
                "specialist": specialist_display,
                "specialist_id": format!("{:?}", result.specialist),
                "action": decision.action,
                "status": format!("{:?}", result.status),
                "output_preview": result.output.chars().take(120).collect::<String>(),
                "duration_ms": result.duration_ms,
                "active_intent": intent,
                "timestamp_ms": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default().as_millis() as u64,
            });
            let _ = event_tx.send(event);
        }

        // Flush learning to SQLite immediately after execution so a crash
        // between 30s checkpoint ticks doesn't lose this learning update.
        // Uses spawn_blocking since save_learning_to() does a sync SQLite write.
        let pm_arc = persistence.clone();
        let specialist_id = result.specialist;
        let ds_name = result.specialist_name.clone();
        let vis_cp = vis.clone();
        let omni_cp = omni.clone();
        let symb_cp = symb.clone();
        let phyg_cp = phyg.clone();
        let arch_cp = arch.clone();
        let dyn_cp: Vec<_> = dyn_specialists.to_vec();

        tokio::spawn(async move {
            use crate::federation::specialist::SpecialistId;
            let pm = pm_arc.lock().await;

            macro_rules! save_if_some {
                ($opt:expr) => {
                    if let Some(ref s) = $opt {
                        if let Err(e) = s.save_learning_to(&pm) {
                            warn!("Post-execute checkpoint failed for {:?}: {}", specialist_id, e);
                        }
                    }
                }
            }

            match specialist_id {
                SpecialistId::Visionary   => save_if_some!(vis_cp),
                SpecialistId::Omnipresent => save_if_some!(omni_cp),
                SpecialistId::Symbiotic   => save_if_some!(symb_cp),
                SpecialistId::Phygital    => save_if_some!(phyg_cp),
                SpecialistId::Archivist   => save_if_some!(arch_cp),
                _ => {
                    if let Some(ref name) = ds_name {
                        if let Some(s) = dyn_cp.iter().find(|s| &s.name == name) {
                            if let Err(e) = s.save_learning_to(&*pm) {
                                warn!("GenericSpecialist '{}' post-exec checkpoint failed: {}", name, e);
                            }
                        }
                    }
                }
            }
        });

        true
    } else {
        false
    }
}

/// Read real system resources using sysinfo.
///
/// This is a **synchronous** function that must be called from a blocking
/// context (`spawn_blocking`) — never call it directly from async code.
/// The `collect_proposals()` method wraps this in `spawn_blocking`.
///
/// Returns a `SystemResources` with actual CPU utilization and available
/// memory. GPU and thermal remain placeholder (require NVML/Metal APIs).
fn read_system_resources_sync() -> crate::federation::specialist::SystemResources {
    use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind};

    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::new().with_cpu_usage())
            .with_memory(MemoryRefreshKind::new().with_ram()),
    );
    // Two refreshes are needed for an accurate CPU delta. Use a short sleep
    // here because we're already on a blocking thread (via spawn_blocking).
    std::thread::sleep(std::time::Duration::from_millis(30));
    sys.refresh_cpu();
    sys.refresh_memory();

    let cpu_used: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
        / sys.cpus().len().max(1) as f32;
    let cpu_available = (100.0 - cpu_used).clamp(0.0, 100.0);

    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let free_mem_mb = if total_mem > used_mem {
        ((total_mem - used_mem) / 1024 / 1024) as u64
    } else {
        512
    };

    // GPU utilization — real via NVML when available, falls back to 60% estimate
    let (gpu_available_percent, thermal_headroom) = read_gpu_resources();

    crate::federation::specialist::SystemResources {
        gpu_available_percent,
        cpu_available_percent: cpu_available,
        memory_available_mb: free_mem_mb as u32,
        thermal_headroom,
    }
}

/// Read GPU utilization and thermal headroom.
///
/// With `--features gpu-metrics`: uses NVML for NVIDIA GPUs.
/// Without the feature or if NVML fails: returns conservative estimates.
fn read_gpu_resources() -> (f32, f32) {
    #[cfg(feature = "gpu-metrics")]
    {
        match read_gpu_nvml() {
            Ok(result) => return result,
            Err(e) => tracing::debug!("NVML GPU read failed: {} — using estimate", e),
        }
    }
    // Fallback: conservative estimate — 60% available, 80% thermal headroom
    // Better than hardcoding 60% would be reading CPU thermal as a proxy,
    // but that requires platform-specific APIs not in sysinfo.
    (60.0, 0.8)
}

#[cfg(feature = "gpu-metrics")]
fn read_gpu_nvml() -> anyhow::Result<(f32, f32)> {
    use nvml_wrapper::Nvml;
    let nvml = Nvml::init()?;
    let device = nvml.device_by_index(0)?;

    let utilization = device.running_compute_processes()
        .map(|_| device.utilization_rates().ok())
        .ok()
        .flatten();

    let gpu_util = utilization.as_ref()
        .map(|u| u.gpu as f32)
        .unwrap_or(40.0); // 40% if can't read — conservative

    let gpu_available = (100.0 - gpu_util).clamp(0.0, 100.0);

    // Thermal: use GPU temperature vs threshold
    let temp_current = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
        .unwrap_or(50) as f32;
    let temp_slowdown = device.temperature_threshold(
        nvml_wrapper::enum_wrappers::device::TemperatureThreshold::Slowdown
    ).unwrap_or(85) as f32;
    let thermal = (1.0 - (temp_current / temp_slowdown)).clamp(0.0, 1.0);

    Ok((gpu_available, thermal))
}

/// Cached system resources to avoid 10 concurrent sysinfo reads when
/// multiple intents arrive simultaneously. The cache expires after 2 seconds.
static CACHED_RESOURCES: std::sync::OnceLock<
    tokio::sync::Mutex<(
        crate::federation::specialist::SystemResources,
        std::time::Instant,
    )>
> = std::sync::OnceLock::new();

/// Async wrapper: reads system resources, using a 2-second in-process cache
/// to prevent N concurrent intent submissions from each spawning a 30ms blocking read.
async fn read_system_resources() -> crate::federation::specialist::SystemResources {
    let (gpu_init, thermal_init) = read_gpu_resources();
    let cache = CACHED_RESOURCES.get_or_init(|| {
        tokio::sync::Mutex::new((
            crate::federation::specialist::SystemResources {
                gpu_available_percent: gpu_init,
                cpu_available_percent: 70.0,
                memory_available_mb: 2048,
                thermal_headroom: thermal_init,
            },
            // Set initial expiry to the past so the first call always refreshes
            std::time::Instant::now() - std::time::Duration::from_secs(10),
        ))
    });

    let mut guard = cache.lock().await;
    let age = guard.1.elapsed();

    if age < std::time::Duration::from_secs(2) {
        // Cache hit — return without hitting sysinfo
        return guard.0.clone();
    }

    // Cache miss — refresh
    let (gpu_fb, thermal_fb) = read_gpu_resources();
    let fresh = tokio::task::spawn_blocking(read_system_resources_sync)
        .await
        .unwrap_or_else(|_| crate::federation::specialist::SystemResources {
            gpu_available_percent: gpu_fb,
            cpu_available_percent: 70.0,
            memory_available_mb: 2048,
            thermal_headroom: thermal_fb,
        });

    *guard = (fresh.clone(), std::time::Instant::now());
    fresh
}

// Intent management methods
impl Federation {
    /// Submit a new user intent to the federation.
    ///
    /// Sets the active intent, then immediately runs one proposal collection
    /// cycle. Returns the intent ID for tracking.
    pub async fn submit_intent(&self, intent: Intent) -> String {
        use crate::federation::enterprise::{AuditEvent, AuditLevel, AuditResult};

        let id = intent.id.clone();
        info!("Federation: new intent submitted: '{}' ({})", intent.content, id);

        // Audit the intent submission
        {
            let event = AuditEvent::new(
                intent.context.get("user_id").cloned().unwrap_or_else(|| "anonymous".to_string()),
                format!("intent_submitted:{}", intent.content.chars().take(60).collect::<String>()),
                AuditLevel::Info,
            )
            .with_resource(id.clone())
            .with_result(AuditResult::Success)
            .with_details(format!("priority={:?}, tags={}", intent.priority, intent.tags.join(",")));
            let _ = self.audit_log.lock().await.record(event);
        }

        *self.active_intent.write().await = Some(intent.clone());

        // Broadcast intent submission so SSE clients know the hive is working on something
        self.broadcast_specialist_event(serde_json::json!({
            "type": "intent_submitted",
            "intent_id": id,
            "content": intent.content.chars().take(200).collect::<String>(),
            "priority": format!("{:?}", intent.priority),
            "timestamp_ms": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_millis() as u64,
        }));

        // RAG recall: query federation memory for relevant prior context.
        // This means when Argus audits code, the next security intent automatically
        // surfaces what Argus previously found — without requiring llama-gguf.
        // The recall results are stamped into the intent context so Odin and every
        // sovereign that receives the Decision can see prior relevant outputs.
        {
            let intent_content = {
                let guard = self.active_intent.read().await;
                guard.as_ref().map(|i| i.content.clone()).unwrap_or_default()
            };
            if !intent_content.is_empty() {
                let recall_text = {
                    let mem = self.federation_memory.lock().await;
                    mem.recall_for("federation", &intent_content, 3)
                };
                if !recall_text.is_empty() {
                    let mut guard = self.active_intent.write().await;
                    if let Some(ref mut intent) = *guard {
                        intent.context.insert("prior_context".to_string(), recall_text);
                        debug!("Federation memory: injected prior context into intent '{}'",
                               intent_content.chars().take(50).collect::<String>());
                    }
                }
            }
        }

        // Odin intercepts first: decompose the intent into subtasks before
        // the hive broadcasts to all sovereigns simultaneously.
        // Odin returns JSON: {"tasks":[{"content":"...","assign_to":"Merlin"},...]}
        // We stamp the decomposition into the active intent's context so:
        //   - collect_proposals() can route to specific sovereigns
        //   - The Guild panel can display the task breakdown
        //   - The SSE stream broadcasts the decomposition event
        self.odin_decompose_intent().await;

        // If Odin successfully decomposed the intent into a DAG, we can execute the DAG directly.
        let has_decomposition = {
            let guard = self.active_intent.read().await;
            guard.as_ref().and_then(|i| i.context.get("odin_decomposition").cloned())
        };

        if let Some(json_str) = has_decomposition {
            let dag_opt = crate::federation::graph::dag::task_dag_from_odin_output(&id, &json_str).ok();
            if let Some(dag) = dag_opt {
                // Execute the DAG layer by layer
                use crate::federation::specialist::Specialist;
                let layers = dag.parallel_layers();
                info!("Executing Odin DAG with {} parallel layers", layers.len());
                for (layer_idx, layer) in layers.into_iter().enumerate() {
                    info!("Executing DAG Layer {}", layer_idx);
                    let mut layer_futures: Vec<std::pin::Pin<Box<dyn std::future::Future<Output = (String, Result<crate::federation::specialist::ExecutionResult, crate::federation::specialist::SpecialistError>)> + Send>>> = Vec::new();
                    for node in layer {
                        if let crate::federation::graph::dag::NodeKind::Task { ref assigned_to, .. } = node.kind {
                            // Find the specialist
                            let dyn_guard = self.dynamic.read().await;
                            let specialist_opt = dyn_guard.iter().find(|s| s.name.to_lowercase() == assigned_to.to_lowercase()).cloned();
                            drop(dyn_guard);

                            if let Some(specialist) = specialist_opt {
                                let s_arc = specialist.clone();
                                let task_id = node.id.clone();
                                let task_label = node.label.clone();
                                layer_futures.push(Box::pin(async move {
                                    let mut ctx = std::collections::HashMap::new();
                                    ctx.insert("task_content".to_string(), task_label);

                                    let decision = crate::federation::specialist::Decision {
                                        proposal_id: format!("dag-task-{}", task_id),
                                        specialist: crate::federation::specialist::SpecialistId::custom(&s_arc.name),
                                        action: "execute_dag_task".to_string(),
                                        allocated_resources: crate::federation::specialist::ResourceRequest::default(),
                                        deadline_ms: 10000,
                                        context: ctx,
                                    };
                                    let result = s_arc.execute(&decision).await;
                                    (task_id, result)
                                }));
                            } else {
                                warn!("DAG execution: Assigned specialist {} not found for task {}", assigned_to, node.id);
                            }
                        }
                    }
                    // Wait for the layer to finish
                    let results = futures_util::future::join_all(layer_futures).await;
                    for (task_id, res) in results {
                        match res {
                            Ok(r) => info!("DAG Task {} completed: {}", task_id, r.output),
                            Err(e) => warn!("DAG Task {} failed: {}", task_id, e),
                        }
                    }
                }
            } else {
                // Trigger immediate proposal collection if parsing fails
                self.collect_proposals().await;
            }
        } else {
            // Trigger immediate proposal collection
            self.collect_proposals().await;
        }

        id
    }

    /// Call Odin (Guild coordinator) to decompose the active intent into subtasks
    /// before the hive broadcasts to all sovereigns.
    ///
    /// Odin runs asynchronously and his output is stamped into the active intent's
    /// context as JSON. If Odin is not loaded or times out, the pipeline continues
    /// unchanged — degrading gracefully to the flat broadcast.
    async fn odin_decompose_intent(&self) {
        use crate::federation::specialist::{Specialist, Decision, ResourceRequest};

        // Find Odin in the dynamic slot
        let odin = {
            let guard = self.dynamic.read().await;
            guard.iter().find(|s| s.name == "Odin").cloned()
        };

        let Some(odin) = odin else {
            // Odin not loaded — proceed without decomposition
            return;
        };

        let intent_content = {
            let guard = self.active_intent.read().await;
            guard.as_ref().map(|i| i.content.clone()).unwrap_or_default()
        };

        if intent_content.is_empty() { return; }

        // Give Odin a decomposition-specific decision context
        let mut ctx = std::collections::HashMap::new();
        ctx.insert("intent".to_string(), intent_content.clone());
        ctx.insert("task".to_string(), "decompose".to_string());
        ctx.insert("output_format".to_string(),
            r#"JSON: {"tasks":[{"content":"...","assign_to":"SovereignName","priority":"Normal"}]}"#
            .to_string());

        // Inject prior context from federation memory into Odin's decision
        // so it can factor past outcomes into its task decomposition
        {
            let guard = self.active_intent.read().await;
            if let Some(ref intent) = *guard {
                if let Some(prior) = intent.context.get("prior_context") {
                    ctx.insert("prior_context".to_string(), prior.clone());
                }
            }
        }

        // Inject available plugins
        let mut available_plugins = Vec::new();
        let sab_dir = crate::workspace::WorkspacePaths::workspace_root().join("sabs");
        if let Ok(entries) = std::fs::read_dir(sab_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    available_plugins.push(name.replace(".wasm", ""));
                }
            }
        }
        if !available_plugins.is_empty() {
            ctx.insert("available_plugins".to_string(), available_plugins.join(", "));
        }

        let decision = Decision {
            proposal_id: format!("odin-decompose-{}", uuid::Uuid::new_v4()),
            specialist: crate::federation::specialist::SpecialistId::custom("Odin"),
            action: "decompose_intent".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: ctx,
        };

        // Run Odin with a 4-second timeout — don't block the pipeline
        let decomp_result = tokio::time::timeout(
            std::time::Duration::from_secs(4),
            odin.execute(&decision),
        ).await;

        match decomp_result {
            Ok(Ok(result)) => {
                // Try to parse Odin's JSON decomposition
                if let Some(json_start) = result.output.find('{') {
                    let json_str = &result.output[json_start..];
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                        // Stamp decomposition into active intent context
                        let mut guard = self.active_intent.write().await;
                        if let Some(intent) = guard.as_mut() {
                            intent.context.insert(
                                "odin_decomposition".to_string(),
                                serde_json::to_string(&v).unwrap_or_default(),
                            );

                            // Extract the assign_to fields so collect_proposals()
                            // can route only to named sovereigns, reducing noise.
                            if let Some(tasks) = v.get("tasks").and_then(|t| t.as_array()) {
                                let assigned: Vec<String> = tasks.iter()
                                    .filter_map(|t| t.get("assign_to")
                                        .and_then(|a| a.as_str())
                                        .map(|s| s.to_string()))
                                    .collect();
                                if !assigned.is_empty() {
                                    intent.context.insert(
                                        "odin_assigned_to".to_string(),
                                        assigned.join(","),
                                    );
                                    info!("Odin assigned tasks to: {}", assigned.join(", "));
                                }
                            }
                        }
                        drop(guard);

                        // Broadcast so Guild panel and SSE consumers see the tasks
                        self.broadcast_specialist_event(serde_json::json!({
                            "type": "guild_decomposition",
                            "specialist": "Odin",
                            "intent": intent_content,
                            "decomposition": v,
                            "timestamp_ms": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default().as_millis() as u64,
                        }));

                        info!("Odin decomposed intent: {}", result.output.chars().take(100).collect::<String>());
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("Odin decomposition failed: {}", e);
            }
            Err(_) => {
                warn!("Odin decomposition timed out — proceeding with flat broadcast");
            }
        }
    }

    /// Submit an intent associated with a specific session.
    ///
    /// The intent is recorded on the session's history, tagged with the
    /// session and user IDs, and forwarded to the federation pipeline.
    /// Returns `(session_id, intent_id)`.
    pub async fn submit_intent_for_session(
        &self,
        session_id: &str,
        intent: Intent,
    ) -> Result<(String, String), String> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{}' not found", session_id))?;
        let added = session.add_intent(intent);
        let intent_id = added.id.clone();
        let full_intent = added.clone();
        drop(sessions);

        // Route through the federation pipeline
        *self.active_intent.write().await = Some(full_intent);
        self.collect_proposals().await;

        // Persist updated session (new intent added)
        self.persist_session(session_id).await;

        Ok((session_id.to_string(), intent_id))
    }

    /// Create a new session. Returns the session ID.
    /// The session is immediately persisted to SQLite so it survives restarts.
    pub async fn create_session(
        &self,
        user_name: impl Into<String>,
        device_id: Option<&str>,
    ) -> String {
        let id = self.sessions
            .write()
            .await
            .create_session(user_name, device_id);
        self.persist_session(&id).await;
        id
    }

    /// Serialize a session to JSON and upsert it in the `federation_sessions`
    /// table.  Best-effort: logs a warning on failure rather than propagating.
    async fn persist_session(&self, session_id: &str) {
        let snapshot = {
            let sessions = self.sessions.read().await;
            sessions.get(session_id).cloned()
        };
        let Some(session) = snapshot else { return };
        let Ok(json) = serde_json::to_string(&session) else { return };
        let state_str = format!("{:?}", session.state);
        let created = session.started_at as i64;
        let pm = self.persistence.lock().await;
        if let Err(e) = pm.save_session(&session.id, &session.user_id, &session.user_name, &state_str, &json, created) {
            warn!("persist_session({}): {}", session_id, e);
        }
    }

    /// After `load_sessions_from_db()`, find the most recently active session's
    /// latest pending intent and restore it as the `active_intent`.
    ///
    /// This ensures specialists resume proposing on in-flight work after a
    /// restart — without this, `active_intent` stays `None` until a new intent
    /// arrives, leaving all specialists in idle mode.
    async fn restore_active_intent_from_sessions(&self) {
        use crate::federation::intent::IntentStatus;

        let sessions = self.sessions.read().await;
        // Find the session most recently active (highest last_active timestamp)
        let best_session = sessions
            .active_sessions()
            .into_iter()
            .max_by_key(|s| s.last_active);

        let Some(session) = best_session else { return };

        // Find the most recent non-completed intent in that session
        let pending_intent = session.intents.iter().rev().find(|i| {
            !matches!(
                i.status,
                IntentStatus::Completed | IntentStatus::Cancelled | IntentStatus::Failed | IntentStatus::Superseded
            )
        });

        if let Some(intent) = pending_intent {
            let mut intent = intent.clone();
            // Tag it as restored so audit trail is clear
            intent.context.insert("restored_on_restart".to_string(), "true".to_string());
            info!(
                "Restored active intent '{}' from session '{}' on restart",
                intent.content, session.id
            );
            *self.active_intent.write().await = Some(intent);
        }
    }

    /// Load all non-expired sessions from the database into SessionManager.
    /// Called during `start_all()` so sessions survive process restarts.
    async fn load_sessions_from_db(&self) {
        let rows = {
            let pm = self.persistence.lock().await;
            pm.load_active_sessions().unwrap_or_default()
        };
        if rows.is_empty() { return; }
        let mut mgr = self.sessions.write().await;
        for (session_id, json) in &rows {
            if mgr.get(session_id).is_some() { continue; } // already in memory
            match serde_json::from_str::<crate::federation::session::Session>(json) {
                Ok(session) => {
                    mgr.insert_session(session);
                }
                Err(e) => {
                    warn!("load_sessions_from_db: failed to deserialise session {}: {}", session_id, e);
                }
            }
        }
        info!("Loaded {} session(s) from database", rows.len());
    }

    /// Get a snapshot of a session's state (clone for HTTP serving).
    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> Option<crate::federation::session::Session> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// List all active sessions.
    pub async fn active_sessions(&self) -> Vec<crate::federation::session::Session> {
        self.sessions.read().await.active_sessions().into_iter().cloned().collect()
    }

    /// Delete a session by ID. Returns `true` if the session existed and was removed.
    /// Marks it Ended in memory and removes it from the database.
    pub async fn delete_session(&self, session_id: &str) -> bool {
        use crate::federation::session::SessionState;
        let existed = {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.state = SessionState::Ended;
                true
            } else {
                false
            }
        };
        if existed {
            let pm = self.persistence.lock().await;
            let _ = pm.delete_session(session_id);
        }
        existed
    }

    /// Tick all sessions: advance idle/expiry state. Purge expired sessions.
    /// Call periodically (e.g., once per minute) from a background task.
    pub async fn tick_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.tick();
        sessions.purge_expired();
    }

    /// Collect proposals from all configured specialists and submit them to
    /// the CommunicationBus for Sentinel arbitration.
    ///
    /// Builds a `SpecialistContext` from the current system state and intent,
    /// calls `propose()` on each specialist in parallel, converts
    /// `ProposedAction` → `Proposal`, and submits each to the bus.
    ///
    /// The Sentinel arbitration loop will pick them up on its next tick
    /// (default: 500ms) and issue decisions to the winners.
    pub async fn collect_proposals(&self) {
        use crate::federation::specialist::{Specialist, SpecialistContext, UserState};
        use crate::federation::proposal::Proposal;

        // Build context from active intent
        let intent = self.active_intent.read().await.clone();
        let intent_activity = intent.as_ref()
            .map(|i| i.content.clone())
            .unwrap_or_else(|| "idle".to_string());

        let context = SpecialistContext {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            user_state: {
                // Read live biometric state from Symbiotic's drain_state when
                // available.  Falls back to neutral defaults when Symbiotic is
                // not configured or hasn't received any BLE samples yet.
                let (stress, focus, fatigue) = self.symbiotic
                    .as_ref()
                    .map(|h| {
                        let s = h.specialist().shared_current_state();
                        (s.stress_level, s.focus_depth, s.fatigue_level)
                    })
                    .unwrap_or((0.3, 0.7, 0.2));
                UserState {
                    stress_level: stress,
                    focus_level: focus,
                    fatigue_level: fatigue,
                    activity: intent_activity.clone(),
                }
            },
            system_resources: {
                // Read real system resources, then cap them to the profile limits.
                let real = read_system_resources().await;
                let caps = self.config.resource_caps();
                crate::federation::specialist::SystemResources {
                    gpu_available_percent: real.gpu_available_percent.min(caps.gpu_available_percent),
                    cpu_available_percent: real.cpu_available_percent.min(caps.cpu_available_percent),
                    memory_available_mb: real.memory_available_mb.min(caps.memory_available_mb),
                    thermal_headroom: real.thermal_headroom.min(caps.thermal_headroom),
                }
            },
            active_specialists: vec![
                crate::federation::specialist::SpecialistId::Visionary,
                crate::federation::specialist::SpecialistId::Omnipresent,
                crate::federation::specialist::SpecialistId::Symbiotic,
                crate::federation::specialist::SpecialistId::Phygital,
                crate::federation::specialist::SpecialistId::Archivist,
            ],
            recent_decisions: vec![],
        };

        // ── Parallel proposal collection ─────────────────────────────────────
        // Previously sequential: each propose().await blocked the next.
        // Now: all specialists propose concurrently, collecting results as they
        // arrive. With 9 sovereigns this cuts collection latency by ~8×.

        // Collect all proposal futures — core specialists + dynamic
        type ProposalResult = (
            Result<Vec<crate::federation::specialist::ProposedAction>,
                   crate::federation::specialist::SpecialistError>,
            String,  // specialist name (for tagging dynamic proposals)
            bool,    // is_dynamic
        );

        let mut proposal_futures: Vec<std::pin::Pin<Box<dyn std::future::Future<
            Output = ProposalResult> + Send>>> = Vec::new();

        macro_rules! push_core_future {
            ($host_opt:expr, $name:literal) => {
                if let Some(h) = &$host_opt {
                    let specialist_arc = h.specialist();
                    let ctx = context.clone();
                    proposal_futures.push(Box::pin(async move {
                        (specialist_arc.propose(&ctx).await, $name.to_string(), false)
                    }));
                }
            };
        }

        push_core_future!(self.visionary,   "Visionary");
        push_core_future!(self.omnipresent, "Omnipresent");
        push_core_future!(self.symbiotic,   "Symbiotic");
        push_core_future!(self.phygital,    "Phygital");
        push_core_future!(self.archivist,   "Archivist");

        // Dynamic specialists — if Odin has assigned tasks, only invite the named
        // sovereigns to propose. This prevents design/biometric outputs from polluting
        // code/security/research intents.
        // If Odin has not decomposed (no odin_assigned_to in context), all propose.
        let odin_assigned: Option<std::collections::HashSet<String>> = {
            let guard = self.active_intent.read().await;
            guard.as_ref()
                .and_then(|i| i.context.get("odin_assigned_to"))
                .map(|s| s.split(',').map(|n| n.trim().to_string()).collect())
        };

        // Update metabolism before collecting proposals
        {
            let mut bio = self.biology.write().await;
            bio.update_metabolism();
        }

        {
            let dyn_guard = self.dynamic.read().await;
            for specialist in dyn_guard.iter() {
                // When Odin has routed, only include assigned sovereigns
                if let Some(ref assigned) = odin_assigned {
                    if !assigned.contains(&specialist.name) {
                        debug!("Odin routing: skipping {} (not in assign_to list)", specialist.name);
                        continue;
                    }
                }
                let s_arc = specialist.clone();
                let ctx = context.clone();
                let name = specialist.name.clone();
                proposal_futures.push(Box::pin(async move {
                    (s_arc.propose(&ctx).await, name, true)
                }));
            }
        }

        // Run all proposals concurrently
        let all_results = futures_util::future::join_all(proposal_futures).await;

        let mut bus_proposals: Vec<Proposal> = vec![];
        for (result, specialist_name, is_dynamic) in all_results {
            match result {
                Ok(proposed_actions) => {
                    for action in proposed_actions {
                        let mut proposal = Proposal::new(
                            action.specialist,
                            action.action_type.clone(),
                            action.description.clone(),
                            action.confidence,
                            action.priority.clone(),
                        )
                        .with_resources(action.required_resources.clone())
                        .with_tags(action.tags.clone());
                        if !intent_activity.is_empty() && intent_activity != "idle" {
                            proposal = proposal.with_metadata("intent", intent_activity.clone());
                            if is_dynamic {
                                proposal = proposal.with_metadata("dynamic_specialist",
                                                                   specialist_name.clone());
                            }
                        }
                        bus_proposals.push(proposal);
                    }
                }
                Err(e) => {
                    warn!("collect_proposals: '{}' propose() error: {}", specialist_name, e);
                }
            }
        }

        let total_proposals = bus_proposals.len();

        // Submit to the bus via Sentinel (holds lock briefly)
        let mut sentinel_guard = self.sentinel.write().await;
        if let Some(sentinel) = sentinel_guard.as_mut() {
            for proposal in bus_proposals {
                let _ = sentinel.communication_bus.submit_proposal(proposal);
            }
        }
        drop(sentinel_guard);

        if total_proposals > 0 {
            info!("collect_proposals: {} proposals submitted to Sentinel bus", total_proposals);
        }
    }

    /// Execute a decision issued by Sentinel.
    ///
    /// Routes the decision to the correct specialist's `execute()` method,
    /// records the result in the audit log, routes it to the originating
    /// session, and stores it in the global result ring buffer.
    ///
    /// This is the public on-demand path. The Sentinel arbitration loop uses
    /// the same underlying `run_decision()` free function so both paths are
    /// identical in behavior.
    pub async fn execute_decision(
        &self,
        decision: crate::federation::specialist::Decision,
    ) {
        let vis = self.visionary.as_ref().map(|h| h.specialist());
        let omni = self.omnipresent.as_ref().map(|h| h.specialist());
        let symb = self.symbiotic.as_ref().map(|h| h.specialist());
        let phyg = self.phygital.as_ref().map(|h| h.specialist());
        let arch = self.archivist.as_ref().map(|h| h.specialist());
        let dyn_vec = self.dynamic.read().await.clone();

        run_decision(
            decision,
            &vis,
            &omni,
            &symb,
            &phyg,
            &arch,
            &dyn_vec,
            &self.persistence,
            &self.specialist_events,
            &self.results,
            &self.active_intent,
            &self.sessions,
            &self.audit_log,
            &self.federation_memory,
        )
        .await;
    }

    /// Get the currently active intent, if any.
    pub async fn current_intent(&self) -> Option<Intent> {
        self.active_intent.read().await.clone()
    }

    /// Record an execution result. Called by specialists after executing.
    /// Results are accessible via `GET /results` on the HTTP status server.
    pub async fn record_result(&self, result: crate::federation::specialist::ExecutionResult) {
        use crate::federation::enterprise::{AuditEvent, AuditLevel, AuditResult};

        // Audit the execution result
        {
            let audit_result = match result.status {
                crate::federation::specialist::ExecutionStatus::Success => AuditResult::Success,
                crate::federation::specialist::ExecutionStatus::Failed => AuditResult::Failure,
                _ => AuditResult::PartialSuccess,
            };
            let event = AuditEvent::new(
                format!("{:?}", result.specialist),
                format!("specialist_executed:{}", result.proposal_id),
                AuditLevel::Info,
            )
            .with_resource(result.proposal_id.clone())
            .with_result(audit_result)
            .with_details(result.output.chars().take(200).collect::<String>());
            let _ = self.audit_log.lock().await.record(event);
        }

        let mut results = self.results.lock().await;
        results.push(result);
        // Keep last 100 results
        if results.len() > 100 {
            let excess = results.len() - 100;
            results.drain(0..excess);
        }
    }

    /// Get recent audit events. Useful for the `/status/audit` endpoint.
    pub async fn recent_audit_events(&self, limit: usize) -> Vec<crate::federation::enterprise::AuditEvent> {
        let log = self.audit_log.lock().await;
        log.query(&crate::federation::enterprise::AuditQuery {
            user_id: None,
            action: None,
            level: None,
            result: None,
            start_time_ms: None,
            end_time_ms: None,
            limit,
        })
    }

    /// Query audit events with full filter support.
    ///
    /// - `limit`: max events to return (default 50, max 1000)
    /// - `since_ms`: only return events with `timestamp_ms >= since_ms`
    /// - `until_ms`: only return events with `timestamp_ms <= until_ms`
    /// - `user_id`: filter by user identity
    /// - `action_contains`: filter where action contains this substring
    pub async fn query_audit_events(
        &self,
        limit: usize,
        since_ms: Option<u64>,
        until_ms: Option<u64>,
        user_id: Option<String>,
    ) -> Vec<crate::federation::enterprise::AuditEvent> {
        let limit = limit.min(1000); // hard cap
        let log = self.audit_log.lock().await;
        log.query(&crate::federation::enterprise::AuditQuery {
            user_id,
            action: None,
            level: None,
            result: None,
            start_time_ms: since_ms,
            end_time_ms: until_ms,
            limit,
        })
    }

    /// Get recent execution results (last N, newest first).
    pub async fn recent_results(
        &self,
        limit: usize,
    ) -> Vec<crate::federation::specialist::ExecutionResult> {
        let results = self.results.lock().await;
        results.iter().rev().take(limit).cloned().collect()
    }

    /// Start the Sentinel arbitration loop in the background.
    ///
    /// Creates a `CommunicationBus`, registers all configured specialists,
    /// creates a `Sentinel`, and spawns a tokio task that calls
    /// `sentinel.arbitrate()` every `config.proposal_review_interval_ms`ms.
    pub async fn spawn_sentinel_loop(&self, interval: Duration) {
        use crate::federation::communication::CommunicationBus;
        use crate::federation::sentinel::{Sentinel, SentinelConfig};
        use crate::federation::specialist::SpecialistId;

        let mut bus = CommunicationBus::new();

        // Register all configured specialists
        if self.visionary.is_some() { bus.register_specialist(SpecialistId::Visionary); }
        if self.omnipresent.is_some() { bus.register_specialist(SpecialistId::Omnipresent); }
        if self.symbiotic.is_some() { bus.register_specialist(SpecialistId::Symbiotic); }
        if self.phygital.is_some() { bus.register_specialist(SpecialistId::Phygital); }
        if self.archivist.is_some() { bus.register_specialist(SpecialistId::Archivist); }

        let config = SentinelConfig::default();
        let sentinel = Sentinel::new(config, bus);

        // Seed Sentinel with real system resources immediately so the first
        // arbitration tick doesn't filter every proposal as not viable.
        let initial_resources = read_system_resources().await;
        sentinel.update_system_resources(initial_resources).await;

        info!(
            "Sentinel started with {} registered specialist(s), arbitrating every {}ms",
            sentinel.communication_bus.specialist_count(),
            interval.as_millis()
        );

        *self.sentinel.write().await = Some(sentinel);

        // Spawn the arbitration loop
        let sentinel_arc = self.sentinel.clone();
        let shutdown = self.sentinel_shutdown.clone();

        // We need to execute decisions after arbitration; collect them from
        // each specialist's channel and call execute_decision(). We store the
        // hosts we need as individual Arcs so the task can be 'static + Send.
        let vis = self.visionary.as_ref().map(|h| h.specialist());
        let omni = self.omnipresent.as_ref().map(|h| h.specialist());
        let symb = self.symbiotic.as_ref().map(|h| h.specialist());
        let phyg = self.phygital.as_ref().map(|h| h.specialist());
        let arch = self.archivist.as_ref().map(|h| h.specialist());
        let dynamic_arc = self.dynamic.clone();
        let persistence_arc = self.persistence.clone();
        let event_tx_arc = self.specialist_events.clone();
        let results_store = self.results.clone();
        // Route results back to the originating session
        let active_intent_arc = self.active_intent.clone();
        let sessions_arc = self.sessions.clone();
        // Audit log — record each execution from the sentinel loop
        let audit_log_arc = self.audit_log.clone();
        // Federation RAG memory
        let fed_memory_arc = self.federation_memory.clone();

        tokio::spawn(async move {
            use crate::federation::specialist::SpecialistId;

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        info!("Sentinel arbitration loop received shutdown signal");
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        // Refresh system resources before each arbitration tick
                        // so the viability filter uses current CPU/memory headroom.
                        let fresh_resources = read_system_resources().await;
                        {
                            let guard = sentinel_arc.read().await;
                            if let Some(sentinel) = guard.as_ref() {
                                sentinel.update_system_resources(fresh_resources).await;
                            }
                        }

                        // Run one arbitration cycle
                        let arb_result = {
                            let guard = sentinel_arc.read().await;
                            if let Some(sentinel) = guard.as_ref() {
                                sentinel.arbitrate().await.ok()
                            } else { None }
                        };

                        if let Some(result) = arb_result {
                            if result.decisions_issued > 0 {
                                info!(
                                    "Sentinel: {} proposals → {} decisions",
                                    result.proposals_reviewed, result.decisions_issued
                                );
                            }
                        }

                        // Drain each specialist's decision channel and execute
                        // decisions issued by the Sentinel.
                        let channels: Vec<(SpecialistId, Arc<crate::federation::communication::MessageChannel>)> = {
                            let guard = sentinel_arc.read().await;
                            if let Some(sentinel) = guard.as_ref() {
                                let mut ch = vec![];
                                for id in [
                                    SpecialistId::Visionary, SpecialistId::Omnipresent,
                                    SpecialistId::Symbiotic, SpecialistId::Phygital,
                                    SpecialistId::Archivist,
                                ] {
                                    if let Some(channel) = sentinel.communication_bus.specialist_channel(&id) {
                                        ch.push((id, Arc::new(channel)));
                                    }
                                }
                                ch
                            } else { vec![] }
                        };

                        for (_specialist_id, channel) in channels {
                            // Non-blocking drain: try_receive until empty
                            loop {
                                let msg: Option<crate::federation::communication::SpecialistMessage> = crate::federation::communication::MessageChannel::try_receive(&channel).await;

                                match msg {
                                    Some(crate::federation::communication::SpecialistMessage::DecisionIssued(decision)) => {
                                        // Delegate to the canonical run_decision() function
                                        // so both the sentinel loop and execute_decision()
                                        // have identical audit/session/storage behavior.
                                        let dyn_vec = dynamic_arc.read().await.clone();
                                        run_decision(
                                            decision,
                                            &vis,
                                            &omni,
                                            &symb,
                                            &phyg,
                                            &arch,
                                            &dyn_vec,
                                            &persistence_arc,
                                            &event_tx_arc,
                                            &results_store,
                                            &active_intent_arc,
                                            &sessions_arc,
                                            &audit_log_arc,
                                            &fed_memory_arc,
                                        ).await;
                                    }
                                    Some(_) => {} // Other message types ignored for now
                                    None => break, // Channel empty, stop draining
                                }
                            }
                        }

                        // Clear processed proposals before next tick
                        {
                            let mut guard = sentinel_arc.write().await;
                            if let Some(sentinel) = guard.as_mut() {
                                sentinel.communication_bus.clear_proposals().await;
                            }
                        }
                    }
                }
            }
        });
    }

    /// Stop the Sentinel arbitration loop.
    pub fn stop_sentinel_loop(&self) {
        self.sentinel_shutdown.notify_one();
    }

    /// Spawn a system sensor loop that reads CPU/memory from sysinfo every 5 seconds
    /// and synthesizes `BiometricSample` entries for Symbiotic's bio_inbox.
    ///
    /// This bridges the gap between the sensor_node enzyme (which reads CPU data
    /// into a shared memory buffer inaccessible from `aaroneous`) and Symbiotic's
    /// biometric classification pipeline.
    ///
    /// Mapping:
    /// - CPU utilization → synthetic heart rate (60–120 BPM; 70 + cpu_pct*0.5)
    /// - Free memory < 20% → battery level 20 (stress signal)
    ///
    /// The loop stops when `stop_sentinel_loop()` is called (shared shutdown notify).
    pub async fn spawn_system_sensor_loop(&self) {
        let Some(symb_host) = &self.symbiotic else { return };
        let symb = symb_host.specialist();
        let shutdown = self.sentinel_shutdown.clone();

        tokio::spawn(async move {
            use crate::federation::biometric::{BiometricKind, BiometricSample};
            use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind};

            loop {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                        // Run sysinfo on blocking thread — it calls sleep(30ms) internally
                        let (cpu_pct, mem_free_pct) = tokio::task::spawn_blocking(|| {
                            let mut sys = System::new_with_specifics(
                                RefreshKind::new()
                                    .with_cpu(CpuRefreshKind::new().with_cpu_usage())
                                    .with_memory(MemoryRefreshKind::new().with_ram()),
                            );
                            std::thread::sleep(std::time::Duration::from_millis(30));
                            sys.refresh_cpu();
                            sys.refresh_memory();

                            let cpu: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
                                / sys.cpus().len().max(1) as f32;
                            let total = sys.total_memory();
                            let free_pct = if total > 0 {
                                (sys.available_memory() as f32 / total as f32) * 100.0
                            } else { 50.0 };
                            (cpu, free_pct)
                        }).await.unwrap_or((50.0, 50.0));

                        // Map CPU % to synthetic BPM (60–120 range)
                        // High CPU (100%) → 120 BPM, idle (0%) → 60 BPM
                        let synthetic_bpm = (60.0 + cpu_pct * 0.6).clamp(60.0, 120.0) as u16;

                        let hr_sample = BiometricSample {
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default().as_secs(),
                            device_id: "system-sensor".to_string(),
                            kind: BiometricKind::HeartRate,
                            value: synthetic_bpm as f32,
                            raw_payload: Vec::new(),
                        };

                        // Push to Symbiotic's bio_inbox (non-blocking)
                        {
                            let mut inbox = symb.bio_inbox.lock();
                            inbox.push_back(hr_sample);
                            // Keep inbox bounded
                            if inbox.len() > 50 {
                                inbox.pop_front();
                            }
                        }

                        // Also push a battery-level sample as a memory-pressure proxy
                        let mem_sample = BiometricSample {
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default().as_secs(),
                            device_id: "system-sensor".to_string(),
                            kind: BiometricKind::BatteryLevel,
                            value: mem_free_pct as f32,
                            raw_payload: Vec::new(),
                        };
                        {
                            let mut inbox = symb.bio_inbox.lock();
                            inbox.push_back(mem_sample);
                            if inbox.len() > 50 {
                                inbox.pop_front();
                            }
                        }

                        tracing::debug!(
                            "SystemSensor: CPU={:.1}% → {}BPM synthetic, mem_free={:.1}%",
                            cpu_pct, synthetic_bpm, mem_free_pct
                        );
                    }
                }
            }
        });
    }

    /// Spawn a background task that calls `tick_sessions()` every 60 seconds,
    /// advancing idle/expiry state and purging expired sessions.
    ///
    /// The task shares the sentinel shutdown signal so it stops cleanly when
    /// `stop_sentinel_loop()` is called.
    pub async fn spawn_session_tick_loop(&self) {
        let sessions = self.sessions.clone();
        let shutdown = self.sentinel_shutdown.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                        let mut mgr = sessions.write().await;
                        mgr.tick();
                        let purged = mgr.purge_expired();
                        if purged > 0 {
                            info!("Session tick: purged {} expired session(s)", purged);
                        }
                    }
                }
            }
        });
    }

    /// Spawn the Autonomous Task Scheduler loop that checks for time-based triggers.
    pub async fn spawn_autonomous_scheduler_loop(&self) {
        let scheduler_arc = self.scheduler.clone();
        let shutdown = self.sentinel_shutdown.clone();
        let nc = self.nats.clone();

        tokio::spawn(async move {
            loop {

                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default().as_millis() as u64;

                        let mut triggered_tasks = Vec::new();
                        {
                            let mut sched = scheduler_arc.write().await;
                            for (_, task) in sched.tasks.iter_mut() {
                                if task.status == "Scheduled" {
                                    if let Some(interval) = task.interval_secs {
                                        if now_ms >= task.last_run_ms + (interval * 1000) {
                                            task.last_run_ms = now_ms;
                                            triggered_tasks.push(task.clone());
                            }
                        }
                                }
                            }
                        }

                        for task in triggered_tasks {
                            info!("Scheduler: Triggering task '{}'", task.name);
                            // We can use the HTTP API or NATS to trigger an intent.
                            // Using NATS directly to the UI backend, or just send a message.
                            let payload = serde_json::json!({
                                "content": task.intent_content,
                                "priority": "High"
                            });
                            // In a real setup, we'd have a specific NATS subject for system intents
                            let payload_str = serde_json::to_string(&payload).unwrap();
                            let _ = nc.publish("system/intent/submit", payload_str.as_bytes());
                        }
                    }
                }
            }
        });
    }
}
