#requires -Version 5.1

function Test-ProjectMoveWriteObserved {
    param(
        [Parameter(Mandatory = $true)][long]$SinceWallTimeMs
    )

    try {
        if ((Get-ProjectMoveUiStage) -in @(
            "submitting",
            "syncing",
            "failed_after_write",
            "completed"
        )) {
            return $true
        }
        $trace = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "trace_recent" `
            -Arguments @{
                phase = "web_chat_project_move_reconciliation"
                since_wall_time_ms = $SinceWallTimeMs
                limit = 5
            }
        return [int]$trace.matched_count -gt 0 -or @($trace.events).Count -gt 0
    } catch {
        return $false
    }
}
