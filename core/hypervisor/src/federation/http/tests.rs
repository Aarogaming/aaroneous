/// Integration tests for the federation HTTP status API.
///
/// We use `tower::ServiceExt::oneshot` to drive the axum router in-process
/// without binding a real port. This is much faster than real-TCP testing
/// and equally exercises the routes, handlers, and serialization.

#[cfg(test)]
mod tests {
    use super::super::router::{AppState, GenerationJobStatus, StatusEnvelope, router};
    use crate::federation::hive::{Federation, SpecialistLearningSummary};
    use crate::federation::links::{Link, LinkType};
    use crate::federation::specialist::{Decision, ResourceRequest, Specialist, SpecialistId};
    use crate::rate_limit::{TokenBucketConfig, TokenBucketLimiter};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn fresh_federation_with_all() -> Arc<Federation> {
        Arc::new(
            Federation::builder(crate::persistence::PersistenceManager::new(":memory:").unwrap())
                .with_all()
                .build(),
        )
    }

    fn fresh_federation_partial() -> Arc<Federation> {
        Arc::new(
            Federation::builder(crate::persistence::PersistenceManager::new(":memory:").unwrap())
                .with_visionary()
                .with_archivist()
                .build(),
        )
    }

    fn empty_federation() -> Arc<Federation> {
        Arc::new(
            Federation::builder(crate::persistence::PersistenceManager::new(":memory:").unwrap())
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
        let req = Request::builder().uri(path).body(Body::empty()).unwrap();
        let response = app
            .oneshot(req)
            .await
            .expect("router oneshot should succeed");
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
    async fn test_healthz_sets_x_request_id_header() {
        let fed = fresh_federation_with_all();
        let app = router(AppState::new(fed));
        let req = Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();
        let response = app
            .oneshot(req)
            .await
            .expect("router oneshot should succeed");
        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .expect("x-request-id header");
        assert!(!request_id.is_empty());
    }

    #[tokio::test]
    async fn test_healthz_preserves_client_x_request_id_header() {
        let fed = fresh_federation_with_all();
        let app = router(AppState::new(fed));
        let req = Request::builder()
            .uri("/healthz")
            .header("x-request-id", "client-request-123")
            .body(Body::empty())
            .unwrap();
        let response = app
            .oneshot(req)
            .await
            .expect("router oneshot should succeed");
        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .expect("x-request-id header");
        assert_eq!(request_id, "client-request-123");
    }

    #[tokio::test]
    async fn test_admin_drain_rejects_new_requests() {
        let fed = fresh_federation_with_all();
        let app = router(AppState::new(fed));

        let drain_req = Request::builder()
            .method("POST")
            .uri("/v1/admin/drain")
            .body(Body::empty())
            .unwrap();
        let drain_resp = app.clone().oneshot(drain_req).await.expect("drain request");
        assert_eq!(drain_resp.status(), StatusCode::OK);

        let status_req = Request::builder()
            .uri("/status")
            .body(Body::empty())
            .unwrap();
        let status_resp = app.oneshot(status_req).await.expect("status request");
        assert_eq!(status_resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_cargo_state_round_trip_persists_jobs_links_and_vault() {
        let fed = fresh_federation_with_all();
        let temp_path = std::env::temp_dir().join(format!(
            "aaroneous_cargo_state_{}.json",
            uuid::Uuid::new_v4()
        ));

        let state = AppState::new_with_state_path(fed.clone(), temp_path.clone());
        let initial_links_len = state.links.read().await.len();
        state
            .generation_jobs
            .lock()
            .await
            .insert("job-1".to_string(), GenerationJobStatus::Running);
        state.links.write().await.push(Link::new(
            "test-link",
            LinkType::Webhook,
            "https://example.com/hook",
        ));
        state
            .vault
            .write()
            .await
            .insert("tensor-a".to_string(), vec![1.0, 2.0, 3.0]);

        state.persist_cargo_state_to(&temp_path).await;

        let loaded = AppState::new_with_state_path(fed.clone(), temp_path.clone());
        let jobs = loaded.generation_jobs.lock().await;
        assert!(jobs.contains_key("job-1"));
        drop(jobs);
        assert!(loaded.links.read().await.len() > initial_links_len);
        assert_eq!(loaded.vault.read().await.status().total_vault_entries, 1);

        let _ = std::fs::remove_file(&temp_path);
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
            fed.visionary()
                .unwrap()
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
        assert!(
            response_str.starts_with("HTTP/1.1 200"),
            "response: {}",
            response_str
        );
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

    // =================================================================
    // Rate-limit middleware
    // =================================================================

    // =================================================================
    // Input validation
    // =================================================================

    #[tokio::test]
    async fn test_validation_chat_completions_empty_messages() {
        let fed = fresh_federation_with_all();
        let state = AppState::new(fed.clone());
        let body = serde_json::json!({ "model": "presenter", "messages": [] });
        let (status, _, body_bytes) = post_json(state.clone(), "/v1/chat/completions", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let s = String::from_utf8_lossy(&body_bytes);
        assert!(s.contains("messages"), "got: {s}");
    }

    #[tokio::test]
    async fn test_validation_chat_completions_oversized_model() {
        let fed = fresh_federation_with_all();
        let state = AppState::new(fed.clone());
        // 200-char model name exceeds the 128-byte limit.
        let long_model = "x".repeat(200);
        let body = serde_json::json!({
            "model": long_model,
            "messages": [{"role": "user", "content": "hi"}],
        });
        let (status, _, body_bytes) = post_json(state.clone(), "/v1/chat/completions", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let s = String::from_utf8_lossy(&body_bytes);
        assert!(s.contains("model"), "got: {s}");
    }

    #[tokio::test]
    async fn test_validation_chat_completions_control_char_in_role() {
        let fed = fresh_federation_with_all();
        let state = AppState::new(fed.clone());
        // Tab in role: validate_string rejects control chars.
        let body = serde_json::json!({
            "model": "presenter",
            "messages": [{"role": "us\ter", "content": "hi"}],
        });
        let (status, _, _) = post_json(state.clone(), "/v1/chat/completions", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_validation_submit_intent_oversized_content() {
        let fed = fresh_federation_with_all();
        let state = AppState::new(fed.clone());
        // 64KB exceeds the 32KB cap.
        let big = "x".repeat(64 * 1024);
        let body = serde_json::json!({ "content": big });
        let (status, _, body_bytes) = post_json(state.clone(), "/intent", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let s = String::from_utf8_lossy(&body_bytes);
        assert!(s.contains("content"), "got: {s}");
    }

    #[tokio::test]
    async fn test_validation_create_session_empty_user_name() {
        let fed = fresh_federation_with_all();
        let state = AppState::new(fed.clone());
        let body = serde_json::json!({ "user_name": "" });
        let (status, _, body_bytes) = post_json(state.clone(), "/sessions", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let s = String::from_utf8_lossy(&body_bytes);
        assert!(s.contains("user_name"), "got: {s}");
    }

    #[tokio::test]
    async fn test_validation_completions_oversized_prompt() {
        let fed = fresh_federation_with_all();
        let state = AppState::new(fed.clone());
        // 512KB prompt exceeds the 256KB cap.
        let big = "x".repeat(512 * 1024);
        let body = serde_json::json!({
            "model": "presenter",
            "prompt": big,
        });
        let (status, _, body_bytes) = post_json(state.clone(), "/v1/completions", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let s = String::from_utf8_lossy(&body_bytes);
        assert!(s.contains("prompt"), "got: {s}");
    }

    /// Helper: POST a JSON value to a path and return the
    /// response status / headers / body. Used by the
    /// validation tests so we don't have to hand-build
    /// request bodies for every case.
    async fn post_json(
        state: AppState,
        path: &str,
        body: &serde_json::Value,
    ) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let app = router(state);
        let req = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(body).unwrap()))
            .unwrap();
        let response = app.oneshot(req).await.expect("router oneshot");
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes()
            .to_vec();
        (status, headers, body)
    }

    // Rate-limit middleware tests continue below
    /// Build an `AppState` whose `rate_limiter` is a fresh,
    /// tightly-bucketed limiter. We can't use the default
    /// config (burst=20, refill=10/s) because the unit tests
    /// would race against the refill; instead we cap burst
    /// and turn refill off.
    fn state_with_tight_rate_limit(fed: Arc<Federation>, burst: f64) -> AppState {
        let mut s = AppState::new(fed);
        s.rate_limiter = Arc::new(TokenBucketLimiter::new(TokenBucketConfig {
            burst,
            // No refill so the bucket drains deterministically.
            refill_per_second: 0.0,
            // Long eviction so the test isn't at the mercy
            // of a sweeper.
            idle_eviction: None,
        }));
        s
    }

    /// Drive `path` through the router. Mirrors the helper at
    /// line ~64 but with a custom `AppState` so we can swap the
    /// rate limiter.
    async fn get_with(
        state: AppState,
        path: &str,
        auth: Option<&str>,
    ) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let app = router(state);
        let mut b = Request::builder().uri(path).body(Body::empty()).unwrap();
        if let Some(t) = auth {
            b = Request::builder()
                .uri(path)
                .header("authorization", format!("Bearer {t}"))
                .body(Body::empty())
                .unwrap();
        }
        let response = app.oneshot(b).await.expect("router oneshot");
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes()
            .to_vec();
        (status, headers, body)
    }

    #[tokio::test]
    async fn test_rate_limit_allows_under_burst() {
        let fed = fresh_federation_with_all();
        let state = state_with_tight_rate_limit(fed.clone(), 5.0);
        // Five /status calls all permitted (auth disabled → IP key).
        for _ in 0..5 {
            let (status, headers, _) = get_with(state.clone(), "/status", None).await;
            assert_eq!(status, StatusCode::OK);
            assert!(
                headers.contains_key("x-ratelimit-remaining"),
                "expected x-ratelimit-remaining header on allow"
            );
        }
    }

    #[tokio::test]
    async fn test_rate_limit_denies_over_burst() {
        let fed = fresh_federation_with_all();
        let state = state_with_tight_rate_limit(fed.clone(), 3.0);
        // Burn the bucket: 3 allowed, 4th denied.
        for _ in 0..3 {
            let (status, _, _) = get_with(state.clone(), "/status", None).await;
            assert_eq!(status, StatusCode::OK);
        }
        let (status, headers, body) = get_with(state.clone(), "/status", None).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert!(
            headers.get("retry-after").is_some(),
            "expected Retry-After header on 429"
        );
        assert_eq!(headers.get("x-ratelimit-remaining").unwrap(), "0");
        let body_str = String::from_utf8_lossy(&body);
        assert!(body_str.to_lowercase().contains("rate limit"));
    }

    #[tokio::test]
    async fn test_rate_limit_bypass_endpoints_never_throttled() {
        let fed = fresh_federation_with_all();
        // Burst=1 so the *next* call would normally 429.
        let state = state_with_tight_rate_limit(fed.clone(), 1.0);
        // Burn the bucket on /status.
        let (s, _, _) = get_with(state.clone(), "/status", None).await;
        assert_eq!(s, StatusCode::OK);
        // Bypass endpoints stay open forever.
        for path in &[
            "/healthz",
            "/readyz",
            "/health",
            "/live",
            "/metrics",
            "/version",
            "/v1/models",
        ] {
            for _ in 0..20 {
                let (status, _, _) = get_with(state.clone(), path, None).await;
                assert!(
                    status == StatusCode::OK
                        || status == StatusCode::SERVICE_UNAVAILABLE
                        || status == StatusCode::NOT_FOUND,
                    "bypass path {path} returned {status}"
                );
                assert!(
                    !status.is_success()
                        || status == StatusCode::OK
                        || status == StatusCode::SERVICE_UNAVAILABLE
                        || status == StatusCode::NOT_FOUND,
                    "bypass path {path} returned {status}"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_rate_limit_keys_are_isolated_by_auth() {
        let fed = fresh_federation_with_all();
        // Burst=1 per key. Two different bearer subjects.
        let state = state_with_tight_rate_limit(fed.clone(), 1.0);
        let (s_a, _, _) = get_with(state.clone(), "/status", Some("alice")).await;
        assert_eq!(s_a, StatusCode::OK);
        // Bob has his own bucket.
        let (s_b, _, _) = get_with(state.clone(), "/status", Some("bob")).await;
        assert_eq!(s_b, StatusCode::OK);
        // Alice's second call is now denied.
        let (s_a2, _, _) = get_with(state.clone(), "/status", Some("alice")).await;
        assert_eq!(s_a2, StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_rate_limit_per_route_uses_independent_buckets() {
        // Per-route override should not share buckets with the
        // default. We construct a state with two tight limiters
        // and confirm the chat route and the default route
        // consume tokens independently.
        let fed = fresh_federation_with_all();
        let mut state = AppState::new(fed.clone());
        // Tight chat bucket: burst=2, refill off.
        let chat_limiter = Arc::new(TokenBucketLimiter::new(TokenBucketConfig {
            burst: 2.0,
            refill_per_second: 0.0,
            idle_eviction: None,
        }));
        // Tight default bucket: burst=3, refill off.
        state.rate_limiter = Arc::new(TokenBucketLimiter::new(TokenBucketConfig {
            burst: 3.0,
            refill_per_second: 0.0,
            idle_eviction: None,
        }));
        state.route_limits = vec![("/v1/chat/completions".to_string(), chat_limiter.clone())];

        // Drain the chat bucket. We POST with a malformed
        // body so the handler returns 4xx quickly (no LLM
        // call) — the middleware still consumes a token.
        for _ in 0..2 {
            let (status, _, _) = post_raw(state.clone(), "/v1/chat/completions", b"{}").await;
            assert_ne!(status, StatusCode::TOO_MANY_REQUESTS);
        }
        // 3rd call: chat bucket empty, expect 429.
        let (status, _, _) = post_raw(state.clone(), "/v1/chat/completions", b"{}").await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

        // /status still has a full default bucket. Burn 3.
        for _ in 0..3 {
            let (status, _, _) = get_with(state.clone(), "/status", None).await;
            assert_eq!(status, StatusCode::OK);
        }
        // 4th default call: 429.
        let (status, _, _) = get_with(state.clone(), "/status", None).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

        // The chat bucket is still 429, the default bucket
        // is also 429 — but they fail for *different* reasons.
        // We confirm the per-route override by counting
        // distinct limiters: chat path and default path have
        // different bucket maps.
        assert_eq!(chat_limiter.len(), 1, "chat limiter saw 1 key");
        assert_eq!(state.rate_limiter.len(), 1, "default limiter saw 1 key");
    }

    /// Helper: POST raw bytes to a path through the router.
    /// Returns the response status / headers / body. Used by
    /// the per-route override test to avoid running the LLM.
    async fn post_raw(
        state: AppState,
        path: &str,
        body: &[u8],
    ) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let app = router(state);
        let req = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_vec()))
            .unwrap();
        let response = app.oneshot(req).await.expect("router oneshot");
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes()
            .to_vec();
        (status, headers, body)
    }

    #[tokio::test]
    async fn test_metrics_breakers_endpoint_returns_json() {
        // /metrics/breakers returns a JSON list (empty until
        // production callers register breakers). The shape
        // must be stable so dashboards can depend on it.
        let fed = fresh_federation_with_all();
        let state = AppState::new(fed.clone());
        let (status, _, body) = get_with(state.clone(), "/metrics/breakers", None).await;
        assert_eq!(status, StatusCode::OK);
        let v: serde_json::Value = serde_json::from_slice(&body).expect("valid JSON");
        assert!(v.get("count").is_some());
        assert!(v.get("breakers").is_some());
        assert!(v["breakers"].is_array());
    }

    #[tokio::test]
    async fn test_rate_limit_real_tcp_burst_then_429() {
        use super::super::server::HttpStatusServer;
        let fed = fresh_federation_with_all();
        // Spawn via the real path. The default config
        // (burst=20) gives plenty of headroom; we hit
        // /status 25 times in a tight loop from the same
        // peer. The first 20 are 200; the rest are 429.
        let server = HttpStatusServer::spawn("127.0.0.1:0".parse().unwrap(), fed)
            .await
            .expect("spawn server");
        let local = server.local_addr();

        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        async fn one_call(addr: std::net::SocketAddr) -> (u16, Option<String>) {
            let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            stream
                .write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
            let mut response = Vec::new();
            stream.read_to_end(&mut response).await.unwrap();
            let s = String::from_utf8_lossy(&response).to_string();
            // Status line: "HTTP/1.1 NNN ..." — extract NNN
            let status = s
                .split_whitespace()
                .nth(1)
                .and_then(|n| n.parse::<u16>().ok())
                .unwrap_or(0);
            let retry = s
                .lines()
                .find(|l| l.to_lowercase().starts_with("retry-after:"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string());
            (status, retry)
        }

        let mut saw_429 = false;
        let mut saw_retry_after = false;
        for _ in 0..25 {
            let (status, retry) = one_call(local).await;
            match status {
                200 => {}
                429 => {
                    saw_429 = true;
                    if retry.is_some() {
                        saw_retry_after = true;
                    }
                }
                other => panic!("unexpected status: {other}"),
            }
        }
        assert!(
            saw_429,
            "expected at least one 429 in 25 calls under burst=20"
        );
        assert!(saw_retry_after, "429 response should include Retry-After");

        server.shutdown().await.expect("shutdown");
    }
}
