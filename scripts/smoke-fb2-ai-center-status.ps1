#requires -Version 7.0

param(
    [string]$SummaryDir = "",
    [string[]]$EvidenceDirs = @(),
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "fb2-visible-readonly-validation.ps1")
. (Join-Path $PSScriptRoot "fb2-data-only-direct-read-validation.ps1")
. (Join-Path $PSScriptRoot "fb2-visible-answer-policy-validation.ps1")
. (Join-Path $PSScriptRoot "fb2-context-projection-log-validation.ps1")
. (Join-Path $PSScriptRoot "fb2-context-sample-request-status.ps1")
. (Join-Path $PSScriptRoot "fb2-context-sample-set-status.ps1")
. (Join-Path $PSScriptRoot "fb2-context-answer-readiness-status.ps1")
. (Join-Path $PSScriptRoot "fb2-user-scenario-audit-status.ps1")
. (Join-Path $PSScriptRoot "fb2-domain-data-blueprint-status.ps1")
. (Join-Path $PSScriptRoot "fb2-goal-readiness-status.ps1")
. (Join-Path $PSScriptRoot "fb2-goal-gap-audit-status.ps1")
. (Join-Path $PSScriptRoot "fb2-live-preflight-request-status.ps1")
. (Join-Path $PSScriptRoot "fb2-ai-center-coordination-status.ps1")
. (Join-Path $PSScriptRoot "fb2-public-contract-summary-status.ps1")
. (Join-Path $PSScriptRoot "fb2-ai-center-contract-smoke-summary.ps1")

function Get-Fb2StatusRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Get-LatestFileByPattern {
    param(
        [string]$Directory,
        [string]$Pattern
    )

    if (-not (Test-Path -LiteralPath $Directory)) {
        return $null
    }
    $files = @(Get-ChildItem -LiteralPath $Directory -Filter $Pattern -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTimeUtc -Descending)
    if ($files.Count -eq 0) {
        return $null
    }
    return $files[0]
}

function Split-Fb2StatusDirectoryList {
    param([string]$Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return @()
    }
    $separator = [System.IO.Path]::PathSeparator
    @($Value -split [regex]::Escape([string]$separator) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

function Get-Fb2StatusSummaryDirectories {
    param(
        [string]$PrimaryDirectory,
        [string[]]$ExtraDirectories
    )

    $root = Get-Fb2StatusRepoRoot
    if ([string]::IsNullOrWhiteSpace($PrimaryDirectory)) {
        $PrimaryDirectory = Join-Path $root "target\fb2-ai-center"
    }

    $candidates = @()
    $candidates += $PrimaryDirectory
    $candidates += @($ExtraDirectories)
    $candidates += Split-Fb2StatusDirectoryList -Value $env:FB2_AI_CENTER_SUMMARY_DIR
    $candidates += Split-Fb2StatusDirectoryList -Value $env:FB2_AI_CENTER_SUMMARY_DIRS

    $seen = @{}
    $result = @()
    foreach ($candidate in $candidates) {
        if ([string]::IsNullOrWhiteSpace($candidate)) {
            continue
        }
        $path = [string]$candidate
        if (-not [System.IO.Path]::IsPathRooted($path)) {
            $path = Join-Path $root $path
        }
        $key = $path.ToLowerInvariant()
        if (-not $seen.ContainsKey($key)) {
            $seen[$key] = $true
            $result += $path
        }
    }
    return @($result)
}

function Get-LatestFileByPatternAcrossDirectories {
    param(
        [string[]]$Directories,
        [string]$Pattern
    )

    $files = @()
    foreach ($directory in @($Directories)) {
        if ([string]::IsNullOrWhiteSpace($directory) -or -not (Test-Path -LiteralPath $directory)) {
            continue
        }
        $files += @(Get-ChildItem -LiteralPath $directory -Filter $Pattern -File -ErrorAction SilentlyContinue)
    }
    $ordered = @($files | Sort-Object LastWriteTimeUtc -Descending)
    if ($ordered.Count -eq 0) {
        return $null
    }
    return $ordered[0]
}

function Get-LatestFb2ContextSampleSetValidationFile {
    param([string[]]$Directories)

    $patterns = @(
        "context-pack-samples-validation*.json",
        "fb2-repo-context-pack-samples-validation*.json"
    )
    $files = @()
    foreach ($pattern in $patterns) {
        $file = Get-LatestFileByPatternAcrossDirectories -Directories $Directories -Pattern $pattern
        if ($null -ne $file) {
            $files += $file
        }
    }
    $ordered = @($files | Sort-Object LastWriteTimeUtc -Descending)
    if ($ordered.Count -eq 0) {
        return $null
    }
    return $ordered[0]
}

function Get-JsonProperty {
    param(
        [object]$Object,
        [string]$Name,
        [object]$Default = $null
    )

    if ($null -eq $Object) {
        return $Default
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $Default
    }
    return $property.Value
}

function Test-TruthyJsonValue {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    return ([string]$Value) -match "^(true|True|1)$"
}

function Get-GitValueOrEmpty {
    param([string[]]$GitArgs)

    try {
        $value = & git @GitArgs 2>$null
        if ($LASTEXITCODE -ne 0) {
            return ""
        }
        return (($value | Out-String).Trim())
    } catch {
        return ""
    }
}

function Build-Fb2AiCenterStatusSnapshot {
    param(
        [string]$Directory,
        [string[]]$ExtraDirectories = @()
    )

    $root = Get-Fb2StatusRepoRoot
    if ([string]::IsNullOrWhiteSpace($Directory)) {
        $Directory = Join-Path $root "target\fb2-ai-center"
    }
    $summaryDirectories = Get-Fb2StatusSummaryDirectories -PrimaryDirectory $Directory -ExtraDirectories $ExtraDirectories

    $latestDataFile = Get-LatestFileByPatternAcrossDirectories -Directories $summaryDirectories -Pattern "data-only-acceptance-*.json"
    $latestFinalFile = Get-LatestFileByPatternAcrossDirectories -Directories $summaryDirectories -Pattern "final-acceptance-*.json"
    $latestReadOnlyFile = Get-LatestFileByPatternAcrossDirectories -Directories $summaryDirectories -Pattern "read-only-direct-read*.json"
    $latestAiCenterLogFile = Get-LatestFileByPatternAcrossDirectories -Directories $summaryDirectories -Pattern "*ai-center.log"
    $latestSampleRequestFile = Get-LatestFileByPatternAcrossDirectories -Directories $summaryDirectories -Pattern "context-pack-sample-request*.json"
    $latestSampleSetFile = Get-LatestFb2ContextSampleSetValidationFile -Directories $summaryDirectories
    $latestPublicContractFile = Get-LatestFileByPatternAcrossDirectories -Directories $summaryDirectories -Pattern "public-contract-status*.json"
    $latestContractSmokeFile = Get-LatestFileByPatternAcrossDirectories -Directories $summaryDirectories -Pattern "contract-smoke-summary*.json"
    $latestData = if ($null -eq $latestDataFile) { $null } else { Read-JsonFileOrNull $latestDataFile.FullName }
    $latestFinal = if ($null -eq $latestFinalFile) { $null } else { Read-JsonFileOrNull $latestFinalFile.FullName }
    $latestReadOnly = if ($null -eq $latestReadOnlyFile) { $null } else { Read-JsonFileOrNull $latestReadOnlyFile.FullName }
    $contextProjectionState = Get-Fb2ContextProjectionLogState -Path $(if ($null -eq $latestAiCenterLogFile) { "" } else { $latestAiCenterLogFile.FullName })
    $sampleRequestState = Get-Fb2ContextSampleRequestState -Path $(if ($null -eq $latestSampleRequestFile) { "" } else { $latestSampleRequestFile.FullName })
    $sampleSetState = Get-Fb2ContextSampleSetState -Path $(if ($null -eq $latestSampleSetFile) { "" } else { $latestSampleSetFile.FullName })
    $answerReadinessState = Get-Fb2ContextAnswerReadinessState -SampleSetState $sampleSetState
    $domainDataBlueprint = Get-Fb2DomainDataBlueprintState
    $publicContractStatus = Get-Fb2PublicContractSummaryState -Path $(if ($null -eq $latestPublicContractFile) { "" } else { $latestPublicContractFile.FullName })
    $contractSmokeSummary = Get-Fb2ContractSmokeSummaryState -Path $(if ($null -eq $latestContractSmokeFile) { "" } else { $latestContractSmokeFile.FullName })

    $feedbackCoverage = Get-JsonProperty $latestData "feedback_coverage"
    $finalEvidence = Get-JsonProperty $latestData "final_acceptance_evidence"
    $fullFinalFeedbackCoverage = Get-JsonProperty $latestFinal "feedback_coverage"
    $fullFinalEvidence = Get-JsonProperty $latestFinal "final_acceptance_evidence"
    $readOnlyComplete = Test-ReadOnlyDirectReadSummaryComplete $latestReadOnly
    $dataSuccess = Test-TruthyJsonValue (Get-JsonProperty $latestData "success")
    $feedbackComplete = Test-TruthyJsonValue (Get-JsonProperty $feedbackCoverage "complete")
    $visibleDirectReadComplete = Test-TruthyJsonValue (Get-JsonProperty $latestData "visible_direct_read_complete")
    $dataOnlyHasCurrentDirectReadGate = $null -ne (Get-JsonProperty $latestData "visible_direct_read_complete" $null)
    $dataDirectReadState = Get-Fb2DataOnlyDirectReadEvidenceState $latestData
    $dataDirectReadComplete = [bool]$dataDirectReadState.complete
    $visibleAnswerPolicyState = Get-Fb2VisibleAnswerPolicyState $latestData
    $visibleAnswerPolicyComplete = [bool]$visibleAnswerPolicyState.complete
    $fullFinalSuccess = Test-TruthyJsonValue (Get-JsonProperty $latestFinal "success")
    $fullFinalFeedbackComplete = Test-TruthyJsonValue (Get-JsonProperty $fullFinalFeedbackCoverage "complete")
    $fullFinalDirectReadState = Get-Fb2DataOnlyDirectReadEvidenceState $latestFinal
    $fullFinalDirectReadComplete = [bool]$fullFinalDirectReadState.complete
    $tokenPresent = -not [string]::IsNullOrWhiteSpace($env:FB2_AI_CENTER_TOKEN)
    $voiceEvidencePath = [string]$env:FB2_VOICE_DEVICE_EVIDENCE_PATH
    $voiceEvidencePathPresent = -not [string]::IsNullOrWhiteSpace($voiceEvidencePath)
    $goalCompletion = Get-Fb2GoalCompletionState `
        -DataSuccess $dataSuccess `
        -FeedbackComplete $feedbackComplete `
        -DataDirectReadComplete $dataDirectReadComplete `
        -ReadOnlyDirectReadComplete $readOnlyComplete `
        -VisibleAnswerPolicyComplete $visibleAnswerPolicyComplete `
        -ContextProjectionComplete ([bool]$contextProjectionState.complete) `
        -VoiceEvidencePathPresent $voiceEvidencePathPresent `
        -FinalEvidence $finalEvidence
    $userScenarioAudit = Get-Fb2UserScenarioAuditState `
        -LatestData $latestData `
        -LatestReadOnly $latestReadOnly `
        -FinalEvidence $finalEvidence `
        -FeedbackCoverage $feedbackCoverage `
        -DataDirectReadState $dataDirectReadState `
        -ContextProjectionState $contextProjectionState `
        -AnswerReadinessState $answerReadinessState
    $goalGapAudit = Get-Fb2GoalGapAuditState `
        -LatestData $latestData `
        -LatestReadOnly $latestReadOnly `
        -FeedbackCoverage $feedbackCoverage `
        -DataDirectReadState $dataDirectReadState `
        -VisibleAnswerPolicyState $visibleAnswerPolicyState `
        -ContextProjectionState $contextProjectionState `
        -SampleRequestState $sampleRequestState `
        -SampleSetState $sampleSetState `
        -AnswerReadinessState $answerReadinessState `
        -UserScenarioAudit $userScenarioAudit `
        -DomainDataBlueprint $domainDataBlueprint `
        -PublicContractStatus $publicContractStatus `
        -ContractSmokeSummary $contractSmokeSummary `
        -GoalCompletion $goalCompletion `
        -LatestDataPath $(if ($null -eq $latestDataFile) { "" } else { $latestDataFile.FullName }) `
        -LatestReadOnlyPath $(if ($null -eq $latestReadOnlyFile) { "" } else { $latestReadOnlyFile.FullName }) `
        -LatestAiCenterLogPath $(if ($null -eq $latestAiCenterLogFile) { "" } else { $latestAiCenterLogFile.FullName }) `
        -TokenPresent $tokenPresent `
        -VoiceEvidencePathPresent $voiceEvidencePathPresent
    $livePreflightRequest = Get-Fb2LivePreflightRequestState `
        -GoalGapAudit $goalGapAudit `
        -UserScenarioAudit $userScenarioAudit `
        -LatestReadOnly $latestReadOnly `
        -SampleSetState $sampleSetState `
        -TokenPresent $tokenPresent
    $coordination = Get-Fb2AiCenterCoordinationStatus `
        -LatestData $latestData `
        -LatestReadOnly $latestReadOnly `
        -FeedbackCoverage $feedbackCoverage `
        -FinalEvidence $finalEvidence `
        -DataDirectReadState $dataDirectReadState `
        -ContextProjectionState $contextProjectionState `
        -GoalCompletion $goalCompletion `
        -LatestDataPath $(if ($null -eq $latestDataFile) { "" } else { $latestDataFile.FullName }) `
        -LatestReadOnlyPath $(if ($null -eq $latestReadOnlyFile) { "" } else { $latestReadOnlyFile.FullName }) `
        -LatestAiCenterLogPath $(if ($null -eq $latestAiCenterLogFile) { "" } else { $latestAiCenterLogFile.FullName }) `
        -SampleRequestState $sampleRequestState `
        -SampleSetState $sampleSetState `
        -AnswerReadinessState $answerReadinessState `
        -UserScenarioAudit $userScenarioAudit `
        -DomainDataBlueprint $domainDataBlueprint `
        -GoalGapAudit $goalGapAudit `
        -LivePreflightRequest $livePreflightRequest `
        -TokenPresent $tokenPresent `
        -VoiceEvidencePathPresent $voiceEvidencePathPresent

    $blockers = @()
    if (-not $voiceEvidencePathPresent) {
        $blockers += "missing_FB2_VOICE_DEVICE_EVIDENCE_PATH_for_full_final"
    }
    if ($null -eq $latestData) {
        $blockers += "missing_data_only_acceptance_summary"
    }
    if ($null -ne $latestData -and -not $dataDirectReadComplete) {
        $blockers += "latest_data_only_summary_predates_visible_direct_read_complete_gate"
    }
    if ($null -ne $latestData -and -not $visibleAnswerPolicyComplete) {
        $blockers += "latest_data_only_summary_missing_visible_answer_policy_evidence"
    }
    if (-not $readOnlyComplete) {
        $blockers += "missing_or_incomplete_read_only_direct_group_read_summary"
    }
    if (-not [bool]$contextProjectionState.complete) {
        $blockers += "missing_or_incomplete_context_projection_log_evidence"
    }
    if (-not $tokenPresent -and -not [bool]$sampleRequestState.complete) {
        $blockers += "missing_or_incomplete_context_pack_sample_request_for_tokenless_handoff"
    }

    $refreshGaps = @()
    if (-not $tokenPresent) {
        $refreshGaps += "missing_FB2_AI_CENTER_TOKEN_for_refreshing_live_context_pack_permission_quality"
        if ([bool]$sampleSetState.complete) {
            $refreshGaps += "context_pack_exported_samples_validated_offline"
            if ([bool]$answerReadinessState.complete) {
                $refreshGaps += "context_answer_readiness_validated_offline"
            }
        } elseif ([bool]$sampleRequestState.complete) {
            $refreshGaps += "context_pack_sample_request_ready_for_fb2_export"
        }
    }
    if ($null -ne $latestData -and -not $dataOnlyHasCurrentDirectReadGate -and $dataDirectReadComplete) {
        $refreshGaps += "latest_data_only_summary_uses_legacy_visible_direct_read_evidence_object"
    }
    if (@($visibleAnswerPolicyState.optional_missing).Count -gt 0) {
        $refreshGaps += "latest_data_only_summary_uses_legacy_visible_answer_policy_optional_fields"
    }
    if (-not [bool]$publicContractStatus.complete) {
        $refreshGaps += "missing_or_incomplete_public_contract_status_summary"
    }
    if (-not [bool]$contractSmokeSummary.complete) {
        $refreshGaps += "missing_or_incomplete_contract_smoke_summary"
    }

    $nextActions = @()
    if (-not $tokenPresent) {
        if ([bool]$sampleSetState.complete) {
            $nextActions += "offline_context_pack_samples_validated_set_FB2_AI_CENTER_TOKEN_to_refresh_live_permission_quality"
        } elseif ([bool]$sampleRequestState.complete) {
            $nextActions += "send_context_pack_sample_request_to_fb2_or_wait_for_exported_samples"
        } else {
            $nextActions += "generate_context_pack_sample_request_or_set_FB2_AI_CENTER_TOKEN"
        }
        } elseif ($dataSuccess -and $feedbackComplete -and ($dataDirectReadComplete -or $readOnlyComplete)) {
            $nextActions += "run_DataOnlyAcceptance_AllowVisibleMessages_for_non_voice_regression_if_user_allows_visible_messages"
    } else {
        $nextActions += "rerun_DataOnlyAcceptance_PreflightOnly_to_refresh_live_context_permission_quality_summary"
    }
    if (-not $voiceEvidencePathPresent) {
        $nextActions += "keep_ASR_TTS_paused_until_final_ready_voice_device_evidence_is_available"
    }

    [ordered]@{
        schema = "fb2.main_project.status_snapshot.v1"
        generated_at = (Get-Date).ToUniversalTime().ToString("o")
        summary_dir = $Directory
        summary_dirs = @($summaryDirectories)
        repo = [ordered]@{
            branch = Get-GitValueOrEmpty -GitArgs @("-C", $root, "branch", "--show-current")
            head = Get-GitValueOrEmpty -GitArgs @("-C", $root, "rev-parse", "--short=8", "HEAD")
            origin_main = Get-GitValueOrEmpty -GitArgs @("-C", $root, "rev-parse", "--short=8", "origin/main")
            status = Get-GitValueOrEmpty -GitArgs @("-C", $root, "status", "--short", "--branch")
        }
        environment = [ordered]@{
            fb2_ai_center_token_present = $tokenPresent
            voice_device_evidence_path_present = $voiceEvidencePathPresent
            voice_device_evidence_path = if ($voiceEvidencePathPresent) { $voiceEvidencePath } else { "" }
        }
        validation_scope = [ordered]@{
            group_chat_evidence = "api_direct_read_summary_only"
            group_chat_api = "/api/me/groups/{group_id}/messages"
            screenshots_accepted_for_group_chat = $false
            writes_group_messages = $false
            stores_message_body = $false
        }
        latest_data_only_acceptance = [ordered]@{
            path = if ($null -eq $latestDataFile) { "" } else { $latestDataFile.FullName }
            exists = $null -ne $latestData
            success = $dataSuccess
            mode = [string](Get-JsonProperty $latestData "mode" "")
            acceptance_scope = [string](Get-JsonProperty $latestData "acceptance_scope" "")
            voice_status = [string](Get-JsonProperty $latestData "voice_status" "")
            feedback_complete = $feedbackComplete
            visible_direct_read_complete = $visibleDirectReadComplete
            has_current_direct_read_gate = $dataOnlyHasCurrentDirectReadGate
            direct_read_evidence_complete = $dataDirectReadComplete
            direct_read_evidence_mode = [string]$dataDirectReadState.mode
            direct_read_evidence_missing = @($dataDirectReadState.missing)
            visible_answer_policy_complete = $visibleAnswerPolicyComplete
            visible_answer_policy_mode = [string]$visibleAnswerPolicyState.mode
            visible_answer_policy_missing = @($visibleAnswerPolicyState.missing)
            visible_answer_policy_optional_missing = @($visibleAnswerPolicyState.optional_missing)
            summary_post_ready_for_mode = Test-TruthyJsonValue (Get-JsonProperty $latestData "summary_post_ready_for_mode")
            final_acceptance_exit_code = [string](Get-JsonProperty $latestData "final_acceptance_exit_code" "")
            visible_chat_exit_code = [string](Get-JsonProperty $latestData "visible_chat_exit_code" "")
            scenario_my_ticket_orders = [string](Get-JsonProperty $finalEvidence "scenario_my_ticket_orders" "")
            platform_order_summary = [string](Get-JsonProperty $finalEvidence "scenario_platform_order_summary" "")
            permission_total_blocks = [string](Get-JsonProperty $finalEvidence "permission_total_blocks" "")
            quality_unmatched_cited_sources = [string](Get-JsonProperty $finalEvidence "quality_unmatched_cited_sources" "")
            quality_non_synthetic_adoption_count = [string](Get-JsonProperty $finalEvidence "quality_non_synthetic_adoption_count" "")
        }
        latest_final_acceptance = [ordered]@{
            path = if ($null -eq $latestFinalFile) { "" } else { $latestFinalFile.FullName }
            exists = $null -ne $latestFinal
            success = $fullFinalSuccess
            mode = [string](Get-JsonProperty $latestFinal "mode" "")
            acceptance_scope = [string](Get-JsonProperty $latestFinal "acceptance_scope" "")
            voice_status = [string](Get-JsonProperty $latestFinal "voice_status" "")
            feedback_complete = $fullFinalFeedbackComplete
            direct_read_evidence_complete = $fullFinalDirectReadComplete
            direct_read_evidence_mode = [string]$fullFinalDirectReadState.mode
            final_acceptance_exit_code = [string](Get-JsonProperty $latestFinal "final_acceptance_exit_code" "")
            visible_chat_exit_code = [string](Get-JsonProperty $latestFinal "visible_chat_exit_code" "")
            scenario_my_ticket_orders = [string](Get-JsonProperty $fullFinalEvidence "scenario_my_ticket_orders" "")
            platform_order_summary = [string](Get-JsonProperty $fullFinalEvidence "scenario_platform_order_summary" "")
            permission_total_blocks = [string](Get-JsonProperty $fullFinalEvidence "permission_total_blocks" "")
            quality_unmatched_cited_sources = [string](Get-JsonProperty $fullFinalEvidence "quality_unmatched_cited_sources" "")
            quality_non_synthetic_adoption_count = [string](Get-JsonProperty $fullFinalEvidence "quality_non_synthetic_adoption_count" "")
        }
        latest_read_only_direct_read = [ordered]@{
            path = if ($null -eq $latestReadOnlyFile) { "" } else { $latestReadOnlyFile.FullName }
            exists = $null -ne $latestReadOnly
            complete = $readOnlyComplete
            evidence = Build-ReadOnlyDirectReadEvidence $latestReadOnly
        }
        latest_ai_center_context_projection = $contextProjectionState
        latest_context_pack_sample_request = $sampleRequestState
        latest_context_pack_sample_set = $sampleSetState
        latest_context_answer_readiness = $answerReadinessState
        latest_user_scenario_audit = $userScenarioAudit
        latest_domain_data_blueprint = $domainDataBlueprint
        latest_public_contract_status = $publicContractStatus
        latest_contract_smoke_summary = $contractSmokeSummary
        readiness = [ordered]@{
            non_voice_historical_evidence_ready = ($dataSuccess -and $feedbackComplete -and ($dataDirectReadComplete -or $readOnlyComplete) -and $visibleAnswerPolicyComplete -and [bool]$contextProjectionState.complete)
            full_final_ready = $false
            asr_tts_status = if ($voiceEvidencePathPresent) { "voice_evidence_path_configured_but_not_verified_by_this_status_script" } else { "deferred_or_missing" }
        }
        goal_completion = $goalCompletion
        goal_gap_audit = $goalGapAudit
        live_preflight_request = $livePreflightRequest
        coordination = $coordination
        blockers = $blockers
        refresh_gaps = $refreshGaps
        next_actions = $nextActions
    }
}

function Invoke-Fb2StatusSelfTest {
    $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-ai-status-{0}" -f ([guid]::NewGuid().ToString("N")))
    New-Item -ItemType Directory -Path $tmp -Force | Out-Null
    try {
        $data = [ordered]@{
            schema = "fb2.main_project.final_acceptance.v1"
            mode = "visible_data_only_acceptance"
            voice_status = "deferred_by_user"
            success = $true
            summary_post_ready_for_mode = $true
            visible_chat_exit_code = 0
            final_acceptance_exit_code = 0
            feedback_coverage = [ordered]@{ complete = $true }
            visible_direct_read_evidence = [ordered]@{
                api = "/api/me/groups/{group_id}/messages and /api/me/groups/{group_id}/summary-posts/{post_id}"
                baseline_messages = "group=ext_fb2_official count=80 sample_message=gai_base text_len=292 text_sha256=abcdef0123456789"
                visible_mention_seed = "group=ext_fb2_official message=gmsg_seed text_len=83 text_sha256=abcdef0123456789"
                visible_mention_reply = "group=ext_fb2_official message=gai_reply text_len=448 text_sha256=abcdef0123456789"
                selected_message_seed = "group=ext_fb2_official message=gmsg_selected text_len=71 text_sha256=abcdef0123456789"
                selected_message_reply = "group=ext_fb2_official message=gai_selected text_len=292 text_sha256=abcdef0123456789"
                summary_post = "group=ext_fb2_official post=gsp_summary status=ready text_len=2291 text_sha256=abcdef0123456789"
            }
            visible_answer_policy_evidence = [ordered]@{
                visible_mention_reply_text = "length=448"
                visible_mention_sources = "patterns=来源|match_id|context_audit_id"
                visible_mention_fact_split = "patterns=数据事实|AI推断"
                visible_mention_risk_boundary = "patterns=风险边界|不保证"
                visible_mention_no_guarantee = ""
                selected_message_reply_text = "length=292"
                selected_message_sources = "patterns=来源|message_id"
                selected_message_fact_split = "patterns=数据事实|AI推断"
                selected_message_risk_boundary = "patterns=风险边界|不建议"
                selected_message_no_guarantee = ""
                selected_message_rejects_claim = "patterns=不合理|风险"
                selected_message_references_claim = "patterns=肯定赢盘|重注"
                summary_post_text = "length=2291"
                summary_post_sources = "patterns=来源|message_id"
                summary_post_fact_split = "patterns=数据事实|群友观点|AI推断"
                summary_post_risk_boundary = "patterns=风险边界|不诱导"
                summary_post_no_guarantee = ""
                summary_post_model_ready = "model_ready=True fallback_used=False fallback_allowed=False"
            }
            final_acceptance_evidence = [ordered]@{
                scenario_my_ticket_orders = "count=10 min=1"
                scenario_platform_order_summary = "count=1 min=1"
                permission_total_blocks = "value=4"
                quality_unmatched_cited_sources = "value=0"
                quality_non_synthetic_adoption_count = "value=1 min=1"
            }
        }
        $readOnly = [ordered]@{
            schema = "fb2.main_project.visible_chat_readonly.v1"
            mode = "read_only_direct_read"
            writes = $false
            group_id = "ext_fb2_official"
            direct_read_complete = $true
            message_count = 80
            sample_message_id = "gai_sample"
            sample_sender = "usr_elon_ai"
            sample_text_len = 292
            sample_text_sha256 = "abcdef0123456789"
            direct_read_evidence = "group=ext_fb2_official count=80 sample_message=gai_sample text_len=292 text_sha256=abcdef0123456789"
            api = "/api/me/groups/{group_id}/messages"
        }
        $data | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $tmp "data-only-acceptance-test.json") -Encoding UTF8
        $readOnly | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $tmp "read-only-direct-read-test.json") -Encoding UTF8
        [ordered]@{
            schema = "fb2.main_project.public_contract_status.v1"
            main_base = "http://example.invalid"
            server = [ordered]@{
                health = "OK"
                versionName = "selftest"
                gitSha = "abc123"
            }
            success = $true
            passed_count = 41
            failed_count = 0
            failed_checks = @()
            contract_summary = [ordered]@{
                domain_data_blueprint_schema = "fb2.main_project.domain_data_blueprint.v1"
                domain_context_index_schema = "fb2.main_project.domain_context_index.v1"
                domain_context_index_count = 8
                domain_context_index_ids = @(
                    "match_index",
                    "odds_snapshot_index",
                    "current_user_ticket_index",
                    "platform_order_risk_index",
                    "group_opinion_index",
                    "opinion_memory_index",
                    "context_audit_index",
                    "feedback_quality_index"
                )
                context_pack_template_schema = "fb2.context_pack_template.v1"
                context_pack_template_wrapper = "fb2_context_pack"
                context_pack_template_sections = @("usage_boundary", "match_facts", "user_order_slice", "platform_order_summary", "group_opinion_slice", "retrieval_evidence", "quality_feedback")
                domain_lane_count = 6
                stores_fb2_business_data_in_main_project = $false
                group_chat_evidence_schema = "fb2.main_project.group_chat_evidence.v1"
                group_chat_test_method = "direct_api_read"
                screenshots_accepted = $false
                required_group_message_fields = @("message_id", "type", "sender_id", "created_at", "text_len", "text_sha256")
                live_tool_count = 3
            }
            limitations = @(
                "public_contract_only_no_fb2_service_token_required",
                "does_not_verify_fb2_live_context_pack_or_orders",
                "does_not_write_or_read_visible_group_flow_beyond_public_contract",
                "does_not_replace_DataOnlyAcceptance_or_FinalAcceptance"
            )
        } | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $tmp "public-contract-status-test.json") -Encoding UTF8
        [ordered]@{
            schema = "fb2.main_project.contract_smoke_summary.v1"
            generated_at = "2026-01-01T00:00:00Z"
            main_base = "http://example.invalid"
            fb2_base = "http://fb2.example.invalid"
            group_id = "official"
            external_user_id = "6fe5aa17-0403-427a-8e91-7f414beca35d"
            fb2_ai_center_token_present = $false
            require_fb2_live = $false
            require_no_skips = $false
            skip_voice_contract_checks = $false
            success = $true
            complete = $true
            failed_count = 0
            skipped_count = 1
            check_count = 12
            gates = [ordered]@{
                chat_bootstrap_ready = $true
                voice_contract_ready = $true
                ai_billing_policy_ready = $true
                live_manifest_ready = $true
                domain_contract_ready = $true
                dynamic_discovery_ready = $true
                protected_service_token_boundary_ready = $true
                fb2_live_data_status = "skipped_missing_FB2_AI_CENTER_TOKEN"
            }
            failed_checks = @()
            skipped_checks = @([ordered]@{ name = "fb2 live data"; detail = "set FB2_AI_CENTER_TOKEN to verify Context Pack scenarios" })
            missing = @()
        } | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $tmp "contract-smoke-summary-test.json") -Encoding UTF8
        [ordered]@{
            schema = "fb2.main_project.context_pack_sample_request.v1"
            scenarios = @(
                [ordered]@{ id = "today_matches_context_pack"; save_as = "target/fb2-ai-center/samples/today_matches_context_pack.json"; expected_source_kinds = @("match", "odds", "context_audit"); validate_command = "validate today" },
                [ordered]@{ id = "my_ticket_context_pack"; save_as = "target/fb2-ai-center/samples/my_ticket_context_pack.json"; expected_source_kinds = @("user_order", "ticket", "context_audit"); validate_command = "validate ticket" },
                [ordered]@{ id = "platform_order_context_pack"; save_as = "target/fb2-ai-center/samples/platform_order_context_pack.json"; expected_source_kinds = @("platform_order_summary", "context_audit"); validate_command = "validate platform" },
                [ordered]@{ id = "group_opinion_context_pack"; save_as = "target/fb2-ai-center/samples/group_opinion_context_pack.json"; expected_source_kinds = @("group_message", "opinion_memory", "context_audit"); validate_command = "validate opinion" }
            )
            redaction_rules = @("Do not include service tokens.")
        } | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $tmp "context-pack-sample-request-test.json") -Encoding UTF8
        $legacySampleSetPath = Join-Path $tmp "context-pack-samples-validation-test.json"
        [ordered]@{
            schema = "fb2.main_project.context_pack_sample_set_validation.v1"
            samples_dir = (Join-Path $tmp "samples")
            complete = $true
            scenario_count = 4
            passed_count = 4
            failed_count = 0
            missing = @()
            secret_like_scenarios = @()
            scenarios = @(
                [ordered]@{ scenario = "today_matches_context_pack"; passed = $true; context_audit_id = "audit-today"; citation_source_count = 22; source_kinds = @("match", "odds", "context_audit"); context_pack_sha256 = "abc" },
                [ordered]@{ scenario = "my_ticket_context_pack"; passed = $true; context_audit_id = "audit-ticket"; citation_source_count = 42; source_kinds = @("match", "odds", "user_order", "ticket", "context_audit"); context_pack_sha256 = "def" },
                [ordered]@{ scenario = "platform_order_context_pack"; passed = $true; context_audit_id = "audit-platform"; citation_source_count = 23; source_kinds = @("platform_order_summary", "context_audit"); context_pack_sha256 = "ghi" },
                [ordered]@{ scenario = "group_opinion_context_pack"; passed = $true; context_audit_id = "audit-opinion"; citation_source_count = 23; source_kinds = @("group_message", "opinion_memory", "context_audit"); context_pack_sha256 = "jkl" }
            )
        } | ConvertTo-Json -Depth 8 | Set-Content -Path $legacySampleSetPath -Encoding UTF8
        (Get-Item -LiteralPath $legacySampleSetPath).LastWriteTimeUtc = [DateTime]::UtcNow.AddMinutes(-5)
        [ordered]@{
            schema = "fb2.main_project.context_pack_sample_set_validation.v1"
            samples_dir = (Join-Path $tmp "samples")
            complete = $true
            scenario_count = 4
            passed_count = 4
            failed_count = 0
            missing = @()
            secret_like_scenarios = @()
            scenarios = @(
                [ordered]@{ scenario = "today_matches_context_pack"; passed = $true; context_audit_id = "audit-fb2repo-today"; citation_source_count = 23; source_kinds = @("match", "odds", "context_audit", "group_message"); context_pack_sha256 = "fb2abc" },
                [ordered]@{ scenario = "my_ticket_context_pack"; passed = $true; context_audit_id = "audit-fb2repo-ticket"; citation_source_count = 43; source_kinds = @("match", "odds", "user_order", "ticket", "context_audit", "group_message"); context_pack_sha256 = "fb2def" },
                [ordered]@{ scenario = "platform_order_context_pack"; passed = $true; context_audit_id = "audit-fb2repo-platform"; citation_source_count = 24; source_kinds = @("platform_order_summary", "context_audit", "match", "odds"); context_pack_sha256 = "fb2ghi" },
                [ordered]@{ scenario = "group_opinion_context_pack"; passed = $true; context_audit_id = "audit-fb2repo-opinion"; citation_source_count = 24; source_kinds = @("group_message", "opinion_memory", "context_audit", "match", "odds"); context_pack_sha256 = "fb2jkl" }
            )
        } | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $tmp "fb2-repo-context-pack-samples-validation-current.json") -Encoding UTF8
        @(
            "OK`tcontext projection body: today matches context pack",
            "OK`tcontext projection wrapper open: today matches context pack",
            "OK`tcontext projection wrapper close: today matches context pack",
            "OK`tcontext projection audit id: today matches context pack`taudit-today",
            "OK`tcontext projection source registry: today matches context pack",
            "OK`tcontext projection section: today matches context pack/usage_boundary`tusage_boundary",
            "OK`tcontext projection section: today matches context pack/match_facts`tmatch_facts",
            "OK`tcontext projection section: today matches context pack/user_order_slice`tuser_order_slice",
            "OK`tcontext projection section: today matches context pack/platform_order_summary`tplatform_order_summary",
            "OK`tcontext projection section: today matches context pack/group_opinion_slice`tgroup_opinion_slice",
            "OK`tcontext projection section: today matches context pack/retrieval_evidence`tretrieval_evidence",
            "OK`tcontext projection section: today matches context pack/quality_feedback`tquality_feedback",
            "OK`tcontext projection source kind: today matches context pack/match`tmatch",
            "OK`tcontext projection source kind: today matches context pack/odds`todds",
            "OK`tcontext projection source kind: today matches context pack/context_audit`tcontext_audit",
            "OK`tcontext projection body: my ticket context pack",
            "OK`tcontext projection wrapper open: my ticket context pack",
            "OK`tcontext projection wrapper close: my ticket context pack",
            "OK`tcontext projection audit id: my ticket context pack`taudit-ticket",
            "OK`tcontext projection source registry: my ticket context pack",
            "OK`tcontext projection section: my ticket context pack/usage_boundary`tusage_boundary",
            "OK`tcontext projection section: my ticket context pack/match_facts`tmatch_facts",
            "OK`tcontext projection section: my ticket context pack/user_order_slice`tuser_order_slice",
            "OK`tcontext projection section: my ticket context pack/platform_order_summary`tplatform_order_summary",
            "OK`tcontext projection section: my ticket context pack/group_opinion_slice`tgroup_opinion_slice",
            "OK`tcontext projection section: my ticket context pack/retrieval_evidence`tretrieval_evidence",
            "OK`tcontext projection section: my ticket context pack/quality_feedback`tquality_feedback",
            "OK`tcontext projection source kind: my ticket context pack/user_order`tuser_order",
            "OK`tcontext projection source kind: my ticket context pack/ticket`tticket",
            "OK`tcontext projection source kind: my ticket context pack/context_audit`tcontext_audit",
            "OK`tscenario: group opinions has summary data`tcount=1 min=1",
            "OK`tscenario: platform order has summary data`tcount=1 min=1",
            "OK`tquality unmatched cited sources`tvalue=0",
            "OK`tquality non-synthetic adoption count`tvalue=1 min=1"
        ) | Set-Content -Path (Join-Path $tmp "data-only-acceptance-test-ai-center.log") -Encoding UTF8
        $snapshot = Build-Fb2AiCenterStatusSnapshot -Directory $tmp
        $failed = 0
        if (-not [bool]$snapshot.latest_data_only_acceptance.success) { $failed++ }
        if (-not [bool]$snapshot.latest_read_only_direct_read.complete) { $failed++ }
        $leakyReadOnly = $readOnly.PSObject.Copy()
        Add-Member -InputObject $leakyReadOnly -NotePropertyName "content" -NotePropertyValue "不应保存的群聊原文"
        if (Test-ReadOnlyDirectReadSummaryComplete $leakyReadOnly) { $failed++ }
        if (-not [bool]$snapshot.readiness.non_voice_historical_evidence_ready) { $failed++ }
        if (-not [bool]$snapshot.latest_data_only_acceptance.direct_read_evidence_complete) { $failed++ }
        if (-not [bool]$snapshot.latest_data_only_acceptance.visible_answer_policy_complete) { $failed++ }
        if ($snapshot.latest_data_only_acceptance.direct_read_evidence_mode -ne "legacy_evidence_object") { $failed++ }
        if (@($snapshot.blockers) -contains "latest_data_only_summary_predates_visible_direct_read_complete_gate") { $failed++ }
        if (-not (@($snapshot.refresh_gaps) -contains "latest_data_only_summary_uses_legacy_visible_direct_read_evidence_object")) { $failed++ }
        if ($snapshot.validation_scope.group_chat_evidence -ne "api_direct_read_summary_only") { $failed++ }
        if ([bool]$snapshot.validation_scope.screenshots_accepted_for_group_chat) { $failed++ }
        if (-not [bool]$snapshot.latest_ai_center_context_projection.complete) { $failed++ }
        if (-not [bool]$snapshot.latest_ai_center_context_projection.today_matches_context_pack.complete) { $failed++ }
        if (-not [bool]$snapshot.latest_ai_center_context_projection.my_ticket_context_pack.complete) { $failed++ }
        if (-not [bool]$snapshot.latest_context_pack_sample_request.complete) { $failed++ }
        if ($snapshot.latest_context_pack_sample_request.scenario_count -ne 4) { $failed++ }
        if (-not [bool]$snapshot.latest_context_pack_sample_set.complete) { $failed++ }
        if ($snapshot.latest_context_pack_sample_set.passed_count -ne 4) { $failed++ }
        if ($snapshot.latest_context_pack_sample_set.path -notmatch "fb2-repo-context-pack-samples-validation") { $failed++ }
        if (@($snapshot.latest_context_pack_sample_set.audit_ids) -notcontains "audit-fb2repo-today") { $failed++ }
        if (-not [bool]$snapshot.latest_context_answer_readiness.complete) { $failed++ }
        if ($snapshot.latest_context_answer_readiness.passed_count -ne 4) { $failed++ }
        $todayReadiness = @($snapshot.latest_context_answer_readiness.scenarios | Where-Object { $_.id -eq "today_matches_analysis" })[0]
        if ($todayReadiness.context_audit_id -ne "audit-fb2repo-today") { $failed++ }
        if ($snapshot.latest_user_scenario_audit.schema -ne "fb2.main_project.user_scenario_audit.v1") { $failed++ }
        if (-not [bool]$snapshot.latest_user_scenario_audit.complete) { $failed++ }
        if ($snapshot.latest_user_scenario_audit.scenario_count -ne 7) { $failed++ }
        if ($snapshot.latest_user_scenario_audit.complete_count -ne 7) { $failed++ }
        if ($snapshot.latest_user_scenario_audit.context_format -ne "xml_wrapped_markdown_context_pack_with_json_metadata") { $failed++ }
        if (-not ([string]$snapshot.latest_user_scenario_audit.mcp_status -match "rest_context_pack")) { $failed++ }
        if ($snapshot.latest_domain_data_blueprint.schema -ne "fb2.main_project.domain_data_blueprint.v1") { $failed++ }
        if (-not [bool]$snapshot.latest_domain_data_blueprint.complete) { $failed++ }
        if ($snapshot.latest_domain_data_blueprint.lane_count -ne 6) { $failed++ }
        if ($snapshot.latest_domain_data_blueprint.context_format -ne "xml_wrapped_markdown_context_pack_with_json_metadata") { $failed++ }
        if ([bool]$snapshot.latest_domain_data_blueprint.stores_fb2_business_data_in_main_project) { $failed++ }
        if (-not (@($snapshot.latest_domain_data_blueprint.required_context_pack_sections) -contains "group_opinion_slice")) { $failed++ }
        if (-not (@($snapshot.latest_domain_data_blueprint.required_metadata) -contains "citation_sources")) { $failed++ }
        if (-not (@($snapshot.latest_domain_data_blueprint.anti_patterns) -contains "full_database_dump")) { $failed++ }
        if ($snapshot.latest_public_contract_status.schema -ne "fb2.main_project.public_contract_status.v1") { $failed++ }
        if (-not [bool]$snapshot.latest_public_contract_status.complete) { $failed++ }
        if (-not [bool]$snapshot.latest_public_contract_status.success) { $failed++ }
        if ($snapshot.latest_public_contract_status.domain_context_index_schema -ne "fb2.main_project.domain_context_index.v1") { $failed++ }
        if ($snapshot.latest_public_contract_status.domain_context_index_count -lt 8) { $failed++ }
        if (-not (@($snapshot.latest_public_contract_status.domain_context_index_ids) -contains "group_opinion_index")) { $failed++ }
        if ($snapshot.latest_public_contract_status.group_chat_test_method -ne "direct_api_read") { $failed++ }
        if ([bool]$snapshot.latest_public_contract_status.screenshots_accepted) { $failed++ }
        if (-not (@($snapshot.latest_public_contract_status.required_group_message_fields) -contains "text_sha256")) { $failed++ }
        if (-not (@($snapshot.latest_public_contract_status.limitations) -contains "does_not_verify_fb2_live_context_pack_or_orders")) { $failed++ }
        if (@($snapshot.refresh_gaps) -contains "missing_or_incomplete_public_contract_status_summary") { $failed++ }
        if ($snapshot.latest_contract_smoke_summary.schema -ne "fb2.main_project.contract_smoke_summary.v1") { $failed++ }
        if (-not [bool]$snapshot.latest_contract_smoke_summary.complete) { $failed++ }
        if (-not [bool]$snapshot.latest_contract_smoke_summary.success) { $failed++ }
        if (-not [bool]$snapshot.latest_contract_smoke_summary.gates.chat_bootstrap_ready) { $failed++ }
        if (-not [bool]$snapshot.latest_contract_smoke_summary.gates.ai_billing_policy_ready) { $failed++ }
        if (-not [bool]$snapshot.latest_contract_smoke_summary.gates.protected_service_token_boundary_ready) { $failed++ }
        if ([string]$snapshot.latest_contract_smoke_summary.gates.fb2_live_data_status -ne "skipped_missing_FB2_AI_CENTER_TOKEN") { $failed++ }
        if (@($snapshot.refresh_gaps) -contains "missing_or_incomplete_contract_smoke_summary") { $failed++ }
        if (-not (@($snapshot.refresh_gaps) -contains "context_pack_exported_samples_validated_offline")) { $failed++ }
        if (-not (@($snapshot.refresh_gaps) -contains "context_answer_readiness_validated_offline")) { $failed++ }
        if (-not [bool]$snapshot.goal_completion.non_voice_ready) { $failed++ }
        if ([bool]$snapshot.goal_completion.full_final_ready) { $failed++ }
        if ($snapshot.goal_completion.stage -ne "non_voice_data_chat_permission_quality_ready_voice_deferred") { $failed++ }
        if (-not (@($snapshot.goal_completion.missing_items) -contains "voice_final_evidence_path_present")) { $failed++ }
        if ($snapshot.goal_gap_audit.schema -ne "fb2.main_project.goal_gap_audit.v1") { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.completed) -contains "direct_group_chat_read")) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.completed) -contains "visible_answer_policy_validated")) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.completed) -contains "context_pack_sample_set_validated")) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.completed) -contains "context_answer_readiness_validated")) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.completed) -contains "user_scenario_audit_validated")) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.completed) -contains "domain_data_blueprint_fixed")) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.completed) -contains "domain_context_index_contract")) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.completed) -contains "main_project_contract_smoke")) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.missing) -contains "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh")) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.missing) -contains "voice_final_evidence")) { $failed++ }
        if (-not [bool]$snapshot.goal_gap_audit.blocked_by_external_secret) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.deferred_by_user) -contains "ASR_TTS_final_evidence")) { $failed++ }
        if ([bool]$snapshot.goal_gap_audit.direct_read_policy.screenshots_accepted) { $failed++ }
        if ($snapshot.goal_gap_audit.next_smallest_action -ne "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly") { $failed++ }
        if (-not [bool]$snapshot.goal_gap_audit.current_flags.direct_group_chat_read_complete) { $failed++ }
        if (-not [bool]$snapshot.goal_gap_audit.current_flags.visible_answer_policy_complete) { $failed++ }
        if (-not [bool]$snapshot.goal_gap_audit.current_flags.domain_context_index_contract_complete) { $failed++ }
        if (-not [bool]$snapshot.goal_gap_audit.current_flags.main_project_contract_smoke_complete) { $failed++ }
        if ($snapshot.goal_gap_audit.evidence_refs.domain_context_index_count -ne 8) { $failed++ }
        if (-not (@($snapshot.goal_gap_audit.evidence_refs.domain_context_index_ids) -contains "odds_snapshot_index")) { $failed++ }
        if ($snapshot.goal_gap_audit.evidence_refs.contract_smoke_check_count -ne 12) { $failed++ }
        if ([string]$snapshot.goal_gap_audit.evidence_refs.contract_smoke_live_data_status -ne "skipped_missing_FB2_AI_CENTER_TOKEN") { $failed++ }
        if ($snapshot.live_preflight_request.schema -ne "fb2.main_project.live_preflight_request.v1") { $failed++ }
        if (-not [bool]$snapshot.live_preflight_request.ready_without_token) { $failed++ }
        if (-not [bool]$snapshot.live_preflight_request.blocked_by_external_secret) { $failed++ }
        if ([bool]$snapshot.live_preflight_request.writes_visible_group_messages) { $failed++ }
        if (-not (@($snapshot.live_preflight_request.missing) -contains "FB2_AI_CENTER_TOKEN")) { $failed++ }
        if (-not ([string]$snapshot.live_preflight_request.commands.data_only_preflight -match "DataOnlyAcceptance")) { $failed++ }
        if (-not ([string]$snapshot.live_preflight_request.commands.visible_regression_requires_authorization -match "AllowVisibleMessages")) { $failed++ }
        if ([string]$snapshot.live_preflight_request.target_user.external_user_id -ne "6fe5aa17-0403-427a-8e91-7f414beca35d") { $failed++ }
        if ([string]$snapshot.live_preflight_request.evidence_policy.group_chat_test_method -ne "direct_api_read") { $failed++ }
        if ([bool]$snapshot.live_preflight_request.evidence_policy.screenshots_accepted) { $failed++ }
        if (-not (@($snapshot.live_preflight_request.evidence_policy.required_group_message_fields) -contains "text_sha256")) { $failed++ }
        if ($snapshot.coordination.schema -ne "fb2.main_project.coordination.v1") { $failed++ }
        if ($snapshot.coordination.summary -ne "non_voice_ready_voice_deferred") { $failed++ }
        if (-not [bool]$snapshot.coordination.acceptance_scope.non_voice_ready) { $failed++ }
        if ([bool]$snapshot.coordination.direct_read_policy.screenshots_accepted) { $failed++ }
        if ([bool]$snapshot.coordination.direct_read_policy.writes_group_messages_in_status) { $failed++ }
        if (-not [bool]$snapshot.coordination.context_pack_sample_set.complete) { $failed++ }
        if (-not [bool]$snapshot.coordination.context_answer_readiness.complete) { $failed++ }
        if (-not [bool]$snapshot.coordination.user_scenario_audit.complete) { $failed++ }
        if (-not [bool]$snapshot.coordination.domain_data_blueprint.complete) { $failed++ }
        if ($snapshot.coordination.domain_data_blueprint.lane_count -ne 6) { $failed++ }
        if (-not ([string]$snapshot.coordination.domain_data_blueprint.mcp_status -match "future_wrapper")) { $failed++ }
        if (-not [bool]$snapshot.coordination.live_preflight_request.ready_without_token) { $failed++ }
        if ([string]$snapshot.coordination.live_preflight_request.group_chat_test_method -ne "direct_api_read") { $failed++ }
        if ([bool]$snapshot.coordination.live_preflight_request.screenshots_accepted) { $failed++ }
        if ([string]$snapshot.coordination.current_evidence.visible_group_id -ne "ext_fb2_official") { $failed++ }
        if ([string]$snapshot.coordination.current_evidence.visible_mention_reply_id -ne "") { $failed++ }
        if (-not ([string]$snapshot.coordination.safe_commands.visible_regression_requires_authorization -match "-AllowVisibleMessages")) { $failed++ }
        if (-not ([string]$snapshot.coordination.next_action_by_owner.fb2_project -match "non-voice")) { $failed++ }
        if ([string]::IsNullOrWhiteSpace([string]$snapshot.repo.head)) { $failed++ }
        Write-Output "== SelfTest Summary =="
        Write-Output "failed=$failed"
        if ($failed -gt 0) {
            exit 1
        }
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($SelfTest) {
    Invoke-Fb2StatusSelfTest
    exit 0
}

$snapshot = Build-Fb2AiCenterStatusSnapshot -Directory $SummaryDir -ExtraDirectories $EvidenceDirs
$json = $snapshot | ConvertTo-Json -Depth 10
if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $dir = Split-Path -Parent $OutputPath
    if ($dir -and -not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    Set-Content -Path $OutputPath -Value $json -Encoding UTF8
}
Write-Output $json
