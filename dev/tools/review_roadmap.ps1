$TodoPath = "d:\Aaroneous\TODO.md"
$ActiveQueuePath = "d:\Aaroneous\dev\docs\audits\active\ACTIVE_AUDIT_QUEUE.md"

Clear-Host
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "          AARONEOUS ROADMAP & MILESTONE PROGRESS           " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

if (Test-Path $ActiveQueuePath) {
    $QueueLines = Get-Content $ActiveQueuePath
    $CompletedQueue = ($QueueLines | Select-String "- \[x\]").Count
    $PendingQueue = ($QueueLines | Select-String "- \[ \]").Count
    $TotalQueue = $CompletedQueue + $PendingQueue
    
    Write-Host "`n[Active Audit Queue]" -ForegroundColor Yellow
    Write-Host "  Completed: $CompletedQueue / $TotalQueue" -ForegroundColor Green
    Write-Host "  Pending:   $PendingQueue" -ForegroundColor $(if ($PendingQueue -gt 0) { "Red" } else { "Green" })
    
    if ($PendingQueue -gt 0) {
        Write-Host "`nNext 3 Priority Items in Queue:" -ForegroundColor Yellow
        $QueueLines | Select-String "- \[ \] \*\*(.*?)\*\*" | Select-Object -First 3 | ForEach-Object {
            Write-Host "  -> $($_.Matches[0].Groups[1].Value)" -ForegroundColor White
        }
    }
}

if (Test-Path $TodoPath) {
    $TodoLines = Get-Content $TodoPath
    $CompletedTodo = ($TodoLines | Select-String "- \[x\]").Count
    $PendingTodo = ($TodoLines | Select-String "- \[ \]").Count
    $TotalTodo = $CompletedTodo + $PendingTodo
    $Percent = [math]::Round(($CompletedTodo / $TotalTodo) * 100, 1)

    Write-Host "`n[Master Roadmap (TODO.md)]" -ForegroundColor Yellow
    Write-Host "  Total Tasks Tracked: $TotalTodo"
    Write-Host "  Completed:           $CompletedTodo ($Percent%)" -ForegroundColor Green
    Write-Host "  Remaining Debt/Gaps: $PendingTodo" -ForegroundColor Cyan
    
    Write-Host "`nActive Subsystems with Pending Debt:" -ForegroundColor Yellow
    $Tiers = @("TIER 1: CRITICAL", "TIER 2: HIGH", "TIER 3: MEDIUM", "TIER 4: LOW")
    foreach ($Tier in $Tiers) {
        $Count = ($TodoLines | Select-String "## .*$Tier" -Context 0, 30 | Out-String | Select-String "- \[ \]" -AllMatches).Matches.Count
        Write-Host "  * $Tier: $Count items pending" -ForegroundColor DarkYellow
    }
}

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Press any key to launch OpenCode with Milestone Review or close this window..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

npx opencode run --agent auditor --auto "Review active roadmap progress. Compare @dev/docs/audits/active/ACTIVE_AUDIT_QUEUE.md with pending Tier 1/2 debt and report the top 3 architectural bottlenecks to address."
