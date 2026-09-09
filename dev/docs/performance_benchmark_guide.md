# Performance Benchmarking & Stress‑Testing Guide

**Purpose**: Provide a rigorous, reproducible methodology for measuring the throughput, latency, and concurrency characteristics of the inter‑ACC ring‑buffer network and the Cratify certification pipeline.

---

## 1️⃣ Benchmarking Strategy

| Metric | Description | Tool / Approach |
|---|---|---|
| **Throughput (msgs/sec)** | Number of messages successfully transferred per second across the ring buffer. | `criterion` benchmarks using a tight loop; custom harness for sustained load.
| **Message Latency (µs)** | End‑to‑end time from send to receive for a single payload. | `tokio::time::Instant` timestamps in a multi‑threaded test harness.
| **Cache‑Line Contention** | Measure false‑sharing effects using hardware performance counters (e.g., `perf stat` on Linux, `xperf` on Windows). | `perf record -e cache‑references,cache‑misses` while running the benchmark.
| **Memory Footprint per ACC Node** | Total heap + stack memory allocated for a single ACC instance, including ring‑buffer allocations. | `malloc_size_of` from `jemalloc‑ctl` or `heaptrack`.
| **Scalability (SPSC vs MPSC)** | Compare Single‑Producer‑Single‑Consumer (SPSC) against Multi‑Producer‑Single‑Consumer (MPSC) configurations across pool sizes (XS‑XL). | Parameterised `criterion` groups per pool size.

### 1.1 SPSC Benchmark (XS‑XL)
```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ringbuf::{RingBuffer, SpscQueue};

fn bench_spsc(c: &mut Criterion) {
    let sizes = [64, 256, 1024, 4096, 16384]; // XS … XL
    for &size in &sizes {
        c.bench_with_input(BenchmarkId::new("spsc", size), &size, |b, &sz| {
            let rb = RingBuffer::<u64>::new(sz);
            let (mut prod, mut cons) = rb.split();
            b.iter(|| {
                prod.push(42).ok();
                let _ = cons.pop();
            });
        });
    }
}
criterion_group!(benches, bench_spsc);
criterion_main!(benches);
```

### 1.2 MPSC Benchmark (XS‑XL)
```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ringbuf::{RingBuffer, MpscQueue};
use std::thread;

fn bench_mpsc(c: &mut Criterion) {
    let sizes = [64, 256, 1024, 4096, 16384];
    for &size in &sizes {
        c.bench_with_input(BenchmarkId::new("mpsc", size), &size, |b, &sz| {
            let rb = RingBuffer::<u64>::new(sz);
            let (mut prod, mut cons) = rb.split();
            let producer = thread::spawn(move || {
                for _ in 0..1000 { let _ = prod.push(42); }
            });
            b.iter(|| {
                let _ = cons.pop();
            });
            producer.join().ok();
        });
    }
}
criterion_group!(benches, bench_mpsc);
criterion_main!(benches);
```

---

## 2️⃣ Acceptance Thresholds

| Metric | Target (per pool) |
|---|---|
| **Throughput** | ≥ 10 M msg/s for XL (≥ 64 KB ring) – linear scaling from XS.
| **Latency (p90)** | ≤ 5 µs for SPSC XL; ≤ 15 µs for MPSC XL.
| **Cache‑Line Contention** | < 2 % L1 cache miss increase vs baseline empty loop.
| **Memory Footprint** | ≤ 2 × `size_of::<RingBuffer>` per node (including alignment padding).
| **Stress‑Test Duration** | Minimum 30 s continuous load without panic or dead‑lock.

If any metric falls outside its target, the implementation is **non‑compliant** and must be revisited before merging.

---

## 3️⃣ Stress‑Testing the Certification Pipeline (`certify.rs`)

1. **Generate a Deeply Nested Crate** – Create a synthetic crate with 10 levels of module nesting, each exposing dozens of public functions and types.
2. **Populate with Heavy Payloads** – Each function returns a `Vec<u8>` of 1 KB to stress memory handling.
3. **Run the Full Cratify Pipeline** on this crate repeatedly (e.g., 100 iterations) while measuring:
   * Total wall‑clock time for `cratify audit` → `certify` → `scaffold`.
   * Peak RSS (resident set size) during the audit.
4. **Automation Script (bash / pwsh)**:
```bash
#!/usr/bin/env bash
set -euo pipefail
CRATE_REPO="https://example.com/deep_nested_crate.git"
for i in {1..100}; do
    cratify inspect --repo $CRATE_REPO --output dev/tmp/inspect_$i.json
    cratify audit   --manifest dev/tmp/inspect_$i.json --output dev/tmp/audit_$i.json
    cratify certify --audit dev/tmp/audit_$i.json --output dev/tmp/cert_$i.seal
    cratify scaffold --certification dev/tmp/cert_$i.seal --target-dir crates/deep_nested_acc_$i
    echo "Iteration $i completed"
done
```
5. **Success Criteria**:
   * All 100 iterations complete without `panic!` or `unsafe` leaks.
   * Average per‑iteration wall‑clock time ≤ 30 s.
   * Peak memory ≤ 1 GiB.

---

## 4️⃣ Reporting & CI Integration

* Add a **`cargo bench`** job to the CI pipeline that runs the SPSC and MPSC benchmarks on a dedicated performance runner.
* Export results as JSON (`--output-format=json`) and compare against the thresholds using a small validation script.
* Fail the CI job if any metric deviates beyond the defined tolerance.

---

## 5️⃣ Commit & Review

```bash
git add dev/docs/performance_benchmark_guide.md
git commit -m "feat(dev/docs): add performance benchmarking & stress‑testing guide"
```

---

**Revision History**
- `2026‑09‑09` – initial version.

---

*End of Specification*
