# phase1_asset_realignment.ps1
# Executes Phase 1 of Master Deconstruction Plan:
# Cleans root cruft, realigns shaders, monitoring, include headers, and runtime data.

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Aaroneous Deconstruction: Phase 1 Asset Realignment" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

# 1. Archive zstd repository
$srcZstd = "d:\Aaroneous\zstd"
$dstZstdArchive = "d:\Aaroneous\dev\archive\legacy_repos\zstd"
if (Test-Path $srcZstd) {
    if (-not (Test-Path "d:\Aaroneous\dev\archive\legacy_repos")) {
        New-Item -ItemType Directory -Path "d:\Aaroneous\dev\archive\legacy_repos" -Force | Out-Null
    }
    Move-Item -Path $srcZstd -Destination $dstZstdArchive -Force
    Write-Host "[ARCHIVED REPO] zstd -> dev/archive/legacy_repos/zstd" -ForegroundColor Green
}

# 2. Archive root web artifacts (package.json, node_modules)
$dstWebStubs = "d:\Aaroneous\dev\archive\web_stubs"
if (-not (Test-Path $dstWebStubs)) {
    New-Item -ItemType Directory -Path $dstWebStubs -Force | Out-Null
}
if (Test-Path "d:\Aaroneous\package.json") {
    Move-Item -Path "d:\Aaroneous\package.json" -Destination $dstWebStubs -Force
    Write-Host "[ARCHIVED] package.json -> dev/archive/web_stubs/" -ForegroundColor Green
}
if (Test-Path "d:\Aaroneous\package-lock.json") {
    Move-Item -Path "d:\Aaroneous\package-lock.json" -Destination $dstWebStubs -Force
    Write-Host "[ARCHIVED] package-lock.json -> dev/archive/web_stubs/" -ForegroundColor Green
}
if (Test-Path "d:\Aaroneous\node_modules") {
    Remove-Item -Path "d:\Aaroneous\node_modules" -Recurse -Force
    Write-Host "[PURGED] Root node_modules purged (MaelstromUI maintains its own)" -ForegroundColor Green
}

# 3. Move shaders to crates/compute/shaders and data/shaders
$srcShaders = "d:\Aaroneous\shaders"
$dstComputeShaders = "d:\Aaroneous\crates\compute\shaders"
$dstDataShaders = "d:\Aaroneous\data\shaders"
if (Test-Path $srcShaders) {
    if (-not (Test-Path $dstComputeShaders)) {
        New-Item -ItemType Directory -Path $dstComputeShaders -Force | Out-Null
    }
    if (-not (Test-Path $dstDataShaders)) {
        New-Item -ItemType Directory -Path $dstDataShaders -Force | Out-Null
    }
    Copy-Item -Path "$srcShaders\*" -Destination $dstComputeShaders -Recurse -Force
    Move-Item -Path $srcShaders -Destination $dstDataShaders -Force
    Write-Host "[REALIGNED] shaders -> crates/compute/shaders & data/shaders" -ForegroundColor Green
}

# 4. Consolidate monitoring under deploy/monitoring
$srcMonitoring = "d:\Aaroneous\monitoring"
$dstDeployMonitoring = "d:\Aaroneous\deploy\monitoring"
if (Test-Path $srcMonitoring) {
    if (-not (Test-Path "d:\Aaroneous\deploy")) {
        New-Item -ItemType Directory -Path "d:\Aaroneous\deploy" -Force | Out-Null
    }
    Move-Item -Path $srcMonitoring -Destination $dstDeployMonitoring -Force
    Write-Host "[REALIGNED] monitoring -> deploy/monitoring" -ForegroundColor Green
}

# 5. Move include/aas_abi.h to sdk/include/aas_abi.h
$srcInclude = "d:\Aaroneous\include"
$dstSdkInclude = "d:\Aaroneous\sdk\include"
if (Test-Path $srcInclude) {
    if (-not (Test-Path $dstSdkInclude)) {
        New-Item -ItemType Directory -Path $dstSdkInclude -Force | Out-Null
    }
    Move-Item -Path "$srcInclude\*" -Destination $dstSdkInclude -Force
    Remove-Item -Path $srcInclude -Force
    Write-Host "[REALIGNED] include -> sdk/include/" -ForegroundColor Green
}

# 6. Consolidate exports into data/exports
$srcExports = "d:\Aaroneous\exports"
$dstDataExports = "d:\Aaroneous\data\exports"
if (Test-Path $srcExports) {
    if (-not (Test-Path $dstDataExports)) {
        New-Item -ItemType Directory -Path $dstDataExports -Force | Out-Null
    }
    Move-Item -Path "$srcExports\*" -Destination $dstDataExports -Force
    Remove-Item -Path $srcExports -Force
    Write-Host "[REALIGNED] exports -> data/exports/" -ForegroundColor Green
}

# 7. Consolidate root genetics into data/genetics
$srcGenetics = "d:\Aaroneous\genetics"
$dstDataGenetics = "d:\Aaroneous\data\genetics"
if (Test-Path $srcGenetics) {
    if (-not (Test-Path $dstDataGenetics)) {
        New-Item -ItemType Directory -Path $dstDataGenetics -Force | Out-Null
    }
    Move-Item -Path "$srcGenetics\*" -Destination $dstDataGenetics -Force
    Remove-Item -Path $srcGenetics -Force
    Write-Host "[REALIGNED] root genetics -> data/genetics/" -ForegroundColor Green
}

# 8. Archive opencode state
if (Test-Path "d:\Aaroneous\opencode") {
    $dstOpencodeArchive = "d:\Aaroneous\dev\archive\opencode"
    Move-Item -Path "d:\Aaroneous\opencode" -Destination $dstOpencodeArchive -Force
    Write-Host "[ARCHIVED] opencode -> dev/archive/opencode" -ForegroundColor Green
}
if (Test-Path "d:\Aaroneous\.opencode") {
    Move-Item -Path "d:\Aaroneous\.opencode" -Destination "d:\Aaroneous\dev\archive\dot_opencode" -Force
    Write-Host "[ARCHIVED] .opencode -> dev/archive/dot_opencode" -ForegroundColor Green
}

# 9. Move extensions to data/extensions
$srcExtensions = "d:\Aaroneous\extensions"
$dstDataExtensions = "d:\Aaroneous\data\extensions"
if (Test-Path $srcExtensions) {
    if (-not (Test-Path $dstDataExtensions)) {
        New-Item -ItemType Directory -Path $dstDataExtensions -Force | Out-Null
    }
    Move-Item -Path "$srcExtensions\*" -Destination $dstDataExtensions -Force
    Remove-Item -Path $srcExtensions -Force
    Write-Host "[REALIGNED] extensions -> data/extensions/" -ForegroundColor Green
}

Write-Host "Phase 1 Asset Realignment Complete!" -ForegroundColor Cyan
