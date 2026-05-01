/// Session: user identity and continuity across interactions.
///
/// A `Session` represents one user's working context with the federation.
/// It ties together:
/// - Who the user is (`user_id`, `user_name`)
/// - What they're working on (sequence of `Intent`s)
/// - Their biometric state (from Symbiotic)
/// - Which device they're on (from Omnipresent)
/// - How long they've been active
///
/// The `SessionManager` tracks all active sessions and provides a central
/// registry for routing intents and results to the correct user.
///
/// # Multi-user design
///
/// Each session is independent: two users can submit intents simultaneously
/// without interference. The federation routes each intent's proposals and
/// results back to the session that submitted it.
///
/// # Example
///
/// ```
/// use a_run::federation::session::{Session, SessionManager};
///
/// let mut manager = SessionManager::new();
/// let session_id = manager.create_session("aaron", None);
/// let session = manager.get(&session_id).unwrap();
/// println!("Session {} for {}", session.id, session.user_name);
/// ```

use crate::federation::intent::{Intent, IntentStatus};
use crate::federation::specialist::ExecutionResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A user session: identity + active context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Stable session identifier (UUID-style hex)
    pub id: String,
    /// Stable user identifier (may persist across sessions for the same person)
    pub user_id: String,
    /// Human-readable display name
    pub user_name: String,
    /// Which device this session originated from (optional)
    pub device_id: Option<String>,
    /// All intents submitted in this session, newest last
    pub intents: Vec<Intent>,
    /// All results produced for this session, newest last
    pub results: Vec<ExecutionResult>,
    /// Unix seconds when this session started
    pub started_at: u64,
    /// Unix seconds of the last activity in this session
    pub last_active: u64,
    /// Current session state
    pub state: SessionState,
    /// Arbitrary metadata (preferences, context hints, etc.)
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionState {
    /// Session is actively being used
    Active,
    /// User has been idle > idle_timeout
    Idle,
    /// Session was explicitly ended
    Ended,
    /// Session expired without explicit end
    Expired,
}

impl Session {
    /// Create a new session for a user.
    pub fn new(user_id: impl Into<String>, user_name: impl Into<String>) -> Self {
        let now = now_secs();
        Self {
            id: session_id(),
            user_id: user_id.into(),
            user_name: user_name.into(),
            device_id: None,
            intents: vec![],
            results: vec![],
            started_at: now,
            last_active: now,
            state: SessionState::Active,
            metadata: HashMap::new(),
        }
    }

    /// Set the originating device ID.
    pub fn with_device(mut self, device_id: impl Into<String>) -> Self {
        self.device_id = Some(device_id.into());
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Record an intent submitted in this session.
    pub fn add_intent(&mut self, mut intent: Intent) -> &Intent {
        // Tag the intent with this session's context
        intent = intent.with_context("session_id", self.id.clone());
        intent = intent.with_context("user_id", self.user_id.clone());
        self.intents.push(intent);
        self.touch();
        self.intents.last().unwrap()
    }

    /// Record an execution result for this session.
    pub fn add_result(&mut self, result: ExecutionResult) {
        self.results.push(result);
        self.touch();
    }

    /// Get the most recent intent, if any.
    pub fn current_intent(&self) -> Option<&Intent> {
        self.intents.last()
    }

    /// Get all pending intents (not yet completed or cancelled).
    pub fn pending_intents(&self) -> Vec<&Intent> {
        self.intents
            .iter()
            .filter(|i| {
                matches!(
                    i.status,
                    IntentStatus::Pending
                        | IntentStatus::Arbitrating
                        | IntentStatus::Executing
                        | IntentStatus::PartialResult
                )
            })
            .collect()
    }

    /// Get results for a specific intent ID.
    pub fn results_for_intent(&self, intent_id: &str) -> Vec<&ExecutionResult> {
        self.results
            .iter()
            .filter(|r| r.proposal_id.starts_with(intent_id))
            .collect()
    }

    /// How long this session has been active (seconds).
    pub fn age_seconds(&self) -> u64 {
        now_secs().saturating_sub(self.started_at)
    }

    /// How long since the last activity (seconds).
    pub fn idle_seconds(&self) -> u64 {
        now_secs().saturating_sub(self.last_active)
    }

    /// Mark the session as active and update last_active.
    pub fn touch(&mut self) {
        self.last_active = now_secs();
        if self.state == SessionState::Idle {
            self.state = SessionState::Active;
        }
    }

    /// End the session explicitly.
    pub fn end(&mut self) {
        self.state = SessionState::Ended;
        self.last_active = now_secs();
    }

    /// Check if the session has been idle for at least `timeout_secs`.
    pub fn is_idle(&self, timeout_secs: u64) -> bool {
        self.idle_seconds() >= timeout_secs
    }
}

/// Manages all active sessions.
///
/// Thread-safety: `SessionManager` is designed to be wrapped in
/// `Arc<tokio::sync::RwLock<SessionManager>>` for async access.
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    /// Seconds of inactivity before a session is marked Idle (default: 5 min)
    idle_timeout_secs: u64,
    /// Seconds of inactivity before a session is marked Expired (default: 24 hours)
    expiry_timeout_secs: u64,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            idle_timeout_secs: 300,       // 5 minutes
            expiry_timeout_secs: 86_400,  // 24 hours
        }
    }

    pub fn with_idle_timeout(mut self, secs: u64) -> Self {
        self.idle_timeout_secs = secs;
        self
    }

    pub fn with_expiry_timeout(mut self, secs: u64) -> Self {
        self.expiry_timeout_secs = secs;
        self
    }

    /// Create a new session and return its ID.
    pub fn create_session(
        &mut self,
        user_name: impl Into<String>,
        device_id: Option<&str>,
    ) -> String {
        let user_name = user_name.into();
        let user_id = slug(&user_name);
        let mut session = Session::new(user_id, user_name);
        if let Some(device) = device_id {
            session.device_id = Some(device.to_string());
        }
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        id
    }

    /// Get an immutable reference to a session.
    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    /// Get a mutable reference to a session.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    /// List all active (non-expired, non-ended) sessions.
    pub fn active_sessions(&self) -> Vec<&Session> {
        self.sessions
            .values()
            .filter(|s| {
                matches!(s.state, SessionState::Active | SessionState::Idle)
            })
            .collect()
    }

    /// Tick: mark idle sessions and expire old ones.
    /// Call periodically (e.g., once per minute).
    pub fn tick(&mut self) {
        for session in self.sessions.values_mut() {
            if session.state == SessionState::Ended || session.state == SessionState::Expired {
                continue;
            }
            if session.idle_seconds() >= self.expiry_timeout_secs {
                session.state = SessionState::Expired;
            } else if session.idle_seconds() >= self.idle_timeout_secs {
                session.state = SessionState::Idle;
            }
        }
    }

    /// Remove all expired and ended sessions. Returns count removed.
    pub fn purge_expired(&mut self) -> usize {
        let before = self.sessions.len();
        self.sessions.retain(|_, s| {
            !matches!(s.state, SessionState::Expired | SessionState::Ended)
        });
        before - self.sessions.len()
    }

    /// Total number of sessions (any state).
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Insert a pre-built session (used when reloading from the database).
    /// If a session with the same ID already exists, it is overwritten.
    pub fn insert_session(&mut self, session: Session) {
        self.sessions.insert(session.id.clone(), session);
    }

    /// Find sessions by user name (case-insensitive prefix match).
    pub fn find_by_user(&self, user_name: &str) -> Vec<&Session> {
        let lower = user_name.to_lowercase();
        self.sessions
            .values()
            .filter(|s| s.user_name.to_lowercase().starts_with(&lower))
            .collect()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn session_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("sess-{:032x}", nanos)
}

fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::intent::Intent;

    #[test]
    fn test_session_creation() {
        let s = Session::new("user-1", "Aaron");
        assert_eq!(s.user_name, "Aaron");
        assert_eq!(s.state, SessionState::Active);
        assert!(s.intents.is_empty());
        assert!(s.results.is_empty());
        assert!(s.age_seconds() < 2);
    }

    #[test]
    fn test_session_unique_ids() {
        let a = Session::new("u1", "A");
        let b = Session::new("u2", "B");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn test_session_add_intent() {
        let mut session = Session::new("u1", "Aaron");
        let intent = Intent::new("redesign dashboard");
        let added = session.add_intent(intent).clone(); // clone to release borrow on session

        assert_eq!(added.content, "redesign dashboard");
        // Session should have injected context
        assert_eq!(
            added.context.get("session_id"),
            Some(&session.id)
        );
        assert_eq!(session.intents.len(), 1);
    }

    #[test]
    fn test_session_current_intent() {
        let mut session = Session::new("u1", "A");
        assert!(session.current_intent().is_none());

        session.add_intent(Intent::new("first"));
        session.add_intent(Intent::new("second"));

        assert_eq!(session.current_intent().unwrap().content, "second");
    }

    #[test]
    fn test_session_pending_intents() {
        let mut session = Session::new("u1", "A");
        let mut intent1 = Intent::new("done");
        intent1.status = IntentStatus::Completed;
        let intent2 = Intent::new("pending");

        session.intents.push(intent1);
        session.add_intent(intent2);

        let pending = session.pending_intents();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].content, "pending");
    }

    #[test]
    fn test_session_idle_detection() {
        let session = Session::new("u1", "A");
        // Fresh session: not idle with 5 min timeout
        assert!(!session.is_idle(300));
        // But idle with 0 timeout
        assert!(session.is_idle(0));
    }

    #[test]
    fn test_session_end() {
        let mut session = Session::new("u1", "A");
        assert_eq!(session.state, SessionState::Active);
        session.end();
        assert_eq!(session.state, SessionState::Ended);
    }

    #[test]
    fn test_session_manager_create_and_get() {
        let mut manager = SessionManager::new();
        let id = manager.create_session("Aaron", None);

        let session = manager.get(&id).unwrap();
        assert_eq!(session.user_name, "Aaron");
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_session_manager_active_sessions() {
        let mut manager = SessionManager::new();
        let id1 = manager.create_session("Alice", None);
        let id2 = manager.create_session("Bob", None);

        assert_eq!(manager.active_sessions().len(), 2);

        manager.get_mut(&id1).unwrap().end();
        assert_eq!(manager.active_sessions().len(), 1);
    }

    #[test]
    fn test_session_manager_purge_expired() {
        let mut manager = SessionManager::new();
        let id1 = manager.create_session("Alice", None);
        let id2 = manager.create_session("Bob", None);

        manager.get_mut(&id1).unwrap().state = SessionState::Expired;
        let removed = manager.purge_expired();

        assert_eq!(removed, 1);
        assert_eq!(manager.session_count(), 1);
        assert!(manager.get(&id2).is_some());
    }

    #[test]
    fn test_session_manager_find_by_user() {
        let mut manager = SessionManager::new();
        manager.create_session("Alice Smith", None);
        manager.create_session("Alice Jones", None);
        manager.create_session("Bob", None);

        let alices = manager.find_by_user("alice");
        assert_eq!(alices.len(), 2);

        let bobs = manager.find_by_user("bob");
        assert_eq!(bobs.len(), 1);

        let nobodys = manager.find_by_user("charlie");
        assert_eq!(nobodys.len(), 0);
    }

    #[test]
    fn test_session_manager_tick_marks_idle() {
        let mut manager = SessionManager::new().with_idle_timeout(0);
        let id = manager.create_session("Alice", None);

        manager.tick();

        let session = manager.get(&id).unwrap();
        assert_eq!(session.state, SessionState::Idle);
    }
}
