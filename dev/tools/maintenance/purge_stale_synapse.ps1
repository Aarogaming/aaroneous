# purge_stale_synapse.ps1
# Safely deletes orphaned .synapse memory-mapped files and temporary lock files in %TEMP%.

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Purging Stale Synapse & Temporary Lock Files" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

$tempPath = [System.IO.Path]::GetTempPath()
$synapseFiles = Get-ChildItem -Path $tempPath -Filter "*.synapse" -ErrorAction SilentlyContinue

if ($synapseFiles) {
    Write-Host "Found $($synapseFiles.Count) .synapse file(s) in $tempPath:" -ForegroundColor Yellow
    foreach ($file in $synapseFiles) {
        try {
            Remove-Item -Path $file.FullName -Force -ErrorAction Stop
            Write-Host "  [DELETED] $($file.FullName)" -ForegroundColor Green
        } catch {
            Write-Host "  [LOCKED] Cannot delete $($file.FullName) - File is currently in use by another process!" -ForegroundColor Red
        }
    }
} else {
    Write-Host "No stale .synapse files found in $tempPath." -ForegroundColor Green
}

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "Cleanup completed." -ForegroundColor Cyan
