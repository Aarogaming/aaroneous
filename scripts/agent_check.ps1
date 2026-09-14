# Use the same canonical gate on Windows. Prefer Git Bash over a broken or
# differently configured WSL distribution; callers may pass an explicit path.
param([string]$BashPath = "")
$ErrorActionPreference = "Stop"
if (-not $BashPath) {
    $gitCommand = Get-Command git -ErrorAction Stop
    $gitRoot = Split-Path (Split-Path $gitCommand.Source -Parent) -Parent
    $candidate = Join-Path $gitRoot "bin/bash.exe"
    if (Test-Path -LiteralPath $candidate) { $BashPath = $candidate }
    else { $BashPath = (Get-Command bash -ErrorAction Stop).Source }
}
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    & $BashPath scripts/agent_check.sh
    $gateStatus = $LASTEXITCODE
} finally { Pop-Location }
exit $gateStatus
