# Weekly Cleanup Script
# Run every Sunday to remove build artifacts

Write-Host "=== Weekly Cleanup Script ===" -ForegroundColor Cyan
Write-Host "Date: $(Get-Date -Format 'yyyy-MM-dd')" -ForegroundColor Yellow
Write-Host ""

# Remove old .rlib files
Write-Host "Removing old .rlib files..." -ForegroundColor Green
$rlibFiles = Get-ChildItem -Path "target/debug/deps" -Filter "*.rlib" -ErrorAction SilentlyContinue
if ($rlibFiles) {
    Remove-Item -Path $rlibFiles.FullName -Force
    Write-Host "  Removed $($rlibFiles.Count) .rlib files" -ForegroundColor Gray
} else {
    Write-Host "  No .rlib files found" -ForegroundColor Gray
}

# Remove old .pdb files
Write-Host "Removing old .pdb files..." -ForegroundColor Green
$pdbFiles = Get-ChildItem -Path "target/debug/deps" -Filter "*.pdb" -ErrorAction SilentlyContinue
if ($pdbFiles) {
    Remove-Item -Path $pdbFiles.FullName -Force
    Write-Host "  Removed $($pdbFiles.Count) .pdb files" -ForegroundColor Gray
} else {
    Write-Host "  No .pdb files found" -ForegroundColor Gray
}

# Remove old query-cache.bin files
Write-Host "Removing old query-cache.bin files..." -ForegroundColor Green
$cacheFiles = Get-ChildItem -Path "target/debug" -Filter "query-cache.bin" -ErrorAction SilentlyContinue
if ($cacheFiles) {
    Remove-Item -Path $cacheFiles.FullName -Force
    Write-Host "  Removed $($cacheFiles.Count) query-cache.bin files" -ForegroundColor Gray
} else {
    Write-Host "  No query-cache.bin files found" -ForegroundColor Gray
}

# Remove old dep-graph files
Write-Host "Removing old dep-graph files..." -ForegroundColor Green
$depGraphFiles = Get-ChildItem -Path "target/debug" -Filter "dep-graph.*" -ErrorAction SilentlyContinue
if ($depGraphFiles) {
    Remove-Item -Path $depGraphFiles.FullName -Force
    Write-Host "  Removed $($depGraphFiles.Count) dep-graph files" -ForegroundColor Gray
} else {
    Write-Host "  No dep-graph files found" -ForegroundColor Gray
}

# Remove old object files
Write-Host "Removing old object files..." -ForegroundColor Green
$objFiles = Get-ChildItem -Path "target/debug/deps" -Filter "*.o" -ErrorAction SilentlyContinue
if ($objFiles) {
    Remove-Item -Path $objFiles.FullName -Force
    Write-Host "  Removed $($objFiles.Count) object files" -ForegroundColor Gray
} else {
    Write-Host "  No object files found" -ForegroundColor Gray
}

# Report repository size
Write-Host ""
Write-Host "=== Repository Size Report ===" -ForegroundColor Cyan
$repoSize = (Get-Item -Path ".\." -Force).Length / 1GB
Write-Host "Repository size: $($repoSize.ToString('F2')) GB" -ForegroundColor Yellow

$targetSize = (Get-ChildItem -Path "target" -Recurse -File).Length / 1GB
Write-Host "Target directory size: $($targetSize.ToString('F2')) GB" -ForegroundColor Yellow

Write-Host "=== Weekly Cleanup Complete ===" -ForegroundColor Green