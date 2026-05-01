//! Federation with HTTP status endpoints.
//!
//! Builds a federation, spawns the HTTP status server on a free port,
//! prints the URLs the user can hit, runs for a few seconds while
//! letting the user `curl` the endpoints, then shuts down cleanly.
//!
//! ## Run it
//!
//! ```text
//! cargo run --example federation_http_status
//! ```
//!
//! Then in another terminal:
//!
//! ```text
//! curl http://127.0.0.1:8765/healthz
//! curl http://127.0.0.1:8765/readyz
//! curl http://127.0.0.1:8765/status | jq
//! curl http://127.0.0.1:8765/status/Visionary | jq
//! ```
//!
//! ## What this exercises
//!
//! - `Federation::builder()` with all 5 specialists
//! - `start_all()` and `shutdown_all()` lifecycle
//! - `HttpStatusServer::spawn()` / `shutdown()`
//! - The `/healthz`, `/readyz`, `/status`, `/status/{kind}` endpoints
//! - Specialist execution driving live status updates

use a_run::federation::hive::Federation;
use a_run::federation::http::HttpStatusServer;
use a_run::federation::specialist::{Decision, ResourceRequest, Specialist, SpecialistId};
use a_run::persistence::PersistenceManager;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

fn make_decision(kind: SpecialistId, idx: usize) -> Decision {
    Decision {
        proposal_id: format!("p-{}", idx),
        specialist: kind,
        action: "demo".to_string(),
        allocated_resources: ResourceRequest::default(),
        deadline_ms: 5000,
        context: HashMap::new(),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    println!("\n=== Aaroneous Federation HTTP Status Demo ===\n");

    // In-memory database - we don't care about persistence in this demo,
    // we just want to show the live HTTP surface.
    let pm = PersistenceManager::new(":memory:")?;
    let fed = Arc::new(Federation::builder(pm).with_all().build());

    // Start the federation
    fed.start_all()
        .await
        .map_err(|e| anyhow::anyhow!("start_all: {}", e))?;
    println!("Federation started ({} specialists)", fed.enabled_count());

    // Spawn the HTTP server. Use a fixed port so the user can curl predictable URLs.
    // (Use port 0 if you want the OS to pick - server.local_addr() will tell you.)
    let addr = "127.0.0.1:8765".parse()?;
    let server = HttpStatusServer::spawn(addr, fed.clone()).await?;
    let local = server.local_addr();

    println!("\nHTTP status server listening on http://{}\n", local);
    println!("Try:");
    println!("  curl http://{}/healthz", local);
    println!("  curl http://{}/readyz", local);
    println!("  curl http://{}/status", local);
    println!("  curl http://{}/status/Visionary", local);

    // While the demo runs, train each specialist a bit so the /status
    // endpoint shows non-zero counts.
    println!("\nTraining each specialist 5 times (you can curl /status to watch)...");
    for kind in [
        SpecialistId::Visionary,
        SpecialistId::Omnipresent,
        SpecialistId::Symbiotic,
        SpecialistId::Phygital,
        SpecialistId::Archivist,
    ] {
        let s: Arc<dyn Specialist + Send + Sync> = match kind {
            SpecialistId::Visionary => fed.visionary().unwrap(),
            SpecialistId::Omnipresent => fed.omnipresent().unwrap(),
            SpecialistId::Symbiotic => fed.symbiotic().unwrap(),
            SpecialistId::Phygital => fed.phygital().unwrap(),
            SpecialistId::Archivist => fed.archivist().unwrap(),
            _ => continue,
        };
        for i in 0..5 {
            s.execute(&make_decision(kind, i)).await?;
            // Small delay so a user can watch the count tick up between curls
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }

    // Print the final summary
    let summary = fed.learning_summary();
    println!("\nFinal in-memory summary:");
    for (name, s) in summary.iter() {
        println!(
            "  {:<14} {} executions, {:.1}% success",
            name,
            s.total_executions,
            s.success_rate_percent()
        );
    }

    println!("\nDemo will hold for 10 more seconds so you can curl /status...");
    println!("(Or press Ctrl-C to exit early; cleanup is best-effort.)");
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Graceful shutdown
    println!("\nShutting down...");
    server.shutdown().await?;
    fed.shutdown_all()
        .await
        .map_err(|e| anyhow::anyhow!("shutdown_all: {}", e))?;
    println!("Done.");

    Ok(())
}
