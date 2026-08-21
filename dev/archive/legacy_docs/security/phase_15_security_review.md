# Aaroneous Security Review — Phase 15

Review date: 2026-06-06
Scope: the four new modules shipped in Phase X/11/12
(`resilience`, `logging`, `rate_limit`, `input_validation`)
plus the related infrastructure changes (AppState, /metrics,
/health, autonomic loop timeouts).

This is a self-review. A second-party review from a
dedicated security engineer is the recommended follow-up
before any public exposure.

---

## 1. Methodology

For each new module I asked:

* Who controls the input?
* What is the worst-case behaviour if the input is hostile?
* Does the module leak information through error messages,
  metrics, or timing?
* Does the module allocate unbounded resources?

I then verified the answers against the unit tests and the
bench harness. The findings are below; the action taken
in each case is noted.

---

## 2. `resilience` (CircuitBreaker, RetryPolicy)

### 2.1 Input control

* `CircuitBreakerConfig` is constructed in code; the only
  operator-controlled value is the duration, which is a
  `Duration` (no parsing, no string input).
* `RetryPolicy::delay_for(attempt)` is a `u32`; no string
  input.

No input-borne attack surface. **No findings.**

### 2.2 Information leakage

`CircuitBreakerError::Open` is a unit variant; it does
not include the breaker name or configuration. Safe to
surface to clients.

`with_retry` returns `RetryError { attempts: u32, last_error: String }`.
The `last_error` is the `Display` output of the inner
error, so it can leak whatever the underlying call chose
to include. This is a known trade-off: the alternative
is to log the inner error and return a generic
"retries exhausted" to the caller, which loses debugging
information.

**Recommendation:** callers that wrap untrusted-input
specialists should sanitize `last_error` before
returning it to the HTTP layer. The default path is
fine for the in-process specialist pipeline.

### 2.3 Resource bounds

* State is `AtomicU8` + 2 `AtomicU64`. Constant.
* No allocation in the hot path.

**No findings.**

### 2.4 Concurrency

* All atomic ops use `Ordering::SeqCst`. This is the
  strongest order and is correct for the state machine;
  it is also the slowest, but the bench shows ~16 ns for
  the full call. Acceptable.
* Lazy Open→HalfOpen transition uses a `compare_exchange`
  to avoid lost updates.

**No findings.**

---

## 3. `logging` (init_logging)

### 3.1 Input control

* Reads `AARONEOUS_LOG` then `RUST_LOG`. The values are
  parsed by `tracing-subscriber` as filter directives.
  A hostile or malformed value causes the filter to be
  ignored, not to panic. **No findings.**

### 3.2 Information leakage

`tracing-subscriber` formats events with the level, target,
and message. The `Json` layer is enabled when stderr is
not a tty. The JSON output is suitable for log
aggregation; no sensitive data is added by this module.

**Recommendation:** do not log full request bodies at
`info!` level; use `debug!`. This is a caller convention,
not enforced by the module.

### 3.3 Resource bounds

* One global subscriber, set once. Idempotent via
  `AtomicBool` flag.
* No per-call allocation in `init_logging`.

**No findings.**

### 3.4 TTY detection

* Uses `libc::isatty(2)` directly on Unix; returns `false`
  on Windows. Safe — `isatty` is a side-effect-free
  syscall on a file descriptor.

**No findings.**

---

## 4. `rate_limit` (TokenBucketLimiter)

### 4.1 Input control — `key_from_request`

`key_from_request` was the highest-risk input in the
review. **Finding:** the previous implementation
formatted the raw `auth_subject` and `peer` strings into
the rate-limit key without any length check. A hostile
client could send a multi-megabyte `Authorization` header
or `X-Forwarded-For` value and force the rate limiter to
hold a multi-megabyte `String` per request.

**Action taken (C27):** `MAX_KEY_COMPONENT_LEN = 512`
constant, truncation logic, two new tests.

### 4.2 Input control — `TokenBucketConfig`

`burst` and `refill_per_second` are `f64`. Both are
clamped inside the limiter:
* `burst` is implicitly bounded by the user code (you
  pass what you pass).
* `refill_per_second == 0.0` is handled explicitly: the
  bucket never recovers, `retry_after` is capped at one
  hour, and the operator can spot the misconfiguration in
  the log.

A negative `refill_per_second` would cause the bucket to
*decrease* over time. That is a misconfiguration, not a
crash. The bench and tests do not cover this case.

**Recommendation (deferred):** add a `validate()` method
on `TokenBucketConfig` that rejects `burst <= 0.0` and
`refill_per_second < 0.0`. Not urgent — the operator is
the only caller and a misconfiguration is a one-time
event, not a per-request attack.

### 4.3 Information leakage

`TokenBucketDecision::Deny { retry_after }` includes a
`Duration`. This is safe to surface in a `Retry-After`
header. The `Allow { tokens_remaining }` variant is also
safe to surface in `X-RateLimit-Remaining`.

**No findings.**

### 4.4 Resource bounds

* Buckets: unbounded in cardinality. The recommended
  mitigation is `sweep_idle` from a background task.
  `idle_eviction` config knob exposes the window.
* Bucket state: `f64` tokens (8 B) + `Instant` (16 B) =
  24 B. ~88 MiB per million keys. See
  `docs/performance/characteristics.md` for the full
  budget.

**No findings.**

### 4.5 Concurrency

* Single `std::sync::Mutex<HashMap>`. On contention this
  is a bottleneck; for the documented request rate
  (≤18M ops/sec/core) it is fine. Switching to `DashMap`
  is on the candidate-optimizations list.

**No findings.**

### 4.6 Panic safety

`expect("rate limiter poisoned")` will panic if another
thread panics while holding the lock. The alternative is
to return a synthetic `Deny` decision. The trade-off is
favourable for a rate limiter: if the process is in a
state where the lock is poisoned, a synthetic `Deny` will
be wrong far more often than a panic, and the panic
indicates a much deeper problem (a specialist thread
panicking).

**No findings; design choice documented.**

---

## 5. `input_validation`

### 5.1 Input control

* `validate_string`: length and content checks. The
  rejection of control characters is now done via a
  byte scan (`< 0x20 || == 0x7F`) on the ASCII fast
  path and a `chars().any(is_control)` walk on the
  UTF-8 slow path. The error message includes the
  offending byte in hex.
* `validate_identifier`: bounded to 128 bytes and a
  fixed character set. The regex-style check is
  implemented as a `chars().find(...)` linear scan;
  safe for short identifiers.
* `validate_bytes`: length check only; no content
  inspection. **Finding (informational):** the
  validator does not catch "binary data that looks
  like an exploit". The intent is that this is the
  *first* line of defense; specialist invocations
  parse the bytes into typed structures, and the
  parser is the second line.
* `validate_range<T: Display>`: the error message
  includes the value. **Finding (informational):**
  for floats, this can produce a long fractional
  expansion. Operators reading logs should be aware.
  Not a security issue.
* `validate_enum`: the `allowed` slice is a compile-time
  constant in all current call sites. If a future caller
  passes a user-controlled slice, the comparison still
  behaves correctly (no injection possible through the
  comparison itself).

**No actionable findings.**

### 5.2 Information leakage

`ValidationError::Display` is a single string. The error
message format is `"{field}: {reason}"`, which is
operator-facing, not user-facing. If a handler returns
this string to the client, the `field` name is leaked.

**Recommendation:** handlers that convert to an HTTP 400
response should map the error to a stable code (e.g.
`bad_input:too_long`) and log the full message. The
client sees the code, the operator sees the details.

### 5.3 Resource bounds

All checks are linear in the input length and allocate at
most one `String` (for the success return). No unbounded
allocation.

**No findings.**

---

## 6. HTTP surface changes (C16)

### 6.1 New routes

* `/health`, `/live` (alias), `/healthz`, `/readyz`:
  no auth, no input. Safe.
* `/metrics`: returns Prometheus text. The body is
  generated by `render_prometheus_metrics`, which formats
  internal counters. The `version` field includes the
  build commit hash; this is acceptable for internal
  monitoring but should be redacted if `/metrics` is
  ever exposed publicly.
* `/version`: returns the build identity. Same note.

**Recommendation:** if `/metrics` is exposed beyond a
trusted network, gate it behind the same auth as the rest
of the API. The current code allows it through without
auth because Prometheus scrapers do not always support
Bearer tokens.

### 6.2 Auth bypass

The auth middleware is configured to bypass the public
routes (`/healthz`, `/readyz`, `/live`, `/metrics`,
`/version`, `/v1/models`). This is intentional and
documented in the deployment runbook.

**No findings.**

---

## 7. Autonomic loop changes (C15)

### 7.1 Shutdown flag

`shutdown: Arc<AtomicBool>` set on `SIGTERM`. There is no
authentication on who can set it; any process with the
same UID can send `SIGTERM`. Standard Unix semantics.

### 7.2 Tick budget

`max_ticks: Arc<AtomicU64>` exposed for runtime
configuration. The default (86,400) corresponds to one
tick per second for 24 hours. An attacker with shell
access can lower it to cause an early exit; this is the
intended behaviour (cooperative shutdown).

**No findings.**

### 7.3 Watchdog

`TICK_WATCHDOG = 10s` is a warning threshold, not a
hard interrupt. A long tick is logged but not killed.
This is the right call: there is no portable in-flight
cancellation in Rust, and a forced kill would lose
work-in-progress.

**No findings.**

---

## 8. Memory and decision integration (C14)

### 8.1 Memory write

`record_execution_memory` writes to
`specialist_memories: HashMap<String, SpecialistMemoryStore>`.
The key is the specialist name, which is internal —
not user input. Bounded by the number of specialists.

**No findings.**

### 8.2 Memory read

`consult_memory` queries the same map and returns a
float score plus a string recommendation. The string is
included in `TaskEvaluation` for logging. If the
recommendation string contains sensitive data, it ends up
in the log.

**Recommendation:** specialist authors should ensure that
recommendation strings are short, structural ("use a
smaller model"), and do not include user data. The
code does not enforce this.

---

## 9. Summary

| Severity | Count | Notes |
|---|---|---|
| Critical | 0 | |
| High | 0 | |
| Medium | 0 | |
| Low (action taken) | 1 | C27 key length truncation |
| Informational | 3 | Log-level convention, error sanitization, internal-only metrics |
| Deferred | 1 | TokenBucketConfig::validate() |

**The system is in a defensible state for internal
production use.** The open items are conventions, not
bugs, and can be addressed in a follow-up phase
(Phase 16 or similar) without blocking deployment.

A formal external security review is recommended before
any public exposure of the API.
