#requires -Version 7.0

param(
    [string]$SummaryDir = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "fb2-visible-readonly-validation.ps1")
. (Join-Path $PSScriptRoot "fb2-data-only-direct-read-validation.ps1")
. (Join-Path $PSScriptRoot "fb2-context-projection-log-validation.ps1")
. (Join-Path $PSScriptRoot "fb2-context-sample-request-status.ps1")
. (Join-Path $PSScriptRoot "fb2-context-sample-set-status.ps1")
. (Join-Path $PSScriptRoot "fb2-goal-readiness-status.ps1")
. (Join-Path $PSScriptRoot "fb2-ai-center-coordination-status.ps1")

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
    param([string]$Directory)

    $root = Get-Fb2StatusRepoRoot
    if ([string]::IsNullOrWhiteSpace($Directory)) {
        $Directory = Join-Path $root "target\fb2-ai-center"
    }

    $latestDataFile = Get-LatestFileByPattern -Directory $Directory -Pattern "data-only-acceptance-*.json"
    $latestReadOnlyFile = Get-LatestFileByPattern -Directory $Directory -Pattern "read-only-direct-read*.json"
    $latestAiCenterLogFile = Get-LatestFileByPattern -Directory $Directory -Pattern "*ai-center.log"
    $latestSampleRequestFile = Get-LatestFileByPattern -Directory $Directory -Pattern "context-pack-sample-request*.json"
    $latestSampleSetFile = Get-LatestFileByPattern -Directory $Directory -Pattern "context-pack-samples-validation*.json"
    $latestData = if ($null -eq $latestDataFile) { $null } else { Read-JsonFileOrNull $latestDataFile.FullName }
    $latestReadOnly = if ($null -eq $latestReadOnlyFile) { $null } else { Read-JsonFileOrNull $latestReadOnlyFile.FullName }
    $contextProjectionState = Get-Fb2ContextProjectionLogState -Path $(if ($null -eq $latestAiCenterLogFile) { "" } else { $latestAiCenterLogFile.FullName })
    $sampleRequestState = Get-Fb2ContextSampleRequestState -Path $(if ($null -eq $latestSampleRequestFile) { "" } else { $latestSampleRequestFile.FullName })
    $sampleSetState = Get-Fb2ContextSampleSetState -Path $(if ($null -eq $latestSampleSetFile) { "" } else { $latestSampleSetFile.FullName })

    $feedbackCoverage = Get-JsonProperty $latestData "feedback_coverage"
    $finalEvidence = Get-JsonProperty $latestData "final_acceptance_evidence"
    $readOnlyComplete = Test-ReadOnlyDirectReadSummaryComplete $latestReadOnly
    $dataSuccess = Test-TruthyJsonValue (Get-JsonProperty $latestData "success")
    $feedbackComplete = Test-TruthyJsonValue (Get-JsonProperty $feedbackCoverage "complete")
    $visibleDirectReadComplete = Test-TruthyJsonValue (Get-JsonProperty $latestData "visible_direct_read_complete")
    $dataOnlyHasCurrentDirectReadGate = $null -ne (Get-JsonProperty $latestData "visible_direct_read_complete" $null)
    $dataDirectReadState = Get-Fb2DataOnlyDirectReadEvidenceState $latestData
    $dataDirectReadComplete = [bool]$dataDirectReadState.complete
    $tokenPresent = -not [string]::IsNullOrWhiteSpace($env:FB2_AI_CENTER_TOKEN)
    $voiceEvidencePath = [string]$env:FB2_VOICE_DEVICE_EVIDENCE_PATH
    $voiceEvidencePathPresent = -not [string]::IsNullOrWhiteSpace($voiceEvidencePath)
    $goalCompletion = Get-Fb2GoalCompletionState `
        -DataSuccess $dataSuccess `
        -FeedbackComplete $feedbackComplete `
        -DataDirectReadComplete $dataDirectReadComplete `
        -ReadOnlyDirectReadComplete $readOnlyComplete `
        -ContextProjectionComplete ([bool]$contextProjectionState.complete) `
        -VoiceEvidencePathPresent $voiceEvidencePathPresent `
        -FinalEvidence $finalEvidence
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
        } elseif ([bool]$sampleRequestState.complete) {
            $refreshGaps += "context_pack_sample_request_ready_for_fb2_export"
        }
    }
    if ($null -ne $latestData -and -not $dataOnlyHasCurrentDirectReadGate -and $dataDirectReadComplete) {
        $refreshGaps += "latest_data_only_summary_uses_legacy_visible_direct_read_evidence_object"
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
            voice_status = [string](Get-JsonProperty $latestData "voice_status" "")
            feedback_complete = $feedbackComplete
            visible_direct_read_complete = $visibleDirectReadComplete
            has_current_direct_read_gate = $dataOnlyHasCurrentDirectReadGate
            direct_read_evidence_complete = $dataDirectReadComplete
            direct_read_evidence_mode = [string]$dataDirectReadState.mode
            direct_read_evidence_missing = @($dataDirectReadState.missing)
            summary_post_ready_for_mode = Test-TruthyJsonValue (Get-JsonProperty $latestData "summary_post_ready_for_mode")
            final_acceptance_exit_code = [string](Get-JsonProperty $latestData "final_acceptance_exit_code" "")
            visible_chat_exit_code = [string](Get-JsonProperty $latestData "visible_chat_exit_code" "")
            scenario_my_ticket_orders = [string](Get-JsonProperty $finalEvidence "scenario_my_ticket_orders" "")
            platform_order_summary = [string](Get-JsonProperty $finalEvidence "scenario_platform_order_summary" "")
            permission_total_blocks = [string](Get-JsonProperty $finalEvidence "permission_total_blocks" "")
            quality_unmatched_cited_sources = [string](Get-JsonProperty $finalEvidence "quality_unmatched_cited_sources" "")
            quality_non_synthetic_adoption_count = [string](Get-JsonProperty $finalEvidence "quality_non_synthetic_adoption_count" "")
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
        readiness = [ordered]@{
            non_voice_historical_evidence_ready = ($dataSuccess -and $feedbackComplete -and ($dataDirectReadComplete -or $readOnlyComplete) -and [bool]$contextProjectionState.complete)
            full_final_ready = $false
            asr_tts_status = if ($voiceEvidencePathPresent) { "voice_evidence_path_configured_but_not_verified_by_this_status_script" } else { "deferred_or_missing" }
        }
        goal_completion = $goalCompletion
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
            schema = "fb2.main_project.context_pack_sample_request.v1"
            scenarios = @(
                [ordered]@{ id = "today_matches_context_pack"; save_as = "target/fb2-ai-center/samples/today_matches_context_pack.json"; expected_source_kinds = @("match", "odds", "context_audit"); validate_command = "validate today" },
                [ordered]@{ id = "my_ticket_context_pack"; save_as = "target/fb2-ai-center/samples/my_ticket_context_pack.json"; expected_source_kinds = @("user_order", "ticket", "context_audit"); validate_command = "validate ticket" },
                [ordered]@{ id = "platform_order_context_pack"; save_as = "target/fb2-ai-center/samples/platform_order_context_pack.json"; expected_source_kinds = @("platform_order_summary", "context_audit"); validate_command = "validate platform" },
                [ordered]@{ id = "group_opinion_context_pack"; save_as = "target/fb2-ai-center/samples/group_opinion_context_pack.json"; expected_source_kinds = @("group_message", "opinion_memory", "context_audit"); validate_command = "validate opinion" }
            )
            redaction_rules = @("Do not include service tokens.")
        } | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $tmp "context-pack-sample-request-test.json") -Encoding UTF8
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
        } | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $tmp "context-pack-samples-validation-test.json") -Encoding UTF8
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
        if (-not [bool]$snapshot.readiness.non_voice_historical_evidence_ready) { $failed++ }
        if (-not [bool]$snapshot.latest_data_only_acceptance.direct_read_evidence_complete) { $failed++ }
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
        if (-not (@($snapshot.refresh_gaps) -contains "context_pack_exported_samples_validated_offline")) { $failed++ }
        if (-not [bool]$snapshot.goal_completion.non_voice_ready) { $failed++ }
        if ([bool]$snapshot.goal_completion.full_final_ready) { $failed++ }
        if ($snapshot.goal_completion.stage -ne "non_voice_data_chat_permission_quality_ready_voice_deferred") { $failed++ }
        if (-not (@($snapshot.goal_completion.missing_items) -contains "voice_final_evidence_path_present")) { $failed++ }
        if ($snapshot.coordination.schema -ne "fb2.main_project.coordination.v1") { $failed++ }
        if ($snapshot.coordination.summary -ne "non_voice_ready_voice_deferred") { $failed++ }
        if (-not [bool]$snapshot.coordination.acceptance_scope.non_voice_ready) { $failed++ }
        if ([bool]$snapshot.coordination.direct_read_policy.screenshots_accepted) { $failed++ }
        if ([bool]$snapshot.coordination.direct_read_policy.writes_group_messages_in_status) { $failed++ }
        if (-not [bool]$snapshot.coordination.context_pack_sample_set.complete) { $failed++ }
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

$snapshot = Build-Fb2AiCenterStatusSnapshot -Directory $SummaryDir
$json = $snapshot | ConvertTo-Json -Depth 10
if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $dir = Split-Path -Parent $OutputPath
    if ($dir -and -not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    Set-Content -Path $OutputPath -Value $json -Encoding UTF8
}
Write-Output $json
