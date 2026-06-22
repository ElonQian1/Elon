#requires -Version 7.0

function Get-Fb2LivePreflightProperty {
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

function Test-Fb2LivePreflightTruthy {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    return ([string]$Value) -match "^(true|True|1)$"
}

function Get-Fb2LivePreflightRequestState {
    param(
        [object]$GoalGapAudit,
        [object]$UserScenarioAudit,
        [object]$LatestReadOnly,
        [object]$SampleSetState,
        [bool]$TokenPresent
    )

    $currentFlags = Get-Fb2LivePreflightProperty $GoalGapAudit "current_flags"
    $directReadComplete = Test-Fb2LivePreflightTruthy (Get-Fb2LivePreflightProperty $currentFlags "direct_group_chat_read_complete")
    $userScenarioComplete = Test-Fb2LivePreflightTruthy (Get-Fb2LivePreflightProperty $UserScenarioAudit "complete")
    $sampleSetComplete = Test-Fb2LivePreflightTruthy (Get-Fb2LivePreflightProperty $SampleSetState "complete")
    $readyWithoutToken = ($directReadComplete -and $userScenarioComplete -and $sampleSetComplete)
    $missing = @()
    if (-not $directReadComplete) { $missing += "direct_group_chat_read" }
    if (-not $userScenarioComplete) { $missing += "user_scenario_audit" }
    if (-not $sampleSetComplete) { $missing += "context_pack_sample_set" }
    if (-not $TokenPresent) { $missing += "FB2_AI_CENTER_TOKEN" }

    [ordered]@{
        schema = "fb2.main_project.live_preflight_request.v1"
        ready_without_token = $readyWithoutToken
        token_present = $TokenPresent
        blocked_by_external_secret = (-not $TokenPresent)
        missing = @($missing)
        no_write_mode = $true
        writes_visible_group_messages = $false
        evidence_policy = [ordered]@{
            group_chat_test_method = "direct_api_read"
            screenshots_accepted = $false
            required_group_message_fields = @("message_id", "text_len", "text_sha256")
            read_only_summary_schema = "fb2.main_project.visible_chat_readonly.v1"
        }
        target_user = [ordered]@{
            fb2_username = "123qwe"
            fb2_password_placeholder = "<FB2_PASSWORD>"
            external_user_id = "6fe5aa17-0403-427a-8e91-7f414beca35d"
            has_historical_order_context = $userScenarioComplete
        }
        target_group = [ordered]@{
            requested_group_id = "official"
            resolved_group_id = [string](Get-Fb2LivePreflightProperty $LatestReadOnly "group_id" "ext_fb2_official")
            direct_read_sample_message_id = [string](Get-Fb2LivePreflightProperty $LatestReadOnly "sample_message_id" "")
            direct_read_sample_text_sha256 = [string](Get-Fb2LivePreflightProperty $LatestReadOnly "sample_text_sha256" "")
        }
        commands = [ordered]@{
            no_write_direct_read = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD>"
            data_only_preflight = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD> -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>"
            visible_regression_requires_authorization = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD> -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>"
        }
        acceptance_gates = @(
            "fb2_authenticated_readiness_ready_or_partial_for_data_only",
            "context_pack_projection_complete",
            "permission_boundary_403_and_audit_summary",
            "quality_unmatched_cited_sources_zero",
            "feedback_coverage_complete",
            "direct_group_chat_read_text_hash_present",
            "user_scenario_audit_complete"
        )
        note = "no_secret_handoff_for_refreshing_live_context_permission_quality_feedback_after_token_is_available"
    }
}
