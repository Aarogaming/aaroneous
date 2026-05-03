/// Axum router definition for the federation HTTP status API.
///
/// The router is factored out from the server so tests can drive it
/// in-process via `tower::ServiceExt::oneshot` without binding a real port.

use crate::federation::hive::{Federation, LearningSummary, SpecialistLearningSummary};
use crate::federation::forge;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::{self, Next},
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse, Json, Response},
    routing::{get, post, delete},
    Router,
    body::Body,
};
use tower_http::cors::{CorsLayer, AllowOrigin, Any};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::convert::Infallible;

/// Status of a background distillation generation job.
#[derive(Clone, Debug, Serialize)]
pub enum GenerationJobStatus {
    Running,
    Done(crate::federation::graph::distillation::GenerationReport),
    Failed(String),
}

/// Registry of background training-data generation jobs.
/// Keyed by job_id (UUID string).
pub type GenerationJobs = Arc<tokio::sync::Mutex<std::collections::HashMap<String, GenerationJobStatus>>>;

/// Shared application state for HTTP handlers.
///
/// Holds an `Arc` to the federation so handlers can read learning state
/// without taking exclusive ownership.
#[derive(Clone)]
pub struct AppState {
    pub federation: Arc<Federation>,
    pub generation_jobs: GenerationJobs,
    pub dissection_jobs: crate::federation::dna::DissectionJobs,
    pub import_jobs: crate::federation::model_registry::ImportJobs,
}

impl AppState {
    pub fn new(federation: Arc<Federation>) -> Self {
        Self {
            federation,
            generation_jobs: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            dissection_jobs: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            import_jobs: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }
}

/// Build the axum router for the federation HTTP status API.
///
/// Returns a `Router` that callers can either:
/// - Serve directly via `axum::serve` (the `HttpStatusServer` does this), OR
/// - Drive in-process via `tower::ServiceExt::oneshot` for testing.
/// API key authentication middleware.
///
/// Reads `AARONEOUS_API_KEY` from the environment at call time.
/// If set, every request to any route other than `/healthz` and `/readyz`
/// must include `Authorization: Bearer <key>` (case-insensitive prefix).
/// If the env var is unset, auth is disabled — development mode.
///
/// Set the key before starting the server:
/// ```sh
/// $env:AARONEOUS_API_KEY = "my-secret-key"
/// cargo run --bin aaroneous -- start
/// ```
async fn api_key_auth(
    headers: HeaderMap,
    req: axum::extract::Request<Body>,
    next: Next,
) -> Response {
    let Some(required_key) = std::env::var("AARONEOUS_API_KEY").ok() else {
        // Auth disabled — pass through
        return next.run(req).await;
    };

    // Allow liveness/readiness probes without auth
    let path = req.uri().path();
    if path == "/healthz" || path == "/readyz" {
        return next.run(req).await;
    }

    let provided = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").or_else(|| s.strip_prefix("bearer ")));

    match provided {
        Some(key) if key == required_key => next.run(req).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized",
                "message": "Set Authorization: Bearer <AARONEOUS_API_KEY>"
            })),
        ).into_response(),
    }
}

pub fn router(state: AppState) -> Router {
    // CORS: allow any origin by default.
    // In production, set AARONEOUS_CORS_ORIGIN=https://yourdomain.com
    // to restrict to a specific origin.
    let cors = match std::env::var("AARONEOUS_CORS_ORIGIN") {
        Ok(origin) if !origin.is_empty() => {
            let hv: axum::http::HeaderValue = origin.parse()
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("*"));
            CorsLayer::new()
                .allow_origin(AllowOrigin::exact(hv))
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
                .allow_headers(Any)
        }
        _ => CorsLayer::permissive(), // Dev mode: allow any origin
    };

    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/status", get(status))
        .route("/status/:kind", get(status_one))
        // Intent management: read current intent, submit new intent
        .route("/intent", get(get_intent).post(submit_intent))
        // Execution results: read recent outputs from specialist executions
        .route("/results", get(get_results))
        // SSE stream: push new results as they arrive (polls every 500ms)
        .route("/results/stream", get(stream_results))
        // Session management: create a session, get session details
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/:id", get(get_session_by_id).delete(delete_session_by_id))
        .route("/sessions/:id/intent", post(submit_session_intent))
        .route("/sessions/:id/results", get(get_session_results))
        .route("/sessions/:id/results/stream", get(stream_session_results))
        // Audit log
        .route("/audit", get(get_audit_log))
        // Learning confidence trends (time-series)
        .route("/learning/trends", get(get_learning_trends))
        // Specialist state — snapshot and real-time push stream for O3DE/XR clients
        .route("/specialists",        get(get_specialists_snapshot))
        .route("/specialists/stream", get(stream_specialists))
        // Dynamic specialist management
        .route("/dynamic-specialists",         get(list_dynamic_specialists).post(add_dynamic_specialist))
        .route("/dynamic-specialists/reload",  post(reload_dynamic_specialists))
        // Models directory listing
        .route("/models",                 get(list_models))
        // Forge: GGUF inspection, recipe generation, and crystallization
        .route("/forge/inspect",              post(forge_inspect))
        .route("/forge/auto-recipe",          post(forge_auto_recipe))
        .route("/forge/single-recipe",        post(forge_single_recipe))
        .route("/forge/crystallize",          post(forge_crystallize))
        .route("/forge/crystallize-roster",   post(forge_crystallize_roster))
        // Multi-hive cluster status
        .route("/cluster", get(cluster_status))
        // Distillation: training data generation, plan inspection, GGUF genome analysis
        .route("/distillation/plan",               get(distillation_plan))
        .route("/distillation/generate",           post(distillation_generate))
        .route("/distillation/jobs/:id",           get(distillation_job_status))
        .route("/distillation/analyze/:sovereign", get(distillation_analyze))
        .route("/distillation/script/:sovereign",  get(distillation_script))
        // RAG memory stats: federation-level + per-sovereign memory counts
        .route("/memory/stats", get(memory_stats))
        // DNA dissection: deep structural analysis of GGUF models
        .route("/dna/dissect",         post(dna_dissect))
        .route("/dna/jobs/:id",        get(dna_job_status))
        .route("/dna/genome/:model",   get(dna_genome))
        .route("/dna/compare",         post(dna_compare))
        .route("/dna/roster",          get(dna_roster))
        // Model import/export — large model lifecycle management
        .route("/models/import",            post(models_import))
        .route("/models/import/jobs/:id",   get(models_import_job_status))
        .route("/models/export/:name",      get(models_export))
        .route("/models/registry",          get(models_registry))
        .with_state(state)
        // Apply CORS and auth layers to all routes
        .layer(cors)
        .layer(middleware::from_fn(api_key_auth))
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

    // Check core specialists first (case-insensitive)
    let core_entry = match kind_lc.as_str() {
        "visionary"   => Some(summary.visionary),
        "omnipresent" => Some(summary.omnipresent),
        "symbiotic"   => Some(summary.symbiotic),
        "phygital"    => Some(summary.phygital),
        "archivist"   => Some(summary.archivist),
        _             => None,
    };

    if let Some(entry) = core_entry {
        return entry.map(Json).ok_or_else(|| (
            StatusCode::NOT_FOUND,
            format!("specialist '{}' is known but not configured in this federation", kind),
        ));
    }

    // Check dynamic (generic) specialists by name (case-insensitive)
    let dynamic = state.federation.dynamic_specialists().await;
    for s in &dynamic {
        if s.name.to_lowercase() == kind_lc {
            let l = s.learning.lock();
            let summary = SpecialistLearningSummary {
                success_count: l.success_count,
                failure_count: l.failure_count,
                total_executions: l.total_executions,
                confidence_score: l.confidence_score,
                history_len: l.execution_history.len(),
                last_updated: l.last_updated,
            };
            return Ok(Json(summary));
        }
    }

    Err((
        StatusCode::NOT_FOUND,
        format!(
            "unknown specialist '{}'. Core: Visionary, Omnipresent, Symbiotic, Phygital, Archivist. Dynamic: {}",
            kind,
            dynamic.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
        ),
    ))
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
        "results": results.iter().map(result_to_json).collect::<Vec<_>>(),
    }))
}

/// Convert an ExecutionResult to a JSON value for SSE/REST responses.
fn result_to_json(r: &crate::federation::specialist::ExecutionResult) -> serde_json::Value {
    serde_json::json!({
        // Use sovereign_name() for core specialists, specialist_name for dynamics
        "specialist": r.specialist_name.as_deref()
            .unwrap_or_else(|| r.specialist.sovereign_name()),
        "specialist_id": format!("{:?}", r.specialist),
        "internal_id": r.specialist.name(),
        "proposal_id": r.proposal_id,
        "status": format!("{:?}", r.status),
        "output": r.output,
        "duration_ms": r.duration_ms,
        "error": r.error,
    })
}

/// GET /results/stream — SSE stream of execution results.
///
/// Sends a `results` event for each batch of new `ExecutionResult` entries
/// as they land in the global ring buffer.  Polls every 500ms.
///
/// Each event `data` is a JSON array of result objects.  Clients receive
/// each result exactly once per connection (tracked by `proposal_id`).
///
/// Connect with:
/// ```sh
/// curl -N http://localhost:8765/results/stream
/// ```
/// or the browser `EventSource` API:
/// ```js
/// const es = new EventSource('/results/stream');
/// es.addEventListener('results', e => console.log(JSON.parse(e.data)));
/// ```
async fn stream_results(
    State(state): State<AppState>,
) -> Sse<impl futures_util::stream::Stream<Item = Result<Event, Infallible>>> {
    let fed = state.federation.clone();

    // unfold state: (seen_proposal_ids, interval, federation_arc)
    let init = (
        std::collections::HashSet::<String>::new(),
        tokio::time::interval(std::time::Duration::from_millis(500)),
        fed,
    );

    let stream = futures_util::stream::unfold(init, |s| async move {
        let (mut seen, mut ticker, fed) = s;
        ticker.tick().await;

        let all = fed.recent_results(50).await;
        let fresh: Vec<_> = all.into_iter()
            .filter(|r| !seen.contains(&r.proposal_id))
            .collect();

        // Mark as seen and serialize
        let new: Vec<serde_json::Value> = fresh.into_iter()
            .map(|r| { seen.insert(r.proposal_id.clone()); result_to_json(&r) })
            .collect();

        let event = if new.is_empty() {
            Event::default().comment("heartbeat")
        } else {
            let data = serde_json::to_string(&new).unwrap_or_default();
            Event::default().data(data).event("results")
        };

        Some((Ok(event), (seen, ticker, fed)))
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
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
            let results: Vec<serde_json::Value> = s.results.iter().rev()
                .map(result_to_json).collect();

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

/// GET /sessions/:id/results/stream — SSE stream of results for a specific session.
///
/// Like `/results/stream` but scoped to one session.  Polls every 500ms.
/// Sends a `results` event with a JSON array of new results.
async fn stream_session_results(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Sse<impl futures_util::stream::Stream<Item = Result<Event, Infallible>>> {
    let fed = state.federation.clone();

    let init = (
        std::collections::HashSet::<String>::new(),
        tokio::time::interval(std::time::Duration::from_millis(500)),
        fed,
        session_id,
    );

    let stream = futures_util::stream::unfold(init, |s| async move {
        let (mut seen, mut ticker, fed, sid) = s;
        ticker.tick().await;

        let new: Vec<serde_json::Value> = match fed.get_session(&sid).await {
            None => vec![],
            Some(session) => {
                let fresh: Vec<_> = session.results.into_iter()
                    .filter(|r| !seen.contains(&r.proposal_id))
                    .collect();
                fresh.into_iter()
                    .map(|r| { seen.insert(r.proposal_id.clone()); result_to_json(&r) })
                    .collect()
            }
        };

        let event = if new.is_empty() {
            Event::default().comment("heartbeat")
        } else {
            let data = serde_json::to_string(&new).unwrap_or_default();
            Event::default().data(data).event("results")
        };

        Some((Ok(event), (seen, ticker, fed, sid)))
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ====================================================================
// Specialist state endpoints (for O3DE / XR / real-time clients)
// ====================================================================

/// GET /specialists — full snapshot of all specialist state
///
/// Returns every configured specialist (core + dynamic) with their
/// current learning state, active intent, and domain.  This is the
/// initial sync payload an O3DE Gem reads on connection; subsequent
/// updates come via GET /specialists/stream.
async fn get_specialists_snapshot(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let summary = state.federation.learning_summary();
    let intent = state.federation.current_intent().await
        .map(|i| i.content)
        .unwrap_or_default();
    let dynamic = state.federation.dynamic_specialists().await;

    let mut specialists: Vec<serde_json::Value> = vec![];

    macro_rules! push_core {
        ($field:expr, $name:literal, $domain:literal) => {
            specialists.push(serde_json::json!({
                "name": $name,
                "domain": $domain,
                "kind": "core",
                "active_intent": intent,
                "learning": $field.as_ref().map(|s| serde_json::json!({
                    "confidence": s.confidence_score,
                    "total_executions": s.total_executions,
                    "success_rate": s.success_rate_percent(),
                    "last_updated": s.last_updated,
                })),
            }));
        };
    }

    // Use sovereign_name() for display, name() for persistence/routing
    use crate::federation::specialist::SpecialistId;
    macro_rules! push_core_id {
        ($field:expr, $id:expr) => {
            specialists.push(serde_json::json!({
                "name": $id.sovereign_name(),
                "internal_name": $id.name(),
                "domain": $id.domain(),
                "kind": "core",
                "active_intent": intent,
                "learning": $field.as_ref().map(|s| serde_json::json!({
                    "confidence": s.confidence_score,
                    "total_executions": s.total_executions,
                    "success_rate": s.success_rate_percent(),
                    "last_updated": s.last_updated,
                })),
            }));
        };
    }
    push_core_id!(summary.visionary,   SpecialistId::Visionary);
    push_core_id!(summary.omnipresent, SpecialistId::Omnipresent);
    push_core_id!(summary.symbiotic,   SpecialistId::Symbiotic);
    push_core_id!(summary.phygital,    SpecialistId::Phygital);
    push_core_id!(summary.archivist,   SpecialistId::Archivist);

    for s in &dynamic {
        let l = s.learning.lock();
        let mem_count = s.memory.lock().count_for(&s.name);

        // Lightweight genome summary from task spec (no GGUF I/O — safe for 200ms poll)
        let genome_summary = crate::federation::graph::task_spec::spec_for(&s.name)
            .map(|spec| serde_json::json!({
                "target_tier":   spec.target_tier.tier_name(),
                "target_params": spec.target_tier.target_params(),
                "always_resident": spec.always_resident,
                "context_window": spec.context_window_tokens,
                // attn_mlp_ratio and specialization_score come from GGUFAnalyzer;
                // use 0.0 as sentinel so O3DE knows to display "no genome" until
                // /distillation/analyze/:sovereign has been called.
                "attn_mlp_ratio": 0.0f32,
                "specialization_score": 0.0f32,
            }));

        specialists.push(serde_json::json!({
            "name": s.name,
            "domain": s.domain,
            "kind": "dynamic",
            "model_path": s.model_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "has_llm": s.has_llm(),
            "active_intent": intent,
            "memory_count": mem_count,
            "genome": genome_summary,
            "learning": serde_json::json!({
                "confidence": l.confidence_score,
                "total_executions": l.total_executions,
                "success_rate": if l.total_executions > 0 {
                    l.success_count as f32 / l.total_executions as f32 * 100.0
                } else { 0.0 },
                "last_updated": l.last_updated,
            }),
        }));
    }

    Json(serde_json::json!({
        "count": specialists.len(),
        "specialists": specialists,
        "sentinel_active": state.federation.sentinel.read().await.is_some(),
    }))
}

/// GET /specialists/stream — SSE push stream of all specialist state changes
///
/// Designed for persistent connection by the O3DE AaroneousGem.
/// Pushes events on:
/// - Intent submission (`type: "intent_submitted"`)
/// - Specialist execution complete (`type: "execution_complete"`)
/// - Heartbeat every 2s (`comment: heartbeat`)
///
/// Connect with:
/// ```sh
/// curl -N http://localhost:8765/specialists/stream
/// ```
/// O3DE C++ Gem example (conceptual):
/// ```cpp
/// // In your SystemComponent::Activate()
/// HttpRequestorRequestBus::Broadcast(&HttpRequestorRequests::AddTextRequest,
///     "http://localhost:8765/specialists/stream", HttpMethod::HTTP_GET,
///     [](const AZStd::string& body, Aws::Http::HttpResponseCode) {
///         // Parse SSE events line by line
///     });
/// ```
async fn stream_specialists(
    State(state): State<AppState>,
) -> Sse<impl futures_util::stream::Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.federation.subscribe_specialist_events();

    let stream = futures_util::stream::unfold(
        (rx, tokio::time::interval(std::time::Duration::from_secs(2))),
        |(mut rx, mut ticker)| async move {
            // Try to receive a broadcast event first (non-blocking)
            let event = tokio::select! {
                // A specialist event was broadcast
                Ok(payload) = rx.recv() => {
                    let data = serde_json::to_string(&payload).unwrap_or_default();
                    Event::default().data(data).event("specialist_update")
                }
                // No event — send heartbeat so the connection stays alive
                _ = ticker.tick() => {
                    Event::default().comment("heartbeat")
                }
            };
            Some((Ok(event), (rx, ticker)))
        }
    );

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ====================================================================
// Audit log endpoint
// ====================================================================

/// Query parameters for `GET /audit`
#[derive(Deserialize, Default)]
struct AuditQueryParams {
    /// Max events to return (default 50, max 1000)
    limit: Option<usize>,
    /// Return only events with timestamp_ms ≥ since_ms
    since_ms: Option<u64>,
    /// Return only events with timestamp_ms ≤ until_ms
    until_ms: Option<u64>,
    /// Filter by user_id
    user_id: Option<String>,
}

/// GET /audit — recent audit events with optional pagination
///
/// Query parameters:
/// - `?limit=N` — return at most N events (default 50, max 1000)
/// - `?since_ms=UNIX_MS` — only events after this timestamp
/// - `?until_ms=UNIX_MS` — only events before this timestamp
/// - `?user_id=USER` — filter by user identity
///
/// Example:
/// ```sh
/// curl 'http://localhost:8765/audit?limit=100&since_ms=1746000000000'
/// ```
async fn get_audit_log(
    State(state): State<AppState>,
    Query(params): Query<AuditQueryParams>,
) -> Json<serde_json::Value> {
    let limit = params.limit.unwrap_or(50);
    let events = state.federation.query_audit_events(
        limit, params.since_ms, params.until_ms, params.user_id
    ).await;

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
        "limit": limit,
        "since_ms": params.since_ms,
        "until_ms": params.until_ms,
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

    let mut resp = serde_json::json!({
        "visionary":   to_json(trends.visionary),
        "omnipresent": to_json(trends.omnipresent),
        "symbiotic":   to_json(trends.symbiotic),
        "phygital":    to_json(trends.phygital),
        "archivist":   to_json(trends.archivist),
    });
    // Add dynamic specialist trends as top-level keys
    if let serde_json::Value::Object(ref mut map) = resp {
        for (name, data) in trends.dynamic {
            map.insert(name, to_json(Some(data)));
        }
    }
    Json(resp)
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

// ====================================================================
// Models directory listing
// ====================================================================

/// GET /models — list all GGUF files in the models directory
///
/// Scans `D:\Aaroneous\models\` (or the current working directory `./models/`)
/// and returns metadata for each `.gguf` file found.  For small models (<500MB),
/// also reads the GGUF header to return architecture and tensor count.
async fn list_models() -> impl IntoResponse {
    let search_paths = [
        std::path::PathBuf::from("D:\\Aaroneous\\models"),
        std::path::PathBuf::from("./models"),
    ];

    let mut models = Vec::new();
    let mut searched_paths = Vec::new();

    for models_dir in &search_paths {
        searched_paths.push(models_dir.to_string_lossy().to_string());
        if !models_dir.exists() { continue; }

        let Ok(entries) = std::fs::read_dir(models_dir) else { continue };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let Some(ext) = path.extension() else { continue };
            if ext != "gguf" { continue; }

            let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let file_name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // For files under 2GB, try to read the header
            let header = if file_size < 2 * 1024 * 1024 * 1024 {
                match forge::read_gguf(&path) {
                    Ok((_idx, meta)) => serde_json::json!({
                        "version": meta.version,
                        "architecture": meta.architecture,
                        "model_name": meta.model_name,
                        "context_length": meta.context_length,
                        "tensor_count": meta.tensor_count,
                    }),
                    Err(_) => serde_json::Value::Null,
                }
            } else {
                serde_json::Value::Null
            };

            models.push(serde_json::json!({
                "file_name": file_name,
                "path": path.to_string_lossy(),
                "size_bytes": file_size,
                "size_mb": file_size as f64 / 1_048_576.0,
                "header": header,
            }));
        }
    }

    Json(serde_json::json!({
        "count": models.len(),
        "searched_paths": searched_paths,
        "models": models,
    }))
}

// ====================================================================
// Dynamic specialist management
// ====================================================================

/// GET /dynamic-specialists — list all currently-loaded GenericSpecialists
async fn list_dynamic_specialists(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let specialists = state.federation.dynamic_specialists().await;
    let list: Vec<serde_json::Value> = specialists.iter().map(|s| {
        let l = s.learning.lock();
        serde_json::json!({
            "name": s.name,
            "domain": s.domain,
            "persistence_key": s.persistence_key,
            "has_llm": s.has_llm(),
            "model_path": s.model_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "total_executions": l.total_executions,
            "confidence_score": l.confidence_score,
            "last_updated": l.last_updated,
        })
    }).collect();

    Json(serde_json::json!({
        "count": list.len(),
        "specialists": list,
    }))
}

/// Request body for POST /dynamic-specialists
#[derive(Deserialize)]
struct AddDynamicSpecialistRequest {
    /// Display name (e.g. "CodeReviewer")
    name: String,
    /// Domain label (e.g. "code_review")
    domain: String,
    /// Optional path to GGUF model. If absent or file not found, uses MockLLM.
    gguf_path: Option<String>,
}

/// POST /dynamic-specialists — add a new GenericSpecialist at runtime (no restart needed)
///
/// Example:
/// ```json
/// {
///   "name": "CodeReviewer",
///   "domain": "code_review",
///   "gguf_path": "D:\\Aaroneous\\models\\qwen-code-1.5b.gguf"
/// }
/// ```
async fn add_dynamic_specialist(
    State(state): State<AppState>,
    Json(req): Json<AddDynamicSpecialistRequest>,
) -> impl IntoResponse {
    use crate::federation::specialists::GenericSpecialist;
    use std::sync::Arc;

    // Check for duplicate name
    let existing = state.federation.dynamic_specialists().await;
    if existing.iter().any(|s| s.name == req.name) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("A specialist named '{}' is already loaded", req.name),
            })),
        ).into_response();
    }

    let specialist = if let Some(ref path) = req.gguf_path {
        let p = std::path::Path::new(path);
        if p.exists() {
            GenericSpecialist::new(&req.name, &req.domain)
                .with_gguf_path(p).await
        } else {
            match GenericSpecialist::new(&req.name, &req.domain).with_mock_llm().await {
                Ok(s) => s,
                Err(e) => return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
                ).into_response(),
            }
        }
    } else {
        match GenericSpecialist::new(&req.name, &req.domain).with_mock_llm().await {
            Ok(s) => s,
            Err(e) => return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
            ).into_response(),
        }
    };

    state.federation.add_generic_specialist(Arc::new(specialist)).await;

    Json(serde_json::json!({
        "ok": true,
        "name": req.name,
        "domain": req.domain,
        "model": req.gguf_path,
        "total_dynamic": state.federation.dynamic_specialists().await.len(),
    })).into_response()
}

/// POST /dynamic-specialists/reload — re-read specialist_registry.json and
/// add any newly-enabled dynamic specialists (without removing existing ones).
async fn reload_dynamic_specialists(
    State(state): State<AppState>,
) -> impl IntoResponse {
    use crate::federation::specialists::GenericSpecialist;
    use std::sync::Arc;

    let registry_path = std::path::Path::new("D:\\Aaroneous\\config\\specialist_registry.json");
    if !registry_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "ok": false,
                "error": "specialist_registry.json not found",
            })),
        ).into_response();
    }

    let content = match std::fs::read_to_string(registry_path) {
        Ok(c) => c,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        ).into_response(),
    };

    let registry: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        ).into_response(),
    };

    let existing_names: std::collections::HashSet<String> = state.federation
        .dynamic_specialists().await
        .iter().map(|s| s.name.clone()).collect();

    let mut added = Vec::new();
    let mut skipped = Vec::new();

    // Support new "dynamic_sovereigns" key (flat object) and old "dynamic_specialists.examples"
    let entries_vec: Vec<serde_json::Value> = registry
        .get("dynamic_sovereigns")
        .and_then(|d| d.as_object())
        .map(|obj| obj.values().cloned().collect())
        .or_else(|| {
            registry.get("dynamic_specialists")
                .and_then(|d| d.get("examples"))
                .and_then(|e| e.as_array())
                .cloned()
        })
        .unwrap_or_default();

    if !entries_vec.is_empty() {
        for entry in &entries_vec {
            let enabled = entry.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
            if !enabled { continue; }

            let name = entry.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
            let domain = entry.get("domain").and_then(|d| d.as_str()).unwrap_or("general");
            let gguf_path = entry.get("gguf_path").and_then(|g| g.as_str()).unwrap_or("");

            if existing_names.contains(name) {
                skipped.push(serde_json::json!({ "name": name, "reason": "already loaded" }));
                continue;
            }

            let specialist = GenericSpecialist::new(name, domain)
                .with_gguf_path(gguf_path).await;
            state.federation.add_generic_specialist(Arc::new(specialist)).await;
            added.push(serde_json::json!({ "name": name, "domain": domain, "gguf": gguf_path }));
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "added": added.len(),
        "skipped": skipped.len(),
        "new_specialists": added,
        "skipped_specialists": skipped,
        "total_dynamic": state.federation.dynamic_specialists().await.len(),
    })).into_response()
}

// ====================================================================
// Forge endpoints
// ====================================================================

/// Request body for POST /forge/inspect
#[derive(Deserialize)]
struct ForgeInspectRequest {
    /// Absolute path to the GGUF file to inspect
    path: String,
}

/// POST /forge/inspect — parse a GGUF file and return its tensor table and metadata
///
/// Example request:
/// ```json
/// {"path": "D:\\Aaroneous\\models\\qwen2.5-1.5b.gguf"}
/// ```
async fn forge_inspect(
    Json(req): Json<ForgeInspectRequest>,
) -> impl IntoResponse {
    let path = std::path::Path::new(&req.path);
    match forge::read_gguf(path) {
        Ok((index, meta)) => {
            let tensors: Vec<serde_json::Value> = index.0
                .values()
                .flat_map(|gm| gm.tensors.iter())
                .map(|(name, tm)| serde_json::json!({
                    "name": name,
                    "shape": tm.shape,
                    "dtype": tm.dtype,
                    "offset": tm.offset,
                    "size": tm.size,
                    "kind": tm.kind,
                }))
                .collect();

            Json(serde_json::json!({
                "ok": true,
                "path": req.path,
                "version": meta.version,
                "tensor_count": meta.tensor_count,
                "architecture": meta.architecture,
                "model_name": meta.model_name,
                "context_length": meta.context_length,
                "tensors": tensors,
                "metadata": meta.kv,
            })).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        ).into_response(),
    }
}

/// Request body for POST /forge/auto-recipe
#[derive(Deserialize)]
struct ForgeAutoRecipeRequest {
    /// Path to the primary (base) model — e.g., Qwen abliterated
    model_a_path: String,
    /// Path to the domain-specialized model
    model_b_path: String,
    /// Recipe ID for the output
    recipe_id: String,
    /// Specialist domain, used to pick the splicing strategy
    /// e.g., "code_review", "legal_analysis", "biomedical_qa"
    domain: String,
}

/// POST /forge/auto-recipe — parse two GGUFs, auto-generate a ForgeRecipe
///
/// Example request:
/// ```json
/// {
///   "model_a_path": "D:\\models\\qwen2.5-1.5b.gguf",
///   "model_b_path": "D:\\models\\qwen-coder-1.5b.gguf",
///   "recipe_id": "code-specialist-v1",
///   "domain": "code_review"
/// }
/// ```
async fn forge_auto_recipe(
    Json(req): Json<ForgeAutoRecipeRequest>,
) -> impl IntoResponse {
    // Parse both GGUFs
    let mut combined_index = forge::GgufIndex::new();
    for path in &[&req.model_a_path, &req.model_b_path] {
        match forge::read_gguf(path.as_str()) {
            Ok((idx, _)) => {
                for (k, v) in idx.0 { combined_index.0.insert(k, v); }
            }
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
                ).into_response();
            }
        }
    }

    let strategy = forge::SplicingStrategy::for_domain(&req.domain);

    match forge::recipe_from_two_models(
        &req.model_a_path,
        &req.model_b_path,
        &req.recipe_id,
        strategy,
        &combined_index,
        std::collections::HashMap::new(),
    ) {
        Ok(recipe) => Json(serde_json::json!({
            "ok": true,
            "recipe_id": recipe.recipe_id,
            "domain": req.domain,
            "segment_count": recipe.segments.len(),
            "recipe": recipe,
        })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "ok": false, "error": e })),
        ).into_response(),
    }
}

/// Request body for POST /forge/single-recipe
#[derive(Deserialize)]
struct ForgeSingleRecipeRequest {
    /// Absolute path to the GGUF file to extract from
    model_path: String,
    /// Recipe ID for the output
    recipe_id: String,
    /// Tensor kinds to include: "attention", "mlp", "embedding", "norm", "other"
    /// Empty = include all tensors from the model
    #[serde(default)]
    include_kinds: Vec<String>,
}

/// POST /forge/single-recipe — generate a ForgeRecipe extracting selected
/// tensor kinds from a single GGUF model.
///
/// Example: extract only attention tensors from a fine-tuned model,
/// then use POST /forge/crystallize to splice them with a base model.
async fn forge_single_recipe(
    Json(req): Json<ForgeSingleRecipeRequest>,
) -> impl IntoResponse {
    match forge::read_gguf(&req.model_path) {
        Err(e) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        ).into_response(),
        Ok((index, _meta)) => {
            let kinds: Vec<forge::TensorKind> = req.include_kinds.iter()
                .filter_map(|s| match s.to_lowercase().as_str() {
                    "attention" | "attn" => Some(forge::TensorKind::Attention),
                    "mlp" | "ffn"        => Some(forge::TensorKind::Mlp),
                    "embedding" | "emb"  => Some(forge::TensorKind::Embedding),
                    "norm"               => Some(forge::TensorKind::Norm),
                    "other"              => Some(forge::TensorKind::Other),
                    _                    => None,
                })
                .collect();

            match forge::recipe_from_single_model(
                &req.model_path, &req.recipe_id, &kinds, &index,
                std::collections::HashMap::new(),
            ) {
                Ok(recipe) => Json(serde_json::json!({
                    "ok": true,
                    "recipe_id": recipe.recipe_id,
                    "segment_count": recipe.segments.len(),
                    "include_kinds": req.include_kinds,
                    "recipe": recipe,
                })).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "ok": false, "error": e })),
                ).into_response(),
            }
        }
    }
}

/// Request body for POST /forge/crystallize
#[derive(Deserialize)]
struct ForgeCrystallizeRequest {
    /// Path to the output GGUF file to write
    output_path: String,
    /// The forging recipe
    recipe: forge::ForgeRecipe,
    /// Source GGUF file paths to index automatically.
    /// Each path is parsed with read_gguf() and added to the index.
    source_paths: Vec<String>,
}

/// POST /forge/crystallize — parse sources, build GgufIndex, crystallize hybrid GGUF
///
/// Example request:
/// ```json
/// {
///   "output_path": "D:\\Aaroneous\\models\\hybrid-v1.gguf",
///   "recipe": {
///     "recipe_id": "hybrid-v1",
///     "segments": [
///       {"source_gguf": "D:\\models\\qwen-a.gguf", "tensor_name": "blk.0.attn_q.weight"},
///       {"source_gguf": "D:\\models\\qwen-b.gguf", "tensor_name": "blk.0.mlp_gate.weight"}
///     ],
///     "metadata_overrides": {"general.name": {"type": "String", "value": "hybrid-v1"}}
///   },
///   "source_paths": ["D:\\models\\qwen-a.gguf", "D:\\models\\qwen-b.gguf"]
/// }
/// ```
async fn forge_crystallize(
    Json(req): Json<ForgeCrystallizeRequest>,
) -> impl IntoResponse {
    // Parse all source GGUFs into a combined index
    let mut combined_index = forge::GgufIndex::new();
    let mut parse_errors: Vec<String> = vec![];

    for src_path in &req.source_paths {
        match forge::read_gguf(src_path) {
            Ok((src_index, _meta)) => {
                // Merge into combined index
                for (key, meta) in src_index.0 {
                    combined_index.0.insert(key, meta);
                }
            }
            Err(e) => {
                parse_errors.push(format!("{}: {}", src_path, e));
            }
        }
    }

    if !parse_errors.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": "Failed to parse source GGUF(s)",
                "details": parse_errors,
            })),
        ).into_response();
    }

    // Crystallize
    let output_path = req.output_path.clone();
    let recipe = req.recipe.clone();
    let mut forge_instance = forge::Forge::new();

    match forge_instance.crystallize(&recipe, &combined_index, &output_path).await {
        Ok(result) => Json(serde_json::json!({
            "ok": true,
            "recipe_id": result.recipe_id,
            "output_path": result.output_path,
            "tensors_spliced": result.tensors_spliced,
            "bytes_written": result.bytes_written,
            "tensors": result.spliced_tensors.iter().map(|t| serde_json::json!({
                "source": t.source,
                "name": t.name,
                "size": t.size,
                "kind": t.kind,
            })).collect::<Vec<_>>(),
        })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        ).into_response(),
    }
}

/// Request body for POST /forge/crystallize-roster
#[derive(Deserialize)]
struct ForgeCrystallizeRosterRequest {
    /// Path to the source GGUF model
    source_path: String,
    /// Output directory for sovereign GGUFs (defaults to same dir as source)
    output_dir: Option<String>,
    /// Only crystallize specific sovereigns by name
    #[serde(default)]
    only: Vec<String>,
    /// Dry run — return the plan without writing files
    #[serde(default)]
    dry_run: bool,
}

/// POST /forge/crystallize-roster — crystallize all sovereigns from one base GGUF
///
/// Reads foundation_v1.gguf (or specified source) once and produces one
/// domain-specialized GGUF per sovereign, with calibrated layer selection
/// and identity metadata embedded in each output file.
///
/// Example request:
/// ```json
/// {
///   "source_path": "D:\\Aaroneous\\models\\foundation_v1.gguf",
///   "output_dir": "D:\\Aaroneous\\models",
///   "only": ["Ariel", "Merlin"]
/// }
/// ```
async fn forge_crystallize_roster(
    Json(req): Json<ForgeCrystallizeRosterRequest>,
) -> impl IntoResponse {
    let source = std::path::Path::new(&req.source_path);
    if !source.exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Source model not found: {}", req.source_path),
            })),
        ).into_response();
    }

    let models_dir = req.output_dir.as_deref()
        .map(std::path::Path::new)
        .or_else(|| source.parent())
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();

    // Parse source for block count
    let total_blocks = match forge::read_gguf(source) {
        Ok((_idx, meta)) => meta.kv.get("llama.block_count")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(28),
        Err(e) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        ).into_response(),
    };

    let mut profiles = forge::SovereignProfile::default_roster(total_blocks);

    // Filter sovereigns if requested
    if !req.only.is_empty() {
        let only_lower: Vec<String> = req.only.iter().map(|s| s.to_lowercase()).collect();
        profiles.retain(|p| only_lower.contains(&p.name.to_lowercase()));
    }

    if req.dry_run {
        let plan: Vec<serde_json::Value> = profiles.iter().map(|p| {
            let bc = p.block_selection.as_ref().map(|b| b.len())
                .or(p.block_count).unwrap_or(total_blocks);
            let size_estimate_mb = (bc as f64 / total_blocks as f64) * 4466.0;
            serde_json::json!({
                "name": p.name,
                "domain": p.domain,
                "output": p.output_filename,
                "blocks": bc,
                "estimated_mb": (size_estimate_mb as u32),
                "kinds": if p.include_kinds.is_empty() { "all".to_string() }
                         else { format!("{:?}", p.include_kinds) },
            })
        }).collect();
        return Json(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "source": req.source_path,
            "total_blocks": total_blocks,
            "sovereigns": plan,
        })).into_response();
    }

    // Run crystallization (this is long-running — consider SSE for progress)
    match forge::crystallize_roster(source, &models_dir, Some(profiles), None).await {
        Ok(result) => Json(serde_json::json!({
            "ok": true,
            "source": result.source_model,
            "output_dir": result.models_dir,
            "duration_secs": result.duration_secs,
            "succeeded": result.succeeded.len(),
            "failed": result.failed.len(),
            "sovereigns": result.succeeded.iter().map(|r| serde_json::json!({
                "name": r.name,
                "output": r.output_path,
                "tensors": r.tensors_included,
                "size_mb": r.size_mb as u32,
                "blocks": r.blocks_selected,
            })).collect::<Vec<_>>(),
            "errors": result.failed.iter().map(|(name, err)| serde_json::json!({
                "name": name, "error": err,
            })).collect::<Vec<_>>(),
        })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        ).into_response(),
    }
}

// ── Distillation endpoints ────────────────────────────────────────────────────

/// GET /distillation/plan
///
/// Returns the LoRA training plan for all 9 sovereigns: base model path,
/// adapter output, hyperparameters, training data path, and current status
/// (no-model / no-data / ready / done).
async fn distillation_plan(State(_state): State<AppState>) -> impl IntoResponse {
    use crate::federation::graph::distillation::generate_distillation_plan;

    // GGUFAnalyzer reads tensor headers (fast but synchronous) — run off the async executor
    let plans = tokio::task::spawn_blocking(|| {
        let models_dir = std::path::PathBuf::from("D:\\Aaroneous\\models");
        let data_dir   = std::path::PathBuf::from("D:\\Aaroneous\\training_data");
        generate_distillation_plan(&models_dir, &data_dir, None)
    }).await.unwrap_or_default();

    let items: Vec<serde_json::Value> = plans.iter().map(|p| {
        let model_exists = std::path::Path::new(&p.base_model_path).exists();
        let data_exists  = std::path::Path::new(&p.training_data_path).exists();
        let merged_exists = std::path::Path::new(&p.merged_gguf_output).exists();
        let status = if merged_exists { "done" }
            else if data_exists { "ready" }
            else if model_exists { "no-data" }
            else { "no-model" };

        serde_json::json!({
            "sovereign":           p.sovereign_name,
            "base_model":          p.base_model_path,
            "lora_adapter_output": p.lora_adapter_output,
            "merged_gguf_output":  p.merged_gguf_output,
            "training_data_path":  p.training_data_path,
            "lora_rank":           p.lora_rank,
            "lora_alpha":          p.lora_alpha,
            "num_epochs":          p.num_epochs,
            "batch_size":          p.batch_size,
            "max_seq_length":      p.max_seq_length,
            "estimated_vram_gb":   p.estimated_vram_gb,
            "estimated_hours":     p.estimated_training_hours,
            "status":              status,
            "notes":               p.notes,
        })
    }).collect();

    Json(serde_json::json!({ "ok": true, "plans": items }))
}

/// Request body for POST /distillation/generate
#[derive(Deserialize)]
struct DistillationGenerateRequest {
    /// Sovereign name (e.g. "Odin", "Wen")
    sovereign: String,
    /// Number of examples to generate (default: 50)
    count: Option<u32>,
}

/// POST /distillation/generate
///
/// Starts a background training-data generation job for a sovereign.
/// Returns immediately with a `job_id`. Poll `GET /distillation/jobs/:id`
/// for status and results.
///
/// CPU inference on a 7B model takes ~2-10 minutes per example.
/// This endpoint is always async — it never blocks.
///
/// Requires the server to be built with `--features llama-gguf` for real inference.
async fn distillation_generate(
    State(state): State<AppState>,
    Json(req): Json<DistillationGenerateRequest>,
) -> impl IntoResponse {
    use crate::llm::{LLMClient, LLMConfig};

    // Without llama-gguf: reject immediately — stub output is worthless.
    #[cfg(not(feature = "llama-gguf"))]
    {
        return Json(serde_json::json!({
            "ok": false,
            "error": "Real inference required. Rebuild: cargo build --features llama-gguf",
            "hint": "Without llama-gguf the GGUF provider returns stub text — training data would be useless.",
        })).into_response();
    }

    #[allow(unreachable_code)]
    {
        let count = req.count.unwrap_or(50).min(500);
        let sovereign = req.sovereign.clone();

        // Build the LLM client — use reduced max_tokens for CPU inference (128 vs 512)
        let llm_config = LLMConfig {
            provider_type: crate::llm::ProviderType::GGUF,
            gguf_model_path: Some(std::path::PathBuf::from("D:\\Aaroneous\\models\\foundation_v1.gguf")),
            max_tokens: 128,  // CPU-friendly: structured JSON outputs fit in 128 tokens
            ..Default::default()
        };

        let llm = match LLMClient::new(llm_config).await {
            Ok(c) => c,
            Err(e) => return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "ok": false,
                    "error": format!("Foundation model failed to load: {}", e),
                    "path": "D:\\Aaroneous\\models\\foundation_v1.gguf",
                })),
            ).into_response(),
        };

        // Generate a job ID and register the job as Running
        let job_id = format!("{}-{}", sovereign.to_lowercase(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0));

        {
            let mut jobs = state.generation_jobs.lock().await;
            jobs.insert(job_id.clone(), GenerationJobStatus::Running);
        }

        // Spawn the generation as a background task — returns immediately
        let jobs_arc = state.generation_jobs.clone();
        let job_id_bg = job_id.clone();
        tokio::task::spawn(async move {
            use crate::federation::graph::distillation::generate_training_examples;
            let training_data_dir = std::path::Path::new("D:\\Aaroneous\\training_data");
            let result = generate_training_examples(&sovereign, count, &llm, training_data_dir).await;
            let mut jobs = jobs_arc.lock().await;
            match result {
                Ok(report) => { jobs.insert(job_id_bg, GenerationJobStatus::Done(report)); }
                Err(e)     => { jobs.insert(job_id_bg, GenerationJobStatus::Failed(e.to_string())); }
            }
        });

        Json(serde_json::json!({
            "ok": true,
            "job_id": job_id,
            "sovereign": req.sovereign,
            "count": count,
            "status": "running",
            "poll": format!("/distillation/jobs/{}", job_id),
            "note": "CPU inference is slow (~2-10 min per example on 7B). Poll the job endpoint for results.",
        })).into_response()
    }
}

/// GET /distillation/jobs/:id
///
/// Poll a background training-data generation job started by POST /distillation/generate.
/// Returns status: "running", "done", or "failed".
async fn distillation_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let jobs = state.generation_jobs.lock().await;
    match jobs.get(&job_id) {
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "ok": false, "error": "Job not found", "job_id": job_id })),
        ).into_response(),
        Some(GenerationJobStatus::Running) => Json(serde_json::json!({
            "ok": true,
            "job_id": job_id,
            "status": "running",
            "note": "Still generating — CPU inference is slow. Check back in 30s.",
        })).into_response(),
        Some(GenerationJobStatus::Done(report)) => Json(serde_json::json!({
            "ok": true,
            "job_id": job_id,
            "status": "done",
            "sovereign":          report.sovereign,
            "examples_generated": report.examples_generated,
            "examples_saved":     report.examples_saved,
            "output_path":        report.output_path,
            "duration_secs":      report.duration_secs,
            "errors":             report.errors.len(),
            "inference_mode":     "llama-gguf (real foundation model inference)",
        })).into_response(),
        Some(GenerationJobStatus::Failed(err)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false, "job_id": job_id, "status": "failed", "error": err,
            })),
        ).into_response(),
    }
}

/// GET /distillation/analyze/:sovereign
///
/// Runs `GGUFAnalyzer` against a sovereign's crystallized GGUF and returns
/// the genome JSON: block structure, dominant weight type, layer distribution.
/// This data is what `genetics.rs` GeneticLocus values should be sourced from.
async fn distillation_analyze(
    State(_state): State<AppState>,
    Path(sovereign): Path<String>,
) -> impl IntoResponse {
    use crate::federation::graph::analyzer::GGUFAnalyzer;

    let model_path = std::path::PathBuf::from(format!(
        "D:\\Aaroneous\\models\\{}-qwen2.5-7b.gguf",
        sovereign.to_lowercase()
    ));

    if !model_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Model not found: {}", model_path.display()),
                "expected": model_path.display().to_string(),
            })),
        ).into_response();
    }

    // GGUFAnalyzer is synchronous (reads file headers) — run off the async executor.
    // stringify the error inside the closure so Result is Send.
    let sov_clone = sovereign.clone();
    let result = tokio::task::spawn_blocking(move || {
        let analyzer = GGUFAnalyzer::default();
        analyzer.analyze(&model_path)
            .map(|a| (a, sov_clone))
            .map_err(|e| e.to_string())
    }).await;

    match result {
        Ok(Ok((analysis, sov))) => {
            let genome = crate::federation::graph::analysis_to_genome_json(&analysis, &sov);
            Json(serde_json::json!({
                "ok": true,
                "sovereign": sovereign,
                "model_path": format!("D:\\Aaroneous\\models\\{}-qwen2.5-7b.gguf", sovereign.to_lowercase()),
                "analysis": {
                    "model_name": analysis.model_name,
                    "architecture": analysis.architecture,
                    "total_parameters_estimate": analysis.total_parameters_estimate,
                    "tensor_count": analysis.tensor_count,
                    "total_blocks": analysis.total_blocks,
                    "overall_sparsity": analysis.overall_sparsity,
                    "attn_mlp_ratio": analysis.attn_mlp_ratio,
                    "depth_gradient": analysis.depth_gradient,
                },
                "genome": genome,
            })).into_response()
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "ok": false, "error": e })),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "ok": false, "error": format!("spawn_blocking panicked: {}", e) })),
        ).into_response(),
    }
}

// ── Memory stats endpoint ─────────────────────────────────────────────────────

/// GET /memory/stats
///
/// Returns RAG memory counts for the federation and each active sovereign.
/// Used by MaelstromUI to show the "memories" badge on sovereign cards.
async fn memory_stats(State(state): State<AppState>) -> impl IntoResponse {
    let fed = &state.federation;
    let federation_total = fed.federation_memory.lock().await.total_count();

    let dynamic = fed.dynamic.read().await;
    let per_sovereign: Vec<serde_json::Value> = dynamic.iter().map(|s| {
        let count = s.memory.lock().count_for(&s.name);
        serde_json::json!({
            "name":         s.name,
            "domain":       s.domain,
            "memory_count": count,
        })
    }).collect();

    Json(serde_json::json!({
        "ok":               true,
        "federation_total": federation_total,
        "per_sovereign":    per_sovereign,
    }))
}

// ── Distillation script endpoint ──────────────────────────────────────────────

/// GET /distillation/script/:sovereign
///
/// Returns the generated unsloth Python training script for a sovereign as
/// plain text. Save to disk and run:  python wen_train.py
///
/// The script reads `training_data/<sovereign>-training.jsonl` (Alpaca format),
/// applies LoRA to the crystallized GGUF, and saves the merged model to
/// `models/<sovereign>-distilled.gguf`.
///
/// Requires: pip install unsloth torch datasets trl transformers
async fn distillation_script(
    State(_state): State<AppState>,
    Path(sovereign): Path<String>,
) -> impl IntoResponse {
    use crate::federation::graph::distillation::generate_distillation_plan;
    use axum::http::header;

    // Find the LoRA spec for this sovereign
    let models_dir = std::path::PathBuf::from("D:\\Aaroneous\\models");
    let data_dir   = std::path::PathBuf::from("D:\\Aaroneous\\training_data");

    let plans = tokio::task::spawn_blocking(move || {
        generate_distillation_plan(&models_dir, &data_dir, None)
    }).await.unwrap_or_default();

    let plan = plans.into_iter()
        .find(|p| p.sovereign_name.to_lowercase() == sovereign.to_lowercase());

    match plan {
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("No distillation plan found for sovereign '{}'", sovereign),
            })),
        ).into_response(),
        Some(spec) => {
            let script = spec.to_unsloth_script();
            let filename = format!("{}_train.py", spec.sovereign_name.to_lowercase());
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE,        "text/plain; charset=utf-8".to_string()),
                    (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename)),
                ],
                script,
            ).into_response()
        }
    }
}

// ── DNA dissection endpoints ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct DnaDissectRequest {
    /// Path to the GGUF model to dissect (e.g. "D:\\Aaroneous\\models\\foundation_v1.gguf")
    /// or just a filename to look up in D:\Aaroneous\models\
    model: String,
    /// If true and a .dna.json sidecar exists, return the cached result immediately
    use_cache: Option<bool>,
}

/// POST /dna/dissect
///
/// Start a background deep structural dissection of a GGUF model.
/// Returns a job_id immediately. Poll GET /dna/jobs/:id for status.
///
/// The dissection reads tensor bytes via memory-mapped I/O (safe for 4GB+ models)
/// and produces a full ModelDNA record: per-block weight statistics, gate sparsity,
/// embedding topology, cross-block correlation, and genetic loci.
async fn dna_dissect(
    State(state): State<AppState>,
    Json(req): Json<DnaDissectRequest>,
) -> impl IntoResponse {
    use crate::federation::dna::{dissect_model, load_dna_sidecar, DissectionJobStatus};

    // Resolve path — accept either absolute path or filename in models dir
    let model_path = if std::path::Path::new(&req.model).is_absolute() {
        std::path::PathBuf::from(&req.model)
    } else {
        std::path::PathBuf::from(format!("D:\\Aaroneous\\models\\{}", req.model))
    };

    if !model_path.exists() {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "ok": false,
            "error": format!("Model not found: {}", model_path.display()),
        }))).into_response();
    }

    // Return cached sidecar if requested and available
    if req.use_cache.unwrap_or(true) {
        if let Some(dna) = load_dna_sidecar(&model_path) {
            return Json(serde_json::json!({
                "ok": true,
                "source": "cache",
                "model": dna.model_name,
                "loci_count": dna.genetic_loci.len(),
                "blocks": dna.num_blocks,
                "dissected_at": dna.dissected_at,
                "dna": dna,
            })).into_response();
        }
    }

    // Start background dissection job
    let job_id = format!("dna-{}-{}",
        model_path.file_stem().and_then(|s| s.to_str()).unwrap_or("model"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis()).unwrap_or(0));

    let jobs_arc = state.dissection_jobs.clone();
    let job_id_bg = job_id.clone();
    let model_name = model_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

    {
        let mut jobs = jobs_arc.lock().await;
        jobs.insert(job_id.clone(), DissectionJobStatus::Running {
            progress: crate::federation::dna::DissectionProgress {
                model: model_name.clone(),
                stage: crate::federation::dna::DissectionStage::ReadingHeader,
                blocks_done: 0,
                blocks_total: 0,
                percent: 0,
                message: "Starting dissection…".into(),
            }
        });
    }

    tokio::task::spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        // Update job status on each progress event
        let jobs_progress = jobs_arc.clone();
        let job_id_prog = job_id_bg.clone();
        tokio::spawn(async move {
            while let Some(progress) = rx.recv().await {
                let mut jobs = jobs_progress.lock().await;
                jobs.insert(job_id_prog.clone(),
                    DissectionJobStatus::Running { progress });
            }
        });

        let result = dissect_model(&model_path, Some(tx)).await;
        let mut jobs = jobs_arc.lock().await;
        match result {
            Ok(dna) => { jobs.insert(job_id_bg, DissectionJobStatus::Done(Box::new(dna))); }
            Err(e)  => { jobs.insert(job_id_bg, DissectionJobStatus::Failed(e.to_string())); }
        }
    });

    Json(serde_json::json!({
        "ok": true,
        "job_id": job_id,
        "model": model_name,
        "status": "running",
        "poll": format!("/dna/jobs/{}", job_id),
    })).into_response()
}

/// GET /dna/jobs/:id
///
/// Poll a background DNA dissection job started by POST /dna/dissect.
async fn dna_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    use crate::federation::dna::DissectionJobStatus;
    let jobs = state.dissection_jobs.lock().await;
    match jobs.get(&job_id) {
        None => (StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "ok": false, "error": "Job not found", "job_id": job_id }))).into_response(),
        Some(DissectionJobStatus::Running { progress }) => Json(serde_json::json!({
            "ok": true, "job_id": &job_id, "status": "running",
            "stage": format!("{:?}", progress.stage),
            "percent": progress.percent,
            "blocks_done": progress.blocks_done,
            "blocks_total": progress.blocks_total,
            "message": &progress.message,
        })).into_response(),
        Some(DissectionJobStatus::Done(dna)) => Json(serde_json::json!({
            "ok": true, "job_id": &job_id, "status": "done",
            "model": &dna.model_name,
            "loci_count": dna.genetic_loci.len(),
            "blocks": dna.num_blocks,
            "parameter_count_m": dna.parameter_count_m,
            "dissection_duration_secs": dna.dissection_duration_secs,
            "splice_boundary": dna.splice_boundary,
            "dna": *dna.clone(),
        })).into_response(),
        Some(DissectionJobStatus::Failed(err)) => (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "ok": false, "job_id": &job_id, "status": "failed", "error": err }))).into_response(),
    }
}

/// GET /dna/genome/:model
///
/// Return the cached DNA sidecar for a model (if dissection has been run).
/// :model is the filename (e.g. "foundation_v1.gguf") or "foundation_v1".
async fn dna_genome(
    State(_state): State<AppState>,
    Path(model): Path<String>,
) -> impl IntoResponse {
    use crate::federation::dna::load_dna_sidecar;

    let filename = if model.ends_with(".gguf") { model.clone() }
        else { format!("{}.gguf", model) };
    let model_path = std::path::PathBuf::from(format!("D:\\Aaroneous\\models\\{}", filename));

    match load_dna_sidecar(&model_path) {
        Some(dna) => Json(serde_json::json!({ "ok": true, "dna": dna })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "ok": false,
            "error": format!("No DNA sidecar found for {}. Run POST /dna/dissect first.", filename),
        }))).into_response(),
    }
}

#[derive(Deserialize)]
struct DnaCompareRequest {
    model_a: String,
    model_b: String,
}

/// POST /dna/compare
///
/// Compare the DNA of two dissected models and return genetic distance metrics.
async fn dna_compare(
    State(_state): State<AppState>,
    Json(req): Json<DnaCompareRequest>,
) -> impl IntoResponse {
    use crate::federation::dna::{load_dna_sidecar, dna_to_genome};
    use crate::genetics::GeneticAnalyzer;

    let resolve = |name: &str| -> std::path::PathBuf {
        let filename = if name.ends_with(".gguf") { name.to_string() } else { format!("{}.gguf", name) };
        std::path::PathBuf::from(format!("D:\\Aaroneous\\models\\{}", filename))
    };

    let path_a = resolve(&req.model_a);
    let path_b = resolve(&req.model_b);

    let dna_a = load_dna_sidecar(&path_a);
    let dna_b = load_dna_sidecar(&path_b);

    match (dna_a, dna_b) {
        (Some(a), Some(b)) => {
            let genome_a = dna_to_genome(&a);
            let genome_b = dna_to_genome(&b);
            let diversity = GeneticAnalyzer::population_diversity(&[genome_a, genome_b]);

            // Per-locus comparison
            // Strip model-name prefix from locus IDs for cross-model comparison.
            // Locus ID format: "<model_prefix>-<locus_key>" where model_prefix is
            // e.g. "foundation_v1" or "odin-qwen2.5-7b".
            // We normalise by keeping only the locus_key (everything after the last
            // double-letter token that identifies the measurement type).
            let locus_key = |id: &str, model_name: &str| -> String {
                // model_name may contain dots (e.g. "wen-qwen2.5-7b.gguf") — escape dots
                let prefix_clean = model_name.replace('.', "_").to_lowercase();
                id.to_lowercase()
                    .trim_start_matches(&prefix_clean)
                    .trim_matches('-')
                    .to_string()
            };

            let loci_diff: Vec<serde_json::Value> = a.genetic_loci.iter()
                .filter_map(|la| {
                    let la_key = locus_key(&la.locus_id, &a.model_name);
                    b.genetic_loci.iter()
                        .find(|lb| locus_key(&lb.locus_id, &b.model_name) == la_key)
                        .map(|lb| serde_json::json!({
                            "locus": la_key,
                            "a": la.value,
                            "b": lb.value,
                            "delta": (lb.value - la.value).abs(),
                            "direction": if lb.value > la.value { "b_higher" } else { "a_higher" },
                            "category": la.category,
                        }))
                })
                .collect();

            // Compute distance on normalised loci (strip model prefix from IDs)
            let genome_a_norm = dna_to_genome(&a);
            let genome_b_norm = dna_to_genome(&b);
            // Override locus IDs to use keys only so GeneticAnalyzer can match them
            use crate::genetics::{SpecialistGenome, GeneticLocus};
            let mk_normalised = |dna: &crate::federation::dna::ModelDNA| {
                let mut g = SpecialistGenome::new(dna.model_name.clone(), dna.model_name.clone(), dna.model_path.clone());
                for rec in &dna.genetic_loci {
                    let key = locus_key(&rec.locus_id, &dna.model_name);
                    let cat = crate::federation::dna::parse_category_pub(&rec.category);
                    let src = crate::federation::dna::parse_source_pub(&rec.source);
                    let locus = GeneticLocus::new(key, cat, rec.value.clamp(0.0, 1.0), src);
                    g.add_locus(locus);
                }
                g
            };
            let gn_a = mk_normalised(&a);
            let gn_b = mk_normalised(&b);
            let distance = crate::genetics::GeneticAnalyzer::distance(&gn_a, &gn_b);

            Json(serde_json::json!({
                "ok": true,
                "model_a": a.model_name,
                "model_b": b.model_name,
                "genetic_distance": distance,
                "population_diversity": diversity,
                "splice_boundaries": {
                    "model_a": a.splice_boundary,
                    "model_b": b.splice_boundary,
                },
                "recommended_splice": if a.splice_boundary != b.splice_boundary {
                    format!("Take blocks 0-{} from {} and {}-end from {}",
                        a.splice_boundary, a.model_name, a.splice_boundary, b.model_name)
                } else {
                    "Both models have the same splice boundary — consider alternating blocks".into()
                },
                "loci_comparison": loci_diff,
            })).into_response()
        }
        (None, _) => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "ok": false, "error": format!("{} has no DNA sidecar. Run POST /dna/dissect first.", req.model_a)
        }))).into_response(),
        (_, None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "ok": false, "error": format!("{} has no DNA sidecar. Run POST /dna/dissect first.", req.model_b)
        }))).into_response(),
    }
}

/// GET /dna/roster
///
/// Returns the DNA status for all 9 sovereign GGUFs + foundation model.
/// Shows which models have been dissected (have a sidecar), their key genome metrics,
/// and the recommended ForgeRecipe splice points between each pair.
async fn dna_roster(State(_state): State<AppState>) -> impl IntoResponse {
    use crate::federation::dna::load_dna_sidecar;

    let models_dir = std::path::Path::new("D:\\Aaroneous\\models");
    let known_models = [
        "foundation_v1", "ariel-qwen2.5-7b", "hermes-qwen2.5-7b",
        "wen-qwen2.5-7b", "kami-qwen2.5-7b", "dionysus-qwen2.5-7b",
        "merlin-qwen2.5-7b", "odin-qwen2.5-7b", "argus-qwen2.5-7b",
        "hephaestus-qwen2.5-7b",
    ];

    let entries: Vec<serde_json::Value> = known_models.iter().map(|name| {
        let path = models_dir.join(format!("{}.gguf", name));
        let exists = path.exists();
        let size_mb = if exists {
            std::fs::metadata(&path).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0)
        } else { 0.0 };

        let dna = if exists { load_dna_sidecar(&path) } else { None };
        let dissected = dna.is_some();

        serde_json::json!({
            "model": name,
            "file_exists": exists,
            "size_mb": size_mb as u64,
            "dissected": dissected,
            "loci_count": dna.as_ref().map(|d| d.genetic_loci.len()).unwrap_or(0),
            "blocks": dna.as_ref().map(|d| d.num_blocks).unwrap_or(0),
            "splice_boundary": dna.as_ref().map(|d| d.splice_boundary).unwrap_or(0),
            "gate_sparsity": dna.as_ref().and_then(|d| d.genome_loci.get("gate_sparsity").copied()),
            "attn_mlp_ratio": dna.as_ref().and_then(|d| d.genome_loci.get("attn_mlp_ratio").copied()),
            "dna_fingerprint": dna.as_ref().map(|d| format!("{:016x}", d.dna_fingerprint)),
            "dissect_endpoint": format!("POST /dna/dissect {{\"model\":\"{}.gguf\"}}", name),
        })
    }).collect();

    let dissected_count = entries.iter().filter(|e| e["dissected"].as_bool().unwrap_or(false)).count();

    Json(serde_json::json!({
        "ok": true,
        "total_models": known_models.len(),
        "dissected": dissected_count,
        "pending": known_models.len() - dissected_count,
        "models": entries,
    }))
}

// ── Model import/export endpoints ─────────────────────────────────────────────

#[derive(Deserialize)]
struct ModelsImportRequest {
    /// Source: absolute path, "hf://owner/repo/file.gguf", or "owner/repo"
    source: String,
    /// Tags for this model (e.g. ["research", "120b"])
    tags: Option<Vec<String>>,
    /// Auto-run DNA dissection after import (default: true)
    auto_dissect: Option<bool>,
    /// Auto-register as a dynamic sovereign specialist (default: false)
    auto_register_sovereign: Option<bool>,
}

/// POST /models/import
///
/// Import a GGUF model from a local path or HuggingFace.
///
/// Examples:
///   {"source": "D:\\models\\llama-3.1-70b-q4.gguf"}
///   {"source": "hf://bartowski/Meta-Llama-3.1-70B-Instruct-GGUF/Meta-Llama-3.1-70B-Instruct-Q4_K_M.gguf"}
///   {"source": "bartowski/Meta-Llama-3.1-70B-Instruct-GGUF", "tags": ["research", "70b"]}
///
/// Returns a job_id immediately. Poll GET /models/import/jobs/:id for progress.
async fn models_import(
    State(state): State<AppState>,
    Json(req): Json<ModelsImportRequest>,
) -> impl IntoResponse {
    use crate::federation::model_registry::import_model;

    let job_id = import_model(
        req.source.clone(),
        req.tags.unwrap_or_default(),
        req.auto_dissect.unwrap_or(true),
        req.auto_register_sovereign.unwrap_or(false),
        state.import_jobs.clone(),
    ).await;

    Json(serde_json::json!({
        "ok": true,
        "job_id": job_id,
        "source": req.source,
        "status": "running",
        "poll": format!("/models/import/jobs/{}", job_id),
        "note": "Large model downloads may take many minutes. Poll for status.",
    }))
}

/// GET /models/import/jobs/:id
///
/// Poll a model import job started by POST /models/import.
async fn models_import_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    use crate::federation::model_registry::ImportStatus;
    let jobs = state.import_jobs.lock().await;
    match jobs.get(&job_id) {
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "ok": false, "error": "Job not found", "job_id": job_id,
        }))).into_response(),
        Some(job) => {
            let status_str = match &job.status {
                ImportStatus::Downloading { percent, bytes_done, bytes_total } =>
                    serde_json::json!({ "stage": "downloading", "percent": percent,
                        "bytes_done": bytes_done, "bytes_total": bytes_total }),
                ImportStatus::Copying =>
                    serde_json::json!({ "stage": "copying" }),
                ImportStatus::Dissecting { percent } =>
                    serde_json::json!({ "stage": "dissecting", "percent": percent }),
                ImportStatus::Registering =>
                    serde_json::json!({ "stage": "registering" }),
                ImportStatus::Done =>
                    serde_json::json!({ "stage": "done" }),
                ImportStatus::Failed(e) =>
                    serde_json::json!({ "stage": "failed", "error": e }),
            };
            Json(serde_json::json!({
                "ok": true,
                "job_id": job_id,
                "model": job.model_name,
                "status": status_str,
            })).into_response()
        }
    }
}

/// GET /models/export/:name
///
/// Stream a GGUF model file as a binary download.
///
/// :name is the filename (e.g. "foundation_v1.gguf") or just "foundation_v1".
/// The response has Content-Type: application/octet-stream and
/// Content-Disposition: attachment so curl/browsers save it as a file.
///
/// This is how you export a sovereign specialist for use elsewhere:
///   curl http://localhost:8765/models/export/odin-qwen2.5-7b.gguf -o odin.gguf
async fn models_export(
    State(_state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    use axum::http::header;
    use axum::body::Body;
    use tokio_util::io::ReaderStream;

    let filename = if name.ends_with(".gguf") { name.clone() }
        else { format!("{}.gguf", name) };
    let model_path = std::path::PathBuf::from(format!("D:\\Aaroneous\\models\\{}", filename));

    if !model_path.exists() {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "ok": false,
            "error": format!("Model not found: {}", filename),
        }))).into_response();
    }

    let file = match tokio::fs::File::open(&model_path).await {
        Ok(f) => f,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "ok": false, "error": format!("Failed to open model: {}", e),
        }))).into_response(),
    };

    let size = model_path.metadata().map(|m| m.len()).unwrap_or(0);
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE,        "application/octet-stream".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename)),
            (header::CONTENT_LENGTH,      size.to_string()),
        ],
        body,
    ).into_response()
}

/// GET /models/registry
///
/// Return the full model registry — all known GGUFs with metadata, DNA status,
/// and sovereign associations.
async fn models_registry(State(_state): State<AppState>) -> impl IntoResponse {
    use crate::federation::model_registry::scan_models_dir;

    let entries = scan_models_dir();
    let total_size_gb: f64 = entries.iter().map(|e| e.size_bytes as f64 / 1_073_741_824.0).sum();
    let dissected_count = entries.iter().filter(|e| e.dna_dissected).count();

    let models: Vec<serde_json::Value> = entries.iter().map(|e| serde_json::json!({
        "name":         e.name,
        "path":         e.path.to_string_lossy(),
        "size_bytes":   e.size_bytes,
        "size_mb":      e.size_bytes / 1_048_576,
        "dna_dissected": e.dna_dissected,
        "sovereign":    e.sovereign,
        "tags":         e.tags,
        "export_url":   format!("/models/export/{}", e.name),
        "dissect_url":  format!("POST /dna/dissect {{\"model\":\"{}\"}}", e.name),
    })).collect();

    Json(serde_json::json!({
        "ok": true,
        "model_count":    entries.len(),
        "dissected_count": dissected_count,
        "total_size_gb":  (total_size_gb * 10.0).round() / 10.0,
        "models_dir":     "D:\\Aaroneous\\models\\",
        "inbox_dir":      "D:\\Aaroneous\\models\\inbox\\",
        "models":         models,
        "import_endpoint": "POST /models/import {\"source\": \"path/or/hf://...\"}",
    }))
}
