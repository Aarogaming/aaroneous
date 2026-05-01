/// Axum router definition for the federation HTTP status API.
///
/// The router is factored out from the server so tests can drive it
/// in-process via `tower::ServiceExt::oneshot` without binding a real port.

use crate::federation::hive::{Federation, LearningSummary, SpecialistLearningSummary};
use crate::federation::forge;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse, Json},
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::convert::Infallible;

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
        // Forge: GGUF inspection, auto-recipe generation, and crystallization
        .route("/forge/inspect",          post(forge_inspect))
        .route("/forge/auto-recipe",      post(forge_auto_recipe))
        .route("/forge/single-recipe",    post(forge_single_recipe))
        .route("/forge/crystallize",      post(forge_crystallize))
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
        "specialist": r.specialist_name.as_deref().unwrap_or_else(|| r.specialist.name()),
        "specialist_id": format!("{:?}", r.specialist),
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
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        ).into_response(),
    }
}
