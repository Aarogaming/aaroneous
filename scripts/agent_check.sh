#!/usr/bin/env bash
set -eo pipefail

echo "=== 1. Cargo Check (All Targets) ==="
cargo check --workspace --all-targets

echo "=== 2. Architectural AST Audit (Cratify Gate) ==="
cargo run -p cratify -- audit core/hypervisor crates/compute crates/orchestration_plane crates/llm_gateway

echo "=== 3. Monorepo Unit Test Suite ==="
cargo test --workspace --lib

echo "=== ALL ARCHITECTURAL GATES PASSED CLEANLY ==="
