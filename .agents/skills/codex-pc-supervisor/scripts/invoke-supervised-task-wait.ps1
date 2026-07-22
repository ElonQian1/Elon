function Get-WaitFailureCode {
    param([System.Exception]$Exception)
    $current = $Exception
    while ($null -ne $current) {
        if ($current.Message -like 'Node API returned HTTP *') { return 'node_api_error' }
        if ($current -is [System.Net.WebException]) {
            if ($current.Status -eq [System.Net.WebExceptionStatus]::Timeout) {
                return 'request_timeout'
            }
            if ($current.Status -in @(
                [System.Net.WebExceptionStatus]::ConnectFailure,
                [System.Net.WebExceptionStatus]::NameResolutionFailure,
                [System.Net.WebExceptionStatus]::ProxyNameResolutionFailure,
                [System.Net.WebExceptionStatus]::ConnectionClosed
            )) { return 'node_unreachable' }
        }
        $current = $current.InnerException
    }
    return 'request_failed'
}

function Resolve-WaitOutcome {
    param(
        [bool]$IsTerminal,
        [string]$Status,
        [int]$EventCount,
        [bool]$CursorReset,
        [bool]$TimedOut
    )
    if ($IsTerminal) { return 'terminal' }
    if ($Status -eq 'waiting_approval') { return 'waiting_approval' }
    if ($EventCount -gt 0 -or $CursorReset) { return 'changed' }
    if ($TimedOut) { return 'no_change_timeout' }
    return 'snapshot'
}

function Invoke-SupervisedWait {
    param(
        [object]$Connection,
        [string]$RequestedTaskId,
        [bool]$CompactResult,
        [int]$BoundarySeconds,
        [int]$InitialSince,
        [int]$RequestedLimit,
        [bool]$LimitWasBound,
        [string]$InitialCursorEpoch,
        [string]$ExpectedState,
        [string]$ExpectedEvidence
    )
    if ($CompactResult) {
        Assert-NodeSupervisionCapability $Connection $script:DeltaWaitCapability 'Compact Wait'
    }
    $terminalStatuses = @('done', 'finished', 'success', 'succeeded', 'failed', 'error', 'canceled', 'cancelled', 'interrupted', 'resume_required')
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    $detail = $null
    $status = ''
    $cursor = if ($InitialSince -ge 0) { $InitialSince } else { 0 }
    $initialCursor = $cursor
    $cursorEpoch = $InitialCursorEpoch
    $waitLimit = if ($LimitWasBound) { $RequestedLimit } else { 25 }
    $collectedEvents = New-Object System.Collections.Generic.List[object]
    $seenEvents = @{}
    $sawCursorReset = $false
    $lastWaitError = $null
    do {
        try {
            $remainingEventCapacity = $waitLimit - $collectedEvents.Count
            if ($remainingEventCapacity -le 0) { break }
            $pageLimit = [Math]::Max(1, [Math]::Min($waitLimit, $remainingEventCapacity))
            $remainingSeconds = [Math]::Max(1, [Math]::Min(15, [Math]::Ceiling($BoundarySeconds - $timer.Elapsed.TotalSeconds)))
            $detail = Get-TaskDetail $Connection $RequestedTaskId $pageLimit $cursor $cursorEpoch $remainingSeconds
            $record = Get-RecordFromDetail $detail
            $status = ([string](Get-ObjectField $record 'status')).ToLowerInvariant()
            if (Merge-TaskDeltaEvents $collectedEvents $seenEvents $detail) {
                $sawCursorReset = $true
            }
            $returnedCursor = [int](Get-ObjectField $detail 'last_event_seq')
            $cursor = Resolve-MonotonicTaskCursor $cursor $returnedCursor `
                ([bool](Get-ObjectField $detail 'cursor_reset')) `
                ([int](Get-ObjectField $detail 'resume_cursor'))
            $returnedEpoch = [string](Get-ObjectField $detail 'cursor_epoch')
            if (-not [string]::IsNullOrWhiteSpace($returnedEpoch)) { $cursorEpoch = $returnedEpoch }
            if ($terminalStatuses -contains $status -or $status -eq 'waiting_approval') { break }
            if ($collectedEvents.Count -ge $waitLimit) { break }
            if ((Get-ObjectField $detail 'has_more') -eq $true) { continue }
        } catch {
            $lastWaitError = $_.Exception
            if ($timer.Elapsed.TotalSeconds -ge $BoundarySeconds) {
                $reason = Get-WaitFailureCode $_.Exception
                throw "$reason`: Compact Wait exhausted its WaitSeconds boundary."
            }
        }
        $remainingSleepMs = [Math]::Floor(($BoundarySeconds - $timer.Elapsed.TotalSeconds) * 1000)
        if ($remainingSleepMs -gt 0) { Start-Sleep -Milliseconds ([Math]::Min(2000, $remainingSleepMs)) }
    } while ($timer.Elapsed.TotalSeconds -lt $BoundarySeconds)
    if ($null -eq $detail -and $null -ne $lastWaitError) {
        $reason = Get-WaitFailureCode $lastWaitError
        throw "$reason`: Compact Wait exhausted its WaitSeconds boundary."
    }
    $isTerminal = $terminalStatuses -contains $status
    $timedOut = -not $isTerminal -and $status -ne 'waiting_approval' -and
        $timer.Elapsed.TotalSeconds -ge $BoundarySeconds
    $waitOutcome = Resolve-WaitOutcome $isTerminal $status $collectedEvents.Count `
        $sawCursorReset $timedOut
    $resultDetail = if ($CompactResult) {
        Select-TaskDeltaChanges `
            (Convert-ToCompactTaskDetail $detail ($collectedEvents.ToArray()) $isTerminal) `
            $ExpectedState $ExpectedEvidence
    } else { $detail }
    Convert-ToJsonResult ([ordered]@{
        ok = $true; action = 'Wait'; protocol = $script:SupervisionProtocol
        node_url = $Connection.BaseUrl; task_id = $RequestedTaskId; status = $status
        wait_outcome = $waitOutcome; timed_out = $timedOut
        elapsed_ms = $timer.ElapsedMilliseconds
        since = $(if ($InitialSince -ge 0) { $InitialSince } else { 0 }); limit = $waitLimit
        next_cursor = $cursor; detail = $resultDetail
        cursor_reset = [bool]($sawCursorReset -or (Get-ObjectField $detail 'cursor_reset'))
        requested_cursor = Get-ObjectField $detail 'requested_cursor'
        old_cursor = Get-ObjectField $detail 'old_cursor'
        new_cursor = Get-ObjectField $detail 'new_cursor'
        resume_cursor = Get-ObjectField $detail 'resume_cursor'
        cursor_epoch = Get-ObjectField $detail 'cursor_epoch'
        requested_cursor_epoch = Get-ObjectField $detail 'requested_cursor_epoch'
        previous_cursor_epoch = Get-ObjectField $detail 'previous_cursor_epoch'
        sidecar_update_epoch = Get-ObjectField $detail 'sidecar_update_epoch'
        delta_from = $initialCursor
        delta_to = $cursor
        delta_event_count = $collectedEvents.Count
        state_digest = $(if ($CompactResult) { Get-ObjectField $resultDetail 'state_digest' } else { $null })
        delta_schema = $(if ($CompactResult) { $script:TaskDeltaSchema } else { $null })
    })
}
