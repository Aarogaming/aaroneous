$ErrorActionPreference = "Stop"

function Write-ColoredHeader {
    param (
        [string]$header
    )
    Write-Host "`n[$header]" -ForegroundColor Cyan
}

Write-ColoredHeader "Gate 1: Workspace Cargo Check"
cargo check --workspace --all-targets
if ($LASTEXITCODE -ne 0) {
    Write-Host "Workspace Cargo Check failed." -ForegroundColor Red
    exit 1
}

Write-ColoredHeader "Gate 2: Architectural AST Audit"
cargo run -p ast_auditor -- audit core/hypervisor crates/compute crates/orchestration_plane crates/llm_gateway
if ($LASTEXITCODE -ne 0) {
    Write-Host "Architectural AST Audit failed." -ForegroundColor Red
    exit 1
}

Write-ColoredHeader "Gate 3: Monorepo Unit Test Suite"
cargo test --workspace --lib
if ($LASTEXITCODE -ne 0) {
    Write-Host "Monorepo Unit Test Suite failed." -ForegroundColor Red
    exit 1
}

Write-ColoredHeader "Gate 4: Golden Dogfooding Harness Verification"
cargo test -p emulator_harness
if ($LASTEXITCODE -ne 0) {
    Write-Host "Golden Dogfooding Harness Verification failed." -ForegroundColor Red
    exit 1
}

Write-Host "`n[SUCCESS] ALL ARCHITECTURAL GATES PASSED CLEANLY." -ForegroundColor Green
exit 0

