# generate_modelfiles.ps1
$searchPaths = @(
    "C:\Users\aarog\.lmstudio\models",
    "D:\Aaroneous\data\genetics\gguf_sources"
)

$discoveredModels = @()

foreach ($path in $searchPaths) {
    if (Test-Path $path) {
        Write-Host "Scanning path: $path" -ForegroundColor Cyan
        $ggufFiles = Get-ChildItem -Path $path -Filter "*.gguf" -Recurse -File
        foreach ($file in $ggufFiles) {
            if ($file.Name.StartsWith("mmproj-")) {
                Write-Host "  [Skip mmproj] $($file.Name)" -ForegroundColor DarkGray
                continue
            }

            $modelfileName = "Modelfile"
            $targetDir = $file.DirectoryName
            $targetPath = Join-Path $targetDir $modelfileName
            $uniqueModelfileName = "Modelfile.$($file.BaseName)"
            $uniqueTargetPath = Join-Path $targetDir $uniqueModelfileName

            $content = @"
FROM $($file.FullName)
PARAMETER stop "<|im_start|>"
PARAMETER stop "<|im_end|>"
PARAMETER num_ctx 8192
"@

            Set-Content -Path $targetPath -Value $content -Encoding UTF8
            Set-Content -Path $uniqueTargetPath -Value $content -Encoding UTF8

            $discoveredModels += [PSCustomObject]@{
                FileName       = $file.Name
                Directory      = $targetDir
                Modelfile      = $targetPath
                UniqueModelfile= $uniqueTargetPath
                SizeBytes      = $file.Length
            }

            Write-Host "  [Generated] $($file.Name) -> $targetPath" -ForegroundColor Green
        }
    }
}

Write-Host "`nSummary: Generated $($discoveredModels.Count) Modelfiles across model repositories." -ForegroundColor Yellow
$discoveredModels | Select-Object FileName, UniqueModelfile | Format-Table -AutoSize
