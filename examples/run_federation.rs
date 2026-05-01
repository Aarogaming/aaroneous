//! End-to-end Federation runner.
//!
//! Builds a federation with all 5 specialists, drives them through some
//! simulated executions, prints a learning summary, and shuts down with
//! a final save. Then re-opens the same DB to demonstrate restart
//! recovery: the second federation has the trained state from the first.
//!
//! ## Run it
//!
//! ```text
//! cargo run --example run_federation
//! ```
//!
//! ## What you'll see
//!
//! - "Generation 1" trains each specialist a different number of times
//! - The learning summary is printed in tabular form
//! - The first federation shuts down (final save to SQLite)
//! - "Generation 2" opens the same DB - immediately recovers the trained
//!   state without any executions of its own
//! - The second federation shuts down cleanly
//!
//! This single example exercises:
//! - `Federation::builder()` with `with_all()`
//! - `Federation::start_all()` / `shutdown_all()`
//! - Specialist `execute()` driving learning
//! - `Federation::learning_summary()` diagnostic
//! - SQLite persistence + restart recovery via `learn_persist`

use a_run::federation::hive::{Federation, LearningSummary};
use a_run::federation::specialist::{
    Decision, ExecutionStatus, ResourceRequest, Specialist, SpecialistId,
};
use a_run::persistence::PersistenceManager;
use std::collections::HashMap;
use std::sync::Arc;

/// Where the example puts its database file. We use a temp directory so the
/// example is hermetic (doesn't dirty the user's working directory).
fn db_path() -> std::path::PathBuf {
    std::env::temp_dir().join("aaroneous-example-federation.db")
}

/// Helper to build a `Decision` for a specific specialist.
fn decision_for(specialist: SpecialistId, idx: usize) -> Decision {
    Decision {
        proposal_id: format!("example-{}-{}", specialist as u8, idx),
        specialist,
        action: "demo".to_string(),
        allocated_resources: ResourceRequest::default(),
        deadline_ms: 5000,
        context: HashMap::new(),
    }
}

/// Print a learning summary in a human-readable table. Only configured
/// specialists are listed.
fn print_summary(label: &str, s: &LearningSummary) {
    println!("\n=== {} ===", label);
    println!(
        "{:<14} {:>10} {:>10} {:>10} {:>10}",
        "specialist", "execs", "success", "rate%", "history"
    );
    println!("{}", "-".repeat(56));
    for (name, sum) in s.iter() {
        println!(
            "{:<14} {:>10} {:>10} {:>10.1} {:>10}",
            name,
            sum.total_executions,
            sum.success_count,
            sum.success_rate_percent(),
            sum.history_len
        );
    }
    println!(
        "{}\n{:<14} {:>10} {:>10}",
        "-".repeat(56),
        "TOTAL",
        s.total_executions(),
        s.total_successes()
    );
}

/// Train each specialist a different number of times so we can later verify
/// the per-specialist counts survived the round trip.
async fn train_specialists(fed: &Federation) -> anyhow::Result<()> {
    let v = fed.visionary().expect("Visionary configured");
    let o = fed.omnipresent().expect("Omnipresent configured");
    let s = fed.symbiotic().expect("Symbiotic configured");
    let p = fed.phygital().expect("Phygital configured");
    let a = fed.archivist().expect("Archivist configured");

    let plan: Vec<(SpecialistId, Arc<dyn Specialist + Send + Sync>, usize)> = vec![
        (SpecialistId::Visionary, v, 4),
        (SpecialistId::Omnipresent, o, 3),
        (SpecialistId::Symbiotic, s, 5),
        (SpecialistId::Phygital, p, 2),
        (SpecialistId::Archivist, a, 6),
    ];

    for (kind, specialist, count) in plan {
        for i in 0..count {
            let result = specialist.execute(&decision_for(kind, i)).await?;
            // We don't assert here - mismatched status would be a real bug,
            // but the example prefers to keep going so the user sees the
            // full output even in degraded conditions.
            if result.status != ExecutionStatus::Success {
                eprintln!(
                    "warn: {:?} execution {} returned {:?}",
                    kind, i, result.status
                );
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Best-effort tracing init - if it fails (e.g., already initialized in a
    // test runner), we just continue without log emission.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let path = db_path();
    let path_str = path.to_string_lossy().to_string();

    // Clean any prior run so the demo starts fresh
    if path.exists() {
        std::fs::remove_file(&path).ok();
    }

    println!(
        "\n=== Aaroneous Federation Demo ===\nDatabase: {}\n",
        path_str
    );

    // ---------- Generation 1: train and persist ----------
    println!("Generation 1: starting fresh, training each specialist...");
    {
        let pm = PersistenceManager::new(&path_str)?;
        let fed = Federation::builder(pm)
            .manual_checkpoints() // example uses shutdown's final save
            .with_all()
            .build();

        fed.start_all()
            .await
            .map_err(|e| anyhow::anyhow!("start_all failed: {}", e))?;

        train_specialists(&fed).await?;

        let summary = fed.learning_summary();
        print_summary("Generation 1 - after training", &summary);

        // Sanity check (and gives the example test something to verify)
        assert_eq!(summary.total_executions(), 4 + 3 + 5 + 2 + 6);

        fed.shutdown_all()
            .await
            .map_err(|e| anyhow::anyhow!("shutdown_all failed: {}", e))?;

        println!("Generation 1 shut down. Database now contains learning state.");
    }

    // ---------- Generation 2: cold start, recover ----------
    println!("\nGeneration 2: opening the same database, no training...");
    {
        let pm = PersistenceManager::new(&path_str)?;
        let fed = Federation::builder(pm)
            .manual_checkpoints()
            .with_all()
            .build();

        let pre_start = fed.learning_summary();
        print_summary("Generation 2 - pre-start (in-memory only)", &pre_start);
        assert_eq!(
            pre_start.total_executions(),
            0,
            "fresh in-memory state should be zero before start_all"
        );

        fed.start_all()
            .await
            .map_err(|e| anyhow::anyhow!("start_all failed: {}", e))?;

        let post_start = fed.learning_summary();
        print_summary("Generation 2 - post-start (loaded from DB)", &post_start);

        assert_eq!(
            post_start.total_executions(),
            4 + 3 + 5 + 2 + 6,
            "state recovered from generation 1 should match"
        );

        fed.shutdown_all()
            .await
            .map_err(|e| anyhow::anyhow!("shutdown_all failed: {}", e))?;
    }

    println!("\nDemo complete. Federation learning state persisted across restart.");
    println!("(DB left in place at {} for inspection)", path_str);

    Ok(())
}
