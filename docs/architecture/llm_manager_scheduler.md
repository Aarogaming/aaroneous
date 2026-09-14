# LLM Manager & Priority-Constrained Scheduler Specification

> **CANONICAL SPECIFICATION**  
> **SCOPE**: Stateless Transducer Interface, Control Plane Separation, and Priority Heap Backoff Scheduling.  
> **APPLIES TO**: `crates/llm_gateway`, `crates/orchestrator`, `crates/capabilities`.  
> **LAST UPDATED**: 2026-09-12

---

## 1. Architectural Boundary: Gateway vs. Orchestrator

The Aaroneous framework enforces a strict architectural boundary between LLM communication transport and stateful task orchestration:

```
+─────────────────────────────────────────────────────────────────────────────+
|                    crates/orchestrator (STATEFUL CONTROL PLANE)             |
|  • Goal Trees & Task DAGs                                                   |
|  • Priority Heap (`Critical`, `Standard`, `Background`)                     |
|  • Jittered Exponential Backoff Scheduler                                   |
|  • Context Window Assembly & Token Budget Allocation                        |
|  • State Reducers & Decision Synthesis                                      |
+─────────────────────────────────────────────────────────────────────────────+
                                       │
                                       │ Stateless Requests (Prompt + Schema)
                                       ▼
+─────────────────────────────────────────────────────────────────────────────+
|                    crates/llm_gateway (STATELESS TRANSPORT PLANE)           |
|  • HTTP / TCP REST & Streaming MCP Wire Protocols                           |
|  • Zero Agent State & Zero Session Retention                                |
|  • Request Transformation & Header Authentication                           |
|  • Raw JSON / Token Stream Parsing                                          |
+─────────────────────────────────────────────────────────────────────────────+
                                       │
                                       │ Network I/O
                                       ▼
                     External Inference Endpoints (Local / Cloud)
                     (Ollama, vLLM, Anthropic, OpenCode, OpenAI)
```

### 1.1 Invariant Rules of the Boundary

1. **Zero Session State in `llm_gateway`**: The gateway retains no conversation history, prompt templates, or working memory between calls. Every invocation is a pure function from a single request packet to a parsed response packet.
2. **Zero Direct Execution by Models**: External models are treated as untrusted, stateless transducers. They never directly execute shell commands, allocate system resources, or mutate repository files.
3. **Control Plane Exclusivity**: The orchestrator alone determines whether an LLM call is required, what context is injected, when retries occur, and how model outputs are validated.

---

## 2. The Stateless Transducer Model

In Aaroneous, language models do not function as autonomous agents. They are modeled mathematically as **stateless transducers**:

$$\mathcal{T}: \Sigma^* \times \mathcal{G} \longrightarrow \Omega$$

Where:
- $\Sigma^*$ is the structured prompt string with bounded token count.
- $\mathcal{G}$ is a deterministic schema (JSON schema, AST grammar, or typed Pod layout).
- $\Omega$ is the validated structured output conforming strictly to $\mathcal{G}$.

```
                 Structured Context Buffer (Bounded Tokens)
                                     │
                                     ▼
                +─────────────────────────────────────────+
                |           Stateless Transducer          |
                |               Model T(x)                |
                +─────────────────────────────────────────+
                                     │
                                     ▼
                           Raw Candidate Output
                                     │
                                     ▼
                +─────────────────────────────────────────+
                |        Grammar & Pod Schema Gate        |
                |          (ast_auditor / syn)            |
                +─────────────────────────────────────────+
                       │                           │
                   Valid Output              Schema Violation
                       ▼                           ▼
               State Reduction             Priority Escalation
               (S_{t+1} = f(S_t, I))       (Re-queue as Critical)
```

If the candidate output fails grammar or schema validation, it is rejected immediately. No malformed tokens ever cross the boundary into Ring 1 or Ring 2.

---

## 3. Priority-Constrained Scheduler & Queue Topology

All outbound transducer tasks pass through a bounded priority heap managed by [`crates/orchestrator`](file:///d:/Aaroneous/crates/orchestrator).

### 3.1 Three-Tier Priority Taxonomy

| Tier | Priority Weight ($W_p$) | Allocation Target | Permitted Workloads |
|---|---|---|---|
| **`Critical`** | $4.0$ | Immediate next tick | Compilation failure repairs, system invariant violations, hypervisor panic triage, safety interlock events. |
| **`Standard`** | $1.0$ | Normal round-robin | Feature code generation, unit test creation, Socratic intent responses, documentation synthesis. |
| **`Background`** | $0.25$ | Spare hypervisor cycles | Offline `.si` distillation, speculative habit mining, vector graph clustering, background doc indexation. |

---

## 4. Jittered Exponential Backoff Dynamics

To prevent thread congestion, rate-limit thrashing, and thundering-herd issues during endpoint saturation or transient failures, retries follow a priority-weighted exponential backoff algorithm:

$$\text{Delay}(p, k) = \min\left(D_{\text{max}},\ \left(D_{\text{base}} \cdot 2^k\right) \cdot \frac{1}{W_p}\right) + J$$

### 4.1 Parameter Definitions

- $D_{\text{base}}$: Base backoff interval (default: $100\,\text{ms}$).
- $k$: Consecutive retry attempt count ($k \in [0, k_{\text{max}}]$ where $k_{\text{max}} = 5$).
- $W_p$: Priority weight ($W_{\text{Critical}} = 4.0$, $W_{\text{Standard}} = 1.0$, $W_{\text{Background}} = 0.25$).
- $D_{\text{max}}$: Upper ceiling cap (default: $30.0\,\text{s}$).
- $J$: Deterministic pseudorandom jitter drawn uniformly from $[0, 0.2 \cdot D_{\text{base}}]$ using a thread-local splitmix64 state seeded by cycle timestamp.

### 4.2 Behavior Across Tiers

Notice the inverse proportionality to $W_p$:
- **`Critical` Tasks ($W_p = 4.0$)**: Backoff interval is compressed by a factor of 4. A first retry occurs after $\approx 25\,\text{ms}$, allowing rapid recovery of broken builds or panic states.
- **`Standard` Tasks ($W_p = 1.0$)**: Follows standard binary exponential backoff ($100\,\text{ms}, 200\,\text{ms}, 400\,\text{ms}, \dots$).
- **`Background` Tasks ($W_p = 0.25$)**: Extended backoff interval ($400\,\text{ms}, 800\,\text{ms}, 1600\,\text{ms}, \dots$) ensuring that background distillation never contends with interactive developer workflows.

---

## 5. Event-Driven Priority Escalation

Tasks can dynamically transition between priority classes based on downstream execution telemetry:

1. **`CompilationFailure` Event**: When a generated module fails `cargo check` or `ast_auditor`, the remediation task is automatically promoted to `Priority::Critical`.
2. **`InvariantViolation` Event**: If an emitted patch attempts an ambient call (`std::env::var`) or heap allocation on a hot path, the task's retry budget is decremented, its weight is boosted to $4.0$, and it is prepended to the front of the scheduler queue.
3. **`StarvationPrevention` Escalation**: If a `Background` task remains unserviced for longer than $T_{\text{starve}} = 60\,\text{s}$, its effective weight is temporarily promoted to $W_{\text{Standard}}$ to guarantee forward progress.
