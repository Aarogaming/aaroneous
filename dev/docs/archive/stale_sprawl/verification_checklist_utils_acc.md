# Pilot Verification Checklist – utils_acc ACC

**Objective** – Provide a concrete, step‑by‑step verification process for the first migrated module (`utils_acc`) before it is merged back to `main`.

---

### 1. Pre‑flight Static Analysis
1. **Cargo check**
   ```bash
   cargo check --workspace --all-targets
   ```
   - Must exit with code 0.
2. **Clippy with zero‑warning policy**
   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```
   - No warnings or lints are permitted. Required lint set includes `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::todo`, and `clippy::unimplemented`.
3. **Pod compliance lint** (custom Cratify lint)
   ```bash
   cargo run -p cratify-lint -- check-pod dev/docs/legacy_refactor_strategy.md src/utils_acc
   ```
   - All public structs/enums must implement `bytemuck::Pod` and `Zeroable`. The command should report `0` failures.
4. **Forbidden API scan**
   ```bash
   cargo run -p cratify-lint -- forbidden-api src/utils_acc
   ```
   - Must not detect `std::thread::spawn`, `async`, `await`, `lazy_static`, `once_cell`, or any `unsafe` blocks.

---

### 2. Cratify Audit & Brand‑Seal Flow
```bash
# 2.1 Inspect (optional sanity check)
cratify inspect --path d:\Aaroneous\crates\utils_acc --mode acc

# 2.2 Audit – strict mode verifies POD & no unwrap/panic
cratify audit --strict --path d:\Aaroneous\crates\utils_acc

# 2.3 Certify – isolate tier produces brand seal
cratify certify --tier isolated --path d:\Aaroneous\crates\utils_acc
```
- **Success criteria**: Each command exits with code 0 and prints a summary line ending with `✔︎ OK`.
- The generated `brand_seal.toml` must be committed alongside the crate.

---

### 3. Performance Benchmarking (Criterion)
1. Install the benchmark harness (already part of the crate’s `benches/` directory).
2. Run the baseline (legacy `utils` module) benchmark:
   ```bash
   cargo bench --bench utils_legacy
   ```
   Record the mean throughput `T_legacy` (operations / second).
3. Run the ACC benchmark:
   ```bash
   cargo bench --bench utils_acc
   ```
   Record the mean throughput `T_acc`.
4. Compute overhead:
   ```text
   overhead = (T_legacy - T_acc) / T_legacy * 100%
   ```
5. **Pass threshold** – `overhead ≤ 5%` (i.e., `T_acc ≥ 0.95 * T_legacy`).
6. Store the numbers in `dev/docs/benchmark_results_utils_acc.md` and attach the file to the PR.

---

### 4. Sign‑off & Merge Criteria
| Item | Required | Evidence |
|---|---|---|
| Static analysis ✓ | All checks in **Section 1** pass. | CI job logs showing `cargo check`, `clippy`, and custom lint exit status 0. |
| Cratify audit & certify ✓ | Commands in **Section 2** succeed. | `brand_seal.toml` present; CI logs show `✔︎ OK` messages. |
| Performance threshold ✓ | Benchmark in **Section 3** meets ≤ 5 % overhead. | `benchmark_results_utils_acc.md` attached to PR. |
| Code review approval ✓ | At least two reviewers approve. | PR approval stamps. |
| Documentation update ✓ | `dev/docs/legacy_refactor_strategy.md` references `utils_acc`; checklist file exists. | Repo tree verification. |

Only when **all** rows are satisfied may the `feature/cratify-legacy-refactor` branch be merged into `main`.

---

*Document version*: `v0.1‑pilot‑utils_acc‑checklist` (2026‑09‑09)
