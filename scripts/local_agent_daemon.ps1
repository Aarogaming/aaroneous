# GOVERNANCE CONFLICT — READ BEFORE ENABLING UNATTENDED
# ------------------------------------------------------
# This daemon writes local-model output directly into the real Aaroneous
# working tree and self-certifies it as "completed" on nothing more than a
# passing `cargo check`. It has no human-review gate.
#
# `Aarogaming/aaroneous-devtools` already ships a stricter replacement for
# this exact lane:
#   - governance/LOCAL_AGENT_CONTROL_PLANE.md: local models may only ever
#     produce unverified *proposals* — never write, claim, or execute
#     directly against product source.
#   - governance/LOCAL_WORKER_SERVICE.md: an already-installed Windows
#     Scheduled Task (`Aaroneous-Devtools-LocalWorker`) polling
#     `worker-jobs/*.toml` every 2 minutes with qwen3.5:9b-q6, read-only
#     against product source, output always pending-owner-review.
#   - governance/COORDINATION_QUEUE.md row C14 is an incident report on a
#     sibling prototype (`agentic_registrar.rs`) that had this exact
#     shape — auto-commit gated by nothing but a bare `cargo check` — and
#     documents the concrete bugs that pattern produces (uncorrelated
#     compile-gate crate name, no path-traversal check, silent no-op
#     rollback leaving broken files in the tree, zero file locking).
#   - `MIGRATION_MANIFEST.md` already lists `scripts/local_agent_delegate.ps1`
#     (the companion script this daemon calls) as transferred out of
#     Aaroneous and explicitly *not* approved as a build/test/CI/runtime
#     dependency here.
#
# Do not run this unattended until it's reconciled with (or retired in
# favor of) the devtools worker-jobs/ pipeline. Flagged 2026-09-18,
# pending owner decision — see docs/handoff/QUEUE.md.
[CmdletBinding()]
param (
    [Parameter(Mandatory=$false)]
    [string]$QueueFile = "dev/tools/task_queue.json",

    [Parameter(Mandatory=$false)]
    [string]$LogFile = "dev/tools/agent_progress_log.md",

    [Parameter(Mandatory=$false)]
    [string]$ObservationLogFile = "dev/tools/agent_observation_log.jsonl",

    [Parameter(Mandatory=$false)]
    [int]$MaxIterations = 10,

    [Parameter(Mandatory=$false)]
    [switch]$SinglePass
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $QueueFile)) {
    Write-Error "Task queue file not found: $QueueFile"
}

Write-Host "=== Aaroneous Local Agent Passive Progress Engine ==="
Write-Host "Queue file: $QueueFile"
Write-Host "Target model: qwen3.5:9b-q6 (http://localhost:11434)"
Write-Host ""

# Appends one structured record per attempt to $ObservationLogFile (JSON
# Lines), independent of $LogFile's human-readable summary. This is what
# lets a session that did not run the daemon itself review what happened
# afterward - $LogFile alone drops every failed attempt, and Write-Host
# output vanishes once the background process exits.
function Write-Observation {
    param(
        [string]$TaskId,
        [string]$Title,
        [string]$TargetFile,
        [int]$Attempt,
        [int]$MaxAttempts,
        [string]$Outcome,
        [double]$DurationSeconds,
        [string]$ErrorText = ""
    )
    $record = [ordered]@{
        timestamp        = (Get-Date).ToString("o")
        task_id          = $TaskId
        title            = $Title
        target_file      = $TargetFile
        attempt          = $Attempt
        max_attempts     = $MaxAttempts
        outcome          = $Outcome
        duration_seconds = [math]::Round($DurationSeconds, 2)
        error            = $ErrorText
    }
    ($record | ConvertTo-Json -Compress) | Add-Content -Path $ObservationLogFile -Encoding UTF8
}

# Runs a native command and throws a terminating error on non-zero exit.
# $ErrorActionPreference = "Stop" only governs cmdlet/script errors, not
# native executable exit codes - a failing `cargo check` prints its error
# and returns control normally, so without this check $generationSuccess
# was being set to $true even when verification had actually failed.
#
# Also captures the command's combined stdout+stderr (still echoed to the
# console as before) so that on failure, the actual compiler diagnostics -
# not just a bare exit code - flow into $lastError, the JSONL observation
# record, and the self-repair prompt fed back to the model. Without this,
# a background daemon's console output was the only place that detail
# ever existed, so retries after a failure ran effectively blind.
function Invoke-Checked {
    param(
        [Parameter(Mandatory=$true)][scriptblock]$Command,
        [Parameter(Mandatory=$true)][string]$Description
    )
    $outputLines = & $Command 2>&1
    $outputLines | ForEach-Object { Write-Host $_ }
    if ($LASTEXITCODE -ne 0) {
        $outputText = ($outputLines | Out-String).Trim()
        throw "$Description failed with exit code ${LASTEXITCODE}:`n$outputText"
    }
}

# Walks up from the target file's directory to find the nearest Cargo.toml
# and returns its [package] name, rather than assuming a fixed path-segment
# position (e.g. "the second path component") is the package name. That
# assumption broke for nested dev crates: dev/rfc0006_poc/abi/src/lib.rs's
# owning package is "rfc0006_abi", not "rfc0006_poc" (the directory name a
# naive $targetParts[1] would have guessed) - and now that Invoke-Checked
# makes a nonzero `cargo check` exit fatal, that mismatch would retry an
# otherwise-valid generation to the failure limit every time.
function Resolve-OwningPackageName {
    param([Parameter(Mandatory=$true)][string]$TargetFile)
    $dir = Split-Path -Parent $TargetFile
    while ($dir -and $dir -ne ".") {
        $manifestPath = Join-Path $dir "Cargo.toml"
        if (Test-Path $manifestPath) {
            $manifestText = Get-Content -Path $manifestPath -Raw -Encoding UTF8
            if ($manifestText -match '(?ms)^\[package\][^\[]*?^\s*name\s*=\s*"([^"]+)"') {
                return $Matches[1]
            }
            throw "Found $manifestPath but could not parse a [package] name field from it"
        }
        $parentDir = Split-Path -Parent $dir
        if ($parentDir -eq $dir) { break }
        $dir = $parentDir
    }
    throw "Could not find an owning Cargo.toml (with a [package] name) for target file: $TargetFile"
}

$iteration = 0
while ($true) {
    $iteration++
    if ($MaxIterations -gt 0 -and $iteration -gt $MaxIterations) {
        Write-Host "Reached maximum iteration limit ($MaxIterations). Exiting daemon loop."
        break
    }

    $queueContent = Get-Content -Path $QueueFile -Raw -Encoding UTF8 | ConvertFrom-Json
    $pendingTasks = @($queueContent.queue | Where-Object { $_.status -eq "pending" })

    if ($pendingTasks.Count -eq 0) {
        Write-Host "No pending tasks remaining in queue. Engine idling..."
        if ($SinglePass) { break }
        Start-Sleep -Seconds 30
        continue
    }

    $task = $pendingTasks[0]
    Write-Host "----------------------------------------------------"
    Write-Host "[$($task.id)] Starting Task: $($task.title)"
    Write-Host "Target File: $($task.target_file)"

    $task.status = "in_progress"
    $queueContent | ConvertTo-Json -Depth 5 | Set-Content -Path $QueueFile -Encoding UTF8

    # Stage prompt file
    $stagedPromptPath = "scripts/staged/prompt_$($task.id).txt"
    $stagedDir = Split-Path -Parent $stagedPromptPath
    if (-not (Test-Path $stagedDir)) { New-Item -ItemType Directory -Path $stagedDir -Force | Out-Null }
    Set-Content -Path $stagedPromptPath -Value $task.prompt -Encoding UTF8

    # Execute code generation via local_agent_delegate.ps1
    $delegateScript = "scripts/local_agent_delegate.ps1"
    $generationSuccess = $false
    $retryCount = 0
    $maxRetries = if ($queueContent.max_retries) { $queueContent.max_retries } else { 3 }
    $lastError = ""

    while ($retryCount -lt $maxRetries -and -not $generationSuccess) {
        $retryCount++
        Write-Host "Execution Pass $retryCount/$maxRetries for $($task.id)..."
        $attemptStart = Get-Date

        try {
            pwsh -File $delegateScript `
                -PromptFile $stagedPromptPath `
                -OutputFile $task.target_file `
                -Model "qwen3.5:9b-q6" `
                -NumPredict 4096 `
                -Temperature 0.0
            if ($LASTEXITCODE -ne 0) {
                throw "local_agent_delegate.ps1 failed with exit code $LASTEXITCODE"
            }

            # Verify formatting and syntax
            Invoke-Checked -Description "cargo fmt" -Command { cargo fmt -- $task.target_file }

            # Verify with cargo check
            $crateName = Resolve-OwningPackageName -TargetFile $task.target_file
            Write-Host "Running compilation check for $crateName..."
            Invoke-Checked -Description "cargo check -p $crateName" -Command { cargo check -p $crateName }

            $generationSuccess = $true
            $lastError = ""
            Write-Host "Verification PASSED for $($task.id)!"
            Write-Observation -TaskId $task.id -Title $task.title -TargetFile $task.target_file `
                -Attempt $retryCount -MaxAttempts $maxRetries -Outcome "passed" `
                -DurationSeconds ((Get-Date) - $attemptStart).TotalSeconds
        } catch {
            $lastError = $_.ToString()
            Write-Host "Verification FAILED on attempt ${retryCount}: ${lastError}"
            Write-Observation -TaskId $task.id -Title $task.title -TargetFile $task.target_file `
                -Attempt $retryCount -MaxAttempts $maxRetries -Outcome "failed" `
                -DurationSeconds ((Get-Date) - $attemptStart).TotalSeconds -ErrorText $lastError

            if ($retryCount -lt $maxRetries) {
                # Append failure traceback to prompt for self-repair
                $repairPrompt = "$($task.prompt)`n`nPREVIOUS ATTEMPT FAILED WITH ERROR:`n$lastError`n`nPlease fix the error and output complete, corrected Rust code."
                Set-Content -Path $stagedPromptPath -Value $repairPrompt -Encoding UTF8
            }
        }
    }

    $timestamp = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
    if ($generationSuccess) {
        $task.status = "completed"
        $logEntry = "## [$timestamp] $($task.id): $($task.title)`n- **Target File**: ``$($task.target_file)```n- **Status**: Completed & Verified (pass $retryCount/$maxRetries)`n- **Model**: ``qwen3.5:9b-q6`` (Ollama GPU)`n`n"
        Add-Content -Path $LogFile -Value $logEntry -Encoding UTF8
        Write-Host "[$($task.id)] Marked COMPLETED and logged."
    } else {
        # Previously logged nowhere but the console: a background daemon's
        # console output is gone once the process exits, so a failed task
        # left no durable trace at all. Now recorded in both logs.
        $task.status = "failed"
        $logEntry = "## [$timestamp] $($task.id): $($task.title)`n- **Target File**: ``$($task.target_file)```n- **Status**: FAILED after $maxRetries attempts`n- **Model**: ``qwen3.5:9b-q6`` (Ollama GPU)`n- **Last Error**: ``$lastError```n`n"
        Add-Content -Path $LogFile -Value $logEntry -Encoding UTF8
        Write-Host "[$($task.id)] Marked FAILED after $maxRetries attempts and logged."
    }

    $queueContent | ConvertTo-Json -Depth 5 | Set-Content -Path $QueueFile -Encoding UTF8

    if ($SinglePass) { break }
}
