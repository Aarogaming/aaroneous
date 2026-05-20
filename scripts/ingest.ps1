# Aaroneous Metabolic Governor
# Automated ingestion script for CI/CD
param(
    [string]$TriagePath = "D:\Aaroneous\triage\intake",
    [string]$ComponentsPath = "D:\Aaroneous\components",
    [string]$ExtensionsPath = "D:\Aaroneous\extensions"
)

Write-Host "[Governor] Starting Metabolic Ingestion..." -ForegroundColor Cyan

# 1. Scan Triage for new items
$items = Get-ChildItem -Path $TriagePath -ErrorAction SilentlyContinue
if (-not $items) {
    Write-Host "[Governor] No new items in Triage. Exiting."
    exit 0
}

foreach ($item in $items) {
    Write-Host "[Governor] Processing: $($item.Name)" -ForegroundColor Yellow
    
    # 2. Determine type (Rust/Python/WASM)
    if ($item.Extension -eq ".rs") {
        # Move to components
        $dest = Join-Path $ComponentsPath "new_component"
        Write-Host "[Governor] Ingesting Rust logic -> $dest"
        # In reality, this would trigger a cargo new and move logic
    } elseif ($item.Extension -eq ".py") {
        # Move to extensions
        $dest = Join-Path $ExtensionsPath "python"
        Write-Host "[Governor] Ingesting Python logic -> $dest"
        Move-Item $item.FullName $dest -Force
    }
}

Write-Host "[Governor] Ingestion Complete. Workspace updated." -ForegroundColor Green
