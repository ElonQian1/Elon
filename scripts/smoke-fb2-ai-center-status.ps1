#requires -Version 7.0

param(
    [string]$SummaryDir = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "fb2-visible-readonly-validation.ps1")

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
    $latestData = if ($null -eq $latestDataFile) { $null } else { Read-JsonFileOrNull $latestDataFile.FullName }
    $latestReadOnly = if ($null -eq $latestReadOnlyFile) { $null } else { Read-JsonFileOrNull $latestReadOnlyFile.FullName }

    $feedbackCoverage = Get-JsonProperty $latestData "feedback_coverage"
    $finalEvidence = Get-JsonProperty $latestData "final_acceptance_evidence"
    $readOnlyComplete = Test-ReadOnlyDirectReadSummaryComplete $latestReadOnly
    $dataSuccess = Test-TruthyJsonValue (Get-JsonProperty $latestData "success")
    $feedbackComplete = Test-TruthyJsonValue (Get-JsonProperty $feedbackCoverage "complete")
    $visibleDirectReadComplete = Test-TruthyJsonValue (Get-JsonProperty $latestData "visible_direct_read_complete")
    $dataOnlyHasCurrentDirectReadGate = $null -ne (Get-JsonProperty $latestData "visible_direct_read_complete" $null)
    $tokenPresent = -not [string]::IsNullOrWhiteSpace($env:FB2_AI_CENTER_TOKEN)
    $voiceEvidencePath = [string]$env:FB2_VOICE_DEVICE_EVIDENCE_PATH
    $voiceEvidencePathPresent = -not [string]::IsNullOrWhiteSpace($voiceEvidencePath)

    $blockers = @()
    if (-not $tokenPresent) {
        $blockers += "missing_FB2_AI_CENTER_TOKEN_for_live_context_pack_permission_quality"
    }
    if (-not $voiceEvidencePathPresent) {
        $blockers += "missing_FB2_VOICE_DEVICE_EVIDENCE_PATH_for_full_final"
    }
    if ($null -eq $latestData) {
        $blockers += "missing_data_only_acceptance_summary"
    }
    if ($null -ne $latestData -and -not $dataOnlyHasCurrentDirectReadGate) {
        $blockers += "latest_data_only_summary_predates_visible_direct_read_complete_gate"
    }
    if (-not $readOnlyComplete) {
        $blockers += "missing_or_incomplete_read_only_direct_group_read_summary"
    }

    $nextActions = @()
    if (-not $tokenPresent) {
        $nextActions += "set_FB2_AI_CENTER_TOKEN_or_use_controlled_wrapper_then_run_DataOnlyAcceptance_PreflightOnly"
    } elseif ($dataSuccess -and $feedbackComplete -and ($visibleDirectReadComplete -or $readOnlyComplete)) {
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
        readiness = [ordered]@{
            non_voice_historical_evidence_ready = ($dataSuccess -and $feedbackComplete -and ($visibleDirectReadComplete -or $readOnlyComplete))
            full_final_ready = $false
            asr_tts_status = if ($voiceEvidencePathPresent) { "voice_evidence_path_configured_but_not_verified_by_this_status_script" } else { "deferred_or_missing" }
        }
        blockers = $blockers
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
            visible_direct_read_complete = $true
            summary_post_ready_for_mode = $true
            visible_chat_exit_code = 0
            final_acceptance_exit_code = 0
            feedback_coverage = [ordered]@{ complete = $true }
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
        $snapshot = Build-Fb2AiCenterStatusSnapshot -Directory $tmp
        $failed = 0
        if (-not [bool]$snapshot.latest_data_only_acceptance.success) { $failed++ }
        if (-not [bool]$snapshot.latest_read_only_direct_read.complete) { $failed++ }
        if (-not [bool]$snapshot.readiness.non_voice_historical_evidence_ready) { $failed++ }
        if ($snapshot.validation_scope.group_chat_evidence -ne "api_direct_read_summary_only") { $failed++ }
        if ([bool]$snapshot.validation_scope.screenshots_accepted_for_group_chat) { $failed++ }
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
