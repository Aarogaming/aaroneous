param (
    [string]$ModelKey = "qwen_uncensored",
    [int]$Port = 8080
)
$Registry = @{
    "qwen_uncensored" = @{ Repo = "HauhauCS/Qwen3.5-9B-Uncensored-Aggressive-GGUF"; File = "Qwen3.5-9B-Uncensored-HauhauCS-Aggressive-Q6_K.gguf" }
    "deepseek_r1"     = @{ Repo = "unsloth/DeepSeek-R1-Distill-Qwen-8B-GGUF"; File = "DeepSeek-R1-Distill-Qwen-8B-Q4_K_M.gguf" }
}
$Target = $Registry[$ModelKey]
Write-Host "REGISTRY: Resolving remote target: \$(\$Target.Repo)/\$(\$Target.File)"
huggingface-cli download \$Target.Repo \$Target.File --local-dir ".\genetics\cache" --local-dir-use-symlinks False
Write-Host "LAUNCHING ENGINE: Preparing runtime at .\genetics\cache\\$(\$Target.File) on port \$Port"
