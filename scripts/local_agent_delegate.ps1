[CmdletBinding()]
param (
    [Parameter(Mandatory=$false)]
    [string]$PromptFile,

    [Parameter(Mandatory=$false)]
    [string]$OutputFile,

    [Parameter(Mandatory=$false)]
    [string[]]$PromptFiles,

    [Parameter(Mandatory=$false)]
    [string[]]$OutputFiles,

    [Parameter(Mandatory=$false)]
    [switch]$Decompose,

    [Parameter(Mandatory=$false)]
    [string]$Endpoint = "http://localhost:8000/v1",

    [Parameter(Mandatory=$false)]
    [string]$Model = "Qwen/Qwen2.5-Coder-14B-Instruct-AWQ",

    [Parameter(Mandatory=$false)]
    [string]$SystemPrompt = "You are a senior Rust systems programmer. Do NOT output any lengthy thinking trace or reasoning. Output only pure, complete Rust code.",

    [Parameter(Mandatory=$false)]
    [int]$MaxTokens = 4096,

    [Parameter(Mandatory=$false)]
    [float]$Temperature = 0.0
)

$ErrorActionPreference = "Stop"

function Test-VllmHealth {
    param([string]$BaseEndpoint)
    try {
        $models = Invoke-RestMethod -Uri "$BaseEndpoint/models" -Method Get -TimeoutSec 5
        return $models.data | Select-Object -ExpandProperty id
    } catch {
        Write-Error "vLLM server unreachable at $BaseEndpoint (expected an OpenAI-compatible server, e.g. 'python -m vllm.entrypoints.openai.api_server --model <model>'): $_"
    }
}

function Invoke-VllmChat {
    param(
        [string]$BaseEndpoint,
        [string]$ModelName,
        [string]$System,
        [string]$UserContent,
        [int]$Tokens,
        [float]$Temp
    )

    $body = @{
        model = $ModelName
        messages = @(
            @{ role = "system"; content = $System },
            @{ role = "user"; content = $UserContent }
        )
        temperature = $Temp
        max_tokens = $Tokens
        stream = $false
    } | ConvertTo-Json -Depth 5

    $response = Invoke-RestMethod -Uri "$BaseEndpoint/chat/completions" -Method Post -Body $body -ContentType "application/json"

    $content = $response.choices[0].message.content

    # Strip markdown code fencing if present
    if ($content -match '(?s)```(?:rust)?\s*(.*?)\s*```') {
        $content = $matches[1]
    }

    return $content
}

function Write-DelegateOutput {
    param([string]$Content, [string]$Path)

    if (-not $Path) {
        Write-Output $Content
        return
    }

    $outDir = Split-Path -Parent $Path
    if ($outDir -and -not (Test-Path $outDir)) {
        New-Item -ItemType Directory -Path $outDir -Force | Out-Null
    }
    Set-Content -Path $Path -Value $Content -Encoding UTF8
    Write-Host "Output successfully written to $Path"
}

# --- Decompose mode: sequential multi-phase delegation ---------------------
# Operationalizes the "Modular Task Decomposition" protocol: each phase's
# prompt is run in order, and every prior phase's generated output is
# appended as context ahead of the next phase's own prompt, so later phases
# (e.g. accessor methods) see the exact code earlier phases (e.g. struct
# definitions) already produced instead of re-deriving it blind.
if ($Decompose) {
    if (-not $PromptFiles -or $PromptFiles.Count -eq 0) {
        Write-Error "-Decompose requires -PromptFiles (an ordered list of per-phase prompt files)."
    }
    if ($OutputFiles -and $OutputFiles.Count -ne $PromptFiles.Count) {
        Write-Error "-OutputFiles must have the same number of entries as -PromptFiles when both are supplied."
    }

    Test-VllmHealth -BaseEndpoint $Endpoint | Out-Null

    $accumulatedContext = ""
    for ($i = 0; $i -lt $PromptFiles.Count; $i++) {
        $phaseFile = $PromptFiles[$i]
        if (-not (Test-Path $phaseFile)) {
            Write-Error "Prompt file not found: $phaseFile"
        }

        $phasePrompt = Get-Content -Path $phaseFile -Raw -Encoding UTF8
        $userContent = if ($accumulatedContext) {
            "Code already produced by earlier phases of this task:`n`n$accumulatedContext`n`n---`n`nCurrent phase:`n$phasePrompt"
        } else {
            $phasePrompt
        }

        Write-Host "[Phase $($i + 1)/$($PromptFiles.Count)] Dispatching $phaseFile to vLLM ($Model)..."
        $generated = Invoke-VllmChat -BaseEndpoint $Endpoint -ModelName $Model -System $SystemPrompt -UserContent $userContent -Tokens $MaxTokens -Temp $Temperature

        $phaseOutputFile = if ($OutputFiles) { $OutputFiles[$i] } else { $null }
        Write-DelegateOutput -Content $generated -Path $phaseOutputFile

        $accumulatedContext = if ($accumulatedContext) { "$accumulatedContext`n`n$generated" } else { $generated }
    }

    return
}

# --- Single-shot mode -------------------------------------------------------
if (-not $PromptFile) {
    Write-Error "Either -PromptFile (single-shot) or -Decompose with -PromptFiles (multi-phase) is required."
}
if (-not (Test-Path $PromptFile)) {
    Write-Error "Prompt file not found: $PromptFile"
}

Test-VllmHealth -BaseEndpoint $Endpoint | Out-Null

$promptContent = Get-Content -Path $PromptFile -Raw -Encoding UTF8
Write-Host "Dispatching prompt to vLLM OpenAI-compatible API ($Model)..."
$generatedContent = Invoke-VllmChat -BaseEndpoint $Endpoint -ModelName $Model -System $SystemPrompt -UserContent $promptContent -Tokens $MaxTokens -Temp $Temperature

Write-DelegateOutput -Content $generatedContent -Path $OutputFile
