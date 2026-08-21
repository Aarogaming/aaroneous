/// Trait that any specialist must implement to be hosted by `SpecialistHost`.
///
/// This formalizes the persistence-key + save/load surface that all 5
/// federation specialists already implement (Visionary, Omnipresent,
/// Symbiotic, Phygital, Archivist). The trait keeps `SpecialistHost` generic
/// without coupling it to specialist internals.
///
/// # Why take `SharedPersistence` instead of `&PersistenceManager`?
///
/// `PersistenceManager` wraps a `rusqlite::Connection` which is `Send` but
/// not `Sync` (SQLite connections are inherently single-threaded). To make
/// the host's checkpoint task `Send` (which `tokio::spawn` requires), the
/// trait works with `Arc<Mutex<PersistenceManager>>` instead of `&PM`.
/// Implementations lock the mutex internally, do the I/O while holding the
/// lock, and release.
use crate::federation::host::SharedPersistence;
use crate::federation::learn_persist::LearnPersistError;

/// Marker trait for specialists that can be wrapped in a `SpecialistHost`.
///
/// All federation specialists implement this. New specialists added later
/// just need to:
///   1. Have a `Mutex<*LearningData>` field
///   2. Define a `PERSISTENCE_KEY` constant
///   3. Implement `save_learning_to`/`load_learning_from`
///   4. Add a 4-line `impl HostableSpecialist`
pub trait HostableSpecialist: Send + Sync {
    /// Stable persistence key (matches the specialist's `PERSISTENCE_KEY`
    /// constant). This is what gets stored in the `specialist_kind` column.
    fn persistence_key() -> &'static str
    where
        Self: Sized;

    /// Save the specialist's current learning state.
    fn save_learning(
        &self,
        pm: &SharedPersistence,
    ) -> impl std::future::Future<Output = Result<(), LearnPersistError>> + Send;

    /// Load learning state into the specialist. Returns true if a row was
    /// found and applied, false if no prior state existed.
    fn load_learning(
        &self,
        pm: &SharedPersistence,
    ) -> impl std::future::Future<Output = Result<bool, LearnPersistError>> + Send;
}

// =====================================================================
// Implementations for each federation specialist.
//
// Each of these is intentionally trivial - they just delegate to the
// methods we already wrote on each specialist. The trait exists so that
// SpecialistHost can be generic; the actual logic lives on the specialist.
// =====================================================================

// All five impls follow the same pattern:
//   1. await the SharedPersistence Mutex lock (yields a Send guard)
//   2. call the specialist's *sync* save/load method while holding the guard
//   3. drop the guard at end of scope
// The future is Send because:
//   - The MutexGuard from tokio::sync::Mutex is Send when the inner type is Send
//   - The sync save/load call returns immediately (no await), so the
//     non-Sync `&PersistenceManager` is never held across a suspension point
impl HostableSpecialist for crate::federation::specialists::Visionary {
    fn persistence_key() -> &'static str {
        Self::PERSISTENCE_KEY
    }
    async fn save_learning(&self, pm: &SharedPersistence) -> Result<(), LearnPersistError> {
        let guard = pm.lock().await;
        self.save_learning_to(&guard)
    }
    async fn load_learning(&self, pm: &SharedPersistence) -> Result<bool, LearnPersistError> {
        let guard = pm.lock().await;
        self.load_learning_from(&guard)
    }
}

impl HostableSpecialist for crate::federation::specialists::Omnipresent {
    fn persistence_key() -> &'static str {
        Self::PERSISTENCE_KEY
    }
    async fn save_learning(&self, pm: &SharedPersistence) -> Result<(), LearnPersistError> {
        let guard = pm.lock().await;
        self.save_learning_to(&guard)
    }
    async fn load_learning(&self, pm: &SharedPersistence) -> Result<bool, LearnPersistError> {
        let guard = pm.lock().await;
        self.load_learning_from(&guard)
    }
}

impl HostableSpecialist for crate::federation::specialists::Symbiotic {
    fn persistence_key() -> &'static str {
        Self::PERSISTENCE_KEY
    }
    async fn save_learning(&self, pm: &SharedPersistence) -> Result<(), LearnPersistError> {
        let guard = pm.lock().await;
        self.save_learning_to(&guard)
    }
    async fn load_learning(&self, pm: &SharedPersistence) -> Result<bool, LearnPersistError> {
        let guard = pm.lock().await;
        self.load_learning_from(&guard)
    }
}

impl HostableSpecialist for crate::federation::specialists::Phygital {
    fn persistence_key() -> &'static str {
        Self::PERSISTENCE_KEY
    }
    async fn save_learning(&self, pm: &SharedPersistence) -> Result<(), LearnPersistError> {
        let guard = pm.lock().await;
        self.save_learning_to(&guard)
    }
    async fn load_learning(&self, pm: &SharedPersistence) -> Result<bool, LearnPersistError> {
        let guard = pm.lock().await;
        self.load_learning_from(&guard)
    }
}

impl HostableSpecialist for crate::federation::specialists::Archivist {
    fn persistence_key() -> &'static str {
        Self::PERSISTENCE_KEY
    }
    async fn save_learning(&self, pm: &SharedPersistence) -> Result<(), LearnPersistError> {
        let guard = pm.lock().await;
        self.save_learning_to(&guard)
    }
    async fn load_learning(&self, pm: &SharedPersistence) -> Result<bool, LearnPersistError> {
        let guard = pm.lock().await;
        self.load_learning_from(&guard)
    }
}
