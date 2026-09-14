param(
    [string]$Prompt,
    [string]$PromptFile,
    [string]$OutputFile,
    [string]$SystemPrompt = "You are a senior Rust systems programmer. Do NOT output any lengthy thinking trace or reasoning. Output only pure, complete Rust code.",
    [string]$Model = "qwen3.5:9b-q6",
    [string]$Endpoint = "http://localhost:11434/api/chat",
    [double]$Temperature = 0.0,
    [int]$NumPredict = 4096,
    [switch]$IncludeThinking
)

$ErrorActionPreference = "Stop"

if ($PromptFile) {
    $Prompt = Get-Content -Path $PromptFile -Raw
}

if (-not $Prompt) {
    throw "Either -Prompt or -PromptFile must be provided."
}

$messages = @()
if ($SystemPrompt) {
    $messages += @{
        role = "system"
        content = $SystemPrompt
    }
}
$messages += @{
    role = "user"
    content = $Prompt
}

$body = @{
    model = $Model
    messages = $messages
    stream = $false
    options = @{
        temperature = $Temperature
        num_predict = $NumPredict
    }
} | ConvertTo-Json -Depth 5

$response = Invoke-RestMethod -Uri $Endpoint -Method Post -Body $body -ContentType "application/json"

if ($IncludeThinking -and $response.message.thinking) {
    Write-Host "--- Local Model Thinking ---" -ForegroundColor DarkGray
    Write-Host $response.message.thinking -ForegroundColor DarkGray
    Write-Host "----------------------------" -ForegroundColor DarkGray
}

$content = $response.message.content

if ($OutputFile) {
    Set-Content -Path $OutputFile -Value $content -Encoding UTF8
    Write-Host "Wrote output to $OutputFile ($($content.Length) bytes)"
}

Write-Output $content
