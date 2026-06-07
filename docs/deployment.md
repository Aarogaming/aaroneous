# Aaroneous Deployment Runbook

A practical guide for taking an Aaroneous build from a clean
checkout to a running service. The intended audience is a
DevOps or SRE engineer who has not read the source.

This runbook reflects the state of `main` after Phase X
maintenance. Where the build is opinionated (e.g. the
`a_run` binary) we name the file; where a piece of
configuration is open, we describe both the default and the
override knob.

---

## 1. Prerequisites

* **Rust** stable, 1.85 or newer. Tested against `rustc 1.96`.
* **MSVC toolchain** on Windows, `build-essential` and
  `pkg-config` on Linux, Xcode CLT on macOS.
* **Git LFS** installed and on `PATH` if you intend to push
  large model artifacts. Aaroneous does not *fetch* LFS at
  build time, but a misconfigured LFS hook will fail the
  `post-commit` step.
* **Optional**:
  * `wasmtime` runtime headers (Aaroneous vendors its own).
  * A CUDA / ROCm installation if you want GPU specialists
    to do work in-process. CPU is the default.

The hypervisor builds in a sandboxed environment, so the only
non-Rust native dep is a working linker.

---

## 2. Build

### Library only

```bash
cargo build --release -p a_run --lib
```

This produces `target/release/a_run.rlib` and is enough for
embedding Aaroneous in another Rust program.

### Standalone hypervisor binary

```bash
cargo build --release -p a_run
```

The binary entry point is `aaroneous` and is located in
`core/hypervisor/src/bin/`. The output binary is
`target/release/aaroneous(.exe)`.

### Whole workspace

```bash
cargo build --release --workspace
```

The workspace contains 18 member crates (the hypervisor
plus the federation, compute, model, and tooling crates).
Use this when you want to run integration tests across
member boundaries.

### Verify a clean tree

```bash
cargo check --workspace --offline
```

A passing `cargo check` is a precondition for merging
deployment changes. The expected output is zero errors and
roughly 100 pre-existing warnings (all `dead_code` or
`unused_imports` in test scaffolding).

---

## 3. Configuration

Configuration is read from environment variables. The
canonical list lives in `core/hypervisor/src/config.rs` and
is re-exported as `Config` and `load_config()`. The most
relevant knobs:

| Variable | Default | Effect |
|---|---|---|
| `AARONEOUS_API_KEY` | (unset) | If set, all routes except `/healthz`, `/readyz`, `/live`, `/metrics`, `/version`, `/v1/models` require `Authorization: Bearer <key>`. |
| `AARONEOUS_LOG` | `info` | `tracing-subscriber` filter, same syntax as `RUST_LOG`. |
| `AARONEOUS_BIND` | `0.0.0.0:8080` | Listen address for the HTTP server. |
| `AARONEOUS_LIVE_GENOMES` | `genetics/` | Directory of pre-loaded specialist genomes, watched for changes. |
| `AARONEOUS_WASM_CACHE` | `target/wasm` | Disk location for compiled wasm32 artifacts. |
| `AARONEOUS_MAX_TICKS` | `86400` | Hard cap on the autonomic loop tick count. The loop will exit and report "tick budget exhausted" once this is hit. |
| `AARONEOUS_TICK_WATCHDOG` | `10s` | A tick that runs longer than this logs a `WARN` but is not interrupted. Set higher on first cold start when JIT warming can take a minute. |

Configuration is **not** reloaded on `SIGHUP`. Restart the
process to pick up changes. The intent is to keep the
behaviour of a running instance predictable; for true
hot-reload of genomes, watch the `genetics/` directory and
use the `/v1/models/reload` endpoint.

---

## 4. First Run

```bash
# 1. Choose a data directory
export AARONEOUS_DATA=/var/lib/aaroneous
mkdir -p "$AARONEOUS_DATA"

# 2. Optional: drop genomes in
cp -r genetics/* "$AARONEOUS_DATA/genomes/"

# 3. Run
AARONEOUS_API_KEY=changeme \
AARONEOUS_BIND=0.0.0.0:8080 \
./target/release/aaroneous
```

You should see log lines like:

```text
INFO aaroneous::federation::http::router: listening on 0.0.0.0:8080
INFO aaroneous::autonomic_loop: tick 1/86400 elapsed=...
```

A full health probe:

```bash
curl http://localhost:8080/healthz  # → 200 OK
curl http://localhost:8080/readyz   # → 200 OK once all adapters are ready
curl http://localhost:8080/metrics  # → Prometheus text
curl http://localhost:8080/version  # → {"name":"aaroneous","version":"..."}
```

---

## 5. Smoke Tests

After a fresh deploy, run the workspace test suite as a
non-destructive verification step. This does *not* mutate
runtime state, it just exercises the test binary, but it
catches many deployment-time mistakes (missing system
libraries, wrong linker flags, etc.):

```bash
cargo test -p a_run --lib --offline
```

Expected result: **994 passed, 13 failed, 3 ignored**.

The 13 failures are pre-existing test logic issues
unrelated to runtime state; see
`docs/maintenance/test-failures-2026-06-06.md` for the
catalogue and the recommended fixes. None of them
indicate a deployment-time problem.

---

## 6. Operational Endpoints

| Path | Method | Auth | Notes |
|---|---|---|---|
| `/healthz` | GET | none | Liveness; always returns 200 if the process is up. |
| `/readyz` | GET | none | Readiness; returns 200 only when all required adapters have reported in. |
| `/live` | GET | none | Alias for `/healthz`. |
| `/metrics` | GET | none | Prometheus text. Useful for scraping. |
| `/version` | GET | none | Build identity. |
| `/status` | GET | api_key | Process-wide status snapshot. |
| `/status/:kind` | GET | api_key | Status of one subsystem (`models`, `genomes`, `links`, ...). |
| `/v1/models` | GET | none | List loaded models. |
| `/v1/models/reload` | POST | api_key | Re-scan the genomes directory. |
| `/v1/chat/completions` | POST | api_key | OpenAI-compatible chat completion. |

When `AARONEOUS_API_KEY` is unset, the API-key-protected
routes return 401. Set the env var to enable them, or run
behind a reverse proxy that injects the header.

---

## 7. Shutdown

The autonomic loop supports cooperative shutdown. Send
`SIGTERM` and the loop will:

1. Set the `shutdown` flag on the next tick boundary.
2. Drain in-flight requests via the HTTP server's
   `with_graceful_shutdown`.
3. Persist state to `$AARONEOUS_DATA/state.json`.
4. Exit 0.

A second `SIGTERM` (or a `SIGINT`) immediately escalates
to a forced shutdown. This is intentional; in CI it is
common to send two signals in a row.

If the loop appears stuck, check the watchdog output:

```bash
grep "tick watchdog" /var/log/aaroneous.log
```

A long-running tick is normal during cold start. Repeated
watchdog warnings under load indicate a slow adapter
rather than a hang.

---

## 8. Backups

Aaroneous state is small. The recommended backup is:

```bash
tar czf aaroneous-backup-$(date +%F).tar.gz \
  "$AARONEOUS_DATA/genomes" \
  "$AARONEOUS_DATA/state.json" \
  "$AARONEOUS_DATA/links_registry.json"
```

Genomes are deterministic: copying them byte-for-byte
produces a bit-identical run. There is no in-memory
state that needs a live snapshot.

---

## 9. Common Failure Modes

| Symptom | Likely cause | Fix |
|---|---|---|
| Port already in use on 8080 | Another process holds the port. | Set `AARONEOUS_BIND=...`. |
| `api key required` on every call | `AARONEOUS_API_KEY` unset. | Set the env var or remove the API-key check from your client. |
| `genome parse error` at startup | A genome file is corrupt. | Re-download or run `aaroneous --validate-genome path/to/genome`. |
| Memory grows unbounded | A specialist is leaking wasm instances. | Restart; this is a known issue tracked in `docs/maintenance/known-issues.md`. |
| Tick budget exhausted within minutes | A loop is misconfigured (e.g. no yield point). | Raise `AARONEOUS_MAX_TICKS` to confirm, then debug. |

---

## 10. Where to Read Next

* `docs/api.md` — every public API, request/response, and example.
* `docs/troubleshooting.md` — runtime errors and their fixes.
* `docs/maintenance/test-failures-2026-06-06.md` — known test failures.
* `INDEX.md` — top-level documentation index.
* `AGENTS.md` — agent roles and the maintenance workflow.
