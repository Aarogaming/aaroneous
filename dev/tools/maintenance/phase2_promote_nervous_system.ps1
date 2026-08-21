# phase2_promote_nervous_system.ps1
# Promotes core/nervous_system to crates/nervous_system

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Aaroneous Deconstruction: Phase 2 Nervous System" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

$srcNervous = "d:\Aaroneous\core\nervous_system"
$dstNervous = "d:\Aaroneous\crates\nervous_system"
$archiveCore = "d:\Aaroneous\dev\archive\legacy_core"

if (-not (Test-Path $archiveCore)) {
    New-Item -ItemType Directory -Path $archiveCore -Force | Out-Null
}

if ((Test-Path $srcNervous) -and (-not (Test-Path $dstNervous))) {
    Copy-Item -Path $srcNervous -Destination $dstNervous -Recurse -Force
    Write-Host "[PROMOTED] core/nervous_system -> crates/nervous_system" -ForegroundColor Green
    
    Move-Item -Path $srcNervous -Destination (Join-Path $archiveCore "nervous_system") -Force
    Write-Host "[ARCHIVED] core/nervous_system -> dev/archive/legacy_core/" -ForegroundColor Yellow
}

Write-Host "Phase 2 Promotion Complete!" -ForegroundColor Cyan
