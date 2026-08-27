# Autonomous Action Threat Model & Security Specification

## 1. Security Philosophy
Aaroneous executes autonomous tasks (AST synthesis, file operations, hardware intercept, P2P networking). All autonomous actions are treated as **security boundaries** guarded by deterministic layers rather than unconstrained LLM agency.

---

## 2. Security Boundaries & Containment Guarantees

### A. Filesystem Sandbox Containment
- **Guaranteed Root**: All file operations (`tool_read_code`, `tool_search_code`, `tool_list_files` in `core/hypervisor/src/mcp_service/service.rs`) canonicalize paths against `WorkspacePaths::discover().root()`.
- **Traversal Prevention**: Relative components (`..`), symlink escapes, and UNC drive traversals outside the workspace root return structured `PermissionDenied` errors.

### B. Network & Interface Hardening
- **Local Bind Default**: Hypervisor and MCP servers bind to `127.0.0.1` by default in `config/config.toml`.
- **API Key Guard**: All MCP HTTP (`/mcp`) and SSE (`/sse`) routes require `Authorization: Bearer <AARONEOUS_API_KEY>` via `mcp_api_key_auth` Axum middleware.
- **P2P Wire Framing**: Live TCP packets use 4-byte length prefixes and type validation to prevent buffer overflows.

### C. Execution & JIT Safety (Sentinel SVDD Guardrail)
- **Latent Manifold Safety**: Before any synthesized code or candidate action vector is dispatched, Sentinel evaluates its distance against the safe SVDD hypersphere ($R = 14.5$).
- **Orthogonal Snapping**: Unsafe candidate vectors exceeding the threshold radius are snapped orthogonally to the nearest safe boundary in $< 2\mu s$.

### D. AST Mutation & Shadow Sandbox Rollback
- **Shadow Sandboxes**: Fabricator and the Dream Engine execute speculative code patches in isolated temp sandboxes.
- **Rollback Guarantee**: If Bayesian posterior confidence $< 0.70$ or unit tests fail, the mutation is immediately rolled back without touching the live tree.
