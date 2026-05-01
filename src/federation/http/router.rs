/// Axum router definition for the federation HTTP status API.
///
/// The router is factored out from the server so tests can drive it
/// in-process via `tower::ServiceExt::oneshot` without binding a real port.

use crate::federation::hive::{Federation, LearningSummary, SpecialistLearningSummary};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
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
