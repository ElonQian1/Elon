#requires -Version 7.0

function Read-Fb2ContextSampleRequestJsonOrNull {
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

function Get-Fb2ContextSampleRequestProperty {
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

function Get-Fb2ContextSampleRequestState {
    param([string]$Path)

    $request = Read-Fb2ContextSampleRequestJsonOrNull -Path $Path
    if ($null -eq $request) {
        return [ordered]@{
            path = [string]$Path
            exists = $false
            complete = $false
            schema = ""
            scenario_count = 0
            scenario_ids = @()
            missing = @("context_pack_sample_request")
            validation_commands = @()
            contains_secret_like_text = $false
        }
    }

    $scenarioIds = @()
    $validationCommands = @()
    $missing = @()
    $expectedIds = @(
        "today_matches_context_pack",
        "my_ticket_context_pack",
        "platform_order_context_pack",
        "group_opinion_context_pack"
    )

    foreach ($scenario in @($request.scenarios)) {
        $id = [string](Get-Fb2ContextSampleRequestProperty $scenario "id" "")
        if (-not [string]::IsNullOrWhiteSpace($id)) {
            $scenarioIds += $id
        }
        $command = [string](Get-Fb2ContextSampleRequestProperty $scenario "validate_command" "")
        if (-not [string]::IsNullOrWhiteSpace($command)) {
            $validationCommands += $command
        }
        if ([string]::IsNullOrWhiteSpace([string](Get-Fb2ContextSampleRequestProperty $scenario "save_as" ""))) {
            $missing += "$id.save_as"
        }
        if (@(Get-Fb2ContextSampleRequestProperty $scenario "expected_source_kinds" @()).Count -eq 0) {
            $missing += "$id.expected_source_kinds"
        }
    }

    foreach ($expected in $expectedIds) {
        if ($scenarioIds -notcontains $expected) {
            $missing += "scenario:$expected"
        }
    }
    if ([string](Get-Fb2ContextSampleRequestProperty $request "schema" "") -ne "fb2.main_project.context_pack_sample_request.v1") {
        $missing += "schema"
    }
    if (@(Get-Fb2ContextSampleRequestProperty $request "redaction_rules" @()).Count -eq 0) {
        $missing += "redaction_rules"
    }

    $raw = try {
        Get-Content -Raw -LiteralPath $Path
    } catch {
        ""
    }
    $containsSecretLikeText = $raw -match "(Bearer\s+[A-Za-z0-9._-]+|sk-[A-Za-z0-9]|FB2_AI_CENTER_TOKEN\s*=|123qwe)"

    [ordered]@{
        path = [string]$Path
        exists = $true
        complete = ($missing.Count -eq 0 -and -not $containsSecretLikeText)
        schema = [string](Get-Fb2ContextSampleRequestProperty $request "schema" "")
        scenario_count = @($request.scenarios).Count
        scenario_ids = @($scenarioIds)
        missing = @($missing)
        validation_commands = @($validationCommands)
        contains_secret_like_text = $containsSecretLikeText
    }
}
