# Cross-Repository Architectural Audit: Maximizing the Layer 0 Ontology

**Status:** Working Draft & Architecture Specification  
**Author:** Gemini Seat (`aaroneous-gemini-seat[bot]`)  
**Scope:** `Aarogaming/aaroneous` (Type-Safe Rust Component Framework) & `Aarogaming/aaroneous-devtools` (Autonomous Verification & Platform Tooling)  
**Date:** 2026-11-18  
**Governing Documents:** [AGENTS.md](../AGENTS.md), [CRATIFY_SPEC.md](CRATIFY_SPEC.md), [MASTER_ARCHITECTURE.md](architecture/MASTER_ARCHITECTURE.md), and [docs/ATLAS.md](ATLAS.md).

---

## 1. Executive Thesis: What Does "Maximizing" Mean?

To maximize our ontology across all repositories does **not** mean introducing more domain-specific abstractions or writing duplicate framework layers. It means **reducing every subsystem to the smallest set of primitives that survive contact with reality without remainder**.

Across both product execution (`aaroneous`) and governance/coordination (`devtools`), every system is fundamentally an **embodiment system**:

$$\text{Observe Environment (Facts)} \longrightarrow \text{Determine Permission (Authority)} \longrightarrow \text{Execute Step (Actions)}$$

The bedrock invariant that must govern both codebases is:

$$\mathbf{Facts} \neq \mathbf{Authority} \neq \mathbf{Actions}$$

- **Facts** describe reality (observe, store, retrieve). Facts **never authorize actions**.
- **Authority** determines permission (policy boundaries, interlocks, deadbands). Authority **never executes transitions**.
- **Actions** execute state transitions (dispatch, invoke, mutate). Actions **never self-authorize**; they consume authority.

---

## 2. The Layer 0 Primitives: Universal Mapping

Every struct, actor, channel, policy, gate, and script in `aaroneous` and `devtools` reduces to one or more of these six primitives:

| Primitive | Definition | In `Aaroneous` (Product) | In `aaroneous-devtools` (Platform) |
| :--- | :--- | :--- | :--- |
| **State** | Discrete, snapshot-able configuration at a tick. Flat memory geometry, zero hidden heap handles. | Memory geometry (`Pod`/`Zeroable`), `core-contracts`, pre-allocated ring buffers (`ipc_bus`). | `COORDINATION_QUEUE.md`, ledger tables, worker intake queues. |
| **Transition** | Deterministic map: $S_{t+1} = f(S_t, I)$. Zero side-effects during reduction; non-allocating hot paths. | Deterministic state reducers (`compute::automata`, `.si` container steps), scan loops. | Steward job state machine, queue claim state transitions (`Claimed` $\to$ `Review` $\to$ `Done`). |
| **Relationship** | Typed, explicit connection between loci with direction and capacity. No ambient proximity. | SWMR ring buffers, declared zero-copy IPC channels, task affinity topologies. | Peer admission graphs, broker lease channels, dependency DAGs (`dependency_review`). |
| **Constraint** | Boundary limiting state space or valid transitions. Fault tolerance via deadband recovery. | Zero ambient authority, SMT interlocks (`governance`), fuel limits, Cratify ratchets. | Merge gate checklist (boxes 1-7), circuit breaker override, capacity ceilings (1 MiB queue). |
| **Trust** | Real-time evaluated attestation of authorization, provenance, or correctness computed from evidence. | AST auditor invariant proofs, memory-safety invariants, constructor injection. | Multi-family independent review quorum, SSH-signed attestations, 3-way revert equality. |
| **Perspective** | The lens through which state space is viewed. Work unit stays constant when lens rotates. | `studio_hud` viewports, projection read-models, `runtime_monitor` telemetry sinks. | Seat roles (Author vs. Reviewer vs. Monitor), checklist report projections, dashboard UI. |

---

## 3. Product Repository Audit (`Aaroneous`)

### 3.1 What Is Cleanly Aligned
1. **Purity of State**: `core-contracts` and `wire` strictly implement flat memory geometry (`#[repr(C)]`, `bytemuck::Pod`, explicit padding). There are zero hidden pointer handles.
2. **Purity of Transition**: `crates/compute/src/automata.rs` operates as a pure, fuel-bounded cascading reducer. Transitions do not trigger ambient I/O or allocate on hot paths.
3. **Purity of Constraint**: `crates/governance` isolates safety interlocks and deadband threshold controllers (`Nominal` $\to$ `Degraded` $\to$ `SafeHold`), decoupling safety rules from execution machinery.

### 3.2 Leaks & Semantic Smells Detected
1. **Fact vs. Authority Smear in Input Ingestion**:
   - *Finding*: In `platform_bridge` and frame ingestors, raw hardware observations (DXGI frames, audio samples) occasionally transition directly into command dispatch without passing through an explicit, typed `FactQualification` boundary.
   - *Fix*: Standardize a strict `FactQualifier` trait: raw observations remain untrusted facts until validated against an explicit schema and injected into the reducer.
2. **Implicit Ambient Time**:
   - *Finding*: While AGENTS.md strictly bans ambient clock calls (`SystemTime::now()` / `Instant::now()`), components occasionally accept raw tick values without a typed contract defining monotonic sequence ordering vs. duration constraints.
   - *Fix*: Decouple sequence progression (`Transition` step index) from timeout/expiration policies (`Constraint` deadline).
3. **Incomplete ATLAS Census**:
   - *Finding*: In `docs/ATLAS.md`, only `core-contracts`, `governance`, and `omni` have formal classification audits. Key engines (`compute`, `hypervisor`, `orchestrator`, `capabilities`, `platform_bridge`) remain unclassified.

---

## 4. Devtools Repository Audit (`aaroneous-devtools`)

### 4.1 What Is Cleanly Aligned
1. **Purity of Trust**: The merge gate (`merge_gate.py`, `review_coverage.py`) establishes an industry-leading trust model: cryptographic signatures (`ssh-ed25519`), independent model family isolation (no self-certification), and strict command evidence retention.
2. **Mathematical Revertibility (C192)**: PR #278 eliminates manual human bottlenecks by checking 3-way inverse git trees (`merge-tree --write-tree`), reducing revert safety to pure mathematical identity ($T_{\text{revert}} == T_{\text{base}}$).

### 4.2 Leaks & Semantic Smells Detected
1. **Coupling in `COORDINATION_QUEUE.md`**:
   - *Finding*: A single queue row currently conflates three distinct primitives:
     - `State`: Current claim status, lane ID.
     - `Transition`: The implementation branch and proposed commit delta.
     - `Trust`: Signed attestation receipts and audit citations.
   - *Fix*: Decouple the claim ledger (`State`) from the implementation attempt (`Transition`) and the terminal verification record (`Trust`).
2. **Ambient Tool Dependencies**:
   - *Finding*: Scripts and integration tests (`job_service`) occasionally rely on implicit host binaries (e.g., `git`, `ssh-keygen`, `curl`) rather than constructor-injected environment wrappers, leading to platform test friction on Windows.
   - *Fix*: Wrap host tools inside explicit `ToolAdapter` configurations mirroring constructor injection in `Aaroneous`.

---

## 5. The Best Path to Maximizing Our Ontology

To achieve maximum ontological coherence across the ecosystem, we should follow this **4-Pillar Convergence Roadmap**:

### Pillar 1: Universal Repository Census (Complete ATLAS.md)
Complete classification audits for the remaining 7 crates in `Aaroneous`:
- `compute`: Reducer automata (`Transition`), MacroArena (`State`), FuelBudget (`Constraint`).
- `hypervisor`: Microkernel loop (`Transition`), HostContext (`State`).
- `orchestrator`: TopologicalScheduler (`Relationship`), CoreAffinity (`Constraint`).
- `ipc_bus`: RingBuffer (`State` + `Relationship`), LocklessSeq (`Constraint`).
- `capabilities`: ToolRegistry (`Relationship`), InvocationContext (`Perspective`).
- `platform_bridge`: OS Viewports (`Perspective`), Hardware Ingest (`Environment Adapter`).
- `llm_gateway`: ModelRouter (`Relationship`), InferenceLease (`Constraint`).

### Pillar 2: Mechanize the Invariant via `ast_auditor`
Currently, $\text{Facts} \neq \text{Authority} \neq \text{Actions}$ is enforced manually by code reviewers. We should mechanize this into `crates/ast_auditor`:
- **Rule A (Observation Purity)**: Types tagged `#[fact]` or belonging to an observation crate cannot implement actuation traits (cannot expose `.execute()`, `.send()`, `.dispatch()`).
- **Rule B (Authority Separation)**: Types tagged `#[policy]` or `#[governor]` cannot mutate state buffers directly; they return pure `Authorization` tokens.
- **Rule C (Action Gating)**: Any function executing an effect must take a valid, non-forgeable `AuthorizationToken` by value (consuming authority).

### Pillar 3: Universal Environment Adapter Standard
Standardize how both repositories interface with the external world:
- Treat `platform_bridge` (DXGI/Win32/WASAPI), `steward/worker` (Git/CLI/SSH), and future embodiment modules (Browser/Game/Network) under one universal Adapter Contract:
  ```rust
  pub trait EnvironmentAdapter {
      type Observation: Fact;
      type ConstraintSet: Invariant;
      type Command: Action;

      fn observe(&mut self) -> Result<Self::Observation, AdapterError>;
      fn check_constraints(&self, intent: &Intent) -> Result<AuthorityToken, GateBlocked>;
      fn execute(&mut self, action: Self::Command, auth: AuthorityToken) -> Result<Receipt, ExecutionError>;
  }
  ```

### Pillar 4: Bridge Runtime State to Cryptographic Trust
Connect devtools' multi-model review system directly to Aaroneous runtime governance:
- Dynamic configuration changes, feature promotions, and autonomous adjustments in `Aaroneous` should require the same cryptographic attestation block verified by `merge_gate.py`.
- Runtime governors reject any unsigned or un-quorumed configuration update, providing zero-trust mechanical safety from PR merge all the way into live kernel execution.

---

## 6. Conclusion

Maximizing our ontology is **not a rewrite**; it is the **purification of existing boundaries**.
By auditing each subsystem down to the 6 Layer 0 primitives, enforcing $\text{Facts} \neq \text{Authority} \neq \text{Actions}$ mechanically in the AST auditor, and applying the Environment Adapter contract universally, repository boundaries dissolve naturally into a single, cohesive reality model.
