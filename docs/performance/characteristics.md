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
