/// Background HTTP server for the federation status API.
///
/// `HttpStatusServer::spawn` binds a TCP listener and starts the server on a
/// background tokio task. The returned `HttpStatusServer` is a handle that
/// the caller can use to:
/// - Look up the actual bound address (useful when binding to port 0)
/// - Trigger a graceful shutdown
///
/// # Why a separate task?
///
/// We want the federation's main work (executing specialists, checkpointing)
/// to keep running while the HTTP server is just monitoring. A dedicated
/// task isolates the server's failures from the federation's hot path.
use super::router::{AppState, router};
use crate::federation::hive::Federation;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// Errors from `HttpStatusServer::spawn`.
#[derive(Debug, thiserror::Error)]
pub enum HttpServerError {
    #[error("refusing non-loopback bind {addr} without AARONEOUS_API_KEY")]
    UnauthenticatedRemoteBind { addr: SocketAddr },

    #[error("failed to bind {addr}: {source}")]
    Bind {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read bound address: {0}")]
    LocalAddr(std::io::Error),

    #[error("server already shut down")]
    AlreadyShutDown,
}

/// Handle to a running federation HTTP server.
///
/// Dropping this handle does NOT stop the server - call `shutdown()` for
/// graceful termination. This design lets callers hold the handle in a
/// long-lived application state without accidentally killing the server.
pub struct HttpStatusServer {
    /// The actual address the listener bound to (may differ from the
    /// requested address if port 0 was used).
    local_addr: SocketAddr,
    /// HTTP application state, retained so shutdown can persist the
    /// in-memory snapshot to `cargo_state.json`.
    state: AppState,
    /// Notifier used to signal the server task to stop.
    shutdown_signal: Arc<Notify>,
    /// Background server task. `Some` until `shutdown()` joins it.
    handle: tokio::sync::Mutex<Option<JoinHandle<()>>>,
}

impl HttpStatusServer {
    /// Bind a TCP listener at the given address and start the server in
    /// the background.
    ///
    /// Pass `addr` with port `0` to let the OS pick a free port; the
    /// chosen address is then available via `local_addr()`.
    pub async fn spawn(
        addr: SocketAddr,
        federation: Arc<Federation>,
    ) -> Result<Self, HttpServerError> {
        let has_api_key = std::env::var("AARONEOUS_API_KEY")
            .map(|key| !key.is_empty())
            .unwrap_or(false);
        if !addr.ip().is_loopback() && !has_api_key {
            return Err(HttpServerError::UnauthenticatedRemoteBind { addr });
        }

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|source| HttpServerError::Bind { addr, source })?;

        let local_addr = listener.local_addr().map_err(HttpServerError::LocalAddr)?;

        let state = AppState::new(federation);
        // Start background vault indexing — non-blocking, fires and forgets
        state.start_vault_indexing();
        // Start link dispatcher — delivers federation events to webhooks/Discord/Slack/etc.
        let state_for_dispatcher = state.clone();
        tokio::spawn(async move {
            state_for_dispatcher.start_link_dispatcher().await;
        });
        // Start the rate-limit sweeper (every minute). Idempotent
        // task; only the spawned loop is left running.
        state.start_rate_limit_sweeper(std::time::Duration::from_secs(60));
        let app = router(state.clone());
        let shutdown_signal = Arc::new(Notify::new());
        let shutdown_signal_for_task = shutdown_signal.clone();

        let handle = tokio::spawn(async move {
            info!("Federation HTTP status server listening on {}", local_addr);

            // `into_make_service_with_connect_info` populates
            // `ConnectInfo<SocketAddr>` on every request, which the
            // rate-limit middleware uses to derive a per-IP key.
            let serve_fut = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                shutdown_signal_for_task.notified().await;
                info!("Federation HTTP server received shutdown signal");
            });

            if let Err(e) = serve_fut.await {
                warn!("Federation HTTP server exited with error: {}", e);
            } else {
                info!("Federation HTTP server stopped cleanly");
            }
        });

        Ok(Self {
            local_addr,
            state,
            shutdown_signal,
            handle: tokio::sync::Mutex::new(Some(handle)),
        })
    }

    /// The actual address the server is listening on.
    ///
    /// This may differ from the address passed to `spawn()` if port 0 was
    /// used to ask the OS for an ephemeral port.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Trigger graceful shutdown and wait for the server task to finish.
    ///
    /// After this returns, the server is no longer accepting connections
    /// and the background task has exited. Calling `shutdown` twice
    /// returns `Err(AlreadyShutDown)` from the second call.
    pub async fn shutdown(&self) -> Result<(), HttpServerError> {
        let mut handle_guard = self.handle.lock().await;
        let handle = handle_guard
            .take()
            .ok_or(HttpServerError::AlreadyShutDown)?;

        self.state.begin_drain();
        self.shutdown_signal.notify_one();

        // Best-effort wait for the task to finish. If it's stuck, we drop
        // the handle (which aborts the task on Drop).
        let timeout = std::time::Duration::from_secs(5);
        if (tokio::time::timeout(timeout, handle).await).is_err() {
            warn!(
                "Federation HTTP server did not shut down within {:?}, abandoning",
                timeout
            );
        }

        self.state.persist_cargo_state().await;

        Ok(())
    }
}
