/// Fluent builder for `Federation`.
///
/// The builder lets callers compose any subset of the 5 specialists with
/// minimum ceremony. Each `with_*` method takes ownership of `self` and
/// returns it, enabling chaining:
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use a_run::federation::hive::Federation;
/// use a_run::persistence::PersistenceManager;
///
/// let pm = PersistenceManager::new("hive.db")?;
/// let fed = Federation::builder(pm)
///     .with_visionary()
///     .with_archivist()
///     .build();
/// # Ok(())
/// # }
/// ```
///
/// # Default behavior
///
/// - No specialists are configured by default. Each must be opted in.
/// - The federation-level config has default 30s checkpoint cadence.
/// - All hosts inherit the federation-level checkpoint cadence unless
///   overridden via `with_*_host_config`.

use super::{Federation, FederationConfig};
use crate::federation::host::{shared, HostConfig, SharedPersistence, SpecialistHost};
use crate::federation::specialists::{Archivist, GenericSpecialist, Omnipresent, Phygital, Symbiotic, Visionary};
use crate::persistence::PersistenceManager;
use std::sync::Arc;
use std::time::Duration;

/// Builder for `Federation`. Constructed via `Federation::builder(pm)`.
pub struct FederationBuilder {
    persistence: SharedPersistence,
    config: FederationConfig,
    // Each specialist either has a pre-built host (Some) or is absent (None).
    visionary: Option<Arc<SpecialistHost<Visionary>>>,
    omnipresent: Option<Arc<SpecialistHost<Omnipresent>>>,
    symbiotic: Option<Arc<SpecialistHost<Symbiotic>>>,
    phygital: Option<Arc<SpecialistHost<Phygital>>>,
    archivist: Option<Arc<SpecialistHost<Archivist>>>,
    /// Pre-built generic specialists to add after federation construction.
    generic: Vec<Arc<GenericSpecialist>>,
}

impl FederationBuilder {
    /// Create a new builder. Pass any `PersistenceManager` (in-memory, file,
    /// etc.) - the builder wraps it in `SharedPersistence`.
    pub fn new(pm: PersistenceManager) -> Self {
        Self {
            persistence: shared(pm),
            config: FederationConfig::default(),
            visionary: None,
            omnipresent: None,
            symbiotic: None,
            phygital: None,
            archivist: None,
            generic: vec![],
        }
    }

    /// Override the federation-level config wholesale
    pub fn with_config(mut self, config: FederationConfig) -> Self {
        self.config = config;
        self
    }

    /// Convenience: set the default checkpoint cadence for all hosts that
    /// don't have an override.
    pub fn checkpoint_every(mut self, interval: Duration) -> Self {
        self.config.default_checkpoint_interval = interval;
        self
    }

    /// Disable automatic checkpointing entirely. Callers must drive
    /// `checkpoint_all()` themselves.
    pub fn manual_checkpoints(mut self) -> Self {
        self.config.default_checkpoint_interval = Duration::ZERO;
        self
    }

    // ------- Default-config specialist additions -------

    /// Add a Visionary specialist with the federation's default host config
    pub fn with_visionary(self) -> Self {
        let cfg = self.config.to_host_config();
        self.with_visionary_host_config(cfg)
    }

    /// Add an Omnipresent specialist with the federation's default host config
    pub fn with_omnipresent(self) -> Self {
        let cfg = self.config.to_host_config();
        self.with_omnipresent_host_config(cfg)
    }

    /// Add a Symbiotic specialist with the federation's default host config
    pub fn with_symbiotic(self) -> Self {
        let cfg = self.config.to_host_config();
        self.with_symbiotic_host_config(cfg)
    }

    /// Add a Phygital specialist with the federation's default host config
    pub fn with_phygital(self) -> Self {
        let cfg = self.config.to_host_config();
        self.with_phygital_host_config(cfg)
    }

    /// Add an Archivist specialist with the federation's default host config
    pub fn with_archivist(self) -> Self {
        let cfg = self.config.to_host_config();
        self.with_archivist_host_config(cfg)
    }

    /// Add all 5 specialists with the federation's default host config
    pub fn with_all(self) -> Self {
        self.with_visionary()
            .with_omnipresent()
            .with_symbiotic()
            .with_phygital()
            .with_archivist()
    }

    // ------- Custom-config specialist additions -------

    pub fn with_visionary_host_config(mut self, cfg: HostConfig) -> Self {
        let v = Arc::new(Visionary::new());
        self.visionary = Some(Arc::new(SpecialistHost::new(
            v,
            self.persistence.clone(),
            cfg,
        )));
        self
    }

    pub fn with_omnipresent_host_config(mut self, cfg: HostConfig) -> Self {
        let o = Arc::new(Omnipresent::new());
        self.omnipresent = Some(Arc::new(SpecialistHost::new(
            o,
            self.persistence.clone(),
            cfg,
        )));
        self
    }

    pub fn with_symbiotic_host_config(mut self, cfg: HostConfig) -> Self {
        let s = Arc::new(Symbiotic::new());
        self.symbiotic = Some(Arc::new(SpecialistHost::new(
            s,
            self.persistence.clone(),
            cfg,
        )));
        self
    }

    pub fn with_phygital_host_config(mut self, cfg: HostConfig) -> Self {
        let p = Arc::new(Phygital::new());
        self.phygital = Some(Arc::new(SpecialistHost::new(
            p,
            self.persistence.clone(),
            cfg,
        )));
        self
    }

    pub fn with_archivist_host_config(mut self, cfg: HostConfig) -> Self {
        let a = Arc::new(Archivist::new());
        self.archivist = Some(Arc::new(SpecialistHost::new(
            a,
            self.persistence.clone(),
            cfg,
        )));
        self
    }

    // ------- Pre-built specialist injection -------
    //
    // For advanced cases where the caller already constructed a specialist
    // (e.g., to call `.with_p2p()` on Omnipresent before handing it over).

    /// Use a pre-built Visionary instance with the federation's default config
    pub fn with_visionary_instance(self, v: Arc<Visionary>) -> Self {
        let cfg = self.config.to_host_config();
        self.with_visionary_instance_host_config(v, cfg)
    }

    pub fn with_visionary_instance_host_config(
        mut self,
        v: Arc<Visionary>,
        cfg: HostConfig,
    ) -> Self {
        self.visionary = Some(Arc::new(SpecialistHost::new(
            v,
            self.persistence.clone(),
            cfg,
        )));
        self
    }

    pub fn with_omnipresent_instance(self, o: Arc<Omnipresent>) -> Self {
        let cfg = self.config.to_host_config();
        self.with_omnipresent_instance_host_config(o, cfg)
    }

    pub fn with_omnipresent_instance_host_config(
        mut self,
        o: Arc<Omnipresent>,
        cfg: HostConfig,
    ) -> Self {
        self.omnipresent = Some(Arc::new(SpecialistHost::new(
            o,
            self.persistence.clone(),
            cfg,
        )));
        self
    }

    pub fn with_symbiotic_instance(self, s: Arc<Symbiotic>) -> Self {
        let cfg = self.config.to_host_config();
        self.with_symbiotic_instance_host_config(s, cfg)
    }

    pub fn with_symbiotic_instance_host_config(
        mut self,
        s: Arc<Symbiotic>,
        cfg: HostConfig,
    ) -> Self {
        self.symbiotic = Some(Arc::new(SpecialistHost::new(
            s,
            self.persistence.clone(),
            cfg,
        )));
        self
    }

    pub fn with_phygital_instance(self, p: Arc<Phygital>) -> Self {
        let cfg = self.config.to_host_config();
        self.with_phygital_instance_host_config(p, cfg)
    }

    pub fn with_phygital_instance_host_config(
        mut self,
        p: Arc<Phygital>,
        cfg: HostConfig,
    ) -> Self {
        self.phygital = Some(Arc::new(SpecialistHost::new(
            p,
            self.persistence.clone(),
            cfg,
        )));
        self
    }

    pub fn with_archivist_instance(self, a: Arc<Archivist>) -> Self {
        let cfg = self.config.to_host_config();
        self.with_archivist_instance_host_config(a, cfg)
    }

    pub fn with_archivist_instance_host_config(
        mut self,
        a: Arc<Archivist>,
        cfg: HostConfig,
    ) -> Self {
        self.archivist = Some(Arc::new(SpecialistHost::new(
            a,
            self.persistence.clone(),
            cfg,
        )));
        self
    }

    // ------- Generic (dynamic) specialists -------

    /// Add a pre-built `GenericSpecialist` to the federation.
    ///
    /// The specialist's GGUF model must already be attached before calling this
    /// (via `GenericSpecialist::with_gguf_path()` or `with_mock_llm()`).
    /// Learning state is loaded from SQLite during `build_async()`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> anyhow::Result<()> {
    /// use a_run::federation::specialists::GenericSpecialist;
    /// use a_run::federation::hive::Federation;
    /// use a_run::persistence::PersistenceManager;
    ///
    /// let coder = GenericSpecialist::new("CodeReviewer", "code_review")
    ///     .with_gguf_path("models/qwen-code-1.8b.gguf").await;
    ///
    /// let pm = PersistenceManager::new("hive.db")?;
    /// let fed = Federation::builder(pm)
    ///     .with_all()
    ///     .with_gguf_specialist(std::sync::Arc::new(coder))
    ///     .build_async().await;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_gguf_specialist(mut self, specialist: Arc<GenericSpecialist>) -> Self {
        self.generic.push(specialist);
        self
    }

    // ------- Build -------

    /// Finalize the builder into a `Federation` (sync version).
    ///
    /// This always succeeds - even an empty federation (no specialists) is
    /// valid (it just has nothing to lifecycle-manage).
    ///
    /// Note: dynamic specialists added via `with_gguf_specialist()` will NOT
    /// have their learning state loaded from the DB in this sync path.
    /// Use `build_async()` to get full persistence support for dynamic specialists.
    pub fn build(self) -> Federation {
        let fed = Federation::from_parts(
            self.persistence,
            self.config,
            self.visionary,
            self.omnipresent,
            self.symbiotic,
            self.phygital,
            self.archivist,
        );
        // Push generic specialists without DB load (sync path)
        if !self.generic.is_empty() {
            // Use try_write — safe here since the federation is freshly constructed
            if let Ok(mut guard) = fed.dynamic.try_write() {
                for s in self.generic {
                    guard.push(s);
                }
            }
        }
        fed
    }

    /// Finalize the builder into a `Federation`, loading dynamic specialist
    /// learning state from SQLite asynchronously.
    ///
    /// Prefer this over `build()` when using `with_gguf_specialist()`.
    pub async fn build_async(self) -> Federation {
        let generic = self.generic.clone();
        let fed = self.build();
        // Load learning for each generic specialist
        for s in generic {
            let _ = fed.add_generic_specialist(s).await;
        }
        fed
    }
}
