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
