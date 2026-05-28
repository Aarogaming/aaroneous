# Phase 6: System Matrix — Computational Logic Systems

**Status:** Implemented and tested  
**Test count:** 768 pass, 0 fail, 3 ignored (`cargo test -p a_run --lib`)  
**Modules:** 13 source files, 38 systems total

---

## Module Map

| Module | File | Systems |
|--------|------|---------|
| **Symbolic Math** | `symbolic_math.rs` | CAS engine: AST, 6 derivative rules, evaluation, simplification |
| **Predictive Models** | `predictive_models.rs` | Kalman filter (1D pos/vel), HMM with Viterbi |
| **Cellular Automata** | `cellular_automata.rs` | FSM compiler, N-body clustering, orbital scheduler, data lifecycle, superposition, tunneling, valence bonding, RNA adapter, VSA space inflator |
| **System Integrity** | `system_integrity.rs` | Byzantine consensus, Zipfian cache, Braess optimizer, Little's law backpressure, Anna Karenina watchdog, asymmetric hash verification |
| **Relativity Engine** | `relativity_engine.rs` | Lorentz clock, Minkowski metric, geodesic curvature, light-cone causality |
| **Fluid Routing** | `fluid_routing.rs` | Navier-Stokes data routing, thermal entropy sweeping |
| **Quantum Surface** | `quantum_surface.rs` | Color confinement, decoherence gate, holographic projection |
| **Inter-Agent** | `inter_agent.rs` | A2A binary protocol, Byzantine rollups |
| **Visual Perception** | `visual_perception.rs` | Adversarial UI defense, action vocab tokenization, UI surface map |
| **Reasoning** | `reasoning.rs` | Viterbi depth scaler, SAE feature steering, SSM linear convolutions |
| **Execution** | `execution.rs` | Speculative execution trees, DAG task scheduler |
| **Compression** | `compression.rs` | BitNet 1.58b ternary quantization, VSA-RAG |
| **Hardware Layer** | `hardware_layer.rs` | Secure enclave isolation, shmem UI overlay |

---

## 1. Symbolic Math Engine (`symbolic_math.rs`)

### CAS AST

```
SymbolicNode
├── Constant(f64)
├── Variable(u64)         — name hashed to u64
├── Expression { op, left, right }
│   └── MathOperator::Add | Subtract | Multiply | Divide | Power
└── Unary { op, operand }
    └── MathOperator::Sine | Cosine | Exponential | NaturalLog
```

### Derivative Rules Implemented

| Rule | Operation | Formula |
|------|-----------|---------|
| Constant | `d/dx(c) = 0` | — |
| Variable | `d/dx(x) = 1` | — |
| Sum | `d/dx(u+v)` | `u' + v'` |
| Difference | `d/dx(u-v)` | `u' - v'` |
| Product | `d/dx(u·v)` | `u·v' + v·u'` |
| Quotient | `d/dx(u/v)` | `(v·u' - u·v') / v²` |
| Power | `d/dx(uⁿ)` | `n·uⁿ⁻¹·u'` |
| Sine | `d/dx(sin(u))` | `cos(u)·u'` |
| Cosine | `d/dx(cos(u))` | `-sin(u)·u'` |
| Exponential | `d/dx(eᵘ)` | `eᵘ·u'` |
| Natural Log | `d/dx(ln(u))` | `(1/u)·u'` |

### Simplification

- `x + 0 → x`, `0 + x → x`
- `x - 0 → x`
- `x * 0 → 0`, `x * 1 → x`
- `x / 1 → x`
- Constant folding (`2 + 3 → 5`, `sin(0) → 0`)

---

## 2. Predictive Models (`predictive_models.rs`)

### Kalman Filter (1D)

State vector: `[position, velocity]ᵀ`  
Observation model: `H = [1, 0]` (position only)  
Process noise `Q` and measurement noise `R` configurable.

**Corrected covariance math** (both predict and update steps verified against textbook derivation):

Predict:
- `x' = F·x` where `F = [[1, dt], [0, 1]]`
- `P' = F·P·Fᵀ + Q`

Update:
- `y = z - H·x` (innovation)
- `S = H·P·Hᵀ + R` (innovation variance)
- `K = P·Hᵀ·S⁻¹` (Kalman gain)
- `x = x + K·y`
- `P = (I - K·H)·P`

### Hidden Markov Model with Viterbi

- N-state, M-symbol model
- Initial, transition, and emission probabilities as `Vec<f64>`
- Viterbi decoding returns most likely state path and probability
- Used for inferring user workflow intent from observed action sequences

---

## 3. Cellular Automata (`cellular_automata.rs`)

### 9. FSM Compiler
Deterministic finite state machine: `(state, input) → (output, next_state)`. Compiles action sequences into compressed transition tables.

### 10. N-Body Clustering
Brute-force O(n²) gravitational simulation. Each VSA vector is a body with mass and 2D position. Used to cluster similar vectors by gravitational attraction with softening factor to prevent singularities.

### 11. Orbital Scheduler
Time-based periodic task scheduling. Each task has a period in ticks; fires an action when its counter reaches the period. Zero allocation at runtime beyond the task list.

### 12. Data Lifecycle Manager
Tier 0 (hot) → Tier 1 → Tier 2 → Tier 3 (cold) promotion/demotion based on access frequency and age. Promote threshold (access count) and demote threshold (ticks since last access) are configurable.

### 13. Superposition
Qubit-inspired probability amplitude overlay. Each of N basis states has `(real, imag)` amplitude. Supports Born rule collapse (max probability) and Hadamard transform (equal superposition).

### 14. Tunneling Gate
Quantum tunneling metaphor with exponential probability: `P = exp(-ΔE / E_barrier)`. When particle energy exceeds barrier energy, tunneling is guaranteed. Below barrier, tunneling probability drops exponentially.

### 15. Valence Bonding
Forms shared bonds between VSA vectors based on popcount similarity (fraction of matching bits). Bonds form when overlap exceeds configurable threshold.

### 16. RNA Adapter
Transcribes byte sequences into opcode/operand instruction pairs. Each 9-byte chunk produces one instruction: opcode = byte[0] % 64, operand = remaining 8 bytes as u64.

### 17. VSA Space Inflator
Dimensionality expansion: maps N-dimensional VSA vectors into N×M dimensional space via linear interpolation. Used to resolve hash collisions in high-density regions.

---

## 4. System Integrity (`system_integrity.rs`)

### 18. Byzantine Consensus Gate
N-replica fault-tolerant agreement tolerating up to `(N-1)/3` faulty nodes. Quorum size = `2f + 1`. Proposals are collected per round; once quorum agrees on a value, it is committed.

### 19. Zipfian L3 Cache Split
Partitions cache into hot/warm/cold tiers based on Zipf distribution. Hot ratio, warm ratio, and cold ratio are configurable. Tiebreakers assign entries by access frequency (hot >100, warm >10, cold ≤10).

### 20. Braess Path Optimizer
Anticipates Braess's paradox (adding capacity can slow the system). Each path has base latency and capacity. Effective latency = `base × (1 + load/capacity)`. Load is assigned greedily to minimize total system latency.

### 21. Little's Law Backpressure
`L = λ × W` — controls admission based on queue length and wait time. When expected queue exceeds max, computes drop probability as `(L - max) / L`. Enforces flow control by rejecting arrivals probabilistically.

### 22. Anna Karenina Watchdog
Detects failure cascades: "All happy families are alike; each unhappy family is unhappy in its own way." Accumulates failure events per component; declares cascade when N+ distinct error codes occur within a time window.

### 23. Asymmetric Hash Verification
Fast to verify (single hash), slow to forge (brute-force nonce search). Verification requires `work_factor` leading zero bits in `hash(data || nonce)`. Forge uses linear search up to `2^work_factor` (capped at 1M for practicality).

---

## 5. Relativity Engine (`relativity_engine.rs`)

### Lorentz Clock
Asymmetric clock synchronization for multi-speed loops. Each subsystem has `local_rate` and `ref_rate`; `γ = local_rate / ref_rate`. `tick()` advances local ticks and returns dilated reference ticks. `sync(ref_dt)` applies the Lorentz-like transform `dt_local = γ * dt_ref`.

### Minkowski Metric
Spacetime distance: `s² = Δx² + Δy² + Δz² - c²Δt²`. Classifies event pairs as timelike (causally connected, `s² < 0`), lightlike (`s² = 0`), or spacelike (disconnected, `s² > 0`).

### Geodesic Curvature
Treats the vector field as warped spacetime. A curvature tensor `Γ[i][j]` deflects position vectors toward density hot-spots via `result[i] = Σ_j Γ[i][j] * pos[j]`. `train()` performs Hebbian-like updates from observed point covariance.

### Light-Cone Causality
Bitmask dependency enforcement: an action fires only if all prerequisites lie in its past light cone (earlier in time AND timelike/lightlike separated). Prevents out-of-order execution and spacelike-disconnected trigger events.

---

## 6. Fluid Routing (`fluid_routing.rs`)

### Navier-Stokes Data Routing
Models data pipelines as hydraulic networks. Pressure `P = λ / μ` (arrival/service rates). `route(data_size)` sends data to the least-pressured channel; if pressure exceeds threshold, 50% overflows to the next best channel. `drain(dt)` services each channel by its service rate.

### Thermal Entropy Sweeping
Computes Shannon entropy `H = -Σ p(i)·log₂ p(i)` on byte buffers. Periodic `tick()` purges buffers exceeding entropy threshold to reduce CPU thermal load. Also provides `sweep_buffer()` for single-buffer use.

---

## 7. Quantum Surface (`quantum_surface.rs`)

### Color Confinement
Assigns 2-bit color charge (Red=00, Green=01, Blue=10, White=11) to each data chunk via folded hash. `pack_by_color()` returns grouped indices of same-color chunks for contiguous 64-byte cache-line packing.

### Decoherence Gate
Hardware-timed interrupt that collapses probabilistic superposition states into binary decisions. `tick(dt_ns)` advances a timer; fires collapse when timeout is reached. `collapse(&SuperpositionState)` snaps to the most probable basis state via Born rule.

### Holographic Projection
Random projection matrix (Johnson-Lindenstrauss style) flattens high-dimensional vectors onto a 2D boolean surface. Each output bit = `sign(Σ projection[i][j] * input[j])`. Supports f64, byte, and u64 inputs. `surface_to_bytes()` packs the bit surface into byte-aligned storage.

---

## 8. Inter-Agent Protocol (`inter_agent.rs`)

### A2A Binary Protocol
Peer-to-peer binary state flags instead of text chat. Each agent sets capability flags (`set_flag(agent_id, capability, state)`). `negotiate()` checks mutual agreement on a capability across two agents. `drain_pending()` returns all unsent flags for batch broadcast.

### Byzantine Rollups
Compresses 1000+ task steps into a single rolling Merkle-like state root. `append(action_hash, pre_state, post_state)` incrementally updates the root. `finalize()` produces the final root hash. Static `verify(steps)` lets external verifiers recompute the root without re-execution.

---

## 9. Visual Perception (`visual_perception.rs`)

### Adversarial UI Defense
Spatial noise filter for the SIMD XOR-Delta loop. A sliding window counts neighboring changed pixels; isolated changes below a density threshold are zeroed out. Prevents adversarial website tracking traps from hijacking the mouse engine.

### Action Vocab Tokenization
Encodes raw mouse/keyboard events into 64-bit multi-hot tokens. Layout: `[event_type:4][x:16][y:16][button:8][key_code:20]`. Sliding window tokenization groups events into compound patterns for GGUF distillation.

### UI Surface Map
Represents application interfaces as a mesh of geometric coordinate buttons. `build_proximity_edges()` connects elements by Euclidean distance. `navigate(start, target)` runs Dijkstra shortest-path routing through the UI graph.

---

## 10. Reasoning (`reasoning.rs`)

### Viterbi Depth Scaler
Runtime adjustment of Viterbi search depth based on environment novelty. Estimates novelty as `-ln(viterbi_prob)`. When novelty exceeds threshold, depth doubles up to max. `decode_with_depth()` re-runs Viterbi at expanded depth if novelty is high.

### SAE Feature Steering
Sparse autoencoder with greedy matching pursuit encoding. `dictionary` of N atoms × M dimensions. `encode(input)` finds sparse codes; `steer(feature_indices, amplification)` selectively amplifies specific features. `decode(codes)` reconstructs the input.

### SSM Linear Convolutions
HiPPO-initialized state-space model (Mamba-style). `A` matrix initialized as legT (normalized Legendre), `B` as identity-like, `C` as random projection. `step(input)` advances state and produces output with configurable decay. Processes long-range sequences in O(state_dim²) per step.

---

## 11. Execution (`execution.rs`)

### Speculative Execution Trees
Forks low-weight parallel alternate execution branches in virtual WASM worker slots. `fork(action_hash, expected_state)` creates a branch; `check_state(actual_state)` keeps winners and discards mismatched branches. Active count is capped by `max_workers`.

### DAG Task Scheduler
Converts tasks into a topological directed acyclic graph. `add_dependency(parent, child)` builds edges. `build()` populates a ready queue with zero-in-degree tasks. `complete(id)` decrements child in-degrees, unblocking dependent tasks for parallel execution.

---

## 12. Compression (`compression.rs`)

### BitNet 1.58b Ternary Quantization
Maps f64 attention outputs to `{-1, 0, +1}` via threshold. `quantize(input)` produces `Vec<TernaryBit>`. `dot_product(a, b)` uses only integer addition/subtraction. `matvec_mul(A, x)` performs ternary matrix-vector multiply entirely with integer ops, eliminating floating-point entirely.

### VSA-RAG
Replaces text search with popcount-based vector retrieval. `store(vector, metadata)` inserts entries with FIFO eviction at capacity. `retrieve(query, k)` returns top-k most similar entries by popcount similarity (fraction of matching bits). `popcount_sim(a, b)` computes `1 - popcount(a XOR b) / (n * 8)`.

---

## 13. Hardware Layer (`hardware_layer.rs`)

### Secure Enclave Isolation
Encrypted key store sealed to a platform measurement (SGX/SEV-SNP style). `seal_key(id, key)` XORs with measurement; `unseal_key(id)` recovers the original. `verify(expected)` checks enclave identity; `attest(nonce)` returns `measurement XOR nonce` as a simplified attestation report.

### Shmem UI Overlay
Zero-copy frame buffer for translucent desktop overlays. `draw_rect()`, `draw_crosshair()`, and `draw_label()` write directly to a pixel buffer. `clear()` resets to transparent. `blend_onto(primary)` alpha-blends the overlay onto a primary framebuffer using `out = overlay * α + bg * (1-α)`.

---

## Performance Characteristics

| System | Complexity | Allocation Pattern |
|--------|-----------|--------------------|
| CAS derivative | O(tree depth) | Recursive (heap for intermediate nodes) |
| Kalman predict+update | O(1) | Stack only |
| HMM Viterbi | O(T·N²) | Pre-allocated lattice |
| N-body step | O(N²) | Pre-allocated force buffer |
| Orbital scheduler tick | O(tasks) | Vec push per firing |
| Data lifecycle rebalance | O(blocks) | Vec iteration only |
| Superposition | O(N) | Vec allocation on init |
| Tunneling | O(1) | Stack only |
| Valence bonding | O(N·M) popcount | No allocation |
| RNA transcription | O(data/9) | Vec push per instruction |
| VSA inflate | O(N·M) | New Vec allocation |
| Byzantine consensus | O(proposals) | Vec push per submit |
| Zipfian cache split | O(1) | Stack only |
| Braess optimizer | O(N·load/step) | Vec iteration |
| Little's law | O(1) | Stack only |
| Anna Karenina | O(events) | Vec push + HashSet |
| Asymmetric verify | O(1) | Stack buffer |
| Asymmetric forge | O(2^work) | Same as verify per attempt |
| Lorentz clock tick | O(1) | Stack only |
| Minkowski interval | O(1) | Stack only |
| Geodesic slide | O(dim²) | New Vec allocation |
| Light-cone check | O(deps) | Vec iteration |
| Navier-Stokes route | O(channels) | Vec iteration |
| Entropy sweep | O(buf_len) | Frequency array on stack |
| Color confinement | O(64) per insert | Vec push |
| Decoherence tick | O(1) | Stack only |
| Holographic project | O(dim·surface) | New Vec allocation |
| A2A protocol | O(1) per flag | HashMap insert |
| Byzantine rollup | O(steps) | Rolling hash (stack) |
| Visual immunity filter | O(width·height·window²) | In-place mutation |
| Action vocab tokenize | O(events) | Vec push per window |
| UI navigate (Dijkstra) | O(V²) | Vec dist/prev arrays |
| Viterbi depth adjust | O(1) | Stack only |
| SAE encode | O(dict·dim·iter) | Vec codes + residual |
| SSM step | O(state² + state·input) | Vec state + output |
| Speculative fork | O(branches) | Vec push |
| DAG complete | O(children) | Vec iteration |
| Ternary quantize | O(dim) | Vec allocation |
| Ternary dot product | O(dim) | Stack only |
| VSA-RAG retrieve | O(entries·vec_len) | Scored Vec sort |
| Secure enclave seal | O(key_len) | Vec copy |
| Shmem overlay draw | O(area) | Vec fill |

---

## Data Flow Between Systems

```
CAS Engine (symbolic_math)
  └─ derive() → simplified expression
     └─ evaluate() → numerical result
        ├─→ Kalman (predictive_models) → state estimate
        │    └─ HMM Viterbi → most likely intent path
        │       ├─ FSM Compiler (cellular_automata) → action
        │       │  ├─ Orbital Scheduler → periodic execution
        │       │  │  └─ Speculative Exec (execution) → parallel branches
        │       │  │     └─ DAG Scheduler (execution) → topological ordering
        │       │  └─ Data Lifecycle → tier promotion/demotion
        │       └─ Viterbi Depth Scaler (reasoning) → depth-adaptive decode
        │          └─ SSM Linear (reasoning) → long-range memory
        ├─→ Lorentz Clock (relativity_engine) → multi-rate sync
        │    └─ Light-Cone Causality (relativity_engine) → dependency guard
        ├─→ Minkowski Metric (relativity_engine) → spatiotemporal clustering
        │    └─ Geodesic Curvature (relativity_engine) → hotspot navigation
        ├─→ Navier-Stokes Router (fluid_routing) → load-balanced dispatch
        │    └─ Entropy Sweeper (fluid_routing) → background noise purge
        ├─→ SAE Steer (reasoning) → feature amplification
        │    └─ Ternary Quant (compression) → integer-only inference
        ├─→ VSA-RAG (compression) → vector retrieval
        ├─→ Visual Immunity (visual_perception) → adversarial filter
        │    └─ Action Vocab (visual_perception) → multi-hot tokenization
        │       └─ UI Surface Map (visual_perception) → graph routing
        ├─→ A2A Protocol (inter_agent) → peer capability exchange
        │    └─ Byzantine Rollup (inter_agent) → verifiable step proofs
        └─→ Secure Enclave (hardware_layer) → sealed key storage
             └─ Shmem Overlay (hardware_layer) → translucent HUD
```
