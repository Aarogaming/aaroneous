# consolidate_orchestrator.ps1
# Merges crates/agents, crates/hive, crates/control, and crates/intelligence into crates/orchestrator

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Consolidating Cluster 1: crates/orchestrator" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

$target = "d:\Aaroneous\crates\orchestrator"
$archive = "d:\Aaroneous\dev\archive\pre_consolidation_crates"

if (-not (Test-Path $archive)) {
    New-Item -ItemType Directory -Path $archive -Force | Out-Null
}
if (-not (Test-Path "$target\src")) {
    New-Item -ItemType Directory -Path "$target\src" -Force | Out-Null
}

# 1. Cargo.toml for orchestrator
$cargoToml = @"
[package]
name = "orchestrator"
version = "0.1.0"
edition = "2021"

[dependencies]
nervous_system = { path = "../nervous_system" }
biology = { path = "../biology" }
compute = { path = "../compute" }
aaroneous_paths = { path = "../paths" }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rand = "0.8"
anyhow = "1.0"
thiserror = "1.0"
"@
Set-Content -Path "$target\Cargo.toml" -Value $cargoToml

# 2. Copy source submodules
Copy-Item -Path "d:\Aaroneous\crates\agents\src\*" -Destination "$target\src" -Recurse -Force
Copy-Item -Path "d:\Aaroneous\crates\hive\src\hive_runtime.rs" -Destination "$target\src\hive_runtime.rs" -Force
Copy-Item -Path "d:\Aaroneous\crates\control\src\control.rs" -Destination "$target\src\control.rs" -Force
Copy-Item -Path "d:\Aaroneous\crates\intelligence\src\llm.rs" -Destination "$target\src\llm.rs" -Force
Copy-Item -Path "d:\Aaroneous\crates\intelligence\src\mdps_router.rs" -Destination "$target\src\mdps_router.rs" -Force
Copy-Item -Path "d:\Aaroneous\crates\intelligence\src\linguistic_transducer.rs" -Destination "$target\src\linguistic_transducer.rs" -Force
Copy-Item -Path "d:\Aaroneous\crates\intelligence\src\aura_ui_manifest.rs" -Destination "$target\src\aura_ui_manifest.rs" -Force

Write-Host "[MIGRATED] Source files copied to crates/orchestrator" -ForegroundColor Green
