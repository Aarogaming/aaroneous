# Repository Size Check Script
# Run to monitor repository size and alert on thresholds

Write-Host "=== Repository Size Check Script ===" -ForegroundColor Cyan
Write-Host "Date: $(Get-Date -Format 'yyyy-MM-dd')" -ForegroundColor Yellow
Write-Host ""

# Size thresholds
$warningThreshold = 100
$criticalThreshold = 150
$targetThreshold = 50

# Repository size
$repoSize = (Get-Item -Path ".\." -Force).Length / 1GB
Write-Host "Repository size: $($repoSize.ToString('F2')) GB" -ForegroundColor Yellow

# Check thresholds
if ($repoSize -ge $criticalThreshold) {
    Write-Host "⚠️  CRITICAL: Repository size ($($repoSize.ToString('F2')) GB) exceeds critical threshold ($($criticalThreshold) GB)!" -ForegroundColor Red
} elseif ($repoSize -ge $warningThreshold) {
    Write-Host "⚠️  WARNING: Repository size ($($repoSize.ToString('F2')) GB) exceeds warning threshold ($($warningThreshold) GB)!" -ForegroundColor Yellow
} else {
    Write-Host "✅ Repository size ($($repoSize.ToString('F2')) GB) is within acceptable range (< $($warningThreshold) GB)" -ForegroundColor Green
}

# Target threshold
if ($repoSize -le $targetThreshold) {
    Write-Host "✅ Repository size ($($repoSize.ToString('F2')) GB) is within target range (< $($targetThreshold) GB)" -ForegroundColor Green
} else {
    Write-Host "🟡 Repository size ($($repoSize.ToString('F2')) GB) exceeds target range ($($targetThreshold) GB)" -ForegroundColor Yellow
}

# Target directory size
$targetSize = (Get-ChildItem -Path "target" -Recurse -File).Length / 1GB
Write-Host "Target directory size: $($targetSize.ToString('F2')) GB" -ForegroundColor Yellow

# Build artifacts size
$buildArtifacts = @()
$buildArtifacts += @{Name="Object files (*.o)"; Size=(Get-ChildItem -Path "target/debug/deps" -Filter "*.o" -ErrorAction SilentlyContinue).Length / 1GB}
$buildArtifacts += @{Name="Query cache (query-cache.bin)"; Size=(Get-ChildItem -Path "target/debug" -Filter "query-cache.bin" -ErrorAction SilentlyContinue).Length / 1GB}
$buildArtifacts += @{Name="Dep-graph (dep-graph.*)"; Size=(Get-ChildItem -Path "target/debug" -Filter "dep-graph.*" -ErrorAction SilentlyContinue).Length / 1GB}
$buildArtifacts += @{Name=".rlib files"; Size=(Get-ChildItem -Path "target/debug/deps" -Filter "*.rlib" -ErrorAction SilentlyContinue).Length / 1GB}
$buildArtifacts += @{Name=".pdb files"; Size=(Get-ChildItem -Path "target/debug/deps" -Filter "*.pdb" -ErrorAction SilentlyContinue).Length / 1GB}

$buildArtifactsSize = 0
$buildArtifacts | ForEach-Object {
    $buildArtifactsSize += $_.Size
    Write-Host "  - $($_.Name): $($_.Size.ToString('F2')) GB" -ForegroundColor Gray
}

Write-Host "Total build artifacts: $($buildArtifactsSize.ToString('F2')) GB" -ForegroundColor Gray

# Documentation count
$docCount = (Get-ChildItem -Path "docs" -Recurse -Filter "*.md" -ErrorAction SilentlyContinue).Count
Write-Host "Documentation files: $($docCount)" -ForegroundColor Gray

# .gitignore check
$gitignorePath = Join-Path $PSScriptRoot ".gitignore"
if (Test-Path $gitignorePath) {
    $gitignoreLines = Get-Content $gitignorePath
    Write-Host ".gitignore: $($gitignoreLines.Count) lines" -ForegroundColor Gray
} else {
    Write-Host ".gitignore: Not found" -ForegroundColor Red
}

# Model files
$modelFiles = Get-ChildItem -Path "D:\Aaroneous" -Filter "*.gguf" -Recurse -ErrorAction SilentlyContinue
if ($modelFiles) {
    Write-Host "Model files: $($modelFiles.Count) files" -ForegroundColor Gray
    foreach ($model in $modelFiles) {
        $modelSize = $model.Length / 1GB
        Write-Host "  - $($model.Name): $($modelSize.ToString('F2')) GB" -ForegroundColor Gray
    }
} else {
    Write-Host "Model files: None found" -ForegroundColor Gray
}

# Recommendations
Write-Host ""
Write-Host "=== Recommendations ===" -ForegroundColor Cyan

if ($repoSize -ge $warningThreshold) {
    Write-Host "1. Run cleanup script: .\scripts\cleanup.ps1" -ForegroundColor Yellow
    Write-Host "2. Review .gitignore exclusions" -ForegroundColor Yellow
    Write-Host "3. Consider externalizing production models" -ForegroundColor Yellow
}

if ($repoSize -ge $criticalThreshold) {
    Write-Host "1. Immediate cleanup required!" -ForegroundColor Red
    Write-Host "2. Review all build artifacts" -ForegroundColor Red
    Write-Host "3. Consider archiving old documentation" -ForegroundColor Red
}

if ($repoSize -le $targetThreshold) {
    Write-Host "✅ Repository size is within target range" -ForegroundColor Green
}

Write-Host ""
Write-Host "=== Size Check Complete ===" -ForegroundColor Green