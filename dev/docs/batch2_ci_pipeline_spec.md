# Batch 2 CI/CD Pipeline Specification

**Objective** – Define the automated workflow that executes static analysis, Pod compliance lints, Cratify audits, and Criterion benchmarking on every pull request for Batch 2 modules.

---

### 1. GitHub Actions Workflow Overview
Create a file `.github/workflows/batch2_ci.yml` (included in the repo) with the following jobs:

| Job | Trigger | Description |
|---|---|---|
| `static_analysis` | `pull_request` on `feature/cratify-batch2-*` branches | Runs `cargo check` and `cargo clippy -- -D warnings`. |
| `pod_lint` | same | Executes custom Cratify pod‑compliance linter. |
| `cratify_audit` | same | Runs `cratify audit --strict` and `cratify certify --tier isolated`. |
| `benchmark` | same | Executes Criterion benchmarks and validates the ≤ 5 % overhead rule. |
| `merge_guard` | same | Aggregates results; fails if any previous job fails.

---

### 2. Job Definitions
#### 2.1 `static_analysis`
```yaml
static_analysis:
  runs-on: windows-latest
  steps:
    - uses: actions/checkout@v3
    - name: Set up Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
    - name: Cargo check
      run: cargo check --workspace --all-targets
    - name: Cargo clippy (zero‑warning)
      run: cargo clippy --workspace --all-targets -- -D warnings
```
#### 2.2 `pod_lint`
```yaml
pod_lint:
  runs-on: windows-latest
  needs: static_analysis
  steps:
    - uses: actions/checkout@v3
    - name: Install Cratify lint binary
      run: cargo install --path crates/cratify-lint --locked
    - name: Run Pod compliance lint
      run: |
        cargo run -p cratify-lint -- check-pod dev/docs/legacy_refactor_strategy.md src/**/*.rs
```
#### 2.3 `cratify_audit`
```yaml
cratify_audit:
  runs-on: windows-latest
  needs: pod_lint
  steps:
    - uses: actions/checkout@v3
    - name: Install Cratify CLI
      run: cargo install --path crates/cratify --locked
    - name: Inspect (optional sanity check)
      run: cratify inspect --path ${{ github.workspace }}/crates/utils_acc --mode acc
    - name: Audit strict
      run: cratify audit --strict --path ${{ github.workspace }}/crates/utils_acc
    - name: Certify isolated tier
      run: cratify certify --tier isolated --path ${{ github.workspace }}/crates/utils_acc
```
#### 2.4 `benchmark`
```yaml
benchmark:
  runs-on: windows-latest
  needs: cratify_audit
  env:
    BENCH_THRESHOLD: 5.0   # percent
  steps:
    - uses: actions/checkout@v3
    - name: Install Criterion harness
      run: cargo install cargo-criterion --locked
    - name: Run legacy baseline benchmark
      id: baseline
      run: |
        cargo bench --bench utils_legacy --quiet > baseline.txt
        echo "::set-output name=mean::$(grep 'Mean' baseline.txt | awk '{print $2}')"
    - name: Run ACC benchmark
      id: acc
      run: |
        cargo bench --bench utils_acc --quiet > acc.txt
        echo "::set-output name=mean::$(grep 'Mean' acc.txt | awk '{print $2}')"
    - name: Compute overhead
      run: |
        baseline=${{ steps.baseline.outputs.mean }}
        acc=${{ steps.acc.outputs.mean }}
        overhead=$(python -c "print(((float(baseline)-float(acc))/float(baseline))*100)")
        echo "Overhead = $overhead %"
        if (( $(python -c "print(overhead > ${{ env.BENCH_THRESHOLD }} )") )); then
          echo "::error ::Performance regression exceeds ${{ env.BENCH_THRESHOLD }}%"
          exit 1
        fi
```
#### 2.5 `merge_guard`
```yaml
merge_guard:
  runs-on: ubuntu-latest
  needs: [static_analysis, pod_lint, cratify_audit, benchmark]
  steps:
    - name: All checks passed
      run: echo "All CI checks succeeded; PR may be merged."
```
---

### 3. Branch‑Protection Rules (GitHub Settings)
1. **Require status checks to pass** – Select the five jobs above (`static_analysis`, `pod_lint`, `cratify_audit`, `benchmark`, `merge_guard`).
2. **Require pull request reviews** – At least two approving reviews, with **code‑owner** review required for core ACC crates.
3. **Restrict who can push** – Only members of the `aaroneous‑engineers` team may push directly to `main`.
4. **Require linear history** – Enforce squash merges only.
5. **Include required approvals** – Ensure the `verification_checklist_utils_acc.md` checklist is referenced in the PR description; a bot can verify its presence and fail the `merge_guard` job if missing.

---

### 4. Failure Handling & Reporting
- Any job that exits with a non‑zero status automatically marks the PR checks as **failed** and blocks merging.
- The `benchmark` job explicitly fails with an `::error` annotation if the measured overhead exceeds the 5 % threshold, providing a clear message in the GitHub UI.
- The `merge_guard` job aggregates the statuses; it will be **skipped** if any dependent job fails, ensuring the overall status is red.

---

### 5. Extensibility
- To add a new Batch 2 module, simply create a feature branch matching `feature/cratify-batch2-<module>`; the workflow triggers automatically.
- New benchmark suites can be added under `benches/` and referenced in the `benchmark` job without modifying the CI logic.

---

*Document version*: `v0.1‑batch2‑ci‑pipeline` (2026‑09‑09)
