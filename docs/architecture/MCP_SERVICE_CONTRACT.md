# MCP Service Authorization Contract (C80 / P02)

## 1. Overview & Objective

The Model Context Protocol (MCP) server provides external AI agents, IDEs, and controllers (such as Devtools or Claude Desktop) with structured tool discovery and execution capabilities.

Prior to Lane C80 (Packet P02), incoming tool calls relied on a self-issued service token with wildcard (`*`) scope and `FullPrivilege` sandbox authority. This violated the least-privilege principle and allowed unauthenticated callers to execute mutating or platform-level tools.

Lane C80 hardens capability dispatch by establishing:
1. **Four Distinct Capability Classes** providing a clear privilege hierarchy.
2. **Caller-Bound Contexts (`CallerContext`)** derived strictly from authenticated transport credentials, ignoring any untrusted caller-supplied fields in request payloads.
3. **Least-Privilege Scoped Dispatch** replacing wildcard self-tokens with narrowly targeted tokens valid only for the single requested tool.
4. **Zero Ambient Clock Dependency** via explicit clock injection (`Clock`, `MockClock`, monotonic timestamps).
5. **Revocation & Expiration Enforcements** rejecting deactivated keys or revoked principals immediately.

---

## 2. Four Distinct Capability Classes

Every capability and tool is categorized into one of four capability classes ordered by increasing privilege:

| Class | Level | Scope & Description | Examples |
|---|---|---|---|
| `ReadOnlyInspection` | 0 | Non-mutating queries, system telemetry, knowledge lookups, and tool discovery. | `knowledge.semantic_query`, `system.status`, `tools/list` |
| `BoundedProposal` | 1 | Dry-run plans, AST transformation suggestions, and patch proposals without host side-effects. | `proposal.draft_patch`, `plan.generate` |
| `Verification` | 2 | Static analysis, code audits, compliance checks, and security diagnostics. | `security.audit`, `review.audit_source` |
| `Mutation` | 3 | State mutations, file modifications, code execution, and platform/hardware actuation. | `code.rewrite_pattern`, `platform.actuate` |

### Least-Privilege Rule
- Callers cannot execute any tool requiring a capability class higher than their granted `max_capability_class`.
- Wildcard scopes (`*`) permit matching all tools **only up to** the caller's authorized capability class; a wildcard scope **cannot** escalate privilege beyond `max_capability_class`.
- Any unclassified or unrecognized tool defaults to requiring `Mutation` (least privilege by default).

---

## 3. Trusted Caller Context (`CallerContext`)

Authorization decisions are made against a validated `CallerContext`:

```rust
pub struct CallerContext {
    pub principal_id: String,
    pub max_capability_class: CapabilityClass,
    pub allowed_scopes: Vec<String>,
    pub sandbox_policy: SandboxPolicy,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub is_revoked: bool,
    pub token_id: Option<uuid::Uuid>,
}
```

### Invariant: Request Payloads Are Untrusted
Callers cannot supply or elevate their own principal, scopes, or capability class inside the JSON-RPC request body (`params`). Any body fields attempting to claim credentials or identities are strictly ignored; the authorization pipeline evaluates only the transport-derived `CallerContext`.

### Authorization Evaluation Flow
When a tool call (`tools/call`) arrives:
1. **Revocation Check**: Rejects with `AuthorizationError::Revoked` if principal or key is revoked.
2. **Expiration Check**: Evaluates `caller.is_expired_at(now_ms)` against injected monotonic time.
3. **Class Hierarchy**: Evaluates `caller.max_capability_class >= required_class`.
4. **Sandbox Policy**: Rejects platform/actuation tools if `Airgapped`; rejects mutating tools if `ReadConstrained`.
5. **Scope Matching**: Ensures tool name is explicitly in `allowed_scopes` or covered by a domain prefix (`domain.*`) or wildcard (`*`).
6. **Least-Privilege Token Issuance**: Mints an ephemeral `CapabilityToken` scoped **only** to `vec![tool_name.to_string()]` with bounded TTL, bound to `caller.principal_id`.
7. **Thermal Gating**: Checks dynamic backpressure (critical thermal backpressure permits only essential security tools).

If authorization fails, the server responds with JSON-RPC error code `-32001` and executes zero side-effects.

---

## 4. Local Development Compatibility Adapter

For backward compatibility in local development environments (e.g. unauthenticated loopback socket):
- The service exposes `CallerContext::local_development(now_ms)`.
- It is bounded to `CapabilityClass::Verification` and `SandboxPolicy::ReadConstrained`.
- It does **not** grant `Mutation` or `FullPrivilege`. Mutating tool calls over unauthenticated channels are rejected by default.
