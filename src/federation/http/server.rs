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

use super::router::{router, AppState};
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
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|source| HttpServerError::Bind { addr, source })?;

        let local_addr = listener
            .local_addr()
            .map_err(HttpServerError::LocalAddr)?;

        let state = AppState::new(federation);
        // Start background vault indexing — non-blocking, fires and forgets
        state.start_vault_indexing();
        let app = router(state);
        let shutdown_signal = Arc::new(Notify::new());
        let shutdown_signal_for_task = shutdown_signal.clone();

        let handle = tokio::spawn(async move {
            info!("Federation HTTP status server listening on {}", local_addr);

            let serve_fut = axum::serve(listener, app)
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
        let handle = handle_guard.take().ok_or(HttpServerError::AlreadyShutDown)?;

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

        Ok(())
    }
}
