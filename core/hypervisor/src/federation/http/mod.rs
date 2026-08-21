pub mod rest_api;
/// Federation HTTP status surface.
///
/// Exposes a small read-only HTTP API for monitoring the federation:
///
/// | Endpoint               | Purpose                                          |
/// |------------------------|--------------------------------------------------|
/// | `GET /healthz`         | Liveness check (always 200 if process is alive)  |
/// | `GET /readyz`          | Readiness check (200 only when all hosts are up) |
/// | `GET /status`          | JSON dump of `Federation::learning_summary()`    |
/// | `GET /status/{kind}`   | JSON dump of one specialist's summary            |
///
/// The split between liveness (`/healthz`) and readiness (`/readyz`) follows
/// Kubernetes convention: liveness asks "is the process running?", readiness
/// asks "is it ready to serve?". A federation that hasn't called `start_all`
/// yet is alive but not ready.
///
/// # Usage
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use std::sync::Arc;
/// use a_run::federation::hive::Federation;
/// use a_run::federation::http::HttpStatusServer;
/// use a_run::persistence::PersistenceManager;
///
/// let pm = PersistenceManager::new("hive.db")?;
/// let fed = Arc::new(Federation::builder(pm).with_all().build());
///
/// fed.start_all().await?;
///
/// let server = HttpStatusServer::spawn("127.0.0.1:8080".parse()?, fed.clone()).await?;
/// // server is now serving /healthz, /readyz, /status, /status/{kind}
///
/// // ... run application ...
///
/// server.shutdown().await;
/// fed.shutdown_all().await?;
/// # Ok(())
/// # }
/// ```
pub mod router;
pub mod server;

#[cfg(test)]
mod tests;

pub use rest_api::server::RestApiServer;
pub use router::{AppState, router as federation_router};
pub use server::{HttpServerError, HttpStatusServer};
