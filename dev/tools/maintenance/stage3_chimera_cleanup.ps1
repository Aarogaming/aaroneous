# stage3_chimera_cleanup.ps1
# Archives old chimera components into dev/archive/legacy_crates/.

$archiveDir = "d:\Aaroneous\dev\archive\legacy_crates"
if (-not (Test-Path $archiveDir)) {
    New-Item -ItemType Directory -Path $archiveDir -Force | Out-Null
}

$legacyLoop = "d:\Aaroneous\components\chimera_marionette_loop"
if (Test-Path $legacyLoop) {
    Move-Item -Path $legacyLoop -Destination (Join-Path $archiveDir "components_chimera_marionette_loop") -Force
    Write-Host "[ARCHIVED] components/chimera_marionette_loop -> dev/archive/legacy_crates/" -ForegroundColor Green
}

$legacyChimeraVm = "d:\Aaroneous\core\chimera_vm"
if (Test-Path $legacyChimeraVm) {
    Move-Item -Path $legacyChimeraVm -Destination (Join-Path $archiveDir "core_chimera_vm") -Force
    Write-Host "[ARCHIVED] core/chimera_vm -> dev/archive/legacy_crates/" -ForegroundColor Green
}

Write-Host "Stage 3 Chimera archive complete." -ForegroundColor Cyan
