# stage8_promote_remaining_components.ps1
# Promotes digestion, skills, scientific_analyzer, control, hive, intelligence to crates/
# and archives obsolete/legacy components.

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Aaroneous Deconstruction: Stage 8 Promotion" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

$archiveDir = "d:\Aaroneous\dev\archive\legacy_components"
if (-not (Test-Path $archiveDir)) {
    New-Item -ItemType Directory -Path $archiveDir -Force | Out-Null
}

# 1. Promote High-Value Engines
$toPromote = @(
    "digestion",
    "skills",
    "scientific_analyzer",
    "control",
    "hive",
    "intelligence"
)

foreach ($crate in $toPromote) {
    $src = "d:\Aaroneous\components\$crate"
    $dst = "d:\Aaroneous\crates\$crate"
    if ((Test-Path $src) -and (-not (Test-Path $dst))) {
        Copy-Item -Path $src -Destination $dst -Recurse -Force
        Write-Host "[PROMOTED] components/$crate -> crates/$crate" -ForegroundColor Green
        
        # Move original to archive
        Move-Item -Path $src -Destination (Join-Path $archiveDir "components_$crate") -Force
        Write-Host "[ARCHIVED] components/$crate -> dev/archive/legacy_components/" -ForegroundColor Yellow
    }
}

# 2. Archive Obsolete Stubs
$toArchive = @(
    "deconstruction",
    "sab",
    "sab_matrix",
    "storage",
    "sabs",
    "agents"
)

foreach ($crate in $toArchive) {
    $src = "d:\Aaroneous\components\$crate"
    if (Test-Path $src) {
        Move-Item -Path $src -Destination (Join-Path $archiveDir "components_$crate") -Force
        Write-Host "[ARCHIVED STUB] components/$crate -> dev/archive/legacy_components/" -ForegroundColor DarkYellow
    }
}

Write-Host "Stage 8 component promotion & archival complete." -ForegroundColor Cyan
