# Repository Audit Script
# Run to check repository health and identify bloat

Write-Host "=== Repository Audit Script ===" -ForegroundColor Cyan
Write-Host "Date: $(Get-Date -Format 'yyyy-MM-dd')" -ForegroundColor Yellow
Write-Host ""

# Repository size
Write-Host "=== Repository Size ===" -ForegroundColor Cyan
$repoSize = (Get-Item -Path ".\." -Force).Length / 1GB
Write-Host "Repository size: $($repoSize.ToString('F2')) GB" -ForegroundColor Yellow

# Target directory size
$targetSize = (Get-ChildItem -Path "target" -Recurse -File).Length / 1GB
Write-Host "Target directory size: $($targetSize.ToString('F2')) GB" -ForegroundColor Yellow

# Object files
Write-Host ""
Write-Host "=== Object Files (`.o`) ===" -ForegroundColor Cyan
$objFiles = Get-ChildItem -Path "target/debug/deps" -Filter "*.o" -ErrorAction SilentlyContinue
if ($objFiles) {
    Write-Host "Found $($objFiles.Count) object files" -ForegroundColor Yellow
    $objSize = $objFiles.Length / 1GB
    Write-Host "Total size: $($objSize.ToString('F2')) GB" -ForegroundColor Yellow
} else {
    Write-Host "No object files found" -ForegroundColor Gray
}

# Query cache files
Write-Host ""
Write-Host "=== Query Cache Files (query-cache.bin) ===" -ForegroundColor Cyan
$cacheFiles = Get-ChildItem -Path "target/debug" -Filter "query-cache.bin" -ErrorAction SilentlyContinue
if ($cacheFiles) {
    Write-Host "Found $($cacheFiles.Count) query-cache.bin files" -ForegroundColor Yellow
    $cacheSize = $cacheFiles.Length / 1GB
    Write-Host "Total size: $($cacheSize.ToString('F2')) GB" -ForegroundColor Yellow
} else {
    Write-Host "No query-cache.bin files found" -ForegroundColor Gray
}

# Dep-graph files
Write-Host ""
Write-Host "=== Dep-Graph Files ===" -ForegroundColor Cyan
$depGraphFiles = Get-ChildItem -Path "target/debug" -Filter "dep-graph.*" -ErrorAction SilentlyContinue
if ($depGraphFiles) {
    Write-Host "Found $($depGraphFiles.Count) dep-graph files" -ForegroundColor Yellow
    $depGraphSize = $depGraphFiles.Length / 1GB
    Write-Host "Total size: $($depGraphSize.ToString('F2')) GB" -ForegroundColor Yellow
} else {
    Write-Host "No dep-graph files found" -ForegroundColor Gray
}

# .rlib files
Write-Host ""
Write-Host "=== .rlib Files ===" -ForegroundColor Cyan
$rlibFiles = Get-ChildItem -Path "target/debug/deps" -Filter "*.rlib" -ErrorAction SilentlyContinue
if ($rlibFiles) {
    Write-Host "Found $($rlibFiles.Count) .rlib files" -ForegroundColor Yellow
    $rlibSize = $rlibFiles.Length / 1GB
    Write-Host "Total size: $($rlibSize.ToString('F2')) GB" -ForegroundColor Yellow
} else {
    Write-Host "No .rlib files found" -ForegroundColor Gray
}

# .pdb files
Write-Host ""
Write-Host "=== .pdb Files ===" -ForegroundColor Cyan
$pdbFiles = Get-ChildItem -Path "target/debug/deps" -Filter "*.pdb" -ErrorAction SilentlyContinue
if ($pdbFiles) {
    Write-Host "Found $($pdbFiles.Count) .pdb files" -ForegroundColor Yellow
    $pdbSize = $pdbFiles.Length / 1GB
    Write-Host "Total size: $($pdbSize.ToString('F2')) GB" -ForegroundColor Yellow
} else {
    Write-Host "No .pdb files found" -ForegroundColor Gray
}

# Documentation count
Write-Host ""
Write-Host "=== Documentation Count ===" -ForegroundColor Cyan
$docCount = (Get-ChildItem -Path "docs" -Recurse -Filter "*.md" -ErrorAction SilentlyContinue).Count
Write-Host "Documentation files: $($docCount)" -ForegroundColor Yellow

# .gitignore check
Write-Host ""
Write-Host "=== .gitignore Coverage ===" -ForegroundColor Cyan
$gitignorePath = Join-Path $PSScriptRoot ".gitignore"
if (Test-Path $gitignorePath) {
    Write-Host ".gitignore exists" -ForegroundColor Green
    $gitignoreLines = Get-Content $gitignorePath
    $ignoreCount = $gitignoreLines.Count
    Write-Host "Lines in .gitignore: $($ignoreCount)" -ForegroundColor Gray
} else {
    Write-Host ".gitignore not found" -ForegroundColor Red
}

# Model files
Write-Host ""
Write-Host "=== Model Files ===" -ForegroundColor Cyan
$modelFiles = Get-ChildItem -Path "D:\Aaroneous" -Filter "*.gguf" -Recurse -ErrorAction SilentlyContinue
if ($modelFiles) {
    Write-Host "Found $($modelFiles.Count) model files" -ForegroundColor Yellow
    foreach ($model in $modelFiles) {
        $modelSize = $model.Length / 1GB
        Write-Host "  - $($model.Name): $($modelSize.ToString('F2')) GB" -ForegroundColor Gray
    }
} else {
    Write-Host "No model files found" -ForegroundColor Gray
}

# Build artifacts summary
Write-Host ""
Write-Host "=== Build Artifacts Summary ===" -ForegroundColor Cyan
$buildArtifacts = @()
$buildArtifacts += @{Name="Object files (*.o)"; Count=$objFiles.Count; Size=$objFiles.Length / 1GB}
$buildArtifacts += @{Name="Query cache (query-cache.bin)"; Count=$cacheFiles.Count; Size=$cacheFiles.Length / 1GB}
$buildArtifacts += @{Name="Dep-graph (dep-graph.*)"; Count=$depGraphFiles.Count; Size=$depGraphFiles.Length / 1GB}
$buildArtifacts += @{Name=".rlib files"; Count=$rlibFiles.Count; Size=$rlibFiles.Length / 1GB}
$buildArtifacts += @{Name=".pdb files"; Count=$pdbFiles.Count; Size=$pdbFiles.Length / 1GB}

$buildArtifacts | ForEach-Object {
    Write-Host "  - $($_.Name): $($_.Count) files, $($_.Size.ToString('F2')) GB" -ForegroundColor Gray
}

Write-Host ""
Write-Host "=== Audit Complete ===" -ForegroundColor Green
Write-Host "Repository size: $($repoSize.ToString('F2')) GB" -ForegroundColor Yellow
Write-Host "Target size: $($targetSize.ToString('F2')) GB" -ForegroundColor Yellow
Write-Host "Documentation: $($docCount) files" -ForegroundColor Yellow