# register_ollama_models.ps1
$ollamaExe = "C:\Users\aarog\AppData\Local\Programs\Ollama\ollama.exe"

function Get-OllamaTag ($fileName) {
    switch -Regex ($fileName) {
        "Qwen3\.5-9B-Q6_K"                                   { return "qwen3.5:9b-q6" }
        "Qwen3\.5-9B-Q4_K_M"                                 { return "qwen3.5:9b-q4" }
        "Qwen3\.5-9B-Q8_0"                                   { return "qwen3.5:9b-q8" }
        "Qwen3\.5-9B-Uncensored-HauhauCS-Aggressive-Q6_K"    { return "qwen3.5-uncensored:9b" }
        "Qwen2\.5-Coder-14B-Instruct-Q4_K_M"                 { return "qwen2.5-coder:14b" }
        "Qwen3-14B-Q4_K_M"                                   { return "qwen3:14b" }
        "Qwen3-Coder-30B-A3B-Instruct-Q3_K_L"                { return "qwen3-coder:30b" }
        "Qwen3\.6-27B-Q8_0"                                  { return "qwen3.6:27b" }
        "DeepSeek-R1-0528-Qwen3-8B-Q3_K_L"                   { return "deepseek-r1-qwen3:8b-q3" }
        "DeepSeek-R1-0528-Qwen3-8B-Q8_0"                     { return "deepseek-r1-qwen3:8b-q8" }
        "DeepSeek-R1-Distill-Llama-8B-Q4_K_M"                { return "deepseek-r1:8b" }
        "OpenCodeReasoning-Nemotron-14B-Q6_K"                { return "opencode-nemotron:14b" }
        "Ministral-3-14B-Reasoning-2512-Q6_K"                { return "ministral-3:14b" }
        "Devstral-Small-2-24B-Instruct-2512-Q3_K_L"          { return "devstral-small:24b" }
        "Meta-Llama-3\.1-8B-Instruct-Q4_K_S"                 { return "llama3.1:8b" }
        "gemma-3n-E4B-it-Q8_0"                               { return "gemma-3n:e4b" }
        "gemma-4-E4B-it-Q8_0"                                { return "gemma-4:e4b" }
        "gpt-oss-20b-MXFP4"                                  { return "gpt-oss:20b" }
        default                                              { return $null }
    }
}

$searchPaths = @(
    "C:\Users\aarog\.lmstudio\models"
)

$registered = @()

foreach ($path in $searchPaths) {
    if (Test-Path $path) {
        $ggufFiles = Get-ChildItem -Path $path -Filter "*.gguf" -Recurse -File
        foreach ($file in $ggufFiles) {
            if ($file.Name.StartsWith("mmproj-")) { continue }

            $tag = Get-OllamaTag $file.BaseName
            if (-not $tag) {
                $cleanName = $file.BaseName.ToLower() -replace '[^a-z0-9\.\-]', '-'
                $tag = "custom-$cleanName"
            }

            # Avoid registering duplicate tags
            if ($registered -contains $tag) { continue }

            $uniqueModelfile = Join-Path $file.DirectoryName "Modelfile.$($file.BaseName)"
            $modelfileToUse = if (Test-Path $uniqueModelfile) { $uniqueModelfile } else { Join-Path $file.DirectoryName "Modelfile" }

            Write-Host "Registering '$tag' from: $modelfileToUse" -ForegroundColor Cyan
            & $ollamaExe create $tag -f $modelfileToUse
            if ($LASTEXITCODE -eq 0) {
                $registered += $tag
                Write-Host "  -> Successfully registered '$tag'" -ForegroundColor Green
            } else {
                Write-Host "  -> Failed to register '$tag'" -ForegroundColor Red
            }
        }
    }
}

Write-Host "`nRegistered $($registered.Count) models in Ollama." -ForegroundColor Yellow
& $ollamaExe list
