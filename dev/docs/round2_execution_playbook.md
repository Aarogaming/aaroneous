# Round 2 Execution Playbook

**Target ACC:** `tokio` (async runtime)

---

## 1. Overview
This playbook describes how to ingest the `tokio` crate into an Accelerated Component Container (ACC) using the Cratify CLI. The process follows the standard Cratify lifecycle:
`cratify inspect → cratify audit → cratify scaffold → cratify translate → cratify verify → cratify harvest`.

---

## 2. Step‑by‑Step Operational Flow
### 2.1 `cratify inspect <crate>`
1. **Fetch source** – Cratify clones the crate from `crates.io` at the minimum version specified (`1.28`).
2. **Generate provisional `cratify.toml`** – A temporary manifest is created with:
   ```toml
   [acc]
   name = "tokio_acc"
   version = "0.1.0"
   max_blast_radius = "isolated"
   
   [dependencies]
   tokio = { version = "1.28", features = ["full"] }
   ```
3. **Static analysis** – The crate’s AST is parsed and stored for later hashing.

### 2.2 `cratify audit`
1. **AST hash** – SHA‑256 of the canonical AST (`source_hash`).
2. **Dependency audit** – Ensure no forbidden dependencies (e.g., `std`‑only crates, `unsafe` crates) are present. The whitelist is defined in `cratify` config.
3. **Zero‑copy contract check** – Verify that all public structs in `tokio` either:
   - Derive `bytemuck::Pod`/`Zeroable`, *or*
   - Are marked with `#[allow(non_pod)]` in a local shim crate (generated in the next step).
4. **Generate `audit_report.json`** – Includes `audit_hash` and any warnings.

### 2.3 `cratify scaffold`
1. **ACC skeleton creation** – Generates `src/lib.rs`, `Cargo.toml`, and a **final** `cratify.toml` (including the `certification` placeholder).
2. **Shim generation (if needed)** – For any non‑Pod types, Cratify automatically creates a thin wrapper crate (`tokio_shim`) that implements `Pod` via `#[repr(C)]` and safely copies the underlying data. The shim is added as a local path dependency.
3. **Apply branding** – Sets `tier.branding = "kernel‑unverified"` initially; will be upgraded after certification.

### 2.4 `cratify translate`
1. **Compile** – Builds the ACC in release mode targeting the isolated microkernel tier.
2. **Layout hashing** – Calls the ABI layout engine (Section 2 of `certification_spec.md`) to produce `abi_layout_hash`.
3. **Seal generation** – Using the hashes from previous stages, computes the final `seal_hash` and embeds it into the binary (`.certseal` section) and the manifest.

### 2.5 `cratify verify`
1. **Re‑hash verification** – Re‑computes `source_hash`, `audit_hash`, `binary_hash`, and `abi_layout_hash` from the built artifacts.
2. **Seal match** – Confirms that the embedded seal matches the manifest entry.
3. **Run unit‑test harness** – Executes any tests shipped with `tokio` (filtered to ACC‑compatible tests).

### 2.6 `cratify harvest`
1. **Package** – Bundles the compiled binary, final `cratify.toml`, and signature (if any) into a signed ACC artifact (`.accpkg`).
2. **Publish** – Stores the artifact in the internal Cratify registry for downstream consumption.

---

## 3. Edge‑Case Handling
| Edge Case | Detection Point | Mitigation Strategy |
|-----------|-----------------|---------------------|
| **Non‑Pod public struct** | `audit` stage | Generate a shim crate that provides a `Pod` representation; if shim cannot be generated, abort with `--force‑nonpod` flag required. |
| **Forbidden dependency** (e.g., `libc` on Windows) | `audit` stage | Fail the audit with a clear error; suggest an alternative crate or conditional compilation. |
| **Feature flag conflict** (e.g., `tokio` features that pull in `std`) | `scaffold` stage | Automatically disable conflicting features and emit a warning; allow override with `--allow‑std`. |
| **ABI breakage** (struct layout change) | `translate` / `verify` | Compare `abi_layout_hash` against the previously published hash; if a **major** change is detected, require `--force‑major` flag. |
| **Signature verification failure** | `verify` stage | Move the ACC to the quarantine tier (`tier = "unverified"`) and log a security event. |

---

## 4. Automation Hooks
- **CI Integration** – A GitHub Action can invoke the full pipeline on push to `crates/tokio`. On success, the ACC artifact is uploaded as a release asset.
- **Telemetry** – The `CapabilityBroker` logs each seal verification outcome for audit trails.

---

*This playbook is intended for developers preparing real‑world ACCs for ingesting third‑party async runtimes such as `tokio`.*
