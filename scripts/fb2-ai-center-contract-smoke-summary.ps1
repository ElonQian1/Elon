#requires -Version 7.0

function Get-Fb2ContractSmokeSummaryProperty {
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

function Test-Fb2ContractSmokeSummaryTruthy {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    return ([string]$Value) -match "^(true|True|1)$"
}

function ConvertTo-Fb2ContractSmokeSummaryText {
    param([object]$Value)

    if ($null -eq $Value) {
        return ""
    }
    return [string]$Value
}

function Test-Fb2ContractSmokeCheck {
    param(
        [object[]]$Checks,
        [string]$Name,
        [string]$Status = "OK"
    )

    foreach ($check in @($Checks)) {
        $checkName = ConvertTo-Fb2ContractSmokeSummaryText (Get-Fb2ContractSmokeSummaryProperty $check "name")
        $checkStatus = ConvertTo-Fb2ContractSmokeSummaryText (Get-Fb2ContractSmokeSummaryProperty $check "status")
        if ($checkName -eq $Name -and $checkStatus -eq $Status) {
            return $true
        }
    }
    return $false
}

function Test-Fb2ContractSmokeOkAll {
    param(
        [object[]]$Checks,
        [string[]]$Names
    )

    foreach ($name in @($Names)) {
        if (-not (Test-Fb2ContractSmokeCheck -Checks $Checks -Name $name -Status "OK")) {
            return $false
        }
    }
    return $true
}

function Get-Fb2ContractSmokeFailedOrSkipped {
    param(
        [object[]]$Checks,
        [string]$Status
    )

    @($Checks | Where-Object {
        (ConvertTo-Fb2ContractSmokeSummaryText (Get-Fb2ContractSmokeSummaryProperty $_ "status")) -eq $Status
    } | ForEach-Object {
        [ordered]@{
            name = ConvertTo-Fb2ContractSmokeSummaryText (Get-Fb2ContractSmokeSummaryProperty $_ "name")
            detail = ConvertTo-Fb2ContractSmokeSummaryText (Get-Fb2ContractSmokeSummaryProperty $_ "detail")
        }
    })
}

function New-Fb2ContractSmokeSummary {
    param(
        [object[]]$Checks,
        [int]$FailedCount,
        [int]$SkippedCount,
        [string]$MainBase,
        [string]$Fb2Base,
        [string]$GroupId,
        [string]$ExternalUserId,
        [bool]$Fb2TokenPresent,
        [bool]$RequireFb2Live,
        [bool]$RequireNoSkips,
        [bool]$SkipVoiceContractChecks
    )

    $checks = @($Checks)
    $chatBootstrapReady = Test-Fb2ContractSmokeOkAll -Checks $checks -Names @(
        "chat-bootstrap aiReply",
        "chat-bootstrap billing",
        "chat-bootstrap voice composer"
    )
    $voiceContractReady = $SkipVoiceContractChecks -or (Test-Fb2ContractSmokeOkAll -Checks $checks -Names @(
        "chat-bootstrap ASR free billing",
        "chat-bootstrap TTS free billing",
        "chat-bootstrap before ASR gate",
        "chat-bootstrap before TTS gate"
    ))
    $aiBillingPolicyReady = Test-Fb2ContractSmokeOkAll -Checks $checks -Names @(
        "chat-bootstrap experience AI billable",
        "chat-bootstrap before AI reply gate",
        "chat-bootstrap AI reply keeps context fetch free"
    )
    $liveManifestReady = Test-Fb2ContractSmokeCheck -Checks $checks -Name "live manifest ready" -Status "OK"
    $domainContractReady = Test-Fb2ContractSmokeOkAll -Checks $checks -Names @(
        "context pack template schema",
        "domain data blueprint schema",
        "domain context index schema",
        "group chat evidence schema"
    )
    $dynamicDiscoveryReady = Test-Fb2ContractSmokeOkAll -Checks $checks -Names @(
        "fb2 integration discovery",
        "fb2 integration routing mode",
        "fb2 integration token header"
    )
    $protectedBoundaryReady = (Test-Fb2ContractSmokeOkAll -Checks $checks -Names @(
        "fb2 readiness requires service token",
        "fb2 tool manifest requires service token"
    )) -or (Test-Fb2ContractSmokeOkAll -Checks $checks -Names @(
        "fb2 authenticated readiness",
        "fb2 authenticated tool manifest"
    ))
    $liveDataReady = (Test-Fb2ContractSmokeCheck -Checks $checks -Name "fb2 tool manifest" -Status "OK") -or
        (Test-Fb2ContractSmokeCheck -Checks $checks -Name "fb2 live data" -Status "SKIP" -and -not $RequireFb2Live)
    $fb2LiveDataStatus = if (Test-Fb2ContractSmokeCheck -Checks $checks -Name "fb2 tool manifest" -Status "OK") {
        "verified"
    } elseif (Test-Fb2ContractSmokeCheck -Checks $checks -Name "fb2 live data" -Status "SKIP") {
        "skipped_missing_FB2_AI_CENTER_TOKEN"
    } else {
        "not_verified"
    }

    $missing = @()
    if ($FailedCount -ne 0) { $missing += "failed_checks" }
    if ($RequireNoSkips -and $SkippedCount -ne 0) { $missing += "skipped_checks_not_allowed" }
    if (-not $chatBootstrapReady) { $missing += "chat_bootstrap_contract" }
    if (-not $voiceContractReady) { $missing += "voice_contract" }
    if (-not $aiBillingPolicyReady) { $missing += "ai_billing_policy" }
    if (-not $liveManifestReady) { $missing += "live_manifest" }
    if (-not $domainContractReady) { $missing += "domain_contract" }
    if (-not $dynamicDiscoveryReady) { $missing += "fb2_dynamic_discovery" }
    if (-not $protectedBoundaryReady) { $missing += "fb2_service_token_boundary" }
    if (-not $liveDataReady) { $missing += "fb2_live_data" }

    [ordered]@{
        schema = "fb2.main_project.contract_smoke_summary.v1"
        generated_at = (Get-Date).ToUniversalTime().ToString("o")
        main_base = $MainBase
        fb2_base = $Fb2Base
        group_id = $GroupId
        external_user_id = $ExternalUserId
        fb2_ai_center_token_present = $Fb2TokenPresent
        require_fb2_live = $RequireFb2Live
        require_no_skips = $RequireNoSkips
        skip_voice_contract_checks = $SkipVoiceContractChecks
        success = ($FailedCount -eq 0)
        complete = (@($missing).Count -eq 0)
        failed_count = $FailedCount
        skipped_count = $SkippedCount
        check_count = @($checks).Count
        gates = [ordered]@{
            chat_bootstrap_ready = $chatBootstrapReady
            voice_contract_ready = $voiceContractReady
            ai_billing_policy_ready = $aiBillingPolicyReady
            live_manifest_ready = $liveManifestReady
            domain_contract_ready = $domainContractReady
            dynamic_discovery_ready = $dynamicDiscoveryReady
            protected_service_token_boundary_ready = $protectedBoundaryReady
            fb2_live_data_status = $fb2LiveDataStatus
        }
        failed_checks = @(Get-Fb2ContractSmokeFailedOrSkipped -Checks $checks -Status "FAIL")
        skipped_checks = @(Get-Fb2ContractSmokeFailedOrSkipped -Checks $checks -Status "SKIP")
        missing = @($missing | Select-Object -Unique)
    }
}

function Write-Fb2ContractSmokeSummary {
    param(
        [object]$Summary,
        [string]$OutputPath
    )

    if ([string]::IsNullOrWhiteSpace($OutputPath)) {
        return
    }
    $dir = Split-Path -Parent $OutputPath
    if ($dir -and -not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    $Summary | ConvertTo-Json -Depth 8 | Set-Content -Path $OutputPath -Encoding UTF8
}

function Read-Fb2ContractSmokeSummaryJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    try {
        Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    } catch {
        $null
    }
}

function Get-Fb2ContractSmokeSummaryState {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return [ordered]@{
            path = ""
            exists = $false
            complete = $false
            schema = ""
            success = $false
            failed_count = 0
            skipped_count = 0
            gates = [ordered]@{}
            missing = @("contract_smoke_summary")
        }
    }

    $summary = Read-Fb2ContractSmokeSummaryJson -Path $Path
    if ($null -eq $summary) {
        return [ordered]@{
            path = $Path
            exists = $true
            complete = $false
            schema = ""
            success = $false
            failed_count = 0
            skipped_count = 0
            gates = [ordered]@{}
            missing = @("contract_smoke_summary_parse_error")
        }
    }

    $missing = @((Get-Fb2ContractSmokeSummaryProperty $summary "missing" @()))
    [ordered]@{
        path = $Path
        exists = $true
        complete = Test-Fb2ContractSmokeSummaryTruthy (Get-Fb2ContractSmokeSummaryProperty $summary "complete")
        schema = ConvertTo-Fb2ContractSmokeSummaryText (Get-Fb2ContractSmokeSummaryProperty $summary "schema")
        success = Test-Fb2ContractSmokeSummaryTruthy (Get-Fb2ContractSmokeSummaryProperty $summary "success")
        main_base = ConvertTo-Fb2ContractSmokeSummaryText (Get-Fb2ContractSmokeSummaryProperty $summary "main_base")
        fb2_base = ConvertTo-Fb2ContractSmokeSummaryText (Get-Fb2ContractSmokeSummaryProperty $summary "fb2_base")
        group_id = ConvertTo-Fb2ContractSmokeSummaryText (Get-Fb2ContractSmokeSummaryProperty $summary "group_id")
        external_user_id = ConvertTo-Fb2ContractSmokeSummaryText (Get-Fb2ContractSmokeSummaryProperty $summary "external_user_id")
        fb2_ai_center_token_present = Test-Fb2ContractSmokeSummaryTruthy (Get-Fb2ContractSmokeSummaryProperty $summary "fb2_ai_center_token_present")
        require_fb2_live = Test-Fb2ContractSmokeSummaryTruthy (Get-Fb2ContractSmokeSummaryProperty $summary "require_fb2_live")
        failed_count = [int](Get-Fb2ContractSmokeSummaryProperty $summary "failed_count" 0)
        skipped_count = [int](Get-Fb2ContractSmokeSummaryProperty $summary "skipped_count" 0)
        check_count = [int](Get-Fb2ContractSmokeSummaryProperty $summary "check_count" 0)
        gates = Get-Fb2ContractSmokeSummaryProperty $summary "gates" ([ordered]@{})
        failed_checks = @((Get-Fb2ContractSmokeSummaryProperty $summary "failed_checks" @()))
        skipped_checks = @((Get-Fb2ContractSmokeSummaryProperty $summary "skipped_checks" @()))
        missing = @($missing | Select-Object -Unique)
    }
}
