# clean_workspace.ps1
# Cleans stale build artifacts, unlinked cargo target dirs, and temporary caches in Aaroneous.

param(
    [switch]$FullClean
)

$workspace = "d:\Aaroneous"
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Aaroneous Workspace Maintenance Cleaner" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

# 1. Clean cargo state files
$cargoStateFiles = Get-ChildItem -Path $workspace -Filter "cargo_state.json" -Recurse -ErrorAction SilentlyContinue
foreach ($cs in $cargoStateFiles) {
    Remove-Item $cs.FullName -Force -ErrorAction SilentlyContinue
    Write-Host "  [REMOVED] $($cs.FullName)" -ForegroundColor Green
}

# 2. Clean temporary logs
$tempLogs = Get-ChildItem -Path (Join-Path $workspace "logs") -Filter "*.err" -ErrorAction SilentlyContinue
foreach ($log in $tempLogs) {
    Remove-Item $log.FullName -Force -ErrorAction SilentlyContinue
    Write-Host "  [REMOVED] $($log.FullName)" -ForegroundColor Green
}

# 3. If FullClean, run cargo clean
if ($FullClean) {
    Write-Host "Running cargo clean across workspace..." -ForegroundColor Yellow
    Push-Location $workspace
    cargo clean
    Pop-Location
}

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "Workspace maintenance complete." -ForegroundColor Green
