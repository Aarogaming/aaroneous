/// Axum router definition for the federation HTTP status API.
///
/// The router is factored out from the server so tests can drive it
/// in-process via `tower::ServiceExt::oneshot` without binding a real port.

use crate::federation::hive::{Federation, LearningSummary, SpecialistLearningSummary};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Shared application state for HTTP handlers.
///
/// Holds an `Arc` to the federation so handlers can read learning state
/// without taking exclusive ownership.
#[derive(Clone)]
pub struct AppState {
    pub federation: Arc<Federation>,
}

impl AppState {
    pub fn new(federation: Arc<Federation>) -> Self {
        Self { federation }
    }
}

/// Build the axum router for the federation HTTP status API.
///
/// Returns a `Router` that callers can either:
/// - Serve directly via `axum::serve` (the `HttpStatusServer` does this), OR
/// - Drive in-process via `tower::ServiceExt::oneshot` for testing.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/status", get(status))
        .route("/status/:kind", get(status_one))
        // Intent management: read current intent, submit new intent
        .route("/intent", get(get_intent).post(submit_intent))
        // Execution results: read recent outputs from specialist executions
        .route("/results", get(get_results))
        // Session management: create a session, get session details
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/:id", get(get_session_by_id).delete(delete_session_by_id))
        .route("/sessions/:id/intent", post(submit_session_intent))
        .route("/sessions/:id/results", get(get_session_results))
        // Audit log
        .route("/audit", get(get_audit_log))
        // Learning confidence trends (time-series)
        .route("/learning/trends", get(get_learning_trends))
        // Multi-hive cluster status
        .route("/cluster", get(cluster_status))
        .with_state(state)
}

// ====================================================================
// Handlers
// ====================================================================

/// Liveness probe. Returns 200 as long as the process is responding.
async fn healthz() -> &'static str {
    "ok"
}

/// Readiness probe. Returns 200 if the federation has at least one specialist
/// configured AND every configured specialist's host has been started.
///
/// We can't directly inspect the host's state from here (it would require
/// adding host accessors to Federation, which leak too much), so we use a
/// proxy: if `learning_summary().total_executions()` is reachable without
/// panicking, and the federation reports at least one configured specialist,
/// we consider it ready. For more precise readiness, the application can
/// add its own readiness gate.
async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    if state.federation.enabled_count() == 0 {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "no specialists configured",
        )
            .into_response();
    }
    // Touching learning_summary forces every present specialist to be
    // accessible. If anything panics here, axum will turn it into a 500.
    let _ = state.federation.learning_summary();
    (StatusCode::OK, "ready").into_response()
}

/// Full status snapshot - all configured specialists.
async fn status(State(state): State<AppState>) -> Json<StatusEnvelope> {
    let summary = state.federation.learning_summary();
    Json(StatusEnvelope::from_summary(&state.federation, summary))
}

/// One-specialist status. The `kind` path segment is matched
/// case-insensitively against canonical names ("Visionary", etc.).
async fn status_one(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> Result<Json<SpecialistLearningSummary>, (StatusCode, String)> {
    let summary = state.federation.learning_summary();
    let kind_lc = kind.to_lowercase();

    let entry = match kind_lc.as_str() {
        "visionary" => summary.visionary,
        "omnipresent" => summary.omnipresent,
        "symbiotic" => summary.symbiotic,
        "phygital" => summary.phygital,
        "archivist" => summary.archivist,
        _ => {
            return Err((
                StatusCode::NOT_FOUND,
                format!("unknown specialist '{}'. Known: Visionary, Omnipresent, Symbiotic, Phygital, Archivist", kind),
            ));
        }
    };

    entry.map(Json).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("specialist '{}' is known but not configured in this federation", kind),
        )
    })
}

// ====================================================================
// Wire types
// ====================================================================

/// Top-level JSON envelope for `GET /status`.
///
/// Wraps `LearningSummary` with federation-level context (total enabled
/// specialists, aggregate counters) for monitoring dashboards that want
/// a single object to dispatch on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEnvelope {
    /// Number of specialists configured in this federation
    pub enabled_count: usize,
    /// Sum of total_executions across all specialists
    pub total_executions: u32,
    /// Sum of success_count across all specialists
    pub total_successes: u32,
    /// Per-specialist learning state (None for not-configured)
    pub specialists: LearningSummary,
}

impl StatusEnvelope {
    fn from_summary(fed: &Federation, summary: LearningSummary) -> Self {
        Self {
            enabled_count: fed.enabled_count(),
            total_executions: summary.total_executions(),
            total_successes: summary.total_successes(),
            specialists: summary,
        }
    }
}

// ====================================================================
// Intent endpoints
// ====================================================================

/// GET /intent — read the current active intent
async fn get_intent(State(state): State<AppState>) -> impl IntoResponse {
    let intent = state.federation.current_intent().await;
    match intent {
        Some(i) => Json(serde_json::json!({
            "id": i.id,
            "content": i.content,
            "version": i.version,
            "status": format!("{:?}", i.status),
            "priority": format!("{:?}", i.priority),
            "assigned_to": i.assigned_to.map(|s| format!("{:?}", s)),
            "tags": i.tags,
            "results_count": i.results.len(),
            "created_at": i.created_at,
            "updated_at": i.updated_at,
        })).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"message": "no active intent"})),
        )
            .into_response(),
    }
}

/// Request body for POST /intent
#[derive(Debug, Deserialize)]
struct SubmitIntentRequest {
    content: String,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    context: std::collections::HashMap<String, String>,
}

/// POST /intent — submit a new user intent
///
/// Body: `{ "content": "redesign the dashboard", "priority": "High", "tags": ["ui"] }`
/// Returns: `{ "id": "intent-...", "message": "Intent submitted" }`
async fn submit_intent(
    State(state): State<AppState>,
    Json(req): Json<SubmitIntentRequest>,
) -> impl IntoResponse {
    use crate::federation::intent::{Intent, IntentPriority, IntentSource};

    let priority = match req.priority.as_deref() {
        Some("High") | Some("high") => IntentPriority::High,
        Some("Critical") | Some("critical") => IntentPriority::Critical,
        Some("Background") | Some("background") => IntentPriority::Background,
        _ => IntentPriority::Normal,
    };

    let mut intent = Intent::new(req.content)
        .with_priority(priority)
        .with_source(IntentSource::Api);
    for tag in req.tags {
        intent = intent.with_tag(tag);
    }
    for (k, v) in req.context {
        intent = intent.with_context(k, v);
    }

    let id = state.federation.submit_intent(intent).await;

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "message": "Intent submitted to federation"
        })),
    )
}

// ====================================================================
// Results endpoint
// ====================================================================

/// GET /results — read recent execution results from specialists
///
/// Query parameter `limit` controls how many results to return (default 20, max 100).
async fn get_results(State(state): State<AppState>) -> Json<serde_json::Value> {
    let results = state.federation.recent_results(20).await;

    Json(serde_json::json!({
        "count": results.len(),
        "results": results.iter().map(|r| serde_json::json!({
            "specialist": format!("{:?}", r.specialist),
            "proposal_id": r.proposal_id,
            "status": format!("{:?}", r.status),
            "output": r.output,
            "duration_ms": r.duration_ms,
            "error": r.error,
        })).collect::<Vec<_>>(),
    }))
}

// ====================================================================
// Session endpoints
// ====================================================================

fn session_to_json(s: &crate::federation::session::Session) -> serde_json::Value {
    serde_json::json!({
        "id": s.id,
        "user_id": s.user_id,
        "user_name": s.user_name,
        "device_id": s.device_id,
        "state": format!("{:?}", s.state),
        "intent_count": s.intents.len(),
        "result_count": s.results.len(),
        "current_intent": s.current_intent().map(|i| serde_json::json!({
            "id": i.id,
            "content": i.content,
            "status": format!("{:?}", i.status),
        })),
        "pending_intent_count": s.pending_intents().len(),
        "age_seconds": s.age_seconds(),
        "idle_seconds": s.idle_seconds(),
        "started_at": s.started_at,
        "last_active": s.last_active,
    })
}

/// GET /sessions — list all active sessions
async fn list_sessions(State(state): State<AppState>) -> Json<serde_json::Value> {
    let sessions = state.federation.active_sessions().await;
    Json(serde_json::json!({
        "count": sessions.len(),
        "sessions": sessions.iter().map(session_to_json).collect::<Vec<_>>(),
    }))
}

/// Request body for POST /sessions
#[derive(Deserialize)]
struct CreateSessionRequest {
    user_name: String,
    #[serde(default)]
    device_id: Option<String>,
}

/// POST /sessions — create a new user session
async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let session_id = state
        .federation
        .create_session(req.user_name.clone(), req.device_id.as_deref())
        .await;

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "session_id": session_id,
            "user_name": req.user_name,
            "message": "Session created",
        })),
    )
}

/// GET /sessions/:id — get session details
async fn get_session_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.federation.get_session(&id).await {
        Some(session) => Json(session_to_json(&session)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"message": format!("Session '{}' not found", id)})),
        )
            .into_response(),
    }
}

/// DELETE /sessions/:id — end and remove a session
async fn delete_session_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.federation.delete_session(&id).await {
        (StatusCode::OK, Json(serde_json::json!({"message": format!("Session '{}' ended", id)}))).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"message": format!("Session '{}' not found", id)}))).into_response()
    }
}

/// Request body for POST /sessions/:id/intent
#[derive(Deserialize)]
struct SessionIntentRequest {
    content: String,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

/// POST /sessions/:id/intent — submit an intent for a specific session
///
/// This is the preferred way to submit intents: associates the intent with
/// a user session for tracking and results routing.
async fn submit_session_intent(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(req): Json<SessionIntentRequest>,
) -> impl IntoResponse {
    use crate::federation::intent::{Intent, IntentPriority, IntentSource};

    let priority = match req.priority.as_deref() {
        Some("High") | Some("high") => IntentPriority::High,
        Some("Critical") | Some("critical") => IntentPriority::Critical,
        Some("Background") | Some("background") => IntentPriority::Background,
        _ => IntentPriority::Normal,
    };

    let mut intent = Intent::new(req.content)
        .with_priority(priority)
        .with_source(IntentSource::Api);
    for tag in req.tags {
        intent = intent.with_tag(tag);
    }

    match state
        .federation
        .submit_intent_for_session(&session_id, intent)
        .await
    {
        Ok((sid, intent_id)) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "session_id": sid,
                "intent_id": intent_id,
                "message": "Intent submitted for session",
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"message": e})),
        )
            .into_response(),
    }
}

/// GET /sessions/:id/results — execution results for a specific session.
///
/// Returns all `ExecutionResult`s associated with this session's intents,
/// newest first. Results are stored on the `Session` object as specialists
/// execute decisions routed to that session.
async fn get_session_results(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let session = state.federation.get_session(&session_id).await;
    match session {
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"message": format!("Session '{}' not found", session_id)})),
        )
            .into_response(),
        Some(s) => {
            let results: Vec<serde_json::Value> = s.results.iter().rev().map(|r| serde_json::json!({
                "specialist": format!("{:?}", r.specialist),
                "proposal_id": r.proposal_id,
                "status": format!("{:?}", r.status),
                "output": r.output,
                "duration_ms": r.duration_ms,
                "error": r.error,
            })).collect();

            Json(serde_json::json!({
                "session_id": session_id,
                "user_name": s.user_name,
                "result_count": results.len(),
                "results": results,
            }))
            .into_response()
        }
    }
}

// ====================================================================
// Audit log endpoint
// ====================================================================

/// GET /audit — recent audit events
///
/// Returns the last 50 audit events from the federation's audit log,
/// newest first. Events include intent submissions, specialist executions,
/// and session activities.
async fn get_audit_log(State(state): State<AppState>) -> Json<serde_json::Value> {
    let events = state.federation.recent_audit_events(50).await;

    let events_json: Vec<serde_json::Value> = events.iter().map(|e| serde_json::json!({
        "event_id": e.event_id,
        "timestamp_ms": e.timestamp_ms,
        "user_id": e.user_id,
        "action": e.action,
        "level": format!("{:?}", e.level),
        "resource": e.resource,
        "result": format!("{:?}", e.result),
        "details": e.details,
    })).collect();

    Json(serde_json::json!({
        "count": events_json.len(),
        "events": events_json,
    }))
}

// ====================================================================
// Learning trends endpoint
// ====================================================================

/// GET /learning/trends — confidence time-series for all specialists
///
/// Returns `{"visionary": [[ts, conf], ...], ...}` per specialist.
/// Each entry is `[unix_seconds, confidence_0_to_1]`.
/// Returns `null` for specialists not configured in this federation.
async fn get_learning_trends(State(state): State<AppState>) -> Json<serde_json::Value> {
    let trends = state.federation.learning_trends();

    fn to_json(v: Option<Vec<(u64, f32)>>) -> serde_json::Value {
        match v {
            None => serde_json::Value::Null,
            Some(pairs) => serde_json::Value::Array(
                pairs.into_iter()
                    .map(|(ts, conf)| serde_json::json!([ts, conf]))
                    .collect()
            ),
        }
    }

    Json(serde_json::json!({
        "visionary":   to_json(trends.visionary),
        "omnipresent": to_json(trends.omnipresent),
        "symbiotic":   to_json(trends.symbiotic),
        "phygital":    to_json(trends.phygital),
        "archivist":   to_json(trends.archivist),
    }))
}

// ====================================================================
// Multi-hive cluster endpoint
// ====================================================================

/// GET /cluster — multi-hive federation status
///
/// Returns the cluster nodes and their health if multi-hive is enabled,
/// or a message indicating it's not configured.
async fn cluster_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let status = state.federation.cluster_status().await;

    if status.is_empty() {
        if state.federation.has_multi_hive().await {
            Json(serde_json::json!({
                "enabled": true,
                "nodes": [],
                "message": "Multi-hive enabled but no peers joined yet"
            }))
        } else {
            Json(serde_json::json!({
                "enabled": false,
                "nodes": [],
                "message": "Multi-hive not enabled. Enable via federation.enable_multi_hive()."
            }))
        }
    } else {
        let nodes: Vec<serde_json::Value> = status
            .iter()
            .map(|(id, s)| serde_json::json!({
                "node_id": id,
                "status": format!("{:?}", s),
            }))
            .collect();

        Json(serde_json::json!({
            "enabled": true,
            "node_count": nodes.len(),
            "nodes": nodes,
        }))
    }
}
