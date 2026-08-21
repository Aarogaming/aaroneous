# stage1_cleanup.ps1
# Removes empty ghost shells in agents/ and moves ad-hoc extraction scripts to dev/archive/scripts/.

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Aaroneous Deconstruction: Stage 1 Cleanup" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

# 1. Remove the 9 ghost agent directories
$ghostAgents = @(
    "asset_forge",
    "code_reviewer",
    "comms_bridge",
    "net_gateway",
    "qa_engineer",
    "sleep_cycle",
    "system_janitor",
    "vcs_operator",
    "web_scraper"
)

foreach ($ag in $ghostAgents) {
    $path = "d:\Aaroneous\agents\$ag"
    if (Test-Path $path) {
        Remove-Item -Path $path -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "  [REMOVED GHOST] $path" -ForegroundColor Green
    }
}

# 2. Create archive directory
$archiveDir = "d:\Aaroneous\dev\archive\scripts"
if (-not (Test-Path $archiveDir)) {
    New-Item -ItemType Directory -Path $archiveDir -Force | Out-Null
    Write-Host "  [CREATED ARCHIVE] $archiveDir" -ForegroundColor Cyan
}

# 3. Move ad-hoc files from scripts/ to dev/archive/scripts/
$scriptsPath = "d:\Aaroneous\scripts"
$files = Get-ChildItem -Path $scriptsPath -File -ErrorAction SilentlyContinue
foreach ($file in $files) {
    Move-Item -Path $file.FullName -Destination $archiveDir -Force
    Write-Host "  [ARCHIVED] $($file.Name) -> dev/archive/scripts/" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "Stage 1 Cleanup complete." -ForegroundColor Green
Write-Host "======================================================" -ForegroundColor Cyan
