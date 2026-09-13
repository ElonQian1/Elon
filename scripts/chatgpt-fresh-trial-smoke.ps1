#requires -Version 5.1

function Invoke-ChatGptFreshTrial {
    param([Parameter(Mandatory)]$Runtime, [ValidateSet('start','state','end')][string]$Mode = 'state')
    $result = Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action chatgpt_private_protocol_probe `
        -Arguments @{mode="fresh_text_trial_$Mode"}
    $id = [string]$result.command_receipt.request_id
    if (!$id) { throw 'fresh_trial_receipt_missing' }
    $until = [DateTimeOffset]::UtcNow.AddSeconds(12)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool ui_state
        $receipt = @($state.command_requests | Where-Object { $_.request_id -eq $id }) | Select-Object -Last 1
        if ($receipt.result.detail) {
            try { $value = [string]$receipt.result.detail | ConvertFrom-Json } catch { throw 'fresh_trial_receipt_invalid' }
            if ($receipt.status -ne 'succeeded' -or $receipt.result.ok -ne $true -or
                $value.schema -cne 'elon.fresh_text_trial.v1') { throw 'fresh_trial_receipt_invalid' }
            return $value
        }
        Start-Sleep -Milliseconds 200
    } while ([DateTimeOffset]::UtcNow -lt $until)
    throw 'fresh_trial_receipt_timeout'
}

function Test-ChatGptFreshRetryEvidence {
    param([AllowNull()]$Before, [AllowNull()]$After, [AllowNull()]$Receipt)
    foreach ($value in @($Before.armed, $Before.pending, $After.dispatched, $After.accepted, $After.reconciled, $After.pending)) {
        if ($value -isnot [bool]) { return $false }
    }
    foreach ($value in @($Before.attempts, $After.attempts, $After.stream_events)) {
        if (($value -isnot [int] -and $value -isnot [long]) -or $value -lt 0) { return $false }
    }
    if (($After.version -isnot [int] -and $After.version -isnot [long]) -or $After.version -notin @(6, 7)) { return $false }
    if ($Before.schema -cne 'elon.fresh_text_trial.v1' -or $After.schema -cne 'elon.fresh_text_trial.v1' -or
        $Before.armed -ne $true -or $Before.pending -ne $false -or
        $After.operation -cne 'regenerate' -or
        $null -eq $Before.attempts -or $After.attempts -ne ($Before.attempts + 1) -or
        $After.dispatched -ne $true -or $After.accepted -ne $true -or $After.reconciled -ne $true -or
        $After.pending -ne $false -or $After.parent_role -cne 'user' -or $After.stream_events -lt 1) { return $false }
    return $Receipt.expected_web_action -ceq 'regenerate_response' -and $Receipt.status -ceq 'succeeded' -and
        $Receipt.result.ok -eq $true -and $Receipt.result.detail -ceq 'private_text_v1:regenerate_accepted'
}

function Close-ChatGptFreshRetryTrial {
    param([Parameter(Mandatory)]$Runtime, [AllowNull()]$Baseline,
        [bool]$TrialRequested, [AllowEmptyString()][string]$Draft)
    try {
        if ($TrialRequested) {
            $end = Invoke-ChatGptFreshTrial -Runtime $Runtime -Mode end
            if ($end.pending -isnot [bool] -or $end.pending -or
                $end.armed -isnot [bool] -or $end.armed) { return $false }
        }
        if (!$Draft) { return $true }
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool ui_state
        if ($null -eq $Baseline.page_generation -or !$Baseline.conversation.url -or
            $state.page_generation -ne $Baseline.page_generation -or
            [string]$state.conversation.url -cne [string]$Baseline.conversation.url -or
            $state.surface -cne 'chatgpt_web' -or $state.streaming -isnot [bool] -or $state.streaming) { return $false }
        if ($state.input.text_length -eq 0 -and [string]$state.input.text -ceq '') { return $true }
        # Only remove this fixture, never a draft edited by the user during the run.
        if ($state.input.text_length -ne $Draft.Length -or [string]$state.input.text -cne $Draft) { return $false }
        Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action set_input_text -Arguments @{text=''} | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $Runtime -TimeoutSec 5 -RequireChatGptForeground `
            -Description 'retry fixture draft cleanup' -Predicate {
                param($s)
                $s.page_generation -eq $Baseline.page_generation -and
                [string]$s.conversation.url -ceq [string]$Baseline.conversation.url -and
                $s.input.text_length -eq 0 -and [string]$s.input.text -ceq ''
            }.GetNewClosure() | Out-Null
        return $true
    } catch {
        Write-Warning 'Fresh retry cleanup unconfirmed; no navigation or replay will follow.'
        return $false
    }
}
