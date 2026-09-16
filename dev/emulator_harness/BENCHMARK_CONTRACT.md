# Bounded Reducer Performance & Assurance Contract

**Status:** Version 1.0 (Active — M27)  
**Target:** dev/emulator_harness (educe_trace_bounded, xtract_state_delta)

---

## 1. Assurance Guarantees

- **Zero Allocation Invariant:** Hot-path trace reduction operations (educe_trace_bounded, xtract_state_delta) MUST execute with exactly zero dynamic heap allocations (GlobalAlloc::alloc calls = 0).
- **Deterministic Replay:** Given an identical sequence of TraceEvent inputs and a target memory address, educe_trace_bounded produces identical ReductionSummary metrics across all platforms (x86_64, ARM64, WASM).
- **Budget Compliance:** Execution halts immediately upon reaching ReductionBudget::max_events, returning TraceError::BudgetExceeded without processing further slice elements.

---

## 2. Measurement Profile & Setup

- **Harness:** Criterion.rs micro-benchmarking framework (enches/reduction_benchmark.rs).
- **Input Batches:** 1,000 to 10,000 TraceEvent contiguous arrays (40-byte Pod layout per event).
- **Environment Rules:**
  - Build profile: --release (opt-level 3, LTO enabled).
  - Target machine: x86_64 / Windows or Linux.
  - Warmup iterations: 100 cycles minimum.

---

## 3. Baseline Latency & Throughput Targets

| Operational Target | Batch Size | Latency Goal | Throughput Goal |
|---|---|---|---|
| Trace Delta Extraction (xtract_state_delta) | 1,000 events | $< 2.5\,\mu\text{s}$ | $> 400\text{k events/ms}$ |
| Bounded Trace Reduction (educe_trace_bounded) | 1,000 events | $< 3.0\,\mu\text{s}$ | $> 330\text{k events/ms}$ |
| Micro-Batch Reduction | 100 events | $< 300\,\text{ns}$ | $> 330\text{k events/ms}$ |

---

*This contract is owned by devtools governance. Run cargo check -p emulator_harness --benches to verify benchmark compilation.*
