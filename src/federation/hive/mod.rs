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

#[cfg(test)]
mod tests;

pub use builder::FederationBuilder;
pub use config::FederationConfig;

use crate::federation::host::{HostError, SharedPersistence, SpecialistHost};
use crate::federation::specialists::{Archivist, Omnipresent, Phygital, Symbiotic, Visionary};
use crate::federation::intent::Intent;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

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

    /// How many specialists are configured in this federation (1..=5)
    pub fn enabled_count(&self) -> usize {
        let mut n = 0;
        if self.visionary.is_some() { n += 1; }
        if self.omnipresent.is_some() { n += 1; }
        if self.symbiotic.is_some() { n += 1; }
        if self.phygital.is_some() { n += 1; }
        if self.archivist.is_some() { n += 1; }
        n
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
        Self {
            persistence,
            config,
            visionary,
            omnipresent,
            symbiotic,
            phygital,
            archivist,
            active_intent: Arc::new(RwLock::new(None)),
            results: Arc::new(Mutex::new(Vec::new())),
            sentinel: Arc::new(RwLock::new(None)),
            sentinel_shutdown: Arc::new(tokio::sync::Notify::new()),
        }
    }
}

// Intent management methods
impl Federation {
    /// Submit a new user intent to the federation.
    ///
    /// This sets the active intent and notifies all specialists (via the
    /// future Sentinel arbitration loop) that there is work to do.
    /// Returns the intent ID for tracking.
    pub async fn submit_intent(&self, intent: Intent) -> String {
        let id = intent.id.clone();
        info!("Federation: new intent submitted: '{}' ({})", intent.content, id);
        *self.active_intent.write().await = Some(intent);
        id
    }

    /// Get the currently active intent, if any.
    pub async fn current_intent(&self) -> Option<Intent> {
        self.active_intent.read().await.clone()
    }

    /// Record an execution result. Called by specialists after executing.
    /// Results are accessible via `GET /results` on the HTTP status server.
    pub async fn record_result(&self, result: crate::federation::specialist::ExecutionResult) {
        let mut results = self.results.lock().await;
        results.push(result);
        // Keep last 100 results
        if results.len() > 100 {
            let excess = results.len() - 100;
            results.drain(0..excess);
        }
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

        info!(
            "Sentinel started with {} registered specialist(s), arbitrating every {}ms",
            sentinel.communication_bus.specialist_count(),
            interval.as_millis()
        );

        *self.sentinel.write().await = Some(sentinel);

        // Spawn the arbitration loop
        let sentinel_arc = self.sentinel.clone();
        let shutdown = self.sentinel_shutdown.clone();
        let active_intent = self.active_intent.clone();
        let results_store = self.results.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        info!("Sentinel arbitration loop received shutdown signal");
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        // Run one arbitration cycle
                        let guard = sentinel_arc.read().await;
                        if let Some(sentinel) = guard.as_ref() {
                            match sentinel.arbitrate().await {
                                Ok(result) => {
                                    if result.decisions_issued > 0 {
                                        info!(
                                            "Sentinel: {} proposals reviewed, {} decisions issued",
                                            result.proposals_reviewed, result.decisions_issued
                                        );
                                    }
                                }
                                Err(e) => {
                                    warn!("Sentinel arbitration error: {}", e);
                                }
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
}
