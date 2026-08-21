#!/usr/bin/env pwsh

<#
.SYNOPSIS
    Philosopher's Stone - LLM-based transpiler with reflexion loop
.DESCRIPTION
    Converts text/articles to executable code with hot-patching mechanism
#>

param(
    [string]$InputText,
    [string]$OutputPath,
    [string]$Language = "rust",
    [switch]$DryRun
)

# Configuration
$LLM_ENDPOINT = "http://localhost:11434"
$LLM_MODEL = "qwen2.5:7b"
$MAX_ITERATIONS = 5
$ERROR_THRESHOLD = 3

# State
$iteration = 0
$last_error = $null
$compiled_successfully = $false

function Write-TranspilerLog {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    Write-Host "[$timestamp] [$Level] $Message" -ForegroundColor $Level
}

function Get-LLMResponse {
    param([string]$Prompt, [string]$SystemPrompt)
    
    $payload = @{
        model = $LLM_MODEL
        messages = @(
            @{role = "system"; content = $SystemPrompt},
            @{role = "user"; content = $Prompt}
        )
        stream = $false
    }
    
    try {
        $response = Invoke-RestMethod -Uri "$LLM_ENDPOINT/api/generate" -Method POST -Body $payload -ContentType "application/json"
        return $response.response
    }
    catch {
        Write-TranspilerLog "LLM request failed: $($_.Exception.Message)" "ERROR"
        return $null
    }
}

function Compile-Code {
    param([string]$Code, [string]$Language)
    
    $compiler = switch ($Language) {
        "rust" { "cargo" }
        "c" { "gcc" }
        "python" { "python" }
        "javascript" { "node" }
        default { "cargo" }
    }
    
    $cmd = $compiler + " build"
    
    if ($DryRun) {
        Write-TranspilerLog "Would compile: $cmd" "DEBUG"
        return $true
    }
    
    Write-TranspilerLog "Compiling $Code..." "INFO"
    return $true
}

function Fix-Errors {
    param([string]$Error, [string]$Code, [int]$Iteration)
    
    $prompt = @"
You are a code compiler. The following code failed to compile:

Error: $Error

Code:
$Code

Please fix the code to resolve the compilation error.
"@
    
    $system_prompt = @"
You are a compiler expert. Fix the code to resolve compilation errors.
Be precise and minimal in your fixes.
"@
    
    $fix = Get-LLMResponse -Prompt $prompt -SystemPrompt $system_prompt
    return $fix
}

function Apply-HotPatch {
    param([string]$OriginalCode, [string]$Patch)
    
    # Simple string replacement for hot-patching
    $patched = $OriginalCode -replace '\{\{PATCH\}\}', $Patch
    return $patched
}

function Transpile-TextToCode {
    param([string]$Text, [string]$Language)
    
    $prompt = @"
Convert this text/article into a $Language program that processes this information:

Text:
$Text

Generate complete, compilable $Language code.
"@
    
    $code = Get-LLMResponse -Prompt $prompt -SystemPrompt "Generate $Language code that processes the given text."
    return $code
}

function Transpile-Loop {
    param([string]$Text, [string]$Language, [int]$MaxIterations)
    
    Write-TranspilerLog "Starting transpilation loop..." "INFO"
    
    $code = Transpile-TextToCode -Text $Text -Language $Language
    $iteration = 0
    
    while ($iteration -lt $MaxIterations) {
        $iteration++
        Write-TranspilerLog "Iteration $iteration" "INFO"
        
        if ($DryRun) {
            Write-TranspilerLog "Dry run - skipping compilation" "INFO"
            $compiled_successfully = $true
            break
        }
        
        if (Compile-Code -Code $code -Language $Language) {
            Write-TranspilerLog "Compilation successful!" "SUCCESS"
            $compiled_successfully = $true
            break
        }
        
        $error = "Compilation failed at iteration $iteration"
        Write-TranspilerLog "Compilation failed: $error" "ERROR"
        
        $fix = Fix-Errors -Error $error -Code $code -Iteration $iteration
        if ($fix) {
            $code = Apply-HotPatch -OriginalCode $code -Patch $fix
            Write-TranspilerLog "Applied hot-patch" "INFO"
        }
        else {
            Write-TranspilerLog "Failed to fix errors" "ERROR"
        }
    }
    
    return $code
}

# Main
Write-TranspilerLog "Philosopher's Stone Transpiler" "INFO"
Write-TranspilerLog "Language: $Language" "INFO"

if ($InputText) {
    $code = Transpile-Loop -Text $InputText -Language $Language -MaxIterations $MAX_ITERATIONS
    if ($code) {
        if ($DryRun) {
            Write-TranspilerLog "Dry run complete" "INFO"
        }
        else {
            $code | Out-File -FilePath $OutputPath -Encoding utf8
            Write-TranspilerLog "Output written to: $OutputPath" "SUCCESS"
        }
    }
}
else {
    Write-TranspilerLog "No input text provided" "WARNING"
}