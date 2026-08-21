/// Persistence layer for the Aaroneous federation.
///
/// Provides SQLite-backed storage for specialist learning states and session data.
/// All operations are synchronous (rusqlite is not async), so callers should
/// hold the `Arc<Mutex<PersistenceManager>>` and perform I/O on a background
/// thread or within a `tokio::task::spawn_blocking` context.
use rusqlite::{Connection, OptionalExtension, Result as SqlResult};

/// Row representation of a specialist's learning state.
///
/// Written by `save_learning_state()` and read by `load_learning_state()`.
/// The `execution_history_json` column holds a versioned JSON envelope:
/// `{"v":2,"outcomes":[true,false,...],"trend":[[ts,conf],...]}`.
#[derive(Debug, Clone)]
pub struct LearningStateRecord {
    pub specialist_kind: String,
    pub success_count: u32,
    pub failure_count: u32,
    pub total_executions: u32,
    pub confidence_score: f32,
    pub execution_history_json: String,
    pub last_updated: u64,
}

/// Row representation of a persisted session.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub session_id: String,
    pub user_id: String,
    pub user_name: String,
    pub state: String,
    pub session_json: String,
    pub created_at: i64,
}

/// SQLite persistence manager for the federation.
///
/// Wraps a single `rusqlite::Connection`. Because SQLite connections are
/// `Send` but not `Sync`, share this type via `Arc<tokio::sync::Mutex<PersistenceManager>>`.
pub struct PersistenceManager {
    db: Connection,
}

impl PersistenceManager {
    /// Open (or create) the SQLite database at the given path.
    ///
    /// Use `":memory:"` for an ephemeral in-memory database (useful in tests).
    pub fn new(db_path: &str) -> SqlResult<Self> {
        let db = Connection::open(db_path)?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let manager = Self { db };
        manager.init_schema()?;
        Ok(manager)
    }

    /// Create all tables required by the federation.
    fn init_schema(&self) -> SqlResult<()> {
        self.db.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS specialist_learning (
                specialist_kind TEXT PRIMARY KEY,
                success_count INTEGER NOT NULL DEFAULT 0,
                failure_count INTEGER NOT NULL DEFAULT 0,
                total_executions INTEGER NOT NULL DEFAULT 0,
                confidence_score REAL NOT NULL DEFAULT 0.5,
                execution_history_json TEXT NOT NULL,
                last_updated INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                user_name TEXT NOT NULL,
                state TEXT NOT NULL,
                session_json TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            ",
        )
    }

    // ── Learning State ──────────────────────────────────────────────

    /// Upsert a learning state record.
    pub fn save_learning_state(&self, record: &LearningStateRecord) -> SqlResult<()> {
        self.db.execute(
            "INSERT INTO specialist_learning
             (specialist_kind, success_count, failure_count, total_executions,
              confidence_score, execution_history_json, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(specialist_kind) DO UPDATE SET
                 success_count = excluded.success_count,
                 failure_count = excluded.failure_count,
                 total_executions = excluded.total_executions,
                 confidence_score = excluded.confidence_score,
                 execution_history_json = excluded.execution_history_json,
                 last_updated = excluded.last_updated",
            rusqlite::params![
                record.specialist_kind,
                record.success_count,
                record.failure_count,
                record.total_executions,
                record.confidence_score,
                record.execution_history_json,
                record.last_updated,
            ],
        )?;
        Ok(())
    }

    /// Load a learning state record by specialist kind.
    /// Returns `None` if no row exists.
    pub fn load_learning_state(
        &self,
        specialist_kind: &str,
    ) -> SqlResult<Option<LearningStateRecord>> {
        self.db
            .query_row(
                "SELECT specialist_kind, success_count, failure_count, total_executions,
                    confidence_score, execution_history_json, last_updated
             FROM specialist_learning WHERE specialist_kind = ?1",
                [specialist_kind],
                |row| {
                    Ok(LearningStateRecord {
                        specialist_kind: row.get(0)?,
                        success_count: row.get(1)?,
                        failure_count: row.get(2)?,
                        total_executions: row.get(3)?,
                        confidence_score: row.get(4)?,
                        execution_history_json: row.get(5)?,
                        last_updated: row.get(6)?,
                    })
                },
            )
            .optional()
    }

    /// Delete a learning state record.
    pub fn delete_learning_state(&self, specialist_kind: &str) -> SqlResult<()> {
        self.db.execute(
            "DELETE FROM specialist_learning WHERE specialist_kind = ?1",
            [specialist_kind],
        )?;
        Ok(())
    }

    /// List all learning state records.
    pub fn list_learning_states(&self) -> SqlResult<Vec<LearningStateRecord>> {
        let mut stmt = self.db.prepare(
            "SELECT specialist_kind, success_count, failure_count, total_executions,
                    confidence_score, execution_history_json, last_updated
             FROM specialist_learning",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(LearningStateRecord {
                specialist_kind: row.get(0)?,
                success_count: row.get(1)?,
                failure_count: row.get(2)?,
                total_executions: row.get(3)?,
                confidence_score: row.get(4)?,
                execution_history_json: row.get(5)?,
                last_updated: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    // ── Sessions ────────────────────────────────────────────────────

    /// Insert or update a session record.
    pub fn save_session(
        &self,
        session_id: &str,
        user_id: &str,
        user_name: &str,
        state: &str,
        session_json: &str,
        created_at: i64,
    ) -> SqlResult<()> {
        self.db.execute(
            "INSERT INTO sessions (session_id, user_id, user_name, state, session_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(session_id) DO UPDATE SET
                 user_id = excluded.user_id,
                 user_name = excluded.user_name,
                 state = excluded.state,
                 session_json = excluded.session_json,
                 created_at = excluded.created_at",
            rusqlite::params![
                session_id,
                user_id,
                user_name,
                state,
                session_json,
                created_at
            ],
        )?;
        Ok(())
    }

    /// Load all active (non-ended) sessions.
    /// Returns a list of `(session_id, session_json)` pairs.
    pub fn load_active_sessions(&self) -> SqlResult<Vec<(String, String)>> {
        let mut stmt = self
            .db
            .prepare("SELECT session_id, session_json FROM sessions WHERE state != 'Ended'")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect()
    }

    /// Delete a session by ID.
    pub fn delete_session(&self, session_id: &str) -> SqlResult<()> {
        self.db
            .execute("DELETE FROM sessions WHERE session_id = ?1", [session_id])?;
        Ok(())
    }
}
