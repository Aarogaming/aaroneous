/// Integration tests for the federation HTTP status API.
///
/// We use `tower::ServiceExt::oneshot` to drive the axum router in-process
/// without binding a real port. This is much faster than real-TCP testing
/// and equally exercises the routes, handlers, and serialization.

#[cfg(test)]
mod tests {
    use super::super::router::{router, AppState, StatusEnvelope};
    use crate::federation::hive::{Federation, SpecialistLearningSummary};
    use crate::federation::specialist::{
        Decision, ResourceRequest, Specialist, SpecialistId,
    };
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn fresh_federation_with_all() -> Arc<Federation> {
        Arc::new(
            Federation::builder(
                crate::persistence::PersistenceManager::new(":memory:").unwrap(),
            )
            .with_all()
            .build(),
        )
    }

    fn fresh_federation_partial() -> Arc<Federation> {
        Arc::new(
            Federation::builder(
                crate::persistence::PersistenceManager::new(":memory:").unwrap(),
            )
            .with_visionary()
            .with_archivist()
            .build(),
        )
    }

    fn empty_federation() -> Arc<Federation> {
        Arc::new(
            Federation::builder(
                crate::persistence::PersistenceManager::new(":memory:").unwrap(),
            )
            .build(),
        )
    }

    fn make_decision(kind: SpecialistId, idx: usize) -> Decision {
        Decision {
            proposal_id: format!("p-{}", idx),
            specialist: kind,
            action: "test".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: HashMap::new(),
        }
    }

    /// Helper: send a GET request through the router and return (status, body bytes).
    async fn get(fed: Arc<Federation>, path: &str) -> (StatusCode, Vec<u8>) {
        let app = router(AppState::new(fed));
        let req = Request::builder()
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.expect("router oneshot should succeed");
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes()
            .to_vec();
        (status, body)
    }

    // =================================================================
    // /healthz
    // =================================================================

    #[tokio::test]
    async fn test_healthz_returns_ok_text() {
        let fed = fresh_federation_with_all();
        let (status, body) = get(fed, "/healthz").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn test_healthz_returns_ok_even_for_empty_federation() {
        // Liveness shouldn't depend on whether specialists are configured
        let fed = empty_federation();
        let (status, _) = get(fed, "/healthz").await;
        assert_eq!(status, StatusCode::OK);
    }

    // =================================================================
    // /readyz
    // =================================================================

    #[tokio::test]
    async fn test_readyz_returns_503_for_empty_federation() {
        let fed = empty_federation();
        let (status, body) = get(fed, "/readyz").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        let body_str = String::from_utf8(body).unwrap();
        assert!(body_str.to_lowercase().contains("no specialists"));
    }

    #[tokio::test]
    async fn test_readyz_returns_200_for_configured_federation() {
        let fed = fresh_federation_with_all();
        let (status, body) = get(fed, "/readyz").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"ready");
    }

    #[tokio::test]
    async fn test_readyz_returns_200_for_partial_federation() {
        // Even one specialist makes the federation "ready"
        let fed = fresh_federation_partial();
        let (status, _) = get(fed, "/readyz").await;
        assert_eq!(status, StatusCode::OK);
    }

    // =================================================================
    // /status (full snapshot)
    // =================================================================

    #[tokio::test]
    async fn test_status_returns_full_envelope() {
        let fed = fresh_federation_with_all();
        let (status, body) = get(fed, "/status").await;
        assert_eq!(status, StatusCode::OK);

        let envelope: StatusEnvelope = serde_json::from_slice(&body).expect("valid JSON");
        assert_eq!(envelope.enabled_count, 5);
        assert_eq!(envelope.total_executions, 0);
        assert_eq!(envelope.total_successes, 0);
        assert!(envelope.specialists.visionary.is_some());
        assert!(envelope.specialists.archivist.is_some());
    }

    #[tokio::test]
    async fn test_status_for_partial_federation_omits_unconfigured() {
        let fed = fresh_federation_partial();
        let (status, body) = get(fed, "/status").await;
        assert_eq!(status, StatusCode::OK);

        let envelope: StatusEnvelope = serde_json::from_slice(&body).unwrap();
        assert_eq!(envelope.enabled_count, 2);
        assert!(envelope.specialists.visionary.is_some());
        assert!(envelope.specialists.archivist.is_some());
        assert!(envelope.specialists.omnipresent.is_none());
        assert!(envelope.specialists.symbiotic.is_none());
        assert!(envelope.specialists.phygital.is_none());
    }

    #[tokio::test]
    async fn test_status_reflects_executions() {
        let fed = fresh_federation_with_all();
        fed.start_all().await.unwrap();

        // Train Visionary 3 times
        for i in 0..3 {
            fed.visionary().unwrap()
                .execute(&make_decision(SpecialistId::Visionary, i))
                .await
                .unwrap();
        }

        let (status, body) = get(fed.clone(), "/status").await;
        assert_eq!(status, StatusCode::OK);

        let envelope: StatusEnvelope = serde_json::from_slice(&body).unwrap();
        assert_eq!(envelope.total_executions, 3);
        let v = envelope.specialists.visionary.expect("Visionary present");
        assert_eq!(v.success_count, 3);
        assert_eq!(v.total_executions, 3);
        assert!((v.success_rate_percent() - 100.0).abs() < 0.01);

        fed.shutdown_all().await.unwrap();
    }

    // =================================================================
    // /status/{kind}
    // =================================================================

    #[tokio::test]
    async fn test_status_one_returns_specific_specialist() {
        let fed = fresh_federation_with_all();
        let (status, body) = get(fed, "/status/Visionary").await;
        assert_eq!(status, StatusCode::OK);

        let summary: SpecialistLearningSummary = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary.total_executions, 0);
        assert_eq!(summary.confidence_score, 0.5);
    }

    #[tokio::test]
    async fn test_status_one_is_case_insensitive() {
        let fed = fresh_federation_with_all();
        let (status, _) = get(fed.clone(), "/status/visionary").await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = get(fed.clone(), "/status/VISIONARY").await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = get(fed, "/status/ViSiOnArY").await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_status_one_unknown_kind_returns_404() {
        let fed = fresh_federation_with_all();
        let (status, body) = get(fed, "/status/Sentinel").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let body_str = String::from_utf8(body).unwrap();
        assert!(body_str.contains("unknown"), "got: {}", body_str);
    }

    #[tokio::test]
    async fn test_status_one_known_but_not_configured_returns_404() {
        // Partial federation: ask for Symbiotic which isn't configured
        let fed = fresh_federation_partial();
        let (status, body) = get(fed, "/status/Symbiotic").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let body_str = String::from_utf8(body).unwrap();
        assert!(
            body_str.contains("not configured"),
            "expected 'not configured' in: {}",
            body_str
        );
    }

    // =================================================================
    // Real TCP integration: spawn server, hit it, shutdown
    // =================================================================

    #[tokio::test]
    async fn test_server_spawn_listen_shutdown() {
        use super::super::server::HttpStatusServer;
        let fed = fresh_federation_with_all();
        // Port 0 = let OS pick a free port
        let addr = "127.0.0.1:0".parse().unwrap();
        let server = HttpStatusServer::spawn(addr, fed)
            .await
            .expect("server should spawn");

        let local = server.local_addr();
        assert_eq!(local.ip().to_string(), "127.0.0.1");
        assert_ne!(local.port(), 0, "OS should assign a real port");

        // Hit /healthz via raw TCP request to verify the server is actually
        // serving. We use a manual HTTP/1.1 request to avoid pulling in a
        // full HTTP client in dev-deps.
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(local).await.unwrap();
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.starts_with("HTTP/1.1 200"), "response: {}", response_str);
        assert!(response_str.contains("ok"), "body should contain ok");

        // Graceful shutdown
        server.shutdown().await.expect("shutdown should succeed");

        // Second shutdown returns AlreadyShutDown
        let second = server.shutdown().await;
        assert!(second.is_err());
    }

    #[tokio::test]
    async fn test_server_with_invalid_address_errors() {
        use super::super::server::{HttpServerError, HttpStatusServer};
        let fed = fresh_federation_with_all();

        // Try to bind to an address we know we can't (port 1, requires privileges
        // on most systems).
        let addr = "127.0.0.1:1".parse().unwrap();
        let result = HttpStatusServer::spawn(addr, fed).await;

        // On systems where this *does* succeed (running as root), we just
        // accept that and shut down. We're testing the error path's shape,
        // not its specific trigger.
        match result {
            Err(HttpServerError::Bind { .. }) => {
                // Expected on most systems
            }
            Ok(server) => {
                // Surprising but not wrong - clean up.
                let _ = server.shutdown().await;
            }
            Err(other) => panic!("unexpected error variant: {:?}", other),
        }
    }
}
