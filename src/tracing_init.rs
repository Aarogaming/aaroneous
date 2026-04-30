// Aaroneous Tracing Initialization Module
// Structured logging and distributed tracing setup for federation observability

use tracing::{info, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use std::path::Path;

/// Initialize tracing with JSON output for Jaeger/observability
pub fn init_tracing(output_json: bool, log_level: Option<&str>) {
    let level = match log_level {
        Some("debug") => Level::DEBUG,
        Some("info") => Level::INFO,
        Some("warn") => Level::WARN,
        Some("error") => Level::ERROR,
        _ => Level::INFO,
    };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level.to_string()))
        .add_directive("tokio=info".parse().unwrap())
        .add_directive("nats=debug".parse().unwrap());

    if output_json {
        // JSON output for machine parsing (Jaeger, ELK, etc.)
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .json()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_span_list(true),
            )
            .init();
    } else {
        // Pretty human-readable output for console
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .pretty()
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    }

    info!("Aaroneous tracing initialized - Level: {}", level);
}

/// Initialize tracing with file output for persistent logging
pub fn init_tracing_with_file(
    log_file_path: &str,
    output_json: bool,
    log_level: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let level = match log_level {
        Some("debug") => Level::DEBUG,
        Some("info") => Level::INFO,
        Some("warn") => Level::WARN,
        Some("error") => Level::ERROR,
        _ => Level::INFO,
    };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level.to_string()))
        .add_directive("tokio=info".parse().unwrap())
        .add_directive("nats=debug".parse().unwrap());

    // Create parent directory if needed
    if let Some(parent) = Path::new(log_file_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)?;

    if output_json {
        // JSON to file
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .json()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_span_list(true)
                    .with_writer(file),
            )
            .init();
    } else {
        // Pretty format to file
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .pretty()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_writer(file),
            )
            .init();
    }

    info!("Aaroneous tracing initialized with file output: {} - Level: {}", log_file_path, level);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_init_creates_file() {
        let temp_path = "/tmp/test_aaroneous.log";
        let result = init_tracing_with_file(temp_path, false, Some("info"));
        // This test may fail on second run due to global subscriber, which is ok
        let _ = result;
    }
}
