# stage2_marionette_cleanup.ps1
# Archives old fragmented marionette folders into dev/archive/legacy_crates/.

$archiveDir = "d:\Aaroneous\dev\archive\legacy_crates"
if (-not (Test-Path $archiveDir)) {
    New-Item -ItemType Directory -Path $archiveDir -Force | Out-Null
}

$legacyAgentsMarionette = "d:\Aaroneous\agents\marionette_host"
if (Test-Path $legacyAgentsMarionette) {
    Move-Item -Path $legacyAgentsMarionette -Destination (Join-Path $archiveDir "agents_marionette_host") -Force
    Write-Host "[ARCHIVED] agents/marionette_host -> dev/archive/legacy_crates/agents_marionette_host" -ForegroundColor Green
}

$legacyComponentsMarionette = "d:\Aaroneous\components\marionette_host"
if (Test-Path $legacyComponentsMarionette) {
    Move-Item -Path $legacyComponentsMarionette -Destination (Join-Path $archiveDir "components_marionette_host") -Force
    Write-Host "[ARCHIVED] components/marionette_host -> dev/archive/legacy_crates/components_marionette_host" -ForegroundColor Green
}

Write-Host "Stage 2 Marionette archive complete." -ForegroundColor Cyan
