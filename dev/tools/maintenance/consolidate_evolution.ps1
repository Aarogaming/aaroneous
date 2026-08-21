# consolidate_evolution.ps1
# Merges crates/genetics, crates/digestion, and crates/skills into crates/evolution

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Consolidating Cluster 2: crates/evolution" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

$target = "d:\Aaroneous\crates\evolution"
$archive = "d:\Aaroneous\dev\archive\pre_consolidation_crates"

if (-not (Test-Path "$target\src")) {
    New-Item -ItemType Directory -Path "$target\src" -Force | Out-Null
}

# 1. Cargo.toml for evolution
$cargoToml = @"
[package]
name = "evolution"
version = "0.1.0"
edition = "2021"

[dependencies]
nervous_system = { path = "../nervous_system" }
aaroneous_paths = { path = "../paths" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
rand = "0.8"
anyhow = "1.0"
thiserror = "1.0"
"@
Set-Content -Path "$target\Cargo.toml" -Value $cargoToml

# 2. Copy source submodules
Copy-Item -Path "d:\Aaroneous\crates\genetics\src\genetics.rs" -Destination "$target\src\genetics.rs" -Force
Copy-Item -Path "d:\Aaroneous\crates\digestion\src\self_digestion.rs" -Destination "$target\src\self_digestion.rs" -Force
Copy-Item -Path "d:\Aaroneous\crates\digestion\src\workspace.rs" -Destination "$target\src\workspace.rs" -Force
Copy-Item -Path "d:\Aaroneous\crates\skills\src\skill_system.rs" -Destination "$target\src\skills.rs" -Force

Write-Host "[MIGRATED] Source files copied to crates/evolution" -ForegroundColor Green
