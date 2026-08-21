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
