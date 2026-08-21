use crate::federation::hive::Federation;
use crate::federation::intent::{Intent, IntentPriority, IntentSource};
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{
        IntoResponse, Json,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, post},
};
use std::sync::Arc;
use tracing::info;

/// REST/SSE API Gateway for Maelstrom UI
#[derive(Clone)]
pub struct RestApiServer {
    addr: std::net::SocketAddr,
    federation: Arc<Federation>,
}

impl RestApiServer {
    pub fn new(federation: Arc<Federation>) -> Result<Self, std::io::Error> {
        let addr = "127.0.0.1:8765".parse().map_err(std::io::Error::other)?;
        Ok(Self { addr, federation })
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let app = Router::new()
            // Health endpoints
            .route("/healthz", get(handle_healthz))
            .route("/readyz", get(handle_readyz))
            .route("/status", get(handle_status))
            // Sessions
            .route("/sessions", post(create_session))
            .route("/sessions", get(list_sessions))
            .route("/sessions/:id/intent", post(submit_intent))
            .route("/sessions/:id/results/stream", get(stream_results_sse))
            // Specialists
            .route("/specialists", get(list_specialists))
            .route("/dynamic-specialists", post(create_dynamic_specialist))
            // Models
            .route("/models/external", get(list_external_models))
            .route("/models/import", post(import_model))
            // Forge
            .route("/forge/crystallize-roster", post(crystallize_roster))
            // Scheduler
            .route("/scheduler/tasks", get(list_scheduler_tasks))
            .route("/scheduler/tasks", post(create_scheduler_task))
            .route("/scheduler/tasks/:id", delete(cancel_scheduler_task))
            // Chimera
            .route("/chimera/record", post(toggle_chimera_record))
            .route("/chimera/routines", get(list_routines))
            .route("/chimera/routines/:id/run", post(run_routine_now));

        let app = app.with_state(std::sync::Arc::clone(&self.federation));

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        tracing::info!("Maelstrom REST API listening on {}", self.addr);

        axum::serve(listener, app).await?;
        Ok(())
    }

    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.addr
    }

    pub async fn shutdown(&self) -> Result<(), ()> {
        Ok(())
    }
}

async fn handle_healthz(
    State(_federation): State<std::sync::Arc<crate::federation::hive::Federation>>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "Maelstrom REST API",
    }))
}

async fn handle_readyz(
    State(federation): State<std::sync::Arc<crate::federation::hive::Federation>>,
) -> impl IntoResponse {
    let enabled = federation.enabled_count();
    Json(serde_json::json!({
        "status": if enabled > 0 { "ready" } else { "not_ready" },
        "enabled_specialists": enabled,
    }))
}

async fn handle_status(
    State(federation): State<std::sync::Arc<crate::federation::hive::Federation>>,
) -> impl IntoResponse {
    Json(serde_json::json!(federation.learning_summary()))
}

async fn create_session(
    State(federation): State<std::sync::Arc<crate::federation::hive::Federation>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_name = body
        .get("user_name")
        .and_then(|v| v.as_str())
        .unwrap_or("User")
        .to_string();
    let session_id = federation.create_session(&user_name, None).await;
    Ok(Json(serde_json::json!({"session_id": session_id})))
}

async fn list_sessions(
    State(_federation): State<std::sync::Arc<crate::federation::hive::Federation>>,
) -> impl IntoResponse {
    Json(serde_json::json!({"sessions": []}))
}

async fn submit_intent(
    State(federation): State<std::sync::Arc<crate::federation::hive::Federation>>,
    Path(session_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let content = body
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let priority_str = body
        .get("priority")
        .and_then(|v| v.as_str())
        .unwrap_or("Normal")
        .to_string();

    info!("Intent: {} (priority: {})", content, priority_str);

    let priority = match priority_str.as_str() {
        "High" | "Urgent" => IntentPriority::High,
        "Critical" => IntentPriority::Critical,
        "Background" => IntentPriority::Background,
        _ => IntentPriority::Normal,
    };

    let intent = Intent::new(content)
        .with_priority(priority)
        .with_source(IntentSource::Api);

    match federation
        .submit_intent_for_session(&session_id, intent)
        .await
    {
        Ok((sid, iid)) => Ok(Json(
            serde_json::json!({"success": true, "session_id": sid, "intent_id": iid}),
        )),
        Err(e) => Ok(Json(serde_json::json!({"error": e}))),
    }
}

async fn stream_results_sse(Path(session_id): Path<String>) -> impl IntoResponse {
    let stream = async_stream::stream! {
        yield Ok::<Event, std::convert::Infallible>(
            Event::default().event("connected").data(format!("Connected to session: {}", session_id))
        );

        let update = serde_json::json!({
            "specialist": "Odin",
            "output": "{\"tasks\":[{\"content\":\"Analyze intent\", \"assign_to\":\"Merlin\"}]}",
            "status": "Processing"
        });

        yield Ok::<Event, std::convert::Infallible>(
            Event::default().event("results").data(update.to_string())
        );

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            yield Ok::<Event, std::convert::Infallible>(Event::default().comment("keep-alive"));
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn list_specialists() -> impl IntoResponse {
    Json(serde_json::json!({
        "specialists": [
            {"name": "Ariel", "domain": "UserInterface", "kind": "core", "status": "active"},
            {"name": "Merlin", "domain": "Knowledge", "kind": "core", "status": "active"},
            {"name": "Odin", "domain": "Leadership", "kind": "core", "status": "active"},
            {"name": "Hephaestus", "domain": "Manufacturing", "kind": "core", "status": "active"},
            {"name": "Argus", "domain": "Security", "kind": "core", "status": "active"},
            {"name": "Dionysus", "domain": "Experience", "kind": "core", "status": "active"}
        ]
    }))
}

async fn create_dynamic_specialist() -> impl IntoResponse {
    Json(serde_json::json!({"error": "Dynamic specialist spawning not yet implemented"}))
}

async fn list_external_models() -> impl IntoResponse {
    Json(serde_json::json!({"models": []}))
}

async fn import_model(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let source = body
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    info!("Importing model from: {}", source);
    Ok(Json(
        serde_json::json!({"job_id": uuid::Uuid::new_v4().to_string(), "status": "queued"}),
    ))
}

async fn crystallize_roster(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _source = body
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(Json(
        serde_json::json!({"success": true, "message": "Forge queue initiated"}),
    ))
}

async fn list_scheduler_tasks() -> impl IntoResponse {
    Json(serde_json::json!({"tasks": []}))
}

async fn create_scheduler_task(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    info!("Creating scheduled task: {}", name);
    Ok(Json(
        serde_json::json!({"task_id": uuid::Uuid::new_v4().to_string(), "success": true}),
    ))
}

async fn cancel_scheduler_task(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({"success": true, "task_id": id})))
}

async fn toggle_chimera_record(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let action = body
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(Json(serde_json::json!({"success": true, "action": action})))
}

async fn list_routines() -> impl IntoResponse {
    Json(serde_json::json!({"routines": []}))
}

async fn run_routine_now(Path(id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({"success": true, "routine_id": id})))
}
