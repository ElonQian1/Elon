#requires -Version 7.0

param(
    [string]$OutputDir = "",
    [string[]]$EvidenceDirs = @(),
    [string]$MainWorkspaceEvidenceDir = "",
    [string]$Fb2RepoPath = "",
    [string]$RefreshSummaryPath = "",
    [string]$HandoffPromptPath = "",
    [switch]$SkipPublicContract,
    [switch]$SkipExportedContextPackSampleValidation,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2RefreshRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2RefreshPath {
    param(
        [string]$Path,
        [string]$Root
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $Root $Path)
}

function Add-Fb2RefreshDirectory {
    param(
        [System.Collections.ArrayList]$Target,
        [hashtable]$Seen,
        [string]$Directory,
        [string]$Root,
        [switch]$RequireExists
    )

    $path = Resolve-Fb2RefreshPath -Path $Directory -Root $Root
    if ([string]::IsNullOrWhiteSpace($path)) {
        return
    }
    if ($RequireExists -and -not (Test-Path -LiteralPath $path)) {
        return
    }
    try {
        $fullPath = [System.IO.Path]::GetFullPath($path)
    } catch {
        $fullPath = $path
    }
    $key = $fullPath.ToLowerInvariant()
    if (-not $Seen.ContainsKey($key)) {
        $Seen[$key] = $true
        [void]$Target.Add($fullPath)
    }
}

function Read-Fb2RefreshJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Resolve-Fb2RefreshTokenBridgeResultPath {
    param(
        [string]$CanonicalPath,
        [string]$FallbackPath
    )

    $canonicalExists = (-not [string]::IsNullOrWhiteSpace($CanonicalPath) -and (Test-Path -LiteralPath $CanonicalPath))
    $fallbackExists = (-not [string]::IsNullOrWhiteSpace($FallbackPath) -and (Test-Path -LiteralPath $FallbackPath))

    if ($canonicalExists -and $fallbackExists) {
        $canonical = Read-Fb2RefreshJson -Path $CanonicalPath
        $fallback = Read-Fb2RefreshJson -Path $FallbackPath
        $canonicalGenerated = ConvertTo-Fb2RefreshDate -Value (Get-Fb2RefreshProperty $canonical "generated_at_utc" $null)
        $fallbackGenerated = ConvertTo-Fb2RefreshDate -Value (Get-Fb2RefreshProperty $fallback "generated_at_utc" $null)
        if ($null -ne $fallbackGenerated -and ($null -eq $canonicalGenerated -or $fallbackGenerated -gt $canonicalGenerated)) {
            return [ordered]@{
                path = $FallbackPath
                source = "custom_output_path_fallback"
            }
        }
    }

    if ($canonicalExists) {
        return [ordered]@{
            path = $CanonicalPath
            source = "canonical_token_bridge_result"
        }
    }
    if ($fallbackExists) {
        return [ordered]@{
            path = $FallbackPath
            source = "custom_output_path_fallback"
        }
    }
    return [ordered]@{
        path = $CanonicalPath
        source = "missing_canonical_token_bridge_result"
    }
}

function ConvertTo-Fb2RefreshDate {
    param([object]$Value)

    if ($null -eq $Value) {
        return $null
    }
    if ($Value -is [DateTimeOffset]) {
        return $Value.ToUniversalTime()
    }
    if ($Value -is [DateTime]) {
        $dateTime = [DateTime]$Value
        if ($dateTime.Kind -eq [DateTimeKind]::Unspecified) {
            $dateTime = [DateTime]::SpecifyKind($dateTime, [DateTimeKind]::Utc)
        }
        return ([DateTimeOffset]$dateTime).ToUniversalTime()
    }
    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) {
        return $null
    }
    try {
        $styles = [System.Globalization.DateTimeStyles]::AssumeUniversal -bor [System.Globalization.DateTimeStyles]::AdjustToUniversal
        return [DateTimeOffset]::Parse($text, [System.Globalization.CultureInfo]::InvariantCulture, $styles).ToUniversalTime()
    } catch {
        return $null
    }
}

function Get-Fb2RefreshProperty {
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

function Get-Fb2RefreshCommandValue {
    param(
        [object]$Primary,
        [object]$Fallback,
        [string]$Name
    )

    $value = [string](Get-Fb2RefreshProperty $Primary $Name "")
    if (-not [string]::IsNullOrWhiteSpace($value)) {
        return $value
    }
    return [string](Get-Fb2RefreshProperty $Fallback $Name "")
}

function Test-Fb2RefreshPublicCheckPassed {
    param(
        [object]$PublicStatus,
        [string]$Id
    )

    foreach ($check in @((Get-Fb2RefreshProperty $PublicStatus "checks" @()))) {
        if ([string](Get-Fb2RefreshProperty $check "id" "") -eq $Id) {
            return [bool](Get-Fb2RefreshProperty $check "passed" $false)
        }
    }
    return $false
}

function New-Fb2RefreshAnswerSourceValidationState {
    param([object]$PublicStatus)

    $schemaReady = Test-Fb2RefreshPublicCheckPassed `
        -PublicStatus $PublicStatus `
        -Id "tool_result_answer_source_validation_schema"
    $toolSourcesReady = Test-Fb2RefreshPublicCheckPassed `
        -PublicStatus $PublicStatus `
        -Id "tool_result_answer_source_validation_tool_sources"

    [ordered]@{
        schema = "fb2.main_project.answer_source_validation_status.v1"
        ready = ($schemaReady -and $toolSourcesReady)
        schema_check = [bool]$schemaReady
        tool_sources_check = [bool]$toolSourcesReady
        feedback_payload_schema = "external_app.answer_source_validation.v1"
        evidence_source = "public_contract_status.checks"
        note = "Tracks whether generated-answer feedback exposes candidate/matched/unmatched source ids and matched_tool_source_ids without changing fb2 cited_sources semantics."
    }
}

function New-Fb2RefreshTokenBridgeLivePreflight {
    param(
        [string]$ResultPath,
        [string]$SummaryPath,
        [string]$ResultSource = "canonical_token_bridge_result",
        [double]$MaxAgeMinutes = 120
    )

    $result = Read-Fb2RefreshJson -Path $ResultPath
    $summaryExists = -not [string]::IsNullOrWhiteSpace($SummaryPath) -and (Test-Path -LiteralPath $SummaryPath)
    if ($null -eq $result) {
        return [ordered]@{
            schema = "fb2.main_project.token_bridge_live_preflight.v1"
            exists = $false
            success = $false
            result_path = $ResultPath
            result_source = $ResultSource
            summary_path = $SummaryPath
            summary_exists = [bool]$summaryExists
            preflight_exit_code = $null
            current_state_exit_code = $null
            token_passed_as_argument = $null
            fb2_password_passed_to_child_argv = $null
            token_written_to_output = $null
            writes_visible_group_messages = $null
            project_network_proxy_policy = ""
            note = "No token bridge live preflight result in current output dir."
            generated_at_utc = ""
            age_minutes = $null
            max_age_minutes = $MaxAgeMinutes
            fresh = $false
        }
    }

    $resultSummaryPath = [string](Get-Fb2RefreshProperty $result "summary_path" $SummaryPath)
    if (-not [string]::IsNullOrWhiteSpace($resultSummaryPath)) {
        $SummaryPath = $resultSummaryPath
        $summaryExists = Test-Path -LiteralPath $SummaryPath
    }
    $generatedAt = ConvertTo-Fb2RefreshDate -Value (Get-Fb2RefreshProperty $result "generated_at_utc" $null)
    $ageMinutes = if ($null -eq $generatedAt) { $null } else { ([DateTimeOffset]::UtcNow - $generatedAt).TotalMinutes }
    $fresh = ($null -ne $ageMinutes -and [double]$ageMinutes -ge 0 -and [double]$ageMinutes -le $MaxAgeMinutes)

    [ordered]@{
        schema = "fb2.main_project.token_bridge_live_preflight.v1"
        exists = $true
        success = [bool](Get-Fb2RefreshProperty $result "success" $false)
        result_path = $ResultPath
        result_source = $ResultSource
        summary_path = $SummaryPath
        summary_exists = [bool]$summaryExists
        preflight_exit_code = Get-Fb2RefreshProperty $result "preflight_exit_code" $null
        current_state_exit_code = Get-Fb2RefreshProperty $result "current_state_exit_code" $null
        token_passed_as_argument = [bool](Get-Fb2RefreshProperty $result "token_passed_as_argument" $true)
        fb2_password_passed_to_child_argv = [bool](Get-Fb2RefreshProperty $result "fb2_password_passed_to_child_argv" $true)
        token_written_to_output = [bool](Get-Fb2RefreshProperty $result "token_written_to_output" $true)
        writes_visible_group_messages = [bool](Get-Fb2RefreshProperty $result "writes_visible_group_messages" $true)
        project_network_proxy_policy = [string](Get-Fb2RefreshProperty $result "project_network_proxy_policy" "")
        current_state_after_tokenless = [bool](Get-Fb2RefreshProperty $result "current_state_after_tokenless" $false)
        generated_at_utc = if ($null -eq $generatedAt) { "" } else { $generatedAt.ToString("o") }
        age_minutes = $ageMinutes
        max_age_minutes = $MaxAgeMinutes
        fresh = $fresh
        note = "This is no-write bridge evidence only; full final still follows completion_matrix."
    }
}

function Test-Fb2RefreshProtectedLivePreflightSatisfied {
    param([object]$TokenBridgeLivePreflight)

    if ($null -eq $TokenBridgeLivePreflight) {
        return $false
    }

    $currentStateExitCode = Get-Fb2RefreshProperty $TokenBridgeLivePreflight "current_state_exit_code" $null
    $currentStateExitCodeOk = ($null -eq $currentStateExitCode) -or ([int]$currentStateExitCode -eq 0)

    return (
        [bool](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "exists" $false) -and
        [bool](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "success" $false) -and
        [bool](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "summary_exists" $false) -and
        [int](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "preflight_exit_code" -1) -eq 0 -and
        $currentStateExitCodeOk -and
        -not [bool](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "token_passed_as_argument" $true) -and
        -not [bool](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "fb2_password_passed_to_child_argv" $true) -and
        -not [bool](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "token_written_to_output" $true) -and
        -not [bool](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "writes_visible_group_messages" $true) -and
        [bool](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "current_state_after_tokenless" $false) -and
        [string](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "project_network_proxy_policy" "") -eq "direct_no_proxy" -and
        [bool](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "fresh" $false)
    )
}

function Resolve-Fb2RefreshNextMinimumAction {
    param(
        [object]$GoalAudit,
        [bool]$ProtectedLivePreflightSatisfied,
        [object]$TokenBridgeLivePreflight
    )

    if ([bool](Get-Fb2RefreshProperty $GoalAudit "full_final_complete" $false)) {
        return "goal_complete"
    }
    if (-not [bool](Get-Fb2RefreshProperty $GoalAudit "data_goal_complete" $false)) {
        return [string](Get-Fb2RefreshProperty $GoalAudit "next_minimum_action" "fix_missing_non_voice_requirements")
    }
    if ($ProtectedLivePreflightSatisfied) {
        return "keep_non_voice_regression_green_resume_ASR_TTS_only_when_user_unpauses"
    }
    if ([bool](Get-Fb2RefreshProperty $TokenBridgeLivePreflight "exists" $false)) {
        return "rerun_token_bridge_with_FB2_VISIBLE_SMOKE_PASSWORD_env_then_refresh_status"
    }
    return [string](Get-Fb2RefreshProperty $GoalAudit "next_minimum_action" "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly")
}

function New-Fb2RefreshExportedSampleValidationState {
    param(
        [string]$Fb2Repo,
        [string]$SamplesDir,
        [string]$OutputPath,
        [bool]$Enabled,
        [bool]$Attempted,
        [string]$SkippedReason,
        [object]$Summary
    )

    $scenarioSummaries = @()
    if ($Summary) {
        $scenarioSummaries = @((Get-Fb2RefreshProperty $Summary "scenarios" @()) | ForEach-Object {
                [ordered]@{
                    scenario = [string](Get-Fb2RefreshProperty $_ "scenario" "")
                    path = [string](Get-Fb2RefreshProperty $_ "path" "")
                    passed = [bool](Get-Fb2RefreshProperty $_ "passed" $false)
                    context_audit_id = [string](Get-Fb2RefreshProperty $_ "context_audit_id" "")
                    citation_source_count = [int](Get-Fb2RefreshProperty $_ "citation_source_count" 0)
                    source_kinds = @((Get-Fb2RefreshProperty $_ "source_kinds" @()) | ForEach-Object { [string]$_ })
                    business_source_kinds = @((Get-Fb2RefreshProperty $_ "business_source_kinds" @()) | ForEach-Object { [string]$_ })
                    quality_history_source_kinds = @((Get-Fb2RefreshProperty $_ "quality_history_source_kinds" @()) | ForEach-Object { [string]$_ })
                    context_pack_sha256 = [string](Get-Fb2RefreshProperty $_ "context_pack_sha256" "")
                    contains_secret_like_text = [bool](Get-Fb2RefreshProperty $_ "contains_secret_like_text" $true)
                }
            })
    }

    [ordered]@{
        enabled = $Enabled
        attempted = $Attempted
        skipped_reason = $SkippedReason
        fb2_repo_path = $Fb2Repo
        samples_dir = $SamplesDir
        output_path = $OutputPath
        success = [bool]($Summary -and $Summary.complete)
        complete = [bool]($Summary -and $Summary.complete)
        scenario_count = if ($Summary) { [int]$Summary.scenario_count } else { 0 }
        passed_count = if ($Summary) { [int]$Summary.passed_count } else { 0 }
        failed_count = if ($Summary) { [int]$Summary.failed_count } else { 0 }
        source_kinds = if ($Summary) { @((Get-Fb2RefreshProperty $Summary "source_kinds" @()) | ForEach-Object { [string]$_ }) } else { @() }
        business_source_kinds = if ($Summary) { @((Get-Fb2RefreshProperty $Summary "business_source_kinds" @()) | ForEach-Object { [string]$_ }) } else { @() }
        quality_history_source_kinds = if ($Summary) { @((Get-Fb2RefreshProperty $Summary "quality_history_source_kinds" @()) | ForEach-Object { [string]$_ }) } else { @() }
        secret_like_scenarios = if ($Summary) { @((Get-Fb2RefreshProperty $Summary "secret_like_scenarios" @()) | ForEach-Object { [string]$_ }) } else { @() }
        scenarios = @($scenarioSummaries)
    }
}

function New-Fb2RefreshOwnerActions {
    param(
        [object]$Status,
        [object]$GoalAudit,
        [bool]$ProtectedLivePreflightSatisfied
    )

    $tokenPresent = [bool]$Status.environment.fb2_ai_center_token_present
    $dataGoalComplete = [bool]$GoalAudit.data_goal_complete
    $fullFinalComplete = [bool]$GoalAudit.full_final_complete

    [ordered]@{
        main_project = if ($dataGoalComplete -and $ProtectedLivePreflightSatisfied) {
            "keep_contract_status_and_token_bridge_regressions_green_until_ASR_TTS_resumes"
        } elseif ($dataGoalComplete -and -not $tokenPresent) {
            "keep_contract_and_status_regressions_green_until_FB2_AI_CENTER_TOKEN_is_available"
        } else {
            "refresh_status_goal_audit_and_handoff_after_each_contract_or_smoke_change"
        }
        fb2_project = if ($dataGoalComplete -and $ProtectedLivePreflightSatisfied) {
            "keep_live_context_pack_orders_platform_summary_group_opinion_and_feedback_endpoints_current"
        } elseif (-not $tokenPresent) {
            "provide_FB2_AI_CENTER_TOKEN_or_export_equivalent_live_Context_Pack_permission_quality_evidence"
        } else {
            "keep_live_context_pack_orders_platform_summary_group_opinion_and_feedback_endpoints_current"
        }
        shared = if ($fullFinalComplete) {
            "final_acceptance_complete"
        } elseif ($dataGoalComplete -and $ProtectedLivePreflightSatisfied) {
            "non_voice_live_preflight_satisfied_by_token_bridge_ASR_TTS_final_evidence_deferred_by_user"
        } elseif ($dataGoalComplete) {
            "run_DataOnlyAcceptance_PreflightOnly_with_token_then_refresh_status_refresh_current_json"
        } else {
            "close_missing_non_voice_requirements_before_visible_or_full_final_acceptance"
        }
    }
}

function New-Fb2RefreshNextCommands {
    param([object]$Status)

    $livePreflight = Get-Fb2RefreshProperty $Status "live_preflight_request"
    $liveCommands = Get-Fb2RefreshProperty $livePreflight "commands"
    $coordination = Get-Fb2RefreshProperty $Status "coordination"
    $safeCommands = Get-Fb2RefreshProperty $coordination "safe_commands"
    $sampleRequestCommand = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "generate_context_pack_sample_request"
    if ([string]::IsNullOrWhiteSpace($sampleRequestCommand)) {
        $sampleRequestCommand = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -PrintExportRequest -ExternalUserId <fb2_user_uuid_with_orders> -OutputPath target\fb2-ai-center\context-pack-sample-request-current.json"
    }
    $sampleSetCommand = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "validate_context_pack_sample_set"
    if ([string]::IsNullOrWhiteSpace($sampleSetCommand)) {
        $sampleSetCommand = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\context-pack-samples-validation-current.json"
    }
    $exportedSampleSetCommand = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "validate_exported_context_pack_sample_set"
    if ([string]::IsNullOrWhiteSpace($exportedSampleSetCommand)) {
        $exportedSampleSetCommand = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir <fb2_repo>\target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\fb2-repo-context-pack-samples-validation-current.json"
    }

    [ordered]@{
        refresh_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1"
        read_status_refresh = "Get-Content -Raw -LiteralPath target\fb2-ai-center\status-refresh-current.json | ConvertFrom-Json"
        generate_context_pack_sample_request = $sampleRequestCommand
        validate_context_pack_sample_set = $sampleSetCommand
        validate_exported_context_pack_sample_set = $exportedSampleSetCommand
        validate_context_projection_log = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-projection-log.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\context-projection-log-validation-current.json"
        validate_user_scenario_audit = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-user-scenario-audit.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\user-scenario-audit-validation-current.json"
        validate_current_state = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-current-state.ps1"
        validate_public_contract_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-public-contract-status.ps1 -OutputPath target\fb2-ai-center\public-contract-status-current.json"
        validate_server_deploy_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-main-server-deploy-status.ps1"
        validate_project_direct_network_policy = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-project-direct-network-policy.ps1 -OutputPath target\fb2-ai-center\project-direct-network-policy-validation-current.json"
        validate_context_format_route = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-format-route.ps1 -OutputPath target\fb2-ai-center\context-format-route-validation-current.json"
        validate_read_only_direct_read = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-readonly-summary.ps1 -SummaryPath target\fb2-ai-center\read-only-direct-read-current.json"
        validate_gap_action_board = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-gap-action-board.ps1"
        validate_evidence_freshness = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-evidence-freshness.ps1"
        validate_evidence_privacy = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-evidence-privacy.ps1"
        validate_completion_matrix = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-completion-matrix.ps1"
        validate_handoff_prompt = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-handoff-prompt.ps1"
        validate_visible_answer_policy = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-answer-policy.ps1 -SummaryPath <DATA_ONLY_ACCEPTANCE_JSON>"
        validate_live_preflight_request = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-live-preflight-request.ps1 -StatusPath target\fb2-ai-center\status-current.json"
        validate_tokenless_continuation = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-tokenless-continuation.ps1 -OutputPath target\fb2-ai-center\tokenless-continuation-validation-current.json"
        no_write_direct_read = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "no_write_direct_read"
        data_only_preflight = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "data_only_preflight"
        data_only_preflight_via_fb2_server_token_bridge = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\run-fb2-ai-center-token-bridge.ps1 -RunDataOnlyPreflight"
        visible_regression_requires_authorization = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "visible_regression_requires_authorization"
    }
}

function Get-Fb2RefreshRequirementGroup {
    param([string]$Id)

    switch -Regex ($Id) {
        "^(context_pack_contract|main_project_contract_smoke|domain_context_index_contract|retrieval_evidence_item_contract|context_query_intent_contract)$" { return "main_project_contract" }
        "^(today_matches_analysis|my_ticket_analysis|platform_order_risk|group_opinion_summary|selected_message_review|group_discussion_summary_post|source_reference_audit)$" { return "user_scenarios" }
        "^(permission_safety|feedback_quality_loop)$" { return "permission_and_quality" }
        "^direct_group_chat_read$" { return "group_chat_direct_read" }
        "^voice_final_evidence$" { return "voice_deferred_by_user" }
        default { return "other" }
    }
}

function Get-Fb2RefreshRequirementOwner {
    param([string]$Group)

    switch ($Group) {
        "main_project_contract" { return "main_project" }
        "user_scenarios" { return "shared" }
        "permission_and_quality" { return "shared" }
        "group_chat_direct_read" { return "shared" }
        "voice_deferred_by_user" { return "paused_by_user" }
        default { return "shared" }
    }
}

function New-Fb2RefreshCompletionMatrix {
    param(
        [object]$Status,
        [object]$GoalAudit,
        [bool]$ProtectedLivePreflightSatisfied = $false
    )

    $requirements = @($GoalAudit.requirements)
    $plannedCapabilities = @((Get-Fb2RefreshProperty $GoalAudit "planned_capabilities" @()))
    $items = @(
        foreach ($requirement in $requirements) {
            $id = [string]$requirement.id
            $group = Get-Fb2RefreshRequirementGroup -Id $id
            [ordered]@{
                id = $id
                group = $group
                owner = Get-Fb2RefreshRequirementOwner -Group $group
                title = [string]$requirement.title
                status = [string]$requirement.status
                complete = [bool]$requirement.complete
                deferred = [bool]$requirement.deferred
                evidence = [string]$requirement.evidence
                missing = [string]$requirement.missing
            }
        }
    )

    $completeCount = @($items | Where-Object { [bool]$_.complete }).Count
    $deferredCount = @($items | Where-Object { [bool]$_.deferred }).Count
    $incompleteCount = @($items | Where-Object { -not [bool]$_.complete -and -not [bool]$_.deferred }).Count
    $tokenPresent = [bool]$Status.environment.fb2_ai_center_token_present
    $fullFinalEvidence = Get-Fb2RefreshProperty $GoalAudit "full_final_completion_evidence"
    $sameBatchFullFinalComplete = -not [bool](Get-Fb2RefreshProperty $fullFinalEvidence "missing_same_batch_full_final" $true)

    [ordered]@{
        schema = "fb2.main_project.completion_matrix.v1"
        totals = [ordered]@{
            total = @($items).Count
            complete = $completeCount
            deferred = $deferredCount
            incomplete = $incompleteCount
        }
        gates = [ordered]@{
            data_goal_complete = [bool]$GoalAudit.data_goal_complete
            full_final_complete = [bool]$GoalAudit.full_final_complete
            token_present = $tokenPresent
            protected_live_preflight_satisfied = $ProtectedLivePreflightSatisfied
            voice_deferred_by_user = @($GoalAudit.deferred_requirements) -contains "voice_final_evidence"
            same_batch_full_final_acceptance_complete = $sameBatchFullFinalComplete
            next_minimum_action = [string]$GoalAudit.next_minimum_action
        }
        groups = [ordered]@{
            main_project_contract = @($items | Where-Object { $_.group -eq "main_project_contract" }).Count
            user_scenarios = @($items | Where-Object { $_.group -eq "user_scenarios" }).Count
            permission_and_quality = @($items | Where-Object { $_.group -eq "permission_and_quality" }).Count
            group_chat_direct_read = @($items | Where-Object { $_.group -eq "group_chat_direct_read" }).Count
            voice_deferred_by_user = @($items | Where-Object { $_.group -eq "voice_deferred_by_user" }).Count
            other = @($items | Where-Object { $_.group -eq "other" }).Count
        }
        requirements = $items
        planned_capabilities = @($plannedCapabilities)
    }
}

function Get-Fb2RefreshPathScope {
    param(
        [string]$Path,
        [string]$OutputDir,
        [string[]]$EvidenceDirs
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return "missing"
    }
    try {
        $fullPath = [System.IO.Path]::GetFullPath($Path)
        $fullOutput = [System.IO.Path]::GetFullPath($OutputDir)
    } catch {
        return "unknown"
    }
    if ($fullPath.StartsWith($fullOutput, [System.StringComparison]::OrdinalIgnoreCase)) {
        return "current_output_dir"
    }
    foreach ($dir in @($EvidenceDirs)) {
        if ([string]::IsNullOrWhiteSpace($dir)) {
            continue
        }
        try {
            $fullDir = [System.IO.Path]::GetFullPath($dir)
        } catch {
            continue
        }
        if ($fullPath.StartsWith($fullDir, [System.StringComparison]::OrdinalIgnoreCase)) {
            return "history_evidence_dir"
        }
    }
    return "outside_evidence_dirs"
}

function New-Fb2RefreshArtifactFreshness {
    param(
        [string]$Name,
        [string]$Path,
        [string]$OutputDir,
        [string[]]$EvidenceDirs,
        [datetime]$NowUtc,
        [switch]$GeneratedInCurrentRun
    )

    $exists = -not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path -LiteralPath $Path)
    $lastWriteUtc = $null
    $ageMinutes = $null
    if ($GeneratedInCurrentRun) {
        # 这两个文件在最终 summary 写入后才落盘；这里用本轮生成时间，避免交接误读为旧 artifact。
        $exists = -not [string]::IsNullOrWhiteSpace($Path)
        $lastWriteUtc = $NowUtc.ToString("o")
        $ageMinutes = 0.0
    } elseif ($exists) {
        $item = Get-Item -LiteralPath $Path
        $lastWriteUtc = $item.LastWriteTimeUtc.ToString("o")
        $ageMinutes = [math]::Round(($NowUtc - $item.LastWriteTimeUtc).TotalMinutes, 2)
    }

    [ordered]@{
        name = $Name
        path = $Path
        exists = [bool]$exists
        source_scope = Get-Fb2RefreshPathScope -Path $Path -OutputDir $OutputDir -EvidenceDirs $EvidenceDirs
        last_write_utc = $lastWriteUtc
        age_minutes = $ageMinutes
    }
}

function New-Fb2RefreshEvidenceFreshness {
    param(
        [string]$OutputDir,
        [string[]]$EvidenceDirs,
        [object]$Files,
        [object]$Status,
        [object]$GoalAudit,
        [bool]$ProtectedLivePreflightSatisfied
    )

    $nowUtc = [datetime]::UtcNow
    $generatedInCurrentRun = @{
        status_refresh = $true
        handoff_prompt = $true
    }
    $artifactNames = @(
        "public_contract_status",
        "server_deploy_status",
        "status",
        "goal_audit",
        "goal_audit_markdown",
        "handoff",
        "handoff_markdown",
        "status_refresh",
        "handoff_prompt",
        "exported_context_pack_sample_set_validation",
        "token_bridge_live_preflight",
        "token_bridge_live_preflight_summary"
    )
    $artifacts = @(
        foreach ($name in $artifactNames) {
            New-Fb2RefreshArtifactFreshness `
                -Name $name `
                -Path ([string](Get-Fb2RefreshProperty $Files $name "")) `
                -OutputDir $OutputDir `
                -EvidenceDirs $EvidenceDirs `
                -NowUtc $nowUtc `
                -GeneratedInCurrentRun:$generatedInCurrentRun.ContainsKey($name)
        }
    )
    $currentOutputCount = @($artifacts | Where-Object { $_.source_scope -eq "current_output_dir" -and $_.exists }).Count
    $historyCount = @($artifacts | Where-Object { $_.source_scope -eq "history_evidence_dir" -and $_.exists }).Count

    [ordered]@{
        schema = "fb2.main_project.evidence_freshness.v1"
        generated_at_utc = $nowUtc.ToString("o")
        note = if ($ProtectedLivePreflightSatisfied) {
            "artifact freshness includes fresh no-write token bridge live preflight; ASR/TTS final evidence remains deferred by user"
        } else {
            "artifact freshness only; protected live fb2 data still requires FB2_AI_CENTER_TOKEN or a fresh token bridge preflight"
        }
        current_output_dir = $OutputDir
        evidence_dirs = @($EvidenceDirs)
        artifact_count = @($artifacts).Count
        current_output_artifact_count = $currentOutputCount
        history_artifact_count = $historyCount
        token_present = [bool]$Status.environment.fb2_ai_center_token_present
        data_goal_complete = [bool]$GoalAudit.data_goal_complete
        full_final_complete = [bool]$GoalAudit.full_final_complete
        artifacts = $artifacts
    }
}

function New-Fb2RefreshGapAction {
    param(
        [string]$Id,
        [string]$Status,
        [string]$Owner,
        [string]$EvidenceNeeded,
        [string]$Command,
        [string]$Notes,
        [bool]$CanRunWithoutSecret,
        [bool]$RequiresVisibleGroupWrite,
        [bool]$DeferredByUser
    )

    [ordered]@{
        id = $Id
        status = $Status
        owner = $Owner
        evidence_needed = $EvidenceNeeded
        command = $Command
        notes = $Notes
        can_run_without_secret = $CanRunWithoutSecret
        requires_visible_group_write = $RequiresVisibleGroupWrite
        deferred_by_user = $DeferredByUser
    }
}

function New-Fb2RefreshGapActionBoard {
    param(
        [object]$Status,
        [object]$GoalAudit,
        [object]$BlockingState,
        [object]$NextCommands,
        [object]$CompletionMatrix,
        [bool]$ProtectedLivePreflightSatisfied,
        [string]$NextMinimumAction
    )

    $missing = @($Status.goal_gap_audit.missing) + @($GoalAudit.missing_non_voice_requirements)
    $missing = @($missing | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
    if ($ProtectedLivePreflightSatisfied) {
        $missing = @($missing | Where-Object { [string]$_ -ne "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh" })
    }
    $deferred = @($GoalAudit.deferred_requirements) + @($Status.goal_gap_audit.deferred_by_user)
    $deferred = @($deferred | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
    $plannedCapabilities = @((Get-Fb2RefreshProperty $CompletionMatrix "planned_capabilities" @()))

    $actions = [System.Collections.ArrayList]::new()
    if ($NextMinimumAction -eq "rerun_token_bridge_with_FB2_VISIBLE_SMOKE_PASSWORD_env_then_refresh_status") {
        [void]$actions.Add((New-Fb2RefreshGapAction `
            -Id "token_bridge_live_preflight_regression" `
            -Status "needs_rerun" `
            -Owner "main_project" `
            -EvidenceNeeded "fresh token bridge result with service token not in argv/output and fb2_password_passed_to_child_argv=false" `
            -Command ([string]$NextCommands.data_only_preflight_via_fb2_server_token_bridge) `
            -Notes "Set FB2_VISIBLE_SMOKE_PASSWORD in the current process, rerun the no-write bridge, then refresh status; do not pass the fb2 password on the child command line." `
            -CanRunWithoutSecret $true `
            -RequiresVisibleGroupWrite $false `
            -DeferredByUser $false))
    }
    foreach ($id in $missing) {
        $text = [string]$id
        if ($text -eq "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh") {
            [void]$actions.Add((New-Fb2RefreshGapAction `
                -Id $text `
                -Status "blocked_by_external_secret" `
                -Owner "fb2_project_and_shared" `
                -EvidenceNeeded "FB2_AI_CENTER_TOKEN or equivalent exported live Context Pack / permission / quality evidence" `
                -Command ([string]$NextCommands.data_only_preflight) `
                -Notes "Run no-write DataOnlyAcceptance preflight after the service token is available; do not treat historical artifacts as a fresh live refresh." `
                -CanRunWithoutSecret $false `
                -RequiresVisibleGroupWrite $false `
                -DeferredByUser $false))
            continue
        }
        if ($text -eq "full_final_acceptance_same_batch_voice_and_visible_chat") {
            [void]$actions.Add((New-Fb2RefreshGapAction `
                -Id $text `
                -Status "waiting_on_voice_and_authorized_visible_regression" `
                -Owner "shared" `
                -EvidenceNeeded "same-batch full final summary with voice_status=required, visible direct-read evidence, feedback coverage and voice final evidence" `
                -Command ([string]$NextCommands.visible_regression_requires_authorization) `
                -Notes "Visible group writes require explicit authorization; this cannot be completed while ASR/TTS final evidence is paused." `
                -CanRunWithoutSecret $false `
                -RequiresVisibleGroupWrite $true `
                -DeferredByUser $false))
            continue
        }
        if ($text -eq "voice_final_evidence" -or $text -eq "ASR_TTS_final_evidence") {
            [void]$actions.Add((New-Fb2RefreshGapAction `
                -Id $text `
                -Status "deferred_by_user" `
                -Owner "paused_by_user" `
                -EvidenceNeeded "real device ASR/TTS final-ready evidence JSON and matching final acceptance run" `
                -Command "" `
                -Notes "ASR/TTS work is intentionally paused by user; keep this visible but do not resume voice work in the current non-voice phase." `
                -CanRunWithoutSecret $false `
                -RequiresVisibleGroupWrite $false `
                -DeferredByUser $true))
            continue
        }

        $requirement = @($CompletionMatrix.requirements | Where-Object { [string]$_.id -eq $text } | Select-Object -First 1)
        $owner = if (@($requirement).Count -gt 0 -and -not [string]::IsNullOrWhiteSpace([string]$requirement[0].owner)) {
            [string]$requirement[0].owner
        } else {
            "shared"
        }
        [void]$actions.Add((New-Fb2RefreshGapAction `
            -Id $text `
            -Status "missing" `
            -Owner $owner `
            -EvidenceNeeded "fresh passing evidence for requirement $text" `
            -Command ([string]$NextCommands.refresh_status) `
            -Notes "Refresh status after the owning side updates its contract, Context Pack output, tool manifest or validation evidence." `
            -CanRunWithoutSecret $true `
            -RequiresVisibleGroupWrite $false `
            -DeferredByUser $false))
    }

    foreach ($id in $deferred) {
        $text = [string]$id
        if (@($actions | Where-Object { [string]$_.id -eq $text }).Count -gt 0) {
            continue
        }
        if ($text -eq "voice_final_evidence" -or $text -eq "ASR_TTS_final_evidence") {
            [void]$actions.Add((New-Fb2RefreshGapAction `
                -Id $text `
                -Status "deferred_by_user" `
                -Owner "paused_by_user" `
                -EvidenceNeeded "real device ASR/TTS final-ready evidence JSON and matching final acceptance run" `
                -Command "" `
                -Notes "ASR/TTS work is intentionally paused by user; do not use data-only acceptance to mark full final complete." `
                -CanRunWithoutSecret $false `
                -RequiresVisibleGroupWrite $false `
                -DeferredByUser $true))
            continue
        }
        [void]$actions.Add((New-Fb2RefreshGapAction `
            -Id $text `
            -Status "deferred" `
            -Owner "shared" `
            -EvidenceNeeded "fresh evidence for deferred requirement $text" `
            -Command "" `
            -Notes "Deferred requirement; keep it visible in handoff until explicitly resumed or completed." `
            -CanRunWithoutSecret $false `
            -RequiresVisibleGroupWrite $false `
            -DeferredByUser $true))
    }

    [ordered]@{
        schema = "fb2.main_project.gap_action_board.v1"
        next_minimum_action = $NextMinimumAction
        blocked_by_external_secret = [bool]$BlockingState.blocked_by_external_secret
        external_secret = [string]$BlockingState.external_secret
        protected_live_preflight_satisfied = $ProtectedLivePreflightSatisfied
        action_count = @($actions).Count
        actions = @($actions)
        planned_capabilities = @($plannedCapabilities)
    }
}

function New-Fb2RefreshBlockingState {
    param(
        [object]$Status,
        [object]$GoalAudit,
        [bool]$ProtectedLivePreflightSatisfied,
        [string]$NextMinimumAction
    )

    $tokenPresent = [bool]$Status.environment.fb2_ai_center_token_present
    $blockedByExternalSecret = (-not $tokenPresent) -and (-not $ProtectedLivePreflightSatisfied)

    [ordered]@{
        blocked_by_external_secret = $blockedByExternalSecret
        external_secret = "FB2_AI_CENTER_TOKEN"
        protected_live_preflight_satisfied = $ProtectedLivePreflightSatisfied
        protected_live_preflight_satisfied_by = if ($ProtectedLivePreflightSatisfied) { "token_bridge_live_preflight" } else { "" }
        deferred_by_user = @($Status.goal_gap_audit.deferred_by_user)
        safe_to_continue_without_secret = @(
            "public_contract_regression",
            "status_refresh_selftest",
            "context_format_route_regression",
            "offline_context_pack_sample_validation",
            "handoff_documentation",
            "token_bridge_live_preflight_regression"
        )
        requires_secret = @(
            "live_context_pack_permission_quality_refresh",
            "current_user_order_live_verification",
            "platform_order_summary_live_verification",
            "feedback_quality_live_refresh"
        )
        next_minimum_action = $NextMinimumAction
    }
}

function Assert-Fb2RefreshSelfTest {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw "SelfTest failed: $Message"
    }
}

function Invoke-Fb2RefreshSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-ai-center-refresh-selftest-" + [guid]::NewGuid().ToString("N"))
    $output = Join-Path $tempRoot "out"
    $missingEvidence = Join-Path $tempRoot "missing-evidence"
    try {
        New-Item -ItemType Directory -Force -Path $output | Out-Null
        # 自测必须无网络、无 token、无主工作区依赖；这里只验证编排脚本能生成稳定机器摘要。
        $raw = & $PSCommandPath -OutputDir $output -MainWorkspaceEvidenceDir $missingEvidence -Fb2RepoPath $missingEvidence -SkipPublicContract
        $summary = $raw | ConvertFrom-Json
        Assert-Fb2RefreshSelfTest ($summary.schema -eq "fb2.main_project.status_refresh.v1") "schema"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.generated_at_utc)) "top-level generated_at_utc"
        Assert-Fb2RefreshSelfTest ([string]$summary.output_dir -eq $output) "output_dir"
        Assert-Fb2RefreshSelfTest (@($summary.evidence_dirs).Count -eq 1) "isolated evidence dirs"
        Assert-Fb2RefreshSelfTest (-not [bool]$summary.public_contract_ready) "public contract skipped"
        Assert-Fb2RefreshSelfTest ([string]$summary.answer_source_validation_status.schema -eq "fb2.main_project.answer_source_validation_status.v1") "answer source validation status schema"
        Assert-Fb2RefreshSelfTest (-not [bool]$summary.answer_source_validation_ready) "answer source validation skipped"
        Assert-Fb2RefreshSelfTest (Test-Path -LiteralPath ([string]$summary.files.status)) "status file exists"
        Assert-Fb2RefreshSelfTest (Test-Path -LiteralPath ([string]$summary.files.goal_audit)) "goal audit file exists"
        Assert-Fb2RefreshSelfTest (Test-Path -LiteralPath ([string]$summary.files.handoff_markdown)) "handoff markdown exists"
        Assert-Fb2RefreshSelfTest (Test-Path -LiteralPath ([string]$summary.files.status_refresh)) "status refresh file exists"
        Assert-Fb2RefreshSelfTest (Test-Path -LiteralPath ([string]$summary.files.handoff_prompt)) "handoff prompt file exists"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.owner_next_actions.main_project)) "main owner action"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.owner_next_actions.fb2_project)) "fb2 owner action"
        Assert-Fb2RefreshSelfTest ([bool]$summary.blocking_state.blocked_by_external_secret) "selftest token blocked"
        Assert-Fb2RefreshSelfTest ([string]$summary.blocking_state.external_secret -eq "FB2_AI_CENTER_TOKEN") "external secret name"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.refresh_status)) "refresh command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.generate_context_pack_sample_request)) "context pack sample request command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_context_pack_sample_set)) "context pack sample set validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_exported_context_pack_sample_set)) "exported context pack sample set validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_context_projection_log)) "context projection log validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_user_scenario_audit)) "user scenario audit validation command"
        Assert-Fb2RefreshSelfTest (-not [bool]$summary.exported_context_pack_sample_set_validation.attempted) "selftest skips missing exported sample validation"
        Assert-Fb2RefreshSelfTest ([string]$summary.exported_context_pack_sample_set_validation.skipped_reason -eq "samples_dir_missing") "selftest exported sample missing reason"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_current_state)) "current state validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_public_contract_status)) "public contract status validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_server_deploy_status)) "server deploy status validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_project_direct_network_policy)) "project direct network policy validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_context_format_route)) "context format route validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_read_only_direct_read)) "read-only direct read validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_gap_action_board)) "gap action validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_evidence_freshness)) "evidence freshness validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_evidence_privacy)) "evidence privacy validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_completion_matrix)) "completion matrix validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_handoff_prompt)) "handoff prompt validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_visible_answer_policy)) "visible answer policy validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_live_preflight_request)) "live preflight request validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_tokenless_continuation)) "tokenless continuation validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.data_only_preflight)) "data-only preflight command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.data_only_preflight_via_fb2_server_token_bridge)) "data-only token bridge preflight command"
        Assert-Fb2RefreshSelfTest ([string]$summary.completion_matrix.schema -eq "fb2.main_project.completion_matrix.v1") "completion matrix schema"
        Assert-Fb2RefreshSelfTest (@($summary.completion_matrix.requirements).Count -gt 0) "completion matrix requirements"
        $plannedCapability = @($summary.planned_capabilities | Where-Object { [string]$_.id -eq "p4_vector" })
        $matrixPlannedCapability = @($summary.completion_matrix.planned_capabilities | Where-Object { [string]$_.id -eq "p4_vector" })
        Assert-Fb2RefreshSelfTest (@($plannedCapability).Count -eq 1) "top-level planned p4 vector capability"
        Assert-Fb2RefreshSelfTest (@($matrixPlannedCapability).Count -eq 1) "completion matrix planned p4 vector capability"
        Assert-Fb2RefreshSelfTest ([int]$summary.planned_capability_count -eq @($summary.planned_capabilities).Count) "planned capability count"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].contract_version -eq "fb2_p4_vector_contract_v1") "planned p4 vector contract"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].source_enumerator_report_version -eq "fb2_p4_source_enumerator_v1") "planned p4 source enumerator report"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].source_enumerator_status -eq "source_specific_no_write_sample_available") "planned p4 source enumerator status"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].source_enumerator_checked_count -eq "reported_by_fb2_source_enumerator_runtime") "planned p4 source enumerator checked count delegated"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].source_enumerator_known_chunk_count -eq "reported_by_fb2_source_enumerator_runtime") "planned p4 source enumerator known chunk count delegated"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].chunk_manifest_report_version -eq "fb2_p4_chunk_manifest_v1") "planned p4 chunk manifest report"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].chunk_manifest_status -eq "id_only_no_write_manifest_available") "planned p4 chunk manifest status"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].chunk_manifest_materialized_entry_count -eq "reported_by_fb2_chunk_manifest_runtime") "planned p4 chunk manifest entry count delegated"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].chunk_manifest_current_scope_complete -eq "reported_by_fb2_chunk_manifest_runtime") "planned p4 chunk manifest scope complete delegated"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].embedding_build_dry_run_report_version -eq "fb2_p4_embedding_build_dry_run_v1") "planned p4 embedding dry-run report"
        Assert-Fb2RefreshSelfTest ([string]$plannedCapability[0].dry_run_status -eq "dry_run_available_no_writes") "planned p4 embedding dry-run status"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].production_grounding) "planned p4 vector not production grounding"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].blocks_data_goal) "planned p4 vector not blocking data goal"
        Assert-Fb2RefreshSelfTest ([bool]$plannedCapability[0].read_only) "planned p4 embedding dry-run read-only"
        Assert-Fb2RefreshSelfTest ([bool]$plannedCapability[0].dry_run) "planned p4 embedding dry-run flag"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].writes_chunk_manifest_file) "planned p4 chunk manifest file not written"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].persists_manifest_rows) "planned p4 manifest rows not persisted"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].writes_embedding_rows) "planned p4 embedding rows not written"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].writes_vector_store) "planned p4 vector store not written"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].writes_public_group_messages) "planned p4 source enumerator writes no public group messages"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].writes_feedback_or_adoption) "planned p4 source enumerator writes no feedback or adoption"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].writes_opinion_index_rows) "planned p4 source enumerator writes no opinion index rows"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].refresh_operations_used) "planned p4 refresh not used"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].source_payload_included) "planned p4 source payload not included"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].embedding_text_included) "planned p4 embedding text not included"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].ready_to_write_embeddings) "planned p4 not ready to write embeddings"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].ready_to_enable_answer_time_vector_candidates) "planned p4 vector answer-time disabled"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].ready_for_shadow_eval) "planned p4 shadow eval not ready"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].answer_time_vector_candidates_enabled) "planned p4 answer-time candidates disabled"
        Assert-Fb2RefreshSelfTest ([bool]$plannedCapability[0].candidate_rows_require_live_hydration) "planned p4 candidates require live hydration"
        Assert-Fb2RefreshSelfTest (-not [bool]$plannedCapability[0].vector_rows_are_model_input) "planned p4 vector rows are not model input"
        Assert-Fb2RefreshSelfTest ([int]$summary.completion_total -eq [int]$summary.completion_matrix.totals.total) "top-level completion total"
        Assert-Fb2RefreshSelfTest ([int]$summary.completion_complete -eq [int]$summary.completion_matrix.totals.complete) "top-level completion complete"
        Assert-Fb2RefreshSelfTest ([int]$summary.completion_deferred -eq [int]$summary.completion_matrix.totals.deferred) "top-level completion deferred"
        Assert-Fb2RefreshSelfTest ([int]$summary.completion_incomplete -eq [int]$summary.completion_matrix.totals.incomplete) "top-level completion incomplete"
        Assert-Fb2RefreshSelfTest ([bool]$summary.voice_deferred_by_user -eq [bool]$summary.completion_matrix.gates.voice_deferred_by_user) "top-level voice deferred flag"
        Assert-Fb2RefreshSelfTest ([bool]$summary.protected_live_preflight_satisfied -eq [bool]$summary.completion_matrix.gates.protected_live_preflight_satisfied) "top-level protected live preflight flag"
        Assert-Fb2RefreshSelfTest ($null -ne $summary.full_final_completion_evidence) "full final completion evidence"
        Assert-Fb2RefreshSelfTest ($null -ne $summary.completion_matrix.gates.same_batch_full_final_acceptance_complete) "same batch full final gate"
        Assert-Fb2RefreshSelfTest ([string]$summary.evidence_freshness.schema -eq "fb2.main_project.evidence_freshness.v1") "evidence freshness schema"
        Assert-Fb2RefreshSelfTest (@($summary.evidence_freshness.artifacts).Count -gt 0) "evidence freshness artifacts"
        Assert-Fb2RefreshSelfTest ([string]$summary.token_bridge_live_preflight.schema -eq "fb2.main_project.token_bridge_live_preflight.v1") "token bridge live preflight schema"
        Assert-Fb2RefreshSelfTest (-not [bool]$summary.token_bridge_live_preflight.exists) "selftest token bridge absent"
        $fallbackCanonicalPath = Join-Path $output "token-bridge-data-only-preflight-current.json"
        $fallbackSummaryPath = Join-Path $output "token-bridge-data-only-preflight-summary-current.json"
        $fallbackResultPath = Join-Path $output "token-bridge-current.json"
        Set-Content -LiteralPath $fallbackSummaryPath -Value '{"success":true}' -Encoding UTF8
        $staleCanonicalResult = [ordered]@{
            success = $true
            generated_at_utc = ([DateTimeOffset]::UtcNow.AddHours(-3)).ToString("o")
            preflight_exit_code = 0
            current_state_exit_code = 0
            summary_path = $fallbackSummaryPath
            token_passed_as_argument = $false
            fb2_password_passed_to_child_argv = $false
            token_written_to_output = $false
            writes_visible_group_messages = $false
            current_state_after_tokenless = $true
            project_network_proxy_policy = "direct_no_proxy"
        }
        Set-Content -LiteralPath $fallbackCanonicalPath -Value ($staleCanonicalResult | ConvertTo-Json -Depth 5) -Encoding UTF8
        $fallbackResult = [ordered]@{
            success = $true
            generated_at_utc = ([DateTimeOffset]::UtcNow).ToString("o")
            preflight_exit_code = 0
            current_state_exit_code = 0
            summary_path = $fallbackSummaryPath
            token_passed_as_argument = $false
            fb2_password_passed_to_child_argv = $false
            token_written_to_output = $false
            writes_visible_group_messages = $false
            current_state_after_tokenless = $true
            project_network_proxy_policy = "direct_no_proxy"
        }
        Set-Content -LiteralPath $fallbackResultPath -Value ($fallbackResult | ConvertTo-Json -Depth 5) -Encoding UTF8
        $fallbackChoice = Resolve-Fb2RefreshTokenBridgeResultPath `
            -CanonicalPath $fallbackCanonicalPath `
            -FallbackPath $fallbackResultPath
        $fallbackPreflight = New-Fb2RefreshTokenBridgeLivePreflight `
            -ResultPath ([string]$fallbackChoice.path) `
            -SummaryPath $fallbackSummaryPath `
            -ResultSource ([string]$fallbackChoice.source)
        Assert-Fb2RefreshSelfTest ([string]$fallbackChoice.source -eq "custom_output_path_fallback") "token bridge custom output fallback selected"
        Assert-Fb2RefreshSelfTest ([string]$fallbackPreflight.result_source -eq "custom_output_path_fallback") "token bridge custom output fallback recorded"
        Assert-Fb2RefreshSelfTest (Test-Fb2RefreshProtectedLivePreflightSatisfied -TokenBridgeLivePreflight $fallbackPreflight) "token bridge custom output fallback satisfies preflight"
        $generatedArtifacts = @($summary.evidence_freshness.artifacts | Where-Object { @("status_refresh", "handoff_prompt") -contains [string]$_.name })
        Assert-Fb2RefreshSelfTest (@($generatedArtifacts).Count -eq 2) "generated artifacts present"
        foreach ($artifact in $generatedArtifacts) {
            Assert-Fb2RefreshSelfTest ([bool]$artifact.exists) "generated artifact exists flag $($artifact.name)"
            Assert-Fb2RefreshSelfTest ([string]$artifact.source_scope -eq "current_output_dir") "generated artifact scope $($artifact.name)"
            Assert-Fb2RefreshSelfTest ([double]$artifact.age_minutes -eq 0.0) "generated artifact current age $($artifact.name)"
        }
        Assert-Fb2RefreshSelfTest ([string]$summary.gap_action_board.schema -eq "fb2.main_project.gap_action_board.v1") "gap action board schema"
        Assert-Fb2RefreshSelfTest (@($summary.gap_action_board.actions).Count -gt 0) "gap action board actions"
        $gapPlannedCapability = @($summary.gap_action_board.planned_capabilities | Where-Object { [string]$_.id -eq "p4_vector" })
        Assert-Fb2RefreshSelfTest (@($gapPlannedCapability).Count -eq 1) "gap action board planned p4 vector capability"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_minimum_action)) "next action"
        "== SelfTest Summary =="
        "failed=0"
    } finally {
        if (Test-Path -LiteralPath $tempRoot) {
            Remove-Item -LiteralPath $tempRoot -Recurse -Force
        }
    }
}

if ($SelfTest) {
    Invoke-Fb2RefreshSelfTest
    exit 0
}

$root = Get-Fb2RefreshRepoRoot
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $root "target\fb2-ai-center"
} else {
    $OutputDir = Resolve-Fb2RefreshPath -Path $OutputDir -Root $root
}
if ([string]::IsNullOrWhiteSpace($Fb2RepoPath)) {
    $Fb2RepoPath = $env:FB2_REPO_PATH
}
if ([string]::IsNullOrWhiteSpace($Fb2RepoPath)) {
    $Fb2RepoPath = "D:\rust\active-projects\fb2"
}
$Fb2RepoPath = Resolve-Fb2RefreshPath -Path $Fb2RepoPath -Root $root
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ([string]::IsNullOrWhiteSpace($RefreshSummaryPath)) {
    $RefreshSummaryPath = Join-Path $OutputDir "status-refresh-current.json"
} else {
    $RefreshSummaryPath = Resolve-Fb2RefreshPath -Path $RefreshSummaryPath -Root $root
}
$refreshParent = Split-Path -Parent $RefreshSummaryPath
if (-not [string]::IsNullOrWhiteSpace($refreshParent)) {
    New-Item -ItemType Directory -Force -Path $refreshParent | Out-Null
}
if ([string]::IsNullOrWhiteSpace($HandoffPromptPath)) {
    $HandoffPromptPath = Join-Path $OutputDir "handoff-prompt-current.md"
} else {
    $HandoffPromptPath = Resolve-Fb2RefreshPath -Path $HandoffPromptPath -Root $root
}
$handoffPromptParent = Split-Path -Parent $HandoffPromptPath
if (-not [string]::IsNullOrWhiteSpace($handoffPromptParent)) {
    New-Item -ItemType Directory -Force -Path $handoffPromptParent | Out-Null
}

$evidence = [System.Collections.ArrayList]::new()
$seen = @{}

# 当前 worktree 的 target 永远排第一；额外目录只作为历史证据兜底，不覆盖更新的当前产物。
Add-Fb2RefreshDirectory -Target $evidence -Seen $seen -Directory $OutputDir -Root $root
foreach ($dir in @($EvidenceDirs)) {
    Add-Fb2RefreshDirectory -Target $evidence -Seen $seen -Directory $dir -Root $root -RequireExists
}

if ([string]::IsNullOrWhiteSpace($MainWorkspaceEvidenceDir)) {
    $MainWorkspaceEvidenceDir = "D:\rust\active-projects\elon cli\target\fb2-ai-center"
}
Add-Fb2RefreshDirectory -Target $evidence -Seen $seen -Directory $MainWorkspaceEvidenceDir -Root $root -RequireExists

$publicPath = Join-Path $OutputDir "public-contract-status-current.json"
$serverDeployStatusPath = Join-Path $OutputDir "server-deploy-status-current.json"
$statusPath = Join-Path $OutputDir "status-current.json"
$goalAuditPath = Join-Path $OutputDir "goal-audit-current.json"
$goalAuditMarkdownPath = Join-Path $OutputDir "goal-audit-current.md"
$handoffPath = Join-Path $OutputDir "handoff-current.json"
$handoffMarkdownPath = Join-Path $OutputDir "handoff-current.md"
$exportedSampleSetValidationPath = Join-Path $OutputDir "fb2-repo-context-pack-samples-validation-current.json"
$exportedSamplesDir = Join-Path $Fb2RepoPath "target\fb2-ai-center\samples"
$exportedSampleValidationSummary = $null
$exportedSampleValidationAttempted = $false
$exportedSampleValidationSkipReason = ""

if ($SkipExportedContextPackSampleValidation) {
    $exportedSampleValidationSkipReason = "skipped_by_flag"
} elseif (-not (Test-Path -LiteralPath $exportedSamplesDir)) {
    $exportedSampleValidationSkipReason = "samples_dir_missing"
} else {
    $exportedSampleValidationAttempted = $true
    & (Join-Path $PSScriptRoot "validate-fb2-context-pack.ps1") `
        -ValidateSampleSet `
        -SamplesDir $exportedSamplesDir `
        -OutputPath $exportedSampleSetValidationPath | Out-Null
    $exportedSampleValidationSummary = Read-Fb2RefreshJson -Path $exportedSampleSetValidationPath
}
if (-not $exportedSampleValidationAttempted -and (Test-Path -LiteralPath $exportedSampleSetValidationPath)) {
    Remove-Item -LiteralPath $exportedSampleSetValidationPath -Force
}
$exportedSampleValidationState = New-Fb2RefreshExportedSampleValidationState `
    -Fb2Repo $Fb2RepoPath `
    -SamplesDir $exportedSamplesDir `
    -OutputPath $exportedSampleSetValidationPath `
    -Enabled (-not [bool]$SkipExportedContextPackSampleValidation) `
    -Attempted $exportedSampleValidationAttempted `
    -SkippedReason $exportedSampleValidationSkipReason `
    -Summary $exportedSampleValidationSummary

if (-not $SkipPublicContract) {
    & (Join-Path $PSScriptRoot "fb2-public-contract-status.ps1") -OutputPath $publicPath | Out-Null
    & (Join-Path $PSScriptRoot "validate-fb2-main-server-deploy-status.ps1") -OutputPath $serverDeployStatusPath | Out-Null
}

& (Join-Path $PSScriptRoot "smoke-fb2-ai-center-status.ps1") `
    -SummaryDir $OutputDir `
    -EvidenceDirs @($evidence) `
    -OutputPath $statusPath | Out-Null

& (Join-Path $PSScriptRoot "fb2-ai-center-goal-audit-report.ps1") `
    -StatusPath $statusPath `
    -OutputPath $goalAuditPath `
    -MarkdownPath $goalAuditMarkdownPath | Out-Null

& (Join-Path $PSScriptRoot "fb2-ai-center-handoff-report.ps1") `
    -StatusPath $statusPath `
    -OutputPath $handoffPath `
    -MarkdownPath $handoffMarkdownPath | Out-Null

$status = Read-Fb2RefreshJson -Path $statusPath
$goalAudit = Read-Fb2RefreshJson -Path $goalAuditPath
$public = Read-Fb2RefreshJson -Path $publicPath
$serverDeployStatus = Read-Fb2RefreshJson -Path $serverDeployStatusPath
$answerSourceValidationStatus = New-Fb2RefreshAnswerSourceValidationState -PublicStatus $public
$files = [ordered]@{
    status_refresh = $RefreshSummaryPath
    public_contract_status = $publicPath
    server_deploy_status = $serverDeployStatusPath
    status = $statusPath
    goal_audit = $goalAuditPath
    goal_audit_markdown = $goalAuditMarkdownPath
    handoff = $handoffPath
    handoff_markdown = $handoffMarkdownPath
    handoff_prompt = $HandoffPromptPath
    exported_context_pack_sample_set_validation = $exportedSampleSetValidationPath
    token_bridge_live_preflight = Join-Path $OutputDir "token-bridge-data-only-preflight-current.json"
    token_bridge_live_preflight_summary = Join-Path $OutputDir "token-bridge-data-only-preflight-summary-current.json"
}
$tokenBridgeResultPathChoice = Resolve-Fb2RefreshTokenBridgeResultPath `
    -CanonicalPath ([string]$files.token_bridge_live_preflight) `
    -FallbackPath (Join-Path $OutputDir "token-bridge-current.json")
$files.token_bridge_live_preflight = [string]$tokenBridgeResultPathChoice.path
$tokenBridgeLivePreflight = New-Fb2RefreshTokenBridgeLivePreflight `
    -ResultPath ([string]$files.token_bridge_live_preflight) `
    -SummaryPath ([string]$files.token_bridge_live_preflight_summary) `
    -ResultSource ([string]$tokenBridgeResultPathChoice.source)
$protectedLivePreflightSatisfied = Test-Fb2RefreshProtectedLivePreflightSatisfied -TokenBridgeLivePreflight $tokenBridgeLivePreflight
$effectiveNextMinimumAction = Resolve-Fb2RefreshNextMinimumAction `
    -GoalAudit $goalAudit `
    -ProtectedLivePreflightSatisfied $protectedLivePreflightSatisfied `
    -TokenBridgeLivePreflight $tokenBridgeLivePreflight
$ownerNextActions = New-Fb2RefreshOwnerActions `
    -Status $status `
    -GoalAudit $goalAudit `
    -ProtectedLivePreflightSatisfied $protectedLivePreflightSatisfied
$blockingState = New-Fb2RefreshBlockingState `
    -Status $status `
    -GoalAudit $goalAudit `
    -ProtectedLivePreflightSatisfied $protectedLivePreflightSatisfied `
    -NextMinimumAction $effectiveNextMinimumAction
$nextCommands = New-Fb2RefreshNextCommands -Status $status
$completionMatrix = New-Fb2RefreshCompletionMatrix `
    -Status $status `
    -GoalAudit $goalAudit `
    -ProtectedLivePreflightSatisfied $protectedLivePreflightSatisfied
$completionMatrix.gates.next_minimum_action = $effectiveNextMinimumAction
$gapActionBoard = New-Fb2RefreshGapActionBoard `
    -Status $status `
    -GoalAudit $goalAudit `
    -BlockingState $blockingState `
    -NextCommands $nextCommands `
    -CompletionMatrix $completionMatrix `
    -ProtectedLivePreflightSatisfied $protectedLivePreflightSatisfied `
    -NextMinimumAction $effectiveNextMinimumAction
$evidenceFreshness = New-Fb2RefreshEvidenceFreshness `
    -OutputDir $OutputDir `
    -EvidenceDirs @($evidence) `
    -Files $files `
    -Status $status `
    -GoalAudit $goalAudit `
    -ProtectedLivePreflightSatisfied $protectedLivePreflightSatisfied
$voiceDeferredByUser = [bool](Get-Fb2RefreshProperty (Get-Fb2RefreshProperty $completionMatrix "gates" $null) "voice_deferred_by_user" $false)
$completionTotals = Get-Fb2RefreshProperty $completionMatrix "totals" $null

$refreshSummary = [pscustomobject]@{
    schema = "fb2.main_project.status_refresh.v1"
    generated_at_utc = [string]$evidenceFreshness.generated_at_utc
    output_dir = $OutputDir
    evidence_dirs = @($evidence)
    files = $files
    public_contract_ready = [bool]($public -and $public.success)
    answer_source_validation_ready = [bool]$answerSourceValidationStatus.ready
    answer_source_validation_status = $answerSourceValidationStatus
    server_deploy_ready = [bool]($serverDeployStatus -and $serverDeployStatus.success)
    server_deploy_status = $serverDeployStatus
    user_scenario_audit_ready = [bool]$status.latest_user_scenario_audit.complete
    non_voice_historical_evidence_ready = [bool]$status.readiness.non_voice_historical_evidence_ready
    data_goal_complete = [bool]$goalAudit.data_goal_complete
    full_final_complete = [bool]$goalAudit.full_final_complete
    token_present = [bool]$status.environment.fb2_ai_center_token_present
    protected_live_preflight_satisfied = $protectedLivePreflightSatisfied
    voice_deferred_by_user = $voiceDeferredByUser
    completion_total = [int](Get-Fb2RefreshProperty $completionTotals "total" 0)
    completion_complete = [int](Get-Fb2RefreshProperty $completionTotals "complete" 0)
    completion_deferred = [int](Get-Fb2RefreshProperty $completionTotals "deferred" 0)
    completion_incomplete = [int](Get-Fb2RefreshProperty $completionTotals "incomplete" 0)
    planned_capability_count = @($completionMatrix.planned_capabilities).Count
    planned_capabilities = @($completionMatrix.planned_capabilities)
    next_minimum_action = $effectiveNextMinimumAction
    owner_next_actions = $ownerNextActions
    blocking_state = $blockingState
    next_commands = $nextCommands
    token_bridge_live_preflight = $tokenBridgeLivePreflight
    exported_context_pack_sample_set_validation = $exportedSampleValidationState
    completion_matrix = $completionMatrix
    full_final_completion_evidence = $goalAudit.full_final_completion_evidence
    gap_action_board = $gapActionBoard
    evidence_freshness = $evidenceFreshness
    missing_non_voice_requirements = @($goalAudit.missing_non_voice_requirements)
    deferred_requirements = @($goalAudit.deferred_requirements)
}

$refreshJson = $refreshSummary | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $RefreshSummaryPath -Value $refreshJson -Encoding UTF8
& (Join-Path $PSScriptRoot "fb2-ai-center-handoff-prompt.ps1") `
    -RefreshPath $RefreshSummaryPath `
    -OutputPath $HandoffPromptPath | Out-Null
& (Join-Path $PSScriptRoot "fb2-ai-center-handoff-report.ps1") `
    -StatusPath $statusPath `
    -RefreshPath $RefreshSummaryPath `
    -OutputPath $handoffPath `
    -MarkdownPath $handoffMarkdownPath | Out-Null
$refreshJson
