#requires -Version 7.0

function Read-Fb2ContextProjectionLogLines {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return @()
    }
    @(Get-Content -LiteralPath $Path -ErrorAction SilentlyContinue)
}

function Test-Fb2ContextProjectionLogCheck {
    param(
        [string[]]$Lines,
        [string]$CheckName
    )

    foreach ($line in @($Lines)) {
        if ($line -like ("OK`t{0}*" -f $CheckName)) {
            return $true
        }
    }
    return $false
}

function Get-Fb2ContextProjectionPackState {
    param(
        [string[]]$Lines,
        [string]$Scenario,
        [string[]]$ExpectedSourceKinds
    )

    $requiredChecks = @(
        "context projection body: $Scenario",
        "context projection wrapper open: $Scenario",
        "context projection wrapper close: $Scenario",
        "context projection audit id: $Scenario",
        "context projection source registry: $Scenario"
    )
    $requiredSections = @(
        "usage_boundary",
        "match_facts",
        "user_order_slice",
        "platform_order_summary",
        "group_opinion_slice",
        "retrieval_evidence",
        "quality_feedback"
    )

    $missing = @()
    foreach ($check in $requiredChecks) {
        if (-not (Test-Fb2ContextProjectionLogCheck -Lines $Lines -CheckName $check)) {
            $missing += $check
        }
    }
    foreach ($section in $requiredSections) {
        $check = "context projection section: $Scenario/$section"
        if (-not (Test-Fb2ContextProjectionLogCheck -Lines $Lines -CheckName $check)) {
            $missing += $check
        }
    }
    foreach ($kind in $ExpectedSourceKinds) {
        $check = "context projection source kind: $Scenario/$kind"
        if (-not (Test-Fb2ContextProjectionLogCheck -Lines $Lines -CheckName $check)) {
            $missing += $check
        }
    }

    [ordered]@{
        scenario = $Scenario
        complete = ($missing.Count -eq 0)
        expected_source_kinds = @($ExpectedSourceKinds)
        missing = $missing
    }
}

function Get-Fb2ContextProjectionLogState {
    param([string]$Path)

    $lines = @(Read-Fb2ContextProjectionLogLines -Path $Path)
    if ($lines.Count -eq 0) {
        return [ordered]@{
            path = [string]$Path
            exists = $false
            complete = $false
            missing = @("ai_center_log")
            today_matches_context_pack = [ordered]@{ complete = $false; missing = @("ai_center_log") }
            my_ticket_context_pack = [ordered]@{ complete = $false; missing = @("ai_center_log") }
            business_data_checks = [ordered]@{
                group_opinion_summary = $false
                platform_order_summary = $false
                quality_unmatched_sources_zero = $false
                non_synthetic_opinion_adoption = $false
            }
        }
    }

    $today = Get-Fb2ContextProjectionPackState -Lines $lines -Scenario "today matches context pack" -ExpectedSourceKinds @("match", "odds", "context_audit")
    $ticket = Get-Fb2ContextProjectionPackState -Lines $lines -Scenario "my ticket context pack" -ExpectedSourceKinds @("user_order", "ticket", "context_audit")
    $checks = [ordered]@{
        group_opinion_summary = Test-Fb2ContextProjectionLogCheck -Lines $lines -CheckName "scenario: group opinions has summary data"
        platform_order_summary = Test-Fb2ContextProjectionLogCheck -Lines $lines -CheckName "scenario: platform order has summary data"
        quality_unmatched_sources_zero = Test-Fb2ContextProjectionLogCheck -Lines $lines -CheckName "quality unmatched cited sources"
        non_synthetic_opinion_adoption = Test-Fb2ContextProjectionLogCheck -Lines $lines -CheckName "quality non-synthetic adoption count"
    }

    $missing = @()
    foreach ($item in @($today.missing)) {
        $missing += $item
    }
    foreach ($item in @($ticket.missing)) {
        $missing += $item
    }
    foreach ($name in @($checks.Keys)) {
        if (-not [bool]$checks[$name]) {
            $missing += $name
        }
    }

    [ordered]@{
        path = [string]$Path
        exists = $true
        complete = ($missing.Count -eq 0)
        missing = $missing
        today_matches_context_pack = $today
        my_ticket_context_pack = $ticket
        business_data_checks = $checks
    }
}
