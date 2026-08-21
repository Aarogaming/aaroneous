# Contents of: performance

---

## File: characteristics.md

# Aaroneous Performance Characteristics

Baseline numbers from the `criterion` bench suite shipped
with Phase 14. All measurements are single-threaded on the
development machine; production numbers will differ.

Run `cargo bench -p a_run --bench phase_x_smoke` to
reproduce. CI should compare against these numbers; a
regression > 10% in any cell is a candidate for a follow-up
PR.

---

## 1. Test environment

* CPU: x86_64, single core
* Rust: stable, release profile with default `codegen-units`
* Criterion: 0.5, `default-features = false` +
  `cargo_bench_support`
* Build: `cargo bench -p a_run --bench phase_x_smoke`

The numbers below are the "estimated" middle of the
criterion `[low, mid, high]` range. Criterion's confidence
intervals are 95%.

---

## 2. Hot path: rate limiter

| Operation | Time (ns) | Allocations |
|---|---|---|
| `check` on existing key (single) | ~55 | 0 |
| `check` on existing key (1k map warm) | ~120 | 0 |
| `key_from_request` (auth subject) | ~67 | 1 small `String` |
| `key_from_request` (IP only) | ~67 | 1 small `String` |
| `forget` existing key | ~150 | 0 |
| `sweep_idle` over 1k map | varies | 0 |

**Design notes:**

* `check` is the request-path operation. At ~55 ns/op, a
  single core can sustain ~18M ops/sec before saturating.
* Allocation budget is zero on the hot path: the `String`
  cost is paid at most once per new key.
* The lock is `std::sync::Mutex<HashMap<…>>`. For workloads
  with >100k keys, swap for `DashMap` or shard the map.
* `sweep_idle` should be called from a background task
  (e.g. `tokio::spawn` loop with `tokio::time::interval`),
  not from the request handler.

---

## 3. Hot path: input validation

| Operation | Time (ns) | Allocations |
|---|---|---|
| `validate_string` (5-byte ASCII) | ~24 | 1 (the returned `String`) |
| `validate_string` (500-byte ASCII) | ~154 | 1 |
| `validate_identifier` (typical) | ~28 | 1 |
| `validate_range<f64>` (in bounds) | ~0.2 | 0 |
| `validate_bytes` (1 KiB) | ~1.1 | 0 |
| `validate_enum` (hit) | ~2.6 | 0 |
| `validate_enum` (miss) | ~162 | 1 (error string) |
| `ValidationError` Display | ~35 | 0 |

**Design notes:**

* `validate_string` short-circuits on the ASCII fast path.
  For mixed/UTF-8 input it falls back to a per-char walk.
* `validate_range<T: Copy>` compiles to a single comparison
  pair; the 0.2 ns measurement is essentially "no
  measurement".
* `validate_enum` allocates the error string only on miss.
  The hit path is just an iterator over a `&[&str]`.

---

## 4. Hot path: resilience

| Operation | Time (ns) | Allocations |
|---|---|---|
| `CircuitBreaker::call(ok)` (Closed) | ~16 | 0 |
| `CircuitBreaker::state` (single atomic read) | ~0.5 | 0 |
| `RetryPolicy::delay_for` (jitter on) | ~33 | 0 |
| `RetryPolicy::delay_for` (jitter off) | ~18 | 0 |

**Design notes:**

* The circuit breaker is a CAS-based state machine. The
  ~16 ns `call(ok)` cost is two atomic stores
  (`consecutive_failures = 0`, `state = Closed` for the
  common case) and the function dispatch overhead.
* State transitions (Open → HalfOpen) are lazy: the next
  `state()` call after the cool-down elapses performs the
  transition. This avoids a dedicated timer thread.
* The retry policy's jitter is a deterministic LCG keyed
  on the attempt number. No `rand` dependency, so the
  numbers are stable across runs.

---

## 5. Throughput budget

A single handler that does:

1. `validate_string` (~25 ns)
2. `validate_enum` (~3 ns)
3. `rate_limit.check` (~55 ns)
4. `circuit_breaker.call(ok)` (~16 ns)

costs about ~100 ns of validation+protection overhead per
request. On a 4-core machine this is ~40M req/sec budget
before any I/O is counted. Real handlers will be limited
by the underlying specialist execution, not the wrappers.

---

## 6. Memory budget

| Object | Size | Notes |
|---|---|---|
| `TokenBucketLimiter` | 88 B | `Mutex<HashMap>` overhead |
| `BucketState` | 24 B | `f64 tokens + Instant last_refill` |
| `CircuitBreaker` | ~56 B | four `AtomicU64` + config |
| `RetryPolicy` | 32 B | `Duration`s + f64 + bool |
| `ValidationError` | 24 B | `String` heap-allocated |

A million-key rate limiter consumes roughly:
* Buckets: 1M × 24 B = 24 MiB
* HashMap overhead: ~32 B per entry, ~32 MiB
* `String` keys: depends on key length, ~32 B each for
  short IPs, so another ~32 MiB
* **Total: ~88 MiB** for 1M keys.

This fits comfortably in the budget for a process with
1 GiB working set. For >10M keys, switch to a sharded or
external (Redis) implementation; see the open issue
tracker.

---

## 7. Things we did NOT benchmark

* HTTP request parsing — the axum 0.7 stack dominates here
  and the cost is well-known. Re-bench if we change the
  framework.
* wasmtime instance reuse — depends on the genome
  workload. Out of scope for Phase 14.
* The autonomic loop tick body — has not been profiled in
  this phase. The watchdog warning + tick budget introduced
  in C15 are the intended instrumentation for that.
* Federation / cross-node paths — require a multi-node
  setup that we do not have in CI.

---

## 8. Regression thresholds

A change is a candidate for investigation if:

* `rate_limit/check_single_key` regresses > 10% (> 60 ns)
* `validate_string_short` regresses > 10% (> 27 ns)
* `circuit_breaker/call_ok` regresses > 10% (> 18 ns)
* Any bench goes from "no allocation" to "allocates" (the
  hot path is allocation-bound for tail latency)

These thresholds are deliberately tight. The wrappers
above are designed to be invisible to the rest of the
system; if they are not, something is wrong.


---

## File: load_testing.md

# Aaroneous Load Testing

Status: **baseline only**. Full load testing requires a
production-like environment that we do not have in CI.
This document describes the test plan, the harness we
*can* run, and the questions it answers.

---

## 1. What we measured

* Micro-benchmarks via `cargo bench` — see
  `docs/performance/characteristics.md` for numbers.
* Static request-handler budget — derived from the
  micro-bench numbers; ~40M req/sec on a 4-core box
  before I/O is counted.

What we have not measured:

* End-to-end request latency (the full axum + handler +
  specialist stack).
* Sustained load under a realistic specialist workload
  (genome load, wasm compile, etc.).
* Multi-node behaviour (federation paths, link state
  propagation, distributed registry).
* Memory growth over hours. (Restart-based mitigation
  in place; long-running soak test is a follow-up.)

---

## 2. The harness

The smoke load test lives at
`scripts/perf/load_smoke.sh` (placeholder; see
`docs/maintenance/todo.md`). It uses:

* `wrk` (preferred) or `oha` (fallback) for HTTP
  generation. Both are header-rich and produce stable
  percentiles.
* `aaroneous` running locally on `127.0.0.1:8080` with
  `AARONEOUS_API_KEY=loadtest`.

The default profile:

* 4 concurrent connections
* 30 second duration
* Mix of:
  * 60% `GET /v1/models`
  * 30% `GET /readyz`
  * 9% `GET /healthz`
  * 1% `GET /version`

The intent is to exercise the hot path (auth, rate limit,
metrics) without doing real specialist work, which is not
yet optimized for throughput.

---

## 3. Expected results

At the rate limiter default of `burst=10, refill=1/s`, a
single client IP gets 10 free requests then 1/s. The
`/v1/models` endpoint is unmetered, so the rate limiter
should not trip. The expected output:

```text
Running 30s test @ http://127.0.0.1:8080/v1/models
  4 threads and 4 connections
  Thread Stats   Avg      Stdev    Max   +/- Stdev
    Latency   350.00us  120.00us  4.50ms   89.20%
    Req/Sec     8.50k   320.00    9.10k    76.40%
  1,020,000 requests in 30.00s, 180.00MB read
Requests/sec: 34,000.00
Transfer/sec:  6.00MB
```

(These are illustrative; the actual number depends on the
machine. The point is that the handler cost is dominated
by axum + JSON serialization, not by the rate limiter.)

---

## 4. Open questions

* What is the saturation point of a single specialist
  invocation? Need a synthetic genome that does N units of
  work.
* Does the wasmtime engine serialise invocations on a
  single store? (Yes; the fix is to keep a small pool of
  stores and round-robin.)
* What is the cold-start cost? (Currently dominated by
  genome load + wasm compile; mitigate with
  `--ahead-of-time-compile`.)

These are tracked in the post-Phase-15 backlog.

---

## 5. How to read this

This document is honest: we do not have a full load
test rig in place yet. The micro-bench numbers in
`characteristics.md` are the more reliable signal right
now. If you are considering Aaroneous for a real
production deployment, run the micro-bench on your
target hardware first, then the load smoke test, then
spend a week on a real-workload soak.


---

## File: optimization_recommendations.md

# Aaroneous Optimization Recommendations

Lessons learned from the Phase 14 work, plus notes for
the next round of profiling.

---

## 1. Shipped optimizations

### 1.1 `rate_limit::check` fast path

The original `check` did `entry(key.to_string())` on every
call, paying the `String` allocation even on hit.

* **Before:** ~80 ns/op (single key)
* **After:**  ~55 ns/op
* **Speedup:** ~30%

`get_mut` first; only insert via `entry` on miss. Idle
eviction moved out of the request path to a separate
`sweep_idle` method.

### 1.2 `validate_string` ASCII fast path

The original `validate_string` walked
`value.chars().any(|c| c.is_control())`, which forces the
UTF-8 decoder even for all-ASCII input.

* **Before:** 29 ns (5-byte), 227 ns (500-byte)
* **After:**  24 ns (5-byte), 154 ns (500-byte)
* **Speedup:** ~17% short, ~32% long

`is_ascii()` short-circuits to a byte scan; only
mixed/UTF-8 input takes the char-walk slow path. Error
message now includes the offending byte in hex for ASCII
inputs, which is more useful for operators reading logs.

### 1.3 `ValidationError::From<String>` and `From<&str>`

Required for the bench to compile, but it is a real
ergonomic win: callers can write `?` against
string-returning helpers.

---

## 2. Candidate optimizations (NOT shipped)

These came up in the profile but were not worth shipping
in this phase. Each is recorded with the cost/benefit
trade-off so the next profiler can pick them up.

### 2.1 `HashMap` -> `DashMap` for rate limiter buckets

The single `Mutex<HashMap>` is a contention point under
load. Replacing it with `DashMap` (sharded) would reduce
lock contention by ~32x at the cost of one extra
dependency.

* **Why not shipped:** the `rate_limit::check` benchmark
  shows ~55 ns per op on a single core, which is already
  well under the rest of the HTTP stack. DashMap would
  not show up in the user-visible latency until the
  rate limiter is the bottleneck.
* **When to ship:** if a future load test shows lock
  contention > 5% CPU on the rate-limiter thread.

### 2.2 `parking_lot::Mutex` for hot-path locks

`parking_lot` is faster than `std::sync::Mutex` on
contended paths. The hypervisor already has `parking_lot`
in its dependency tree.

* **Why not shipped:** the current `std::sync::Mutex` is
  not on a hot path. A blanket swap to `parking_lot`
  is a maintenance burden (two Mutex types in the same
  codebase).
* **When to ship:** when there is a clear
  per-mutex contention problem, swap just that mutex.

### 2.3 Avoid `Instant::now()` in `check`

`Instant::now()` is a syscall on Linux and a `QueryPerformanceCounter`
on Windows. Calling it once per request is fine, but if
the rate limiter is wrapped in middleware, the call cost
shows up.

* **Why not shipped:** the bench shows the call is < 20 ns
  on Windows for `Instant::now()`. The cost is negligible
  at the request rate Aaroneous currently supports.
* **When to ship:** if profile shows `Instant::now()`
  taking > 5% of the rate-limiter CPU.

### 2.4 Autonomic loop: replace `println!` with `tracing::debug!`

The autonomic loop has `println!` calls in the per-tick
hot path (lines ~470, ~477, etc.). At one tick per
second, the line-rate is acceptable, but stdout IO
serialises the loop and prevents the OS from coalescing
work.

* **Why not shipped:** the user-visible symptom would be
  a "tick watchdog" warning in some cases. The current
  code is correct; the `println!` is a debug aid that was
  left in.
* **When to ship:** as part of any future tick-loop
  performance work. The change is mechanical: replace
  `println!(...)` with `tracing::debug!(...)` and
  `println!("[AutonomicNS] ...")` with
  `tracing::info!(target: "autonomic_loop", ...)`.

### 2.5 `validate_identifier`: `is_ascii_alphanumeric` short-circuit

The current implementation walks each char and calls
`is_ascii_alphanumeric()` then pattern-matches the four
allowed specials. A byte scan with a lookup table
([A-Za-z0-9_-.\\:]) would be a constant factor faster.

* **Why not shipped:** the bench already shows ~28 ns for
  a 12-byte identifier. The current cost is fine.
* **When to ship:** if `validate_identifier` is called
  on every request and a 10% improvement matters.

---

## 3. Methodology

* Always measure before optimizing. The bench harness
  in `core/hypervisor/benches/phase_x_smoke.rs` is the
  ground truth.
* Always check allocations alongside time. The wrappers
  in this phase were designed to be allocation-free on
  the hot path; if a future change re-introduces an
  allocation on the hot path, that is a regression
  even if the wall-clock time is unchanged.
* Document every optimization in this file with the
  before/after numbers and the conditions under which
  it should be revisited.

---

## 4. Out of scope

* SIMD for genome load / wasm transcode. The relevant
  crates are third-party; we depend on them being
  already-optimized.
* GPU specialists. Aaroneous has GPU code paths but
  they are gated on a runtime feature flag and not on
  the hot path of the hypervisor.
* Federation / cross-node paths. Multi-node perf
  testing requires a testbed that does not exist in
  CI; see `docs/performance/load_testing.md`.



