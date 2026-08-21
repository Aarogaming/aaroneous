# stage4_omni_cleanup.ps1
# Archives old constellation components into dev/archive/legacy_crates/.

$archiveDir = "d:\Aaroneous\dev\archive\legacy_crates"
if (-not (Test-Path $archiveDir)) {
    New-Item -ItemType Directory -Path $archiveDir -Force | Out-Null
}

$legacyConstellation = "d:\Aaroneous\components\constellation"
if (Test-Path $legacyConstellation) {
    Move-Item -Path $legacyConstellation -Destination (Join-Path $archiveDir "components_constellation") -Force
    Write-Host "[ARCHIVED] components/constellation -> dev/archive/legacy_crates/" -ForegroundColor Green
}

Write-Host "Stage 4 Omni archive complete." -ForegroundColor Cyan
