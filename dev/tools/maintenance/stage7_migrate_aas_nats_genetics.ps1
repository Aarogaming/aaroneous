# stage7_migrate_aas_nats_genetics.ps1
# Migrates AAS to sdk/python, nats to bin/nats, chromosomes to data/chromosomes, and promotes genetics to crates/genetics.

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Aaroneous Deconstruction: Stage 7 Asset Migration" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

# 1. Migrate AAS to sdk/python
$srcAas = "d:\Aaroneous\AAS"
$dstSdkPy = "d:\Aaroneous\sdk\python"
if ((Test-Path $srcAas) -and (-not (Test-Path $dstSdkPy))) {
    Move-Item -Path $srcAas -Destination $dstSdkPy -Force
    Write-Host "[MIGRATED] AAS -> sdk/python" -ForegroundColor Green
}

# 2. Migrate nats to bin/nats
$srcNats = "d:\Aaroneous\nats"
$dstBinNats = "d:\Aaroneous\bin\nats"
if ((Test-Path $srcNats) -and (-not (Test-Path $dstBinNats))) {
    Move-Item -Path $srcNats -Destination $dstBinNats -Force
    Write-Host "[MIGRATED] nats -> bin/nats" -ForegroundColor Green
}

# 3. Migrate chromosomes to data/chromosomes
$srcChromosomes = "d:\Aaroneous\chromosomes"
$dstDataChromosomes = "d:\Aaroneous\data\chromosomes"
if ((Test-Path $srcChromosomes) -and (-not (Test-Path $dstDataChromosomes))) {
    Move-Item -Path $srcChromosomes -Destination $dstDataChromosomes -Force
    Write-Host "[MIGRATED] chromosomes -> data/chromosomes" -ForegroundColor Green
}

# 4. Promote genetics to crates/genetics
$srcGenetics = "d:\Aaroneous\components\genetics"
$dstGenetics = "d:\Aaroneous\crates\genetics"
if ((Test-Path $srcGenetics) -and (-not (Test-Path $dstGenetics))) {
    Copy-Item -Path $srcGenetics -Destination $dstGenetics -Recurse -Force
    Write-Host "[PROMOTED] components/genetics -> crates/genetics" -ForegroundColor Green

    # Archive original components/genetics
    $archiveComp = "d:\Aaroneous\dev\archive\legacy_components\components_genetics"
    Move-Item -Path $srcGenetics -Destination $archiveComp -Force
    Write-Host "[ARCHIVED] components/genetics -> dev/archive/legacy_components/" -ForegroundColor Yellow
}

Write-Host "Stage 7 migration complete." -ForegroundColor Cyan
