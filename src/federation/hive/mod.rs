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
use std::sync::Arc;
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
        }
    }
}
