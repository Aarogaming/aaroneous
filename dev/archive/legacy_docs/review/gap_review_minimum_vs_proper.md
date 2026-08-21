# Aaroneous: Gap Review — What's Minimum vs. What's Proper

Date: 2026-06-06
Scope: the four new modules (`resilience`, `logging`,
`rate_limit`, `input_validation`) and the related
infrastructure (autonomic_loop, decision_engine, HTTP
middleware, /metrics surface).
Honest tone: this is the maintainer's own audit, not a
sales pitch.

---

## TL;DR

The four new modules are **internally correct** (the
unit tests pass, the public API is stable, the bench
harness is in place) but **three of the four are not
actually wired into the rest of the system.** The system
runs, the modules exist, but they don't do work in
production. To call Aaroneous "properly implemented,"
the following gaps need closing.

| Module | Status | Gap |
|---|---|---|
| `resilience` | Internal API complete; **not used** by the call sites it was designed for | `action_executor` does not gate network calls on a circuit breaker; HTTP layer does not use `with_retry`. |
| `logging` | Defined and re-exported; **never called** at startup | `init_logging()` is not invoked by `bin/a_run.rs`; `tracing` events are silently dropped. |
| `rate_limit` | Internal API complete; **not wired** into the HTTP middleware | The router has CORS + api_key_auth layers, but no rate-limit layer. 100% of inbound traffic is unrate-limited. |
| `input_validation` | Internal API complete; **not used** by any handler | Handlers accept `serde_json::Value` directly and rely on serde for shape; no length / character / range checks. |

Below: per-module gap list, with effort estimate and
priority.

---

## 1. `resilience` (CircuitBreaker, RetryPolicy)

### 1.1 What we have

* Atomic state machine, Open/Closed/HalfOpen, lazy
  Open→HalfOpen transition.
* Exponential backoff with deterministic LCG jitter
  (no `rand` dep).
* `with_retry` and `with_circuit_breaker` helpers.
* 11 unit tests + 8 graceful-degradation tests.
* ~620 lines, no runtime deps.

### 1.2 What's missing for "properly implemented"

| Gap | Effort | Priority |
|---|---|---|
| **Wire CB into `action_executor`**: every wasm call to a remote specialist should be `with_circuit_breaker` | M | High |
| **Wire CB into HTTP client paths**: if the binary makes outbound HTTP (it does, in `links/`), those calls should be CB-protected | M | High |
| **No async support**: the entire API is sync. Real systems have async deps. Need `call_async` and `with_retry_async` | M | High |
| **No timeouts**: `call` blocks forever if the inner closure hangs. Need `with_timeout` and a per-call deadline | S | High |
| **No bulkhead pattern**: a CB protects one dep; you also want a "max N concurrent calls" limit. Missing entirely | M | Medium |
| **Consecutive-failures-only trip**: real systems use sliding windows (Hystrix, resilience4j) so a 1/1000 success rate doesn't trip the breaker | L | Medium |
| **No per-error classification**: every `Err` counts as a failure. Should distinguish retryable (5xx, timeout) from non-retryable (4xx) | S | Medium |
| **No metrics integration**: trips, half-open probes, rejections are not in `/metrics` | S | Medium |
| **No per-breaker labels**: the `name` field is only in logs, not in metrics | XS | Low |
| **No `Clone` impl**: `CircuitBreaker` is intentionally not Clone (it owns atomic state), but `Arc<CircuitBreaker>` is the pattern; should document | XS | Low |
| **No half-open probe limit**: comment says "we accept the cost: at most one extra probe". Real systems use a CAS token to guarantee exactly one probe | S | Medium |
| **`expect("rate limiter poisoned")` in `with_retry`**: the `std::thread::sleep` between attempts blocks the thread. In a tokio runtime this is a problem | S | High (if async) |

### 1.3 The minimum we did vs. the proper version

* **Minimum:** the modules exist, have tests, are
  documented, and work in isolation.
* **Proper:** the modules are *applied* to every
  external-call site, with timeouts, async support, and
  metrics integration.

---

## 2. `logging` (init_logging)

### 2.1 What we have

* `init_logging()` idempotent.
* Reads `AARONEOUS_LOG` then `RUST_LOG`.
* TTY detection via `isatty(2)` on Unix.
* JSON-when-redirected, ANSI-on-tty.
* 1 unit test.

### 2.2 What's missing for "properly implemented"

| Gap | Effort | Priority |
|---|---|---|
| **`init_logging()` is never called**: search the codebase — it's defined and re-exported, but `bin/a_run.rs` doesn't call it. **All `tracing` events go to /dev/null in production.** | XS | Critical |
| **No `tracing::instrument` annotations**: handlers, specialist invocations, and the autonomic loop body are not wrapped in spans. You can't correlate a log line to a request | L | High |
| **No request ID middleware**: HTTP requests don't get a `request_id`; you can't grep the logs for one request | M | High |
| **No per-target filter levels**: `tracing-subscriber` supports `info,sqlx=warn,hyper=info`, but we hand-code a single global filter | XS | Low |
| **No log rotation**: stdout is the sink. A long-running deployment fills the disk. Real systems use `tracing-appender` with rotation | S | Medium |
| **No audit log**: security events (auth failures, privilege changes, key rotations) are not separately captured | M | Medium |
| **No sampling**: at high throughput, every event is too noisy; we need rate-limited sampling | M | Low |
| **`autonomic_loop` still uses `println!`**: 40+ println calls. Even after `init_logging` is called, these are not structured and not filterable | S | High |

### 2.3 The minimum we did vs. the proper version

* **Minimum:** the module exists and is unit-tested.
* **Proper:** the module is *called at startup*, every
  subsystem uses `tracing` instead of `println!`, every
  request has a span with a request ID, and there's a
  separate audit log.

---

## 3. `rate_limit` (TokenBucketLimiter)

### 3.1 What we have

* Token bucket per-key, ~55 ns/op.
* Idle eviction via `sweep_idle`.
* `key_from_request` with 512-byte cap.
* 18 unit tests + 1 thread-safety test + bench.
* ~310 lines, no runtime deps.

### 3.2 What's missing for "properly implemented"

| Gap | Effort | Priority |
|---|---|---|
| **Not wired into the HTTP layer**: the router has CORS + api_key_auth. There is no rate-limit layer. Every request bypasses the limiter. | S | Critical |
| **No per-route limits**: a single global bucket per key. Real systems want `/v1/chat` to have a tighter limit than `/v1/models` | M | High |
| **No quota**: rate is per-second. Quota (per-day, per-month) is a different concept and is not implemented | L | Medium |
| **No whitelisting**: an authenticated admin should be able to bypass the limit | XS | Medium |
| **No headers returned**: real rate-limited responses include `X-RateLimit-Remaining`, `X-RateLimit-Reset`, and `Retry-After` headers. The decision enum supports this but no handler emits them | XS | High |
| **No distributed rate limiting**: single-node only. A multi-node deployment has 1/N of the per-key rate | L | Low (deferred to multi-node) |
| **No metrics**: rate-limit denials are not in `/metrics` | XS | Medium |
| **No sliding-window option**: token bucket is one of several algorithms; for some workloads a sliding-window counter is fairer | M | Low |
| **No `Retry-After` in RFC 7231 form**: we return `Duration`; HTTP wants an integer or HTTP-date | XS | Low |

### 3.3 The minimum we did vs. the proper version

* **Minimum:** the limiter is a callable API.
* **Proper:** the limiter is a *middleware* in front of
  the actual handlers, returns proper headers, is
  per-route, and reports metrics.

---

## 4. `input_validation`

### 4.1 What we have

* `validate_string`, `validate_optional_string`,
  `validate_range`, `validate_bytes`,
  `validate_identifier`, `validate_enum`.
* ASCII fast path for `validate_string` (29ns → 24ns).
* 13 unit tests + 1 bench.
* `From<String>` and `From<&str>` for `ValidationError`.
* ~210 lines, no runtime deps.

### 4.2 What's missing for "properly implemented"

| Gap | Effort | Priority |
|---|---|---|
| **Not used by any handler**: search the codebase — no handler in `router.rs` calls a `validate_*` function. Handlers accept `serde_json::Value` and rely on serde for shape | S | Critical |
| **No document-level validation**: a single-field check is fine, but real systems have nested JSON bodies that need schema validation. We have `security_hardener` for that, but it's not integrated either | L | Medium |
| **No collect-all-errors mode**: `validate_string` returns on the first error. Real APIs want a list of all validation errors so the client can fix them in one round-trip | S | Medium |
| **No localization**: error messages are English | M | Low |
| **No sanitization mode**: we reject. Some systems want to sanitize (strip control chars but keep the rest). Should be a separate function | S | Low |
| **No schema derivation from types**: every handler writes its own validation calls. A `derive(Validate)` macro would be the proper way | L | Low (deferred) |
| **No max-nesting-depth check**: `validate_string` is a flat check. A request body with 10000 levels of nested objects would crash serde | S | Medium |

### 4.3 The minimum we did vs. the proper version

* **Minimum:** the helpers exist and pass tests.
* **Proper:** the helpers are *called by every handler*
  on every request, and a request body that fails a
  validation returns a structured 400 with all errors
  collected.

---

## 5. `autonomic_loop` (tick loop)

### 5.1 What we have

* `Arc<AtomicBool>` shutdown flag.
* `Arc<AtomicU64>` tick budget (86,400 default).
* 10s tick watchdog warning.
* 40+ `println!` calls in the hot loop (deferred).

### 5.2 What's missing for "properly implemented"

| Gap | Effort | Priority |
|---|---|---|
| **40+ `println!` calls in the hot path**: each tick generates several lines of stdout. They are not structured, not filterable, and the println!s in the inner loops run per-tick | M | High |
| **No drain on shutdown**: when `shutdown` flag is set, the loop exits immediately. In-flight tasks (e.g. a `record_execution_memory` write) are abandoned | M | High |
| **No per-tick metrics**: tick count, tick duration histogram, watchdog count, shutdown count are not in `/metrics` | S | Medium |
| **No tick jitter**: if multiple Aaroneous instances boot in lockstep, they will all wake on the same boundary and thundering-herd the dependency | XS | Low |
| **No graceful task queue draining**: there's a `task_id` flow but no bounded queue with backpressure | L | Medium |
| **No panic recovery**: if a per-tick closure panics, the loop dies. Should catch_unwind and log | S | High |
| **No time budget per tick**: the watchdog is a warning, not a hard limit. A 60-second tick still runs to completion | S | Medium |
| **`thread::sleep` blocks the runtime**: if the autonomic loop runs in a tokio task, `std::thread::sleep` blocks the executor | S | High (if tokio) |

### 5.3 The minimum we did vs. the proper version

* **Minimum:** the loop has cooperative shutdown and a
  tick budget.
* **Proper:** the loop drains on shutdown, has per-tick
  metrics, uses `tracing` instead of `println!`, catches
  panics, and the watchdog is enforceable.

---

## 6. `decision_engine` (memory integration)

### 6.1 What we have

* `consult_memory` returns `(score, recommendation)`.
* `record_execution_memory` writes the outcome.
* `TaskEvaluation` extended with `memory_informed`.
* 0.3 weight blend.

### 6.2 What's missing for "properly implemented"

| Gap | Effort | Priority |
|---|---|---|
| **`SpecialistMemoryStore` is unbounded**: `Arc<RwLock<HashMap<String, MemoryEntry>>>` with no eviction. Long-running deployments will OOM | M | Critical |
| **No memory decay**: a memory entry from 2 years ago has the same weight as one from yesterday. Real systems use time-decay or recency weighting | S | High |
| **Memory is in-process only**: a restart loses all memory. Real systems persist (sqlite, sled, or rocksdb) | L | High |
| **Memory is not in `/metrics`**: hit rate, miss rate, total entries, average confidence — none in metrics | S | Medium |
| **Two parallel memory paths**: `autonomic_loop::consult_specialist_memory` and `decision_engine::consult_memory` are separate code paths. The decision engine may consult memory that the loop has never written to (or vice versa) | M | High |
| **No memory backing-store abstraction**: a trait so we can swap in `sled` or `sqlite` later | M | Medium |
| **Memory write-back is fire-and-forget**: `record_execution_memory` does not return a result; failed writes are invisible | S | Medium |
| **No TTL on memory entries**: combined with no eviction, this is the OOM vector above | XS | Critical |
| **No memory versioning**: if a specialist's behavior changes (e.g. a new genome), old memories may be misleading | M | Low |

### 6.3 The minimum we did vs. the proper version

* **Minimum:** the decision engine reads and writes
  memory.
* **Proper:** the memory store is bounded, persisted,
  decayed, observable, and the only place the
  autonomic loop and decision engine write to.

---

## 7. HTTP surface (`/metrics`, `/healthz`, etc.)

### 7.1 What we have

* `/healthz`, `/readyz`, `/live`, `/metrics`, `/version`.
* `render_prometheus_metrics` is a pure function.
* 4 metric families: uptime, build_info, operation
  counters, operation duration.

### 7.2 What's missing for "properly implemented"

| Gap | Effort | Priority |
|---|---|---|
| **No histograms**: we have gauges and counters. A real system wants HTTP request duration as a histogram (p50, p95, p99) | M | High |
| **Most new modules don't emit metrics**: rate limit denials, circuit breaker trips, input validation rejections — none of these are visible in `/metrics` | M | High |
| **No labels**: `aaroneous_operation_count{op="..."}` has one label. Real systems want `op`, `status`, `tenant`, etc. | M | Medium |
| **`/metrics` is unauthenticated**: the deployment runbook says "expose on trusted network only". If it's exposed publicly, scrapers can DOS the system by hitting `/metrics` repeatedly | XS | Medium |
| **No `X-RateLimit-Remaining` / `Retry-After` headers** on rate-limited responses | XS | High (depends on §3) |
| **No OpenAPI/Swagger doc**: there is no machine-readable description of the API surface | L | Low |
| **No request logging middleware**: every request should log method, path, status, duration, request_id | M | High |
| **No per-route error rate metric**: `/v1/chat/completions` failing 50% of the time should be visible | S | High |
| **`/readyz` is too coarse**: returns 200 or 503. Should be able to return "ready but degraded" | M | Medium |
| **No 503 retry-after hint**: when the binary is at tick budget, /readyz could tell clients "retry in 60s" | XS | Low |

### 7.3 The minimum we did vs. the proper version

* **Minimum:** the endpoints exist and emit some text.
* **Proper:** every subsystem reports its state through
  metrics, requests are logged with structured context,
  and the surface is documented in OpenAPI.

---

## 8. Test coverage

### 8.1 What we have

* 1025 unit tests pass.
* 8 graceful-degradation integration tests.
* 14 micro-benchmarks.
* 13 pre-existing test failures documented.

### 8.2 What's missing for "properly implemented"

| Gap | Effort | Priority |
|---|---|---|
| **No property-based tests**: a token bucket, a circuit breaker state machine, and `validate_string` are all great candidates for `proptest`. The current tests are example-based | M | High |
| **No fuzz tests**: HTTP body parsers and input validators should be fuzzed with `cargo-fuzz`. We have 0 fuzz harnesses | M | High |
| **No HTTP-level tests**: the rate limiter and input validator are not tested *as* middleware. We test the helpers but not the request path | M | High |
| **No chaos tests**: kill a thread, drop a connection, force a panic. The graceful_degradation tests don't cover this | L | Medium |
| **No load test rig**: `docs/performance/load_testing.md` describes a plan but the harness is not committed | M | High |
| **No coverage tool in CI**: `cargo tarpaulin` or `cargo-llvm-cov` would give a coverage number | S | Medium |
| **No `cargo deny` / `cargo audit`**: dependency vulnerabilities are not checked in CI | XS | Medium |
| **No mutation testing**: `cargo mutants` would catch tests that pass without exercising the code under test | L | Low |
| **No benchmark regression CI**: bench numbers are local; a 2x regression would not be caught | M | Medium |
| **The 13 pre-existing test failures are still failing**: they've been documented but not fixed | varies | Low |
| **No test for the new modules' integration with each other**: e.g. "rate-limit-deny should be wrapped by a circuit breaker" | S | Medium |

### 8.3 The minimum we did vs. the proper version

* **Minimum:** the new modules have unit tests.
* **Proper:** the new modules have property tests,
  fuzz tests, HTTP-level integration tests, chaos
  tests, and a CI gate that runs all of them.

---

## 9. Observability beyond logging/metrics

| Gap | Effort | Priority |
|---|---|---|
| **No distributed tracing**: spans are not emitted by handlers. A real request leaves no trace across the specialist dispatch boundary | L | High |
| **No request ID propagation**: log lines can't be correlated to a request | M | High (depends on §2) |
| **No error budgets / SLOs**: an SLO is a function of metrics; we don't have SLOs because we don't have histograms | M | Medium |
| **No alerting hooks**: `tracing` can fire alerts via `tracing-subscriber`; we don't use that | M | Low |

---

## 10. Operational concerns

| Gap | Effort | Priority |
|---|---|---|
| **No state persistence on shutdown**: the runbook mentions `state.json`, but I see no code that writes it on shutdown | M | High |
| **No state restoration on startup**: if state.json exists, no code reads it | M | High |
| **No `/v1/admin/drain` endpoint**: an operator can't tell the system to stop accepting new requests and finish in-flight ones | S | Medium |
| **No backup script**: the runbook describes a backup, but there's no script in `scripts/` | XS | Low |
| **No rollback plan**: there's no previous-version tag, no documented rollback command | S | Low |
| **No chaos-day drills**: a system that hasn't been tested under failure is a system that will fail under failure | L | Low |
| **No `--validate-genome` flag**: the runbook mentions it but I see no code that implements it | M | Medium |
| **Tick budget doesn't restart on signal**: when the binary restarts, the tick counter resets. Long-running deployments should track the budget across restarts | S | Low |

---

## 11. Summary: ranked by impact

| # | Gap | Impact | Effort |
|---|---|---|---|
| 1 | `init_logging()` not called at startup | High | XS |
| 2 | Rate limit not wired into HTTP middleware | Critical | S |
| 3 | Input validation not used by handlers | High | S |
| 4 | `SpecialistMemoryStore` is unbounded (OOM) | Critical | M |
| 5 | 40+ `println!` in autonomic_loop hot path | High | S |
| 6 | No async / timeout in resilience | High | M |
| 7 | No request ID + tracing spans in HTTP layer | High | M |
| 8 | No histograms in /metrics | High | M |
| 9 | No CB / retry on action_executor / HTTP client | High | M |
| 10 | No state persistence on shutdown | High | M |
| 11 | No per-route rate limits | Medium | M |
| 12 | No property / fuzz / HTTP-level tests | High | M-L |
| 13 | Two parallel memory paths (autonomic + decision) | High | M |
| 14 | No bulkhead pattern | Medium | M |
| 15 | No sliding-window / per-error-class in CB | Medium | S-M |

---

## 12. Recommendation: what to do next

The user asked "what do we actually need to expand on to
have properly implemented and fully functional systems,
doing more than their minimum."

The honest answer is in two layers:

1. **The four new modules are *unused* in production.**
   The fastest single change with the highest impact is
   to wire them in: call `init_logging()` at startup,
   add a rate-limit middleware, call `validate_*` in
   every handler. This is the "minimum + ε" that gets
   the system from "modules exist" to "modules do
   something."

2. **Once wired in, the missing pieces matter.** A
   bounded memory store, async/timeout-aware resilience,
   and request-ID tracing are the next layer of
   production-readiness. These are the "minimum + ε + δ"
   items that make the system "properly implemented."

The rest — bulkhead, sliding-window CB, distributed
tracing, state persistence — is the "minimum + ε + δ +
..." that the maintainer would tackle after the system
is actually wired up.

The user gets to choose which layer to invest in next.
The maintainer's recommendation, in priority order, is:

* **Tier 1 (must do, ~1 day):** #1, #2, #3, #5, #8, #11.
  These are the wiring-up work. They convert the
  minimum into something that does work.
* **Tier 2 (should do, ~1 week):** #4, #6, #7, #9, #10,
  #12, #13. These make the system properly implemented.
* **Tier 3 (deferred, follow-up phases):** #14, #15,
  distributed rate limiting, state restoration on
  startup, audit log, SLOs, chaos drills.

The full audit table is the input to a Phase 16/17/18
roadmap, not a single big commit.
