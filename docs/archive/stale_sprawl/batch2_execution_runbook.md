# Batch 2 Execution Runbook

**Objective** – Provide a clear, step‑by‑step developer guide for executing a Batch 2 legacy module refactor from start to finish.

---

### 1. Inventory & Scoring
1. Open a PowerShell terminal at the repository root (`d:\Aaroneous`).
2. Run the scoring script you created earlier (see `dev/docs/batch2_scoring_framework.md`). Example command:
   ```
   python scripts/score_batch2.py --output scores.csv
   ```
   - The script enumerates all candidate `.rs` files, assigns a tier, computes `Score = TierWeight × (LOC/1000)`, and writes `scores.csv` sorted descending.
3. Inspect `scores.csv` and pick the highest‑scoring **Low** or **Medium** module that fits within the sprint budget.
4. Record the chosen module name (e.g., `config_loader`) for the next steps.

---

### 2. Create Isolated Feature Branch
```bash
# From repository root
git checkout main
git pull origin main
git checkout -b feature/cratify-batch2-<module_name>
```
Replace `<module_name>` with the selected module (e.g., `config_loader`).

---

### 3. Code Transformations
| Transformation | What to Do | Tools / Tips |
|---|---|---|
| Replace `Vec<T>` | Convert to fixed‑size arrays `[T; N]` or `RingBuffer<T, const CAP: usize>` where `CAP` matches the maximum expected size. | Use `arrayvec` pattern or the `ringbuf` crate already vendored in `cratify`. |
| Strip `unwrap`/`expect`/`panic!` | Change calls to return `Result<_, CratifyError>` and propagate with `?`. Add explicit error mapping where needed. | Run `cargo clippy` with `clippy::unwrap_used` to locate remaining instances. |
| Enforce POD Types | Add `#[repr(C)]`, derive `Copy`, `Clone`, `bytemuck::Pod`, `bytemuck::Zeroable` to all public structs/enums. | Use the helper macro `#[derive(Pod, Zeroable)]` from `bytemuck`. |
| Remove Global Mutable State | Eliminate `lazy_static!`, `once_cell::sync::Lazy`, static `Mutex`/`RwLock`. Replace with explicit message‑passing via lock‑free ring buffers. | Search for `lazy_static`/`once_cell` with grep. |
| Disallow `unsafe` | Move any required unsafe code into a separate core‑tier crate and expose a safe façade. | Create `crates/<module>_core` if needed, then add a thin wrapper in the ACC crate. |
| Add `#[cfg_attr(test, derive(Debug))]` (optional) | Improves test diagnostics without affecting release binary. | Simple attribute addition. |

**Tip** – Perform each transformation in a separate commit so the history remains reviewable.

---

### 4. Local Validation
```bash
# Ensure you are on the feature branch
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
# Run Cratify lint for POD compliance
cargo run -p cratify-lint -- check-pod dev/docs/legacy_refactor_strategy.md src/<module_path>
# Run Cratify audit & certify (replace path as appropriate)
cratify audit --strict --path crates/<module_name>_acc
cratify certify --tier isolated --path crates/<module_name>_acc
# Benchmark against legacy baseline
cargo bench --bench <module_name>_legacy
cargo bench --bench <module_name>_acc
# Verify overhead ≤ 5 %
# (You can reuse the benchmark script from batch2_scoring_framework.md)
```
All commands must exit with status 0. If any step fails, fix the code before proceeding.

---

### 5. Open Pull Request & CI Verification
1. Push the branch:
   ```bash
   git add .
   git commit -m "feat: batch2 refactor – <module_name> ACC migration"
   git push origin feature/cratify-batch2-<module_name>
   ```
2. Open a PR against `main` on GitHub.
3. In the PR description, reference the runbook and include a link to the `verification_checklist_utils_acc.md` (or a module‑specific checklist) to satisfy the merge‑guard rule.
4. GitHub Actions will automatically trigger the **Batch 2 CI pipeline** (see `dev/docs/batch2_ci_pipeline_spec.md`).
5. Verify that all jobs (`static_analysis`, `pod_lint`, `cratify_audit`, `benchmark`, `merge_guard`) pass.
6. Request at least two reviewers (including a code‑owner for ACC crates). Once approvals are obtained and CI is green, merge using **squash‑merge** to keep a linear history.

---

### 6. Post‑Merge Cleanup
- Delete the feature branch locally and remotely:
  ```bash
  git checkout main
  git branch -d feature/cratify-batch2-<module_name>
  git push origin --delete feature/cratify-batch2-<module_name>
  ```
- Update the inventory scoring CSV to mark the module as **completed**.
- Add the newly created ACC crate to the next sprint’s documentation overview (`dev/docs/round3_execution_playbook.md`).

---

*Document version*: `v0.1‑batch2‑execution‑runbook` (2026‑09‑09)
