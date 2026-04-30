/// Specialist Learning Persistence Bridge
///
/// Bridges each specialist's in-memory `*LearningData` struct to the SQLite
/// `specialist_learning` table managed by `crate::persistence::PersistenceManager`.
///
/// # Design
///
/// Each specialist has its own `*LearningData` type (e.g., `VisionaryLearningData`,
/// `OmnipresentLearningData`). They all have the same shape. The
/// `PersistableLearning` trait exposes a uniform serialization view so the
/// bridge can talk to any of them without coupling persistence to specialist
/// internals.
///
/// # Atomic save/load
///
/// `save_learning_state` and `load_learning_state` lock the in-memory
/// `Arc<Mutex<*LearningData>>` once, do the I/O on the locked data, and
/// release. No background tasks, no async cancellation hazards.
///
/// # Why not auto-save?
///
/// We deliberately don't save on every `record_result()` call. That would
/// thrash SQLite for high-frequency executions. Instead, callers
/// (the runtime / runner / specialist-host) decide when to checkpoint -
/// e.g., every N executions, on graceful shutdown, or on a timer.

use crate::persistence::{LearningStateRecord, PersistenceManager};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
mod specialist_e2e_tests;

#[derive(Debug, Error)]
pub enum LearnPersistError {
    #[error("database error: {0}")]
    Database(String),

    #[error("serialization error: {0}")]
    Serde(String),
}

impl From<rusqlite::Error> for LearnPersistError {
    fn from(e: rusqlite::Error) -> Self {
        LearnPersistError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for LearnPersistError {
    fn from(e: serde_json::Error) -> Self {
        LearnPersistError::Serde(e.to_string())
    }
}

/// Snapshot of in-memory learning data, used as the wire format between
/// specialists and the persistence layer.
///
/// All `*LearningData` types in `federation::specialists::*` produce this
/// snapshot via `PersistableLearning::snapshot()`, and rebuild themselves
/// from one via `PersistableLearning::restore_from()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningSnapshot {
    pub success_count: u32,
    pub failure_count: u32,
    pub total_executions: u32,
    pub confidence_score: f32,
    pub execution_history: Vec<bool>,
    pub last_updated: u64,
}

impl LearningSnapshot {
    /// A fresh "neutral" snapshot for a brand-new specialist
    pub fn neutral() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            total_executions: 0,
            confidence_score: 0.5,
            execution_history: vec![],
            last_updated: 0,
        }
    }

    /// Convert to the SQL record (serializing the history vector to JSON)
    pub fn to_record(
        &self,
        specialist_kind: impl Into<String>,
    ) -> Result<LearningStateRecord, LearnPersistError> {
        let history_json = serde_json::to_string(&self.execution_history)?;
        Ok(LearningStateRecord {
            specialist_kind: specialist_kind.into(),
            success_count: self.success_count,
            failure_count: self.failure_count,
            total_executions: self.total_executions,
            confidence_score: self.confidence_score,
            execution_history_json: history_json,
            last_updated: self.last_updated,
        })
    }

    /// Build from a SQL record (deserializing the history vector from JSON)
    pub fn from_record(record: &LearningStateRecord) -> Result<Self, LearnPersistError> {
        let execution_history: Vec<bool> =
            serde_json::from_str(&record.execution_history_json)?;
        Ok(Self {
            success_count: record.success_count,
            failure_count: record.failure_count,
            total_executions: record.total_executions,
            confidence_score: record.confidence_score,
            execution_history,
            last_updated: record.last_updated,
        })
    }
}

/// Trait implemented by every `*LearningData` type to support persistence.
///
/// The trait is intentionally minimal. Implementors only need to expose
/// snapshot/restore semantics; persistence orchestration happens in this
/// module.
pub trait PersistableLearning {
    /// Take a snapshot of the current in-memory state for serialization.
    fn snapshot(&self) -> LearningSnapshot;

    /// Replace the current state with the contents of the snapshot.
    fn restore_from(&mut self, snapshot: LearningSnapshot);
}

/// Save an in-memory learning state to SQLite.
///
/// `specialist_kind` should be a stable identifier like "Visionary" so the
/// row can be located on subsequent loads.
pub fn save_learning<L: PersistableLearning>(
    pm: &PersistenceManager,
    specialist_kind: &str,
    learning: &L,
) -> Result<(), LearnPersistError> {
    let snapshot = learning.snapshot();
    let record = snapshot.to_record(specialist_kind)?;
    pm.save_learning_state(&record)?;
    Ok(())
}

/// Load a learning state from SQLite into an in-memory specialist.
///
/// Returns `Ok(true)` if a row existed and was applied, `Ok(false)` if no
/// previous state was saved (in which case the specialist keeps its current
/// in-memory state, typically the neutral default).
pub fn load_learning<L: PersistableLearning>(
    pm: &PersistenceManager,
    specialist_kind: &str,
    learning: &mut L,
) -> Result<bool, LearnPersistError> {
    match pm.load_learning_state(specialist_kind)? {
        Some(record) => {
            let snapshot = LearningSnapshot::from_record(&record)?;
            learning.restore_from(snapshot);
            Ok(true)
        }
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal test impl: a struct that just wraps a `LearningSnapshot`.
    /// Lets us exercise the persistence path without coupling to any real
    /// specialist's `LearningData`.
    #[derive(Debug, Clone, PartialEq)]
    struct Probe(LearningSnapshot);

    impl PersistableLearning for Probe {
        fn snapshot(&self) -> LearningSnapshot {
            self.0.clone()
        }
        fn restore_from(&mut self, snapshot: LearningSnapshot) {
            self.0 = snapshot;
        }
    }

    fn fresh_db() -> PersistenceManager {
        // SQLite ":memory:" gives us a private in-memory database per test
        PersistenceManager::new(":memory:").expect("open in-memory db")
    }

    #[test]
    fn test_snapshot_neutral_defaults() {
        let s = LearningSnapshot::neutral();
        assert_eq!(s.success_count, 0);
        assert_eq!(s.failure_count, 0);
        assert_eq!(s.total_executions, 0);
        assert_eq!(s.confidence_score, 0.5);
        assert!(s.execution_history.is_empty());
    }

    #[test]
    fn test_snapshot_round_trip_through_record() {
        let original = LearningSnapshot {
            success_count: 7,
            failure_count: 3,
            total_executions: 10,
            confidence_score: 0.7,
            execution_history: vec![true, true, false, true],
            last_updated: 1700000000,
        };

        let record = original.to_record("TestKind").unwrap();
        assert_eq!(record.specialist_kind, "TestKind");
        assert_eq!(record.success_count, 7);

        let recovered = LearningSnapshot::from_record(&record).unwrap();
        assert_eq!(recovered, original);
    }

    #[test]
    fn test_save_then_load_round_trip() {
        let pm = fresh_db();
        let snapshot = LearningSnapshot {
            success_count: 4,
            failure_count: 1,
            total_executions: 5,
            confidence_score: 0.8,
            execution_history: vec![true, true, true, false, true],
            last_updated: 1700000123,
        };
        let probe = Probe(snapshot.clone());

        save_learning(&pm, "Visionary", &probe).expect("save should succeed");

        let mut empty_probe = Probe(LearningSnapshot::neutral());
        let loaded = load_learning(&pm, "Visionary", &mut empty_probe)
            .expect("load should succeed");
        assert!(loaded, "load should report true when row exists");
        assert_eq!(empty_probe.0, snapshot, "loaded state should match saved");
    }

    #[test]
    fn test_load_returns_false_when_no_row() {
        let pm = fresh_db();
        let mut probe = Probe(LearningSnapshot::neutral());
        let loaded = load_learning(&pm, "NeverSaved", &mut probe).unwrap();
        assert!(!loaded, "load should report false when no row");
        // Probe should be unchanged
        assert_eq!(probe.0, LearningSnapshot::neutral());
    }

    #[test]
    fn test_save_overwrites_existing_row() {
        let pm = fresh_db();

        let v1 = Probe(LearningSnapshot {
            success_count: 1,
            failure_count: 0,
            total_executions: 1,
            confidence_score: 1.0,
            execution_history: vec![true],
            last_updated: 100,
        });
        save_learning(&pm, "Visionary", &v1).unwrap();

        let v2 = Probe(LearningSnapshot {
            success_count: 5,
            failure_count: 5,
            total_executions: 10,
            confidence_score: 0.5,
            execution_history: vec![false; 5].into_iter().chain(vec![true; 5]).collect(),
            last_updated: 200,
        });
        save_learning(&pm, "Visionary", &v2).unwrap();

        let mut loaded = Probe(LearningSnapshot::neutral());
        load_learning(&pm, "Visionary", &mut loaded).unwrap();
        assert_eq!(loaded, v2, "second save should overwrite first");
    }

    #[test]
    fn test_save_load_multiple_kinds_independently() {
        let pm = fresh_db();

        let visionary = Probe(LearningSnapshot {
            success_count: 10,
            failure_count: 0,
            total_executions: 10,
            confidence_score: 1.0,
            execution_history: vec![true; 10],
            last_updated: 100,
        });
        let symbiotic = Probe(LearningSnapshot {
            success_count: 0,
            failure_count: 10,
            total_executions: 10,
            confidence_score: 0.0,
            execution_history: vec![false; 10],
            last_updated: 200,
        });

        save_learning(&pm, "Visionary", &visionary).unwrap();
        save_learning(&pm, "Symbiotic", &symbiotic).unwrap();

        let mut loaded_v = Probe(LearningSnapshot::neutral());
        let mut loaded_s = Probe(LearningSnapshot::neutral());
        load_learning(&pm, "Visionary", &mut loaded_v).unwrap();
        load_learning(&pm, "Symbiotic", &mut loaded_s).unwrap();

        assert_eq!(loaded_v, visionary);
        assert_eq!(loaded_s, symbiotic);
    }

    #[test]
    fn test_delete_learning_state() {
        let pm = fresh_db();
        let probe = Probe(LearningSnapshot {
            success_count: 1,
            failure_count: 0,
            total_executions: 1,
            confidence_score: 1.0,
            execution_history: vec![true],
            last_updated: 100,
        });
        save_learning(&pm, "Visionary", &probe).unwrap();

        pm.delete_learning_state("Visionary").unwrap();

        let mut loaded = Probe(LearningSnapshot::neutral());
        let result = load_learning(&pm, "Visionary", &mut loaded).unwrap();
        assert!(!result, "after delete, load should report false");
    }

    #[test]
    fn test_list_learning_states() {
        let pm = fresh_db();

        let probe1 = Probe(LearningSnapshot {
            success_count: 1,
            failure_count: 1,
            total_executions: 2,
            confidence_score: 0.5,
            execution_history: vec![true, false],
            last_updated: 100,
        });
        save_learning(&pm, "Visionary", &probe1).unwrap();
        save_learning(&pm, "Omnipresent", &probe1).unwrap();
        save_learning(&pm, "Symbiotic", &probe1).unwrap();

        let all = pm.list_learning_states().expect("list should succeed");
        assert_eq!(all.len(), 3);
        let names: Vec<&str> = all.iter().map(|r| r.specialist_kind.as_str()).collect();
        assert!(names.contains(&"Visionary"));
        assert!(names.contains(&"Omnipresent"));
        assert!(names.contains(&"Symbiotic"));
    }

    #[test]
    fn test_persistable_trait_round_trip_via_probe() {
        let original = LearningSnapshot {
            success_count: 3,
            failure_count: 2,
            total_executions: 5,
            confidence_score: 0.6,
            execution_history: vec![true, true, false, true, false],
            last_updated: 12345,
        };

        let mut probe = Probe(original.clone());
        let taken = probe.snapshot();
        assert_eq!(taken, original);

        let new_state = LearningSnapshot::neutral();
        probe.restore_from(new_state.clone());
        assert_eq!(probe.snapshot(), new_state);
    }
}
