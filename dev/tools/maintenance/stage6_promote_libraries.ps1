# stage6_promote_libraries.ps1
# Promotes components/compute and components/biology to crates/, and archives root .sab directories.

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Aaroneous Deconstruction: Stage 6 Library Promotion" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

# 1. Promote compute to crates/compute
$srcCompute = "d:\Aaroneous\components\compute"
$dstCompute = "d:\Aaroneous\crates\compute"
if ((Test-Path $srcCompute) -and (-not (Test-Path $dstCompute))) {
    Copy-Item -Path $srcCompute -Destination $dstCompute -Recurse -Force
    Write-Host "[PROMOTED] components/compute -> crates/compute" -ForegroundColor Green
}

# 2. Promote biology to crates/biology
$srcBiology = "d:\Aaroneous\components\biology"
$dstBiology = "d:\Aaroneous\crates\biology"
if ((Test-Path $srcBiology) -and (-not (Test-Path $dstBiology))) {
    Copy-Item -Path $srcBiology -Destination $dstBiology -Recurse -Force
    Write-Host "[PROMOTED] components/biology -> crates/biology" -ForegroundColor Green
}

# 3. Archive root .sab folders
$archiveSabs = "d:\Aaroneous\dev\archive\legacy_sabs"
if (-not (Test-Path $archiveSabs)) {
    New-Item -ItemType Directory -Path $archiveSabs -Force | Out-Null
}

$sabFolders = @(
    "genesis_architect.sab",
    "resource_governor.sab",
    "security_sentinel.sab",
    "telemetry_aggregator.sab"
)

foreach ($sab in $sabFolders) {
    $p = "d:\Aaroneous\$sab"
    if (Test-Path $p) {
        Move-Item -Path $p -Destination $archiveSabs -Force
        Write-Host "[ARCHIVED ROOT SAB] $sab -> dev/archive/legacy_sabs/" -ForegroundColor Yellow
    }
}

# 4. Archive components/foundry
$archiveCrates = "d:\Aaroneous\dev\archive\legacy_crates"
$srcFoundry = "d:\Aaroneous\components\foundry"
if (Test-Path $srcFoundry) {
    Move-Item -Path $srcFoundry -Destination (Join-Path $archiveCrates "components_foundry") -Force
    Write-Host "[ARCHIVED] components/foundry -> dev/archive/legacy_crates/" -ForegroundColor Yellow
}

# 5. Archive templates/universal_sab
$archiveTemplates = "d:\Aaroneous\dev\archive\templates"
if (-not (Test-Path $archiveTemplates)) {
    New-Item -ItemType Directory -Path $archiveTemplates -Force | Out-Null
}
$srcSabTemplate = "d:\Aaroneous\templates\universal_sab"
if (Test-Path $srcSabTemplate) {
    Move-Item -Path $srcSabTemplate -Destination $archiveTemplates -Force
    Write-Host "[ARCHIVED] templates/universal_sab -> dev/archive/templates/" -ForegroundColor Yellow
}

Write-Host "Stage 6 library promotion complete." -ForegroundColor Cyan
