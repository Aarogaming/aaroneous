#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== 1. Workspace compilation (all targets) ==="
cargo check --workspace --all-targets
echo "=== 2. Workspace tests (including integration and documentation) ==="
cargo test --workspace
echo "=== 3. Architectural syntax audit ==="
cargo run -p ast_auditor -- audit core/ crates/ dev/emulator_harness/
echo "=== 4. Zero-stub and soundness inspection ==="
if git grep -n -E '(\btodo!\(|\bunimplemented!\(|unsafe impl.*Pod)' -- crates/ core/ dev/; then
    echo "Forbidden source patterns found" >&2
    exit 1
else
    scan_status=$?
    if [ "$scan_status" -ne 1 ]; then exit "$scan_status"; fi
fi
echo "=== 5. Golden emulator harness ==="
cargo test -p emulator_harness
echo "=== ALL REQUIRED GATES PASSED ==="
