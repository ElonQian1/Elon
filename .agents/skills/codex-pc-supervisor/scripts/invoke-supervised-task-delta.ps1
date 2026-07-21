function Get-ObjectDigest {
    param([object]$Value)
    $json = $Value | ConvertTo-Json -Depth 20 -Compress
    [byte[]]$bytes = $script:Utf8NoBomStrict.GetBytes($json)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { [byte[]]$digest = $sha.ComputeHash($bytes) } finally { $sha.Dispose() }
    return -join ($digest | ForEach-Object { $_.ToString('x2') })
}

function Assert-NodeSupervisionCapability {
    param([object]$Connection, [string]$Capability, [string]$Operation)
    if ($Connection.SupervisionProtocol -ne $script:SupervisionProtocol -or
        @($Connection.SupervisionCapabilities) -notcontains $Capability) {
        throw "$Operation requires node capability '$Capability' under $($script:SupervisionProtocol). Update/restart the Yilong node before retrying; legacy fallback is disabled to avoid silent context or event loss."
    }
}

function Assert-NodeSupervisionAnyCapability {
    param([object]$Connection, [string[]]$Capabilities, [string]$Operation)
    if ($Connection.SupervisionProtocol -ne $script:SupervisionProtocol) {
        throw "$Operation requires $($script:SupervisionProtocol); the node advertises '$($Connection.SupervisionProtocol)'."
    }
    foreach ($capability in $Capabilities) {
        if (@($Connection.SupervisionCapabilities) -contains $capability) { return $capability }
    }
    throw "$Operation requires one of [$($Capabilities -join ', ')]. Update/restart the Yilong node before retrying; undeclared legacy fallback is disabled."
}

function Merge-TaskDeltaEvents {
    param(
        [System.Collections.Generic.List[object]]$CollectedEvents,
        [hashtable]$SeenEvents,
        [object]$Detail
    )
    $reset = (Get-ObjectField $Detail 'cursor_reset') -eq $true
    if ($reset) {
        $CollectedEvents.Clear()
        $SeenEvents.Clear()
    }
    $epoch = [string](Get-ObjectField $Detail 'cursor_epoch')
    if ([string]::IsNullOrWhiteSpace($epoch)) {
        throw 'delta_wait_v1 response is missing cursor_epoch; refusing a lossy merge.'
    }
    foreach ($eventView in @((Get-ObjectField $Detail 'events'))) {
        $seq = [int](Get-ObjectField $eventView 'seq')
        if ($seq -le 0) {
            throw 'delta_wait_v1 response contains an event without a positive seq.'
        }
        $eventKey = "${epoch}:$seq"
        if (-not $SeenEvents.ContainsKey($eventKey)) {
            $SeenEvents[$eventKey] = $true
            $CollectedEvents.Add($eventView)
        }
    }
    return $reset
}

function Convert-ToCompactTaskDetail {
    param([object]$Detail, [object[]]$EventViews = $null, [bool]$TerminalSnapshot = $false)
    $record = Get-RecordFromDetail $Detail
    $supervision = Get-ObjectField $Detail 'supervision'
    $evidence = Get-ObjectField $supervision 'evidence'
    $runtime = Get-ObjectField $Detail 'runtime'
    $sourceEvents = if ($null -eq $EventViews) { @(Get-ObjectField $Detail 'events') } else { @($EventViews) }
    $events = @($sourceEvents | ForEach-Object {
        $event = Get-ObjectField $_ 'event'
        $item = Get-ObjectField $event 'item'
        [ordered]@{
            seq = Get-ObjectField $_ 'seq'; type = Get-ObjectField $event 'type'
            phase = Get-ObjectField $event 'phase'; lifecycle = Get-ObjectField $event 'lifecycle'
            item_type = Get-ObjectField $item 'type'; status = Get-ObjectField $item 'status'
            exit_code = Get-ObjectField $item 'exit_code'; command = Get-ObjectField $item 'command'
        }
    })
    $evidenceTotals = [ordered]@{
        event_count = Get-ObjectField $evidence 'event_count'
        tool_calls = Get-ObjectField $evidence 'tool_calls'
        tool_results = Get-ObjectField $evidence 'tool_results'
        failed_tools = Get-ObjectField $evidence 'failed_tools'
        file_change_events = Get-ObjectField $evidence 'file_change_events'
        agent_messages = Get-ObjectField $evidence 'agent_messages'
        terminal_event_seen = Get-ObjectField $evidence 'terminal_event_seen'
    }
    $terminalEvidence = if ($TerminalSnapshot) { [ordered]@{
        changed_files = @(Get-ObjectField $evidence 'changed_files')
        command_exit_codes = @(Get-ObjectField $evidence 'command_exit_codes')
        failure_summaries = @(Get-ObjectField $evidence 'failure_summaries')
    } } else { $null }
    return [ordered]@{
        record = [ordered]@{
            task_id = Get-ObjectField $record 'task_id'; status = Get-ObjectField $record 'status'
            error = Get-ObjectField $record 'error'; finished_at_ms = Get-ObjectField $record 'finished_at_ms'
        }
        runtime = [ordered]@{
            phase = Get-ObjectField $runtime 'phase'; current_command = Get-ObjectField $runtime 'current_command'
            last_progress = Get-ObjectField $runtime 'last_progress'; heartbeat = Get-ObjectField $runtime 'heartbeat'
            idle_duration = Get-ObjectField $runtime 'idle_duration'; timeout_policy = Get-ObjectField $runtime 'timeout_policy'
        }
        approval_state = Get-ObjectField $Detail 'approval_state'
        evidence_totals = $evidenceTotals
        evidence_digest = Get-ObjectDigest $evidence
        terminal_evidence = $terminalEvidence
        events = $events
        last_event_seq = Get-ObjectField $Detail 'last_event_seq'
        has_more = Get-ObjectField $Detail 'has_more'
        state_digest = Get-ObjectDigest ([ordered]@{
            status = Get-ObjectField $record 'status'; error = Get-ObjectField $record 'error'
            runtime = $runtime; approval_state = Get-ObjectField $Detail 'approval_state'
        })
    }
}

function Select-TaskDeltaChanges {
    param(
        [System.Collections.IDictionary]$Compact,
        [string]$ExpectedStateDigest = '',
        [string]$ExpectedEvidenceDigest = ''
    )
    $stateChanged = [string]::IsNullOrWhiteSpace($ExpectedStateDigest) -or
        $ExpectedStateDigest -cne [string]$Compact.state_digest
    $evidenceChanged = [string]::IsNullOrWhiteSpace($ExpectedEvidenceDigest) -or
        $ExpectedEvidenceDigest -cne [string]$Compact.evidence_digest
    if (-not $stateChanged) {
        $Compact.record = $null
        $Compact.runtime = $null
        $Compact.approval_state = $null
    }
    if (-not $evidenceChanged) {
        $Compact.evidence_totals = $null
    }
    $Compact['state_changed'] = $stateChanged
    $Compact['evidence_changed'] = $evidenceChanged
    return $Compact
}
