# Pre-Existing Test Runtime Failures (2026-06-06)

After fixing **140 pre-existing test build errors** (commit `69bb099`), `cargo test -p a_run` runs successfully with **983 pass / 13 fail / 3 ignored**. The 13 failures are runtime assertion mismatches, **none caused by recent changes**. They fall into three categories.

## Category 1: Test-Logic Bugs (assertions inconsistent with implementation)

### 1.1 phase_5_integration_tests — throttle threshold off-by-one
**File**: `core/hypervisor/src/phase_5_integration_tests.rs:21,118,201`
**Failure**: `left: Normal, right: Metabolic` (and similar)
**Root cause**: `set_expression_rate(0.7)` calls `SystemBiology::set_expression_rate(0.7)`. The implementation checks `clamped_rate < 0.7` for Metabolic, so 0.7 falls into `Normal` (the else branch). The test asserts Metabolic.
**Fix**: Change the test to use `set_expression_rate(0.6)` (or whatever value falls strictly below the implementation's 0.7 threshold). The implementation is correct; the test value is wrong.

### 1.2 phase_5_integration_tests — bias calculation inverted
**File**: `core/hypervisor/src/phase_5_integration_tests.rs:118`
**Failure**: `assertion failed: bias_agg.risk_threshold > 0.5`
**Root cause**: For an "aggressive" specialist with `ambition=0.9, strictness=0.1`:
- `tension_factor = (0.1 - 0.9).clamp(-1, 1) = -0.8`
- `risk_threshold = (0.5 + (-0.8 * 0.4)).clamp(0.1, 0.9) = 0.18`
The test expects > 0.5, but the formula gives 0.18. The semantics are inverted: high ambition + low strictness is correctly modeled as "low risk threshold" (aggressive), but the test asserts the opposite.
**Fix**: Either swap strictness/ambition in the test setup, or change the test assertion to `< 0.5`.

## Category 2: Test Setup Incomplete (missing prerequisite steps)

### 2.1 predictive_load_balancer — single measurement
**File**: `core/hypervisor/src/predictive_load_balancer.rs:360,374`
**Failure**: `None` returned from `select_best_specialist`; `strategy == Emergency`
**Root cause**: `predict_loads()` only inserts a prediction when `history.len() >= 2`. The tests call `record_measurement` exactly once per specialist, so no predictions are populated. With no predictions, `select_best_specialist` returns `None` and `recommend_distribution` falls through to `Emergency` as the default.
**Fix**: Each test should call `record_measurement(specialist, ...)` at least 2 times to satisfy the trend-analysis precondition.

### 2.2 distributed_checkpoint — single replication
**File**: `core/hypervisor/src/distributed_checkpoint.rs:359`
**Failure**: `assertion failed: result.is_ok()`
**Root cause**: `is_checkpoint_stable` requires `nodes_replicated.len() >= 2` AND `replication_complete == true`. The test only records replication to a single node (`"node_2"`), so stability check fails and `recover_from_checkpoint` returns `Err("not stable")`.
**Fix**: Add a second `record_replication(&id, "node_3")` call before `recover_from_checkpoint`.

### 2.3 state_replicator — replica key filter
**File**: `core/hypervisor/src/state_replicator.rs:323`
**Failure**: `assertion failed: result.is_ok()`
**Root cause**: `failover` looks for replicas with `k.contains(&self.node_id)` (i.e., replicas of the current node), not replicas of the failed primary. After the test calls `replicate_state` (which updates primary_state) and `receive_replication` from `"node_2"`, the replica key is `replica_node_2`, which does not contain the current node's id `"node_1"`. So the filter is empty and failover returns `Err("No replica available")`.
**Fix**: The test's setup is logically broken: the current node ("node_1") has no replicas of its own state, so it cannot fail over to one. Either pre-populate `replica_states` with a key containing `"node_1"`, or change `failover` to look for replicas of the **failed** primary, not the current node.

## Category 3: Implementation Logic Bugs (pre-existing)

### 3.1 adaptive_learning_rate — reversed trend analysis
**File**: `core/hypervisor/src/adaptive_learning_rate.rs:284,313,340`
**Failure**: `left: Decelerate, right: Accelerate` (and similar)
**Root cause**: `analyze_trend` does `self.loss_history.iter().rev().take(self.window_size).collect()` (reversed). Then `windows(2)` iterates pairs where `w[0]` is the *older* loss. The filter `w[1] < w[0]` is intended to detect "loss decreased", but in the reversed list, `w[1]` is the *newer* loss, so `w[1] < w[0]` actually means "newer < older", i.e., loss increased. The trend is the opposite of what's reported.
**Fix**: Remove `.rev()` from the iter chain, OR reverse the comparison directions (`w[1] > w[0]` for improving, `w[1] < w[0] * 0.95` for diverging, etc.).

### 3.2 unified_learning — f32 precision loss
**File**: `core/hypervisor/src/unified_learning.rs:649,670`
**Failure**: `left: 0.800000011920929, right: 0.8`
**Root cause**: `learn_from_dopamine(..., dopamine_reward: f32, confidence: f32)` narrows incoming `0.8` (f64 literal) to f32 (0.8000000119), then casts back to f64. The test uses `assert_eq!` on f64, exposing the f32 round-trip.
**Fix**: Change the public signature to take `f64` (and the internal `update_specialist_metabolism` can cast to f32 only at the metabolism-field boundary). OR change the test to use approximate equality: `assert!((result.learning_signal - 0.8).abs() < 1e-6)`.

### 3.3 security_hardener — injection detection
**File**: `core/hypervisor/src/security_hardener.rs:397`
**Failure**: `assertion failed: !result.is_valid`
**Root cause**: The test feeds an injection pattern and expects `is_valid == false`. The validation function returns `is_valid == true`, meaning the pattern is not being detected.
**Fix**: Inspect the detection rule set; the pattern in the test may not match the regexes/whitelists/blacklists. Either add the pattern to the rejection rules or update the test fixture.

## Summary

| Category | Count | Recommended Action |
|----------|-------|--------------------|
| Test-logic bug (assertion wrong) | 4 | Fix test values |
| Test setup incomplete (missing steps) | 5 | Add prerequisite calls |
| Implementation logic bug (reversed, precision) | 4 | Fix impl logic or signatures |
| **Total** | **13** | |

All 13 failures are pre-existing in commits prior to `69bb099` (the test-build fix). They should be addressed in a follow-up sprint with one commit per category, not in a single hotfix.

## Build Status After C12 (`69bb099`)

- `cargo check --workspace --offline`: 0 errors, ~100 warnings (all pre-existing)
- `cargo test --workspace --no-run --offline`: builds 18 test executables, no errors
- `cargo test -p a_run --offline`: 983 pass, 13 fail, 3 ignored (98.7% pass rate)
