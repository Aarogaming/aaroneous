# stage6_archive_old_components.ps1
# Archives old components into dev/archive/legacy_components/.

$archiveDir = "d:\Aaroneous\dev\archive\legacy_components"
if (-not (Test-Path $archiveDir)) {
    New-Item -ItemType Directory -Path $archiveDir -Force | Out-Null
}

$componentsToArchive = @("paths", "biology", "compute")
foreach ($comp in $componentsToArchive) {
    $src = "d:\Aaroneous\components\$comp"
    if (Test-Path $src) {
        Move-Item -Path $src -Destination (Join-Path $archiveDir "components_$comp") -Force
        Write-Host "[ARCHIVED COMPONENT] components/$comp -> dev/archive/legacy_components/components_$comp" -ForegroundColor Green
    }
}

Write-Host "Component archive complete." -ForegroundColor Cyan
