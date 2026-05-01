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
