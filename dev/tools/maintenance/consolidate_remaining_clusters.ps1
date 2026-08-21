# consolidate_remaining_clusters.ps1
# Consolidates Clusters 2, 3, and 4

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Consolidating Clusters 2, 3, 4" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

$archive = "d:\Aaroneous\dev\archive\pre_consolidation_crates"

# 1. Cluster 3: Copy scientific_analyzer into chimera
$chimeraAnalysis = "d:\Aaroneous\crates\chimera\src\analysis"
if (-not (Test-Path $chimeraAnalysis)) {
    New-Item -ItemType Directory -Path $chimeraAnalysis -Force | Out-Null
}
Copy-Item -Path "d:\Aaroneous\crates\scientific_analyzer\src\*" -Destination $chimeraAnalysis -Recurse -Force
Write-Host "[MIGRATED] scientific_analyzer -> crates/chimera/src/analysis" -ForegroundColor Green

# 2. Cluster 4: Copy sabs into omni
$omniMatrix = "d:\Aaroneous\crates\omni\src\matrix"
if (-not (Test-Path $omniMatrix)) {
    New-Item -ItemType Directory -Path $omniMatrix -Force | Out-Null
}
Copy-Item -Path "d:\Aaroneous\crates\sabs\src\*" -Destination $omniMatrix -Recurse -Force
Write-Host "[MIGRATED] sabs -> crates/omni/src/matrix" -ForegroundColor Green

# 3. Archive source crates for clusters 2, 3, 4
Move-Item -Path "d:\Aaroneous\crates\genetics" -Destination "$archive\genetics" -Force
Move-Item -Path "d:\Aaroneous\crates\digestion" -Destination "$archive\digestion" -Force
Move-Item -Path "d:\Aaroneous\crates\skills" -Destination "$archive\skills" -Force
Move-Item -Path "d:\Aaroneous\crates\scientific_analyzer" -Destination "$archive\scientific_analyzer" -Force
Move-Item -Path "d:\Aaroneous\crates\sabs" -Destination "$archive\sabs" -Force
Write-Host "[ARCHIVED] Source crates for evolution, chimera, omni archived" -ForegroundColor Green

Write-Host "Consolidation File Operations Complete!" -ForegroundColor Cyan
