// Structured logging via the `tracing` facade.
//
// `tracing` is already in Cargo.toml. This module:
//   1. Provides a single `init_logging()` entry point that
//      installs a subscriber exactly once. Subsequent calls are
//      no-ops so library code that wants to log can call it
//      defensively without paying the cost of duplicate init.
//   2. Honours the `AARONEOUS_LOG` env var (standard `tracing`
//      convention, same as `RUST_LOG`) for filter directives.
//   3. Defaults to the `fmt` subscriber with ANSI when stderr is
//      a tty, and JSON when stderr is redirected (so production
//      deployments get machine-parseable logs out of the box).
//
// Callers should prefer `tracing::info!` / `warn!` / `error!` /
// `debug!` / `trace!` over `println!`/`eprintln!` for any new code.
// Existing `println!` calls in the codebase are left untouched in
// this pass; a follow-up commit can sweep them.

use std::sync::atomic::{AtomicBool, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize structured logging. Idempotent: safe to call from
/// multiple entry points (e.g. the binary's main and a test
/// harness). Returns `true` if this call performed the init,
/// `false` if logging was already initialized.
pub fn init_logging() -> (bool, Option<tracing_appender::non_blocking::WorkerGuard>) {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return (false, None);
    }
    let guard = install_subscriber();
    (true, Some(guard))
}

/// Install the tracing subscriber using the `AARONEOUS_LOG` or
/// `RUST_LOG` env var for the filter, and the fmt subscriber
/// (ANSI-on-tty, JSON-otherwise) as the formatter.
fn install_subscriber() -> tracing_appender::non_blocking::WorkerGuard {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter = EnvFilter::try_from_env("AARONEOUS_LOG")
        .or_else(|_| EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| EnvFilter::new("info,a_run=debug"));

    // Rotating file appender: logs/aaroneous.log
    let file_appender = tracing_appender::rolling::daily("logs", "aaroneous.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Heuristic: if stderr is redirected (no tty), prefer JSON. The
    // `is_terminal` check is in `std` since 1.70, no extra dep.
    let is_tty = atty_stderr();
    if is_tty {
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_target(true).with_level(true))
            .with(fmt::layer().with_writer(non_blocking))
            .try_init();
    } else {
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(false),
            )
            .with(fmt::layer().with_writer(non_blocking))
            .try_init();
    }
    guard
}

#[cfg(unix)]
fn atty_stderr() -> bool {
    extern "C" {
        fn isatty(fd: i32) -> i32;
    }
    unsafe { isatty(2) != 0 }
}

#[cfg(not(unix))]
fn atty_stderr() -> bool {
    // On Windows, default to non-TTY so JSON output is the default
    // unless the operator explicitly opts in. Windows console tty
    // detection would require `kernel32::GetConsoleMode` which we
    // avoid to keep the dep surface small.
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_logging_is_idempotent() {
        // The subscriber install is process-global; we can only
        // assert that the second call returns false (already
        // initialized) once at least one call has succeeded.
        let first = init_logging();
        let second = init_logging();
        // We don't assert on `first` because in test contexts
        // another test in the same binary may have already
        // installed a subscriber. We do require idempotence.
        assert!(!second.0);
        // First call is either true (we installed it) or false
        // (someone else did). Either way, no panic.
        let _ = first;
    }
}
