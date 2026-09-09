# Hypervisor Refactoring Blueprint

**Objective** – Define the explicit structural changes required to refactor the hypervisor core into a zero‑copy, `bytemuck::Pod`‑compliant Aaroneous Cratify Crate (ACC).

---

### 1. Executor & Scheduler (`core/hypervisor/src/executor/` & `scheduler/`)
#### 1.1 Task Identifier Refactor
- **Current**: `String`‑based task IDs (e.g., `task_id: String`).
- **Target**: Replace with a fixed‑size `u64` identifier (`type TaskId = u64;`).
- **Rationale**: A `u64` is a POD type, occupies a single cache line, and can be safely transmitted through lock‑free ring buffers.
- **Implementation Steps**:
  1. Introduce `pub type TaskId = u64;` in a shared `types.rs` module.
  2. Update all function signatures, structs, and enums that previously used `String` to accept `TaskId`.
  3. Where a human‑readable name is still needed (debug output only), maintain a separate optional `TaskMeta { id: TaskId, name: Option<[u8; 32]> }` that is compiled out of the ACC (`#[cfg(feature = "debug")]`).

#### 1.2 Error‑Handling Refactor
- **Current**: Frequent `.unwrap()` / `.expect()` on `Result`s and `Option`s.
- **Target**: Propagate errors using `Result<_, HypervisorError>` and `?` operator. All public APIs return `Result<_, HypervisorError>`.
- **HypervisorError** definition (placed in `error.rs`):
  ```rust
  #[repr(u8)]
  #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
  pub enum HypervisorError {
      InvalidTaskId = 0,
      QueueOverflow = 1,
      SchedulerFailure = 2,
      // … add as needed
  }
  impl std::fmt::Display for HypervisorError {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          write!(f, "{:?}", self)
      }
  }
  impl std::error::Error for HypervisorError {}
  ```
- **Action**: Replace each `.unwrap()` with a match that returns the appropriate `HypervisorError` variant, then use `?` to bubble up.
- **Static‑analysis**: Run `cargo clippy -D clippy::unwrap_used -D clippy::expect_used` as part of CI.

---

### 2. State & Store (`core/hypervisor/src/state/` & `store/`)
#### 2.1 `AppState` Layout Redesign
- **Current fields** (simplified):
  ```rust
  pub struct AppState {
      tasks: HashMap<String, Task>,
      ready_queue: Vec<TaskId>,
      // many dynamic collections …
  }
  ```
- **Target POD layout**:
  ```rust
  #[repr(C)]
  #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
  pub struct Task {
      pub id: TaskId,
      pub priority: u8,
      pub state: TaskState, // another POD enum
      // Fixed‑size payload (e.g., 128 bytes) for task‑specific data
      pub payload: [u8; 128],
  }

  #[repr(C)]
  #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
  pub struct AppState {
      // Max 1024 concurrent tasks – compile‑time constant
      pub tasks: [Task; MAX_TASKS],
      // Bitmap to indicate slot occupation
      pub task_bitmap: [u8; MAX_TASKS / 8],
      // Ring buffer for ready queue (SPSC/MPSC depending on usage)
      pub ready_queue: RingBuffer<TaskId, READY_CAP>,
  }
  ```
- **Constants** (in `constants.rs`):
  ```rust
  pub const MAX_TASKS: usize = 1024;
  pub const READY_CAP: usize = 2048; // power‑of‑two for efficient masking
  ```
- **Benefits**: No heap allocation, deterministic memory footprint, full `Pod` compliance.

#### 2.2 Store Refactor (Persistent KV)
- Replace any `HashMap<String, Any>` used for configuration or metadata with a static array of key/value slots:
  ```rust
  #[repr(C)]
  #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
  pub struct ConfigSlot {
      pub key: [u8; 32],   // fixed‑size UTF‑8, zero‑padded
      pub value: [u8; 64], // encoded value, POD
  }

  pub const MAX_CFG: usize = 256;
  pub type ConfigStore = [ConfigSlot; MAX_CFG];
  ```
- Access is performed via linear scan (acceptable for ≤256 entries) or a compile‑time perfect hash if needed.

---

### 3. Verification Gates
| Gate | Description | Implementation |
|---|---|---|
| **Static Analysis** | `cargo check`, `cargo clippy -D warnings`, `clippy::unwrap_used`, `clippy::expect_used` | CI job `static_analysis` (see `batch2_ci_pipeline_spec.md`). |
| **Pod Compliance** | All public structs/enums derive `Pod` and `Zeroable` | Run `cratify-lint check-pod` on `core/hypervisor/src/**/*.rs`. |
| **Cratify Audit** | Verify no forbidden APIs, correct isolation tier | `cratify audit --strict --path crates/hypervisor_acc`. |
| **Unit Tests** | Tests for task creation, queue push/pop, state bitmap correctness, error propagation. | Include `tests/` module; CI runs `cargo test`. |
| **Benchmark Regression** | Measure scheduler throughput vs. legacy implementation; enforce ≤ 5 % overhead. | `criterion` benchmark under `benches/hypervisor.rs`; CI `benchmark` job fails if overhead > 5 %. |
| **Ring Buffer Correctness** | Property‑based test using `proptest` to ensure no overflow/underflow under concurrent push/pop. | Added in `tests/ringbuffer_prop.rs`. |

---

### 4. Migration Checklist (for developers)
1. **Run inventory script** – confirm `core/hypervisor` is flagged as **Medium** tier.
2. **Create branch** `feature/cratify-batch2-hypervisor`.
3. **Apply code transformations** following sections 1–2.
4. **Run local validation** (`cargo check`, `clippy`, `cratify audit`, benchmarks).
5. **Add/Update tests** to cover new POD structures and error paths.
6. **Open PR** – attach `verification_checklist_utils_acc.md` (or a hypervisor‑specific checklist).
7. **Pass CI** – all jobs in the Batch 2 pipeline must be green before merge.

---

*Document version*: `v0.1‑hypervisor‑refactor‑blueprint` (2026‑09‑09)
