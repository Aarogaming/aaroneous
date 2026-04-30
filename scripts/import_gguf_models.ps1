# Aaroneous GGUF Model Import Script
# Copies top 5 ROI models from LM Studio to genetics extraction folder
# Run: powershell -ExecutionPolicy Bypass -File import_gguf_models.ps1

param(
    [string]$SourcePath = "$env:USERPROFILE\.lmstudio\models",
    [string]$DestPath = "D:\Aaroneous\genetics\gguf_sources",
    [switch]$Verify,
    [switch]$Cleanup
)

$ErrorActionPreference = "Stop"

Write-Host "╔════════════════════════════════════════════════════════════╗"
Write-Host "║  Aaroneous GGUF Model Import for Genetics Harvesting      ║"
Write-Host "╚════════════════════════════════════════════════════════════╝"
Write-Host ""

# Define top 5 models (ROI order)
$Models = @(
    @{
        Name = "Qwen3-Next-80B"
        Pattern = "*Qwen3-Next-80B-A3B-Instruct-Q4_K_M.gguf"
        Dest = "qwen3-next-80b.gguf"
        Size = 45.16
        Role = "Odin (Strategic Planner Base)"
    },
    @{
        Name = "Hermes-4-70B"
        Pattern = "*Hermes-4-70B-Q4_K_M.gguf"
        Dest = "hermes-4-70b.gguf"
        Size = 39.6
        Role = "Merlin (Pattern Synthesizer Base)"
    },
    @{
        Name = "Qwen3-Coder-30B"
        Pattern = "*Qwen3-Coder-30B-A3B-Instruct-Q4_K_M.gguf"
        Dest = "qwen3-coder-30b.gguf"
        Size = 17.35
        Role = "Hephaestus (Executor Base) + Code Specialization"
    },
    @{
        Name = "GLM-4.7-Flash"
        Pattern = "*GLM-4.7-Flash-Q4_K_M.gguf"
        Dest = "glm-4.7-flash.gguf"
        Size = 16.89
        Role = "Architectural Diversity (Non-Transformer)"
    },
    @{
        Name = "Gemma-3-27B"
        Pattern = "*gemma-3-27B-it-QAT-Q4_0.gguf"
        Dest = "gemma-3-27b.gguf"
        Size = 14.5
        Role = "Dionysus (Learner) + Latest Architecture"
    }
)

# Validate source exists
if (-not (Test-Path $SourcePath)) {
    Write-Host "❌ ERROR: LM Studio models path not found: $SourcePath"
    exit 1
}

if (-not (Test-Path $DestPath)) {
    Write-Host "❌ ERROR: Destination path not found: $DestPath"
    Write-Host "   Create it with: New-Item -ItemType Directory -Path '$DestPath'"
    exit 1
}

Write-Host "📁 Source: $SourcePath"
Write-Host "📁 Destination: $DestPath"
Write-Host ""

# Find and copy models
$TotalSize = 0
$SuccessCount = 0
$FailCount = 0
$Models | ForEach-Object {
    $model = $_
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Host "🔍 Searching for: $($model.Name)"
    Write-Host "   Role: $($model.Role)"
    Write-Host "   Expected Size: $($model.Size) GB"
    
    $sourceFile = Get-ChildItem -Path $SourcePath -Recurse -Filter $model.Pattern -ErrorAction SilentlyContinue | Select-Object -First 1
    
    if ($null -eq $sourceFile) {
        Write-Host "❌ NOT FOUND: $($model.Pattern)"
        $FailCount++
        return
    }
    
    $sizeMB = [math]::Round($sourceFile.Length / 1MB, 2)
    $sizeGB = [math]::Round($sourceFile.Length / 1GB, 2)
    
    Write-Host "✓ FOUND: $($sourceFile.Name)"
    Write-Host "  Size: $sizeGB GB ($sizeMB MB)"
    Write-Host "  Source: $($sourceFile.FullName)"
    
    $destFile = Join-Path $DestPath $model.Dest
    
    if (Test-Path $destFile) {
        $existingSize = [math]::Round((Get-Item $destFile).Length / 1GB, 2)
        Write-Host "  ⚠️  Already exists: $destFile ($existingSize GB)"
        
        if ($Cleanup) {
            Write-Host "  🗑️  Removing old version..."
            Remove-Item $destFile -Force
        } else {
            Write-Host "  ⏭️  Skipping (use -Cleanup to force re-import)"
            return
        }
    }
    
    Write-Host "📥 Copying to: $destFile"
    Write-Host "  This may take several minutes..."
    
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        Copy-Item -Path $sourceFile.FullName -Destination $destFile -Force
        $stopwatch.Stop()
        $seconds = [math]::Round($stopwatch.Elapsed.TotalSeconds, 2)
        
        Write-Host "✅ SUCCESS: Copied in $seconds seconds ($([math]::Round($sizeGB/$seconds*60, 2)) GB/min)"
        $TotalSize += $sizeGB
        $SuccessCount++
        
        if ($Verify) {
            Write-Host "  🔐 Verifying..."
            $srcHash = (Get-FileHash $sourceFile.FullName).Hash
            $dstHash = (Get-FileHash $destFile).Hash
            
            if ($srcHash -eq $dstHash) {
                Write-Host "  ✓ Checksum verified - integrity confirmed"
            } else {
                Write-Host "  ❌ Checksum mismatch - copy may be corrupted"
                $FailCount++
            }
        }
    } catch {
        Write-Host "❌ FAILED: $_"
        $FailCount++
    }
}

Write-Host ""
Write-Host "╔════════════════════════════════════════════════════════════╗"
Write-Host "║  Import Summary                                            ║"
Write-Host "╚════════════════════════════════════════════════════════════╝"
Write-Host ""
Write-Host "✅ Successful: $SuccessCount / $($Models.Count)"
Write-Host "❌ Failed: $FailCount / $($Models.Count)"
Write-Host "📊 Total Size Imported: $([math]::Round($TotalSize, 2)) GB"
Write-Host ""

if ($SuccessCount -eq $Models.Count) {
    Write-Host "🎉 ALL MODELS IMPORTED SUCCESSFULLY!"
    Write-Host ""
    Write-Host "Next Steps:"
    Write-Host "1. Verify models in: $DestPath"
    Write-Host "2. Run genetic extraction: aaroneous extract-genetics"
    Write-Host "3. Monitor extraction progress in: genetics/extracted_profiles"
    exit 0
} elseif ($SuccessCount -gt 0) {
    Write-Host "⚠️  PARTIAL SUCCESS: $SuccessCount models imported"
    Write-Host "   Check failed imports above and retry"
    exit 1
} else {
    Write-Host "❌ NO MODELS IMPORTED"
    Write-Host "   Ensure LM Studio models are in: $SourcePath"
    exit 2
}
