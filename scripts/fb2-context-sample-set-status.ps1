#requires -Version 7.0

function Read-Fb2ContextSampleSetJsonOrNull {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return $null
    }

    try {
        Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
    } catch {
        $null
    }
}

function Get-Fb2ContextSampleSetProperty {
    param(
        [object]$Object,
        [string]$Name,
        [object]$Default = $null
    )

    if ($null -eq $Object) {
        return $Default
    }
    if ($Object -is [System.Collections.IDictionary] -and $Object.Contains($Name)) {
        return $Object[$Name]
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $Default
    }
    return $property.Value
}

function Get-Fb2ContextSampleSetState {
    param([string]$Path)

    $summary = Read-Fb2ContextSampleSetJsonOrNull -Path $Path
    if ($null -eq $summary) {
        return [ordered]@{
            path = [string]$Path
            exists = $false
            success = $false
            complete = $false
            schema = ""
            samples_dir = ""
            scenario_count = 0
            passed_count = 0
            failed_count = 0
            scenario_ids = @()
            audit_ids = @()
            source_kinds = @()
            missing = @("context_pack_sample_set_validation")
            secret_like_scenarios = @()
        }
    }

    $scenarioIds = @()
    $auditIds = @()
    $sourceKinds = @()
    $scenarioStates = @()
    foreach ($scenario in @($summary.scenarios)) {
        $id = [string](Get-Fb2ContextSampleSetProperty $scenario "scenario" "")
        if (-not [string]::IsNullOrWhiteSpace($id)) {
            $scenarioIds += $id
        }
        $auditId = [string](Get-Fb2ContextSampleSetProperty $scenario "context_audit_id" "")
        if (-not [string]::IsNullOrWhiteSpace($auditId)) {
            $auditIds += $auditId
        }
        $scenarioSourceKinds = @((Get-Fb2ContextSampleSetProperty $scenario "source_kinds" @()) | ForEach-Object { [string]$_ })
        $sourceKinds += $scenarioSourceKinds
        $scenarioStates += [ordered]@{
            scenario = $id
            passed = [bool](Get-Fb2ContextSampleSetProperty $scenario "passed" $false)
            context_audit_id = $auditId
            citation_source_count = [int](Get-Fb2ContextSampleSetProperty $scenario "citation_source_count" 0)
            source_kinds = @($scenarioSourceKinds | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
            context_pack_chars = [int](Get-Fb2ContextSampleSetProperty $scenario "context_pack_chars" 0)
            context_pack_sha256 = [string](Get-Fb2ContextSampleSetProperty $scenario "context_pack_sha256" "")
        }
    }

    $expectedIds = @(
        "today_matches_context_pack",
        "my_ticket_context_pack",
        "platform_order_context_pack",
        "group_opinion_context_pack"
    )
    $missing = @((Get-Fb2ContextSampleSetProperty $summary "missing" @()))
    foreach ($expected in $expectedIds) {
        if ($scenarioIds -notcontains $expected) {
            $missing += "scenario:$expected"
        }
    }
    if ([string](Get-Fb2ContextSampleSetProperty $summary "schema" "") -ne "fb2.main_project.context_pack_sample_set_validation.v1") {
        $missing += "schema"
    }

    $summaryComplete = ([bool](Get-Fb2ContextSampleSetProperty $summary "complete" $false) -and $missing.Count -eq 0)
    $successRaw = Get-Fb2ContextSampleSetProperty $summary "success" $null
    $summarySuccess = if ($null -eq $successRaw) { $summaryComplete } else { [bool]$successRaw }

    [ordered]@{
        path = [string]$Path
        exists = $true
        success = ($summarySuccess -and $missing.Count -eq 0)
        complete = $summaryComplete
        schema = [string](Get-Fb2ContextSampleSetProperty $summary "schema" "")
        samples_dir = [string](Get-Fb2ContextSampleSetProperty $summary "samples_dir" "")
        scenario_count = [int](Get-Fb2ContextSampleSetProperty $summary "scenario_count" 0)
        passed_count = [int](Get-Fb2ContextSampleSetProperty $summary "passed_count" 0)
        failed_count = [int](Get-Fb2ContextSampleSetProperty $summary "failed_count" 0)
        scenario_ids = @($scenarioIds)
        audit_ids = @($auditIds | Select-Object -Unique)
        source_kinds = @($sourceKinds | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
        scenarios = @($scenarioStates)
        missing = @($missing)
        secret_like_scenarios = @((Get-Fb2ContextSampleSetProperty $summary "secret_like_scenarios" @()))
    }
}
