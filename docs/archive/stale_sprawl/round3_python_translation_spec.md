# Python‑to‑Rust Microkernel Translation Blueprint

**Version:** v1.2.0 (Aaroneous Framework)

---

## 1. Objective
Provide a deterministic pipeline (Round 3) that ingests legacy Python modules, performs static auditing, and translates them into clean, zero‑copy Rust Accelerated Component Containers (ACCs) suitable for the Cratify microkernel.

---

## 2. End‑to‑End Pipeline Overview
```mermaid
flowchart TD
    A[cratify inspect <python_pkg>] --> B[cratify audit]
    B --> C[IR generation (python_ir.rs)]
    C --> D[Type‑mapping & cleanup (translate.rs)]
    D --> E[cratify scaffold]
    E --> F[cratify translate]
    F --> G[cratify verify]
    G --> H[cratify harvest]
```

- **Inspect** – Pulls the Python package from PyPI (or local source) and extracts the abstract syntax tree (AST).
- **Audit** – Runs static analysis (mypy‑style type checking, security linting) and verifies that the package does not import forbidden native extensions.
- **IR Generation** – Converts the Python AST into a language‑agnostic intermediate representation (`PythonIR`).
- **Translate** – Maps `PythonIR` constructs to Rust equivalents, inserting explicit zero‑copy contracts.
- **Scaffold** – Generates a new Rust crate skeleton with `cratify.toml`.

---

## 3. Deterministic Intermediate Representation (PythonIR)
| Python Construct | PythonIR Node | Target Rust Mapping |
|------------------|---------------|--------------------|
| `int`, `float`, `bool` | `PrimLiteral { ty: Primitive, value: String }` | `i64`, `f64`, `bool` (Pod) |
| `list` | `Array { elem: Box<PythonIR>, len: Option<usize> }` | `Vec<T>` (Pod if `T: Pod`) |
| `dict` | `Map { key: Box<PythonIR>, value: Box<PythonIR> }` | `HashMap<K, V>` (may need boxing; not Pod – fallback to serialization) |
| `class` | `Struct { name: String, fields: Vec<(String, PythonIR)> }` | `#[repr(C)] struct` with `Pod` derives (if all fields are Pod) |
| `bytes` | `ByteSlice { len: usize }` | `&[u8]` or `Bytes` from the `bytes` crate (Pod) |
| `function` | `Function { name, args, returns, body }` | Stand‑alone `fn` in Rust; arguments/returns converted via the above rules |
| `async def` | `AsyncFunction { … }` | `async fn` using `tokio` runtime (wrapped in ACC) |
| `import` of C extensions | `ForeignModule` | **Reject** – flagged in audit as forbidden.

The IR is a **tree** with deterministic ordering (alphabetical field names, sorted dict keys) to guarantee reproducible hashing.

---

## 4. Type‑Mapping & Zero‑Copy Guarantees
1. **Pod‑Eligibility Check** – For each `Struct` node, the translator verifies that **all** fields are POD (primitive, other POD structs, or fixed‑size arrays). If any field fails, the struct is marked `non_pod`.
2. **Shim Generation** – Non‑Pod structs receive an automatically generated **shim** wrapper that implements `Pod` by flattening the fields into a packed byte array (`[u8; N]`). The shim provides `to_pod()` / `from_pod()` helpers.
3. **Fallback Path** – If a `dict` cannot be represented as a POD map (due to dynamic keys), the translator falls back to **serde JSON** serialization, marking the payload with `serialization = "json"` in the manifest. This path is **not** zero‑copy and will be logged as a warning.
4. **Signature Preservation** – Function signatures are retained in Rust as `extern "C" fn` where possible, enabling direct calls from other ACCs via the `CapabilityBroker`.

---

## 5. Integration with LLM Offload Router (`translate.rs`)
- The **LLM router** receives the `PythonIR` JSON representation and invokes a local Qwen model to perform **semantic cleanup**:
  - Infer missing type annotations.
  - Suggest efficient POD replacements for dynamic containers.
  - Detect anti‑patterns (e.g., global mutable state) and rewrite them into thread‑local storage.
- The router returns an enriched IR that the **type‑mapper** consumes to generate Rust code.
- All LLM‑generated suggestions are **deterministic** when the same seed and model version are used, enabling reproducible builds.

---

## 6. Fallback & Failure Modes
| Failure Condition | Handling |
|-------------------|----------|
| Unmappable C extension import | Abort audit with `ForbiddenForeignModule` error. |
| Struct with mixed POD / non‑POD fields | Generate shim; if shim size exceeds `max_payload_size` (4 KB) abort with `PayloadTooLarge`. |
| LLM refuses to produce a type‑safe mapping | Emit warning, fall back to JSON serialization, and set `serialization = "json"` in the manifest. |
| Runtime test harness fails | `cratify verify` stops and marks the ACC as **quarantine**; the developer must fix the Python source.

---

## 7. Manifest Extensions for Python‑derived ACCs
```toml
[origin]
source = "pypi"
package = "example_pkg"
version = "1.2.3"

[certification]
# Same fields as normal ACCs (seal, abi_layout_hash, etc.)

[metadata]
# Indicates that the ACC was generated from Python.
origin_language = "python"
translation_mode = "static"
```
These fields allow the hypervisor to distinguish Python‑origin ACCs for policy decisions (e.g., sandbox stricter).

---

## 8. Future Extensions
- **Incremental IR caching** – Store generated `PythonIR` objects in a content‑addressable store (iroh) to avoid re‑parsing unchanged packages.
- **GPU‑accelerated translation** – Offload heavy AST traversal to the `burn_gpu` crate.
- **Round 4** – Automatic generation of Python bindings for Rust ACCs (reverse direction).

---

*This blueprint prepares the Cratify toolchain for a robust Round 3 that brings legacy Python workloads into the zero‑copy ACC ecosystem.*
