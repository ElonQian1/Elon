#requires -Version 7.0

param(
    [string]$MainBase = "",
    [string]$MainToken = "",
    [string]$Fb2Base = "",
    [string]$Fb2AiCenterToken = "",
    [string]$Fb2UserToken = "",
    [string]$Fb2Username = "",
    [string]$Fb2Password = "",
    [string]$GroupId = "official",
    [string]$ExternalUserId = "",
    [string]$VoiceDeviceEvidencePath = "",
    [string]$SummaryPath = "",
    [int]$RequestTimeoutSec = 45,
    [int]$PollTimeoutSec = 90,
    [int]$FeedbackPollTimeoutSec = 45,
    [int]$PollIntervalSec = 3,
    [int]$MinFeedbackCount = 2,
    [int]$MinMatchedCitedSourceCount = 2,
    [int]$MinNonSyntheticFeedbackCount = 1,
    [int]$MinOpinionAdoptionCount = 1,
    [int]$QualityFeedbackSampleLimit = 10,
    [double]$MaxLargeContextPackRate = 0.75,
    [double]$MaxCitationUnmatchedRate = 0,
    [double]$MaxMissingContextRate = 0,
    [double]$MaxWrongContextRate = 0,
    [switch]$SelfTest,
    [switch]$PreflightOnly,
    [switch]$AllowVisibleMessages
)

$ErrorActionPreference = "Stop"

if (-not $MainBase) { $MainBase = $env:ELON_MAIN_BASE }
if (-not $MainBase) { $MainBase = "http://43.139.149.158:8080" }
if (-not $MainToken) { $MainToken = $env:ELON_MAIN_TOKEN }
if (-not $Fb2Base) { $Fb2Base = $env:FB2_API_BASE }
if (-not $Fb2Base) { $Fb2Base = "http://123.207.48.146:8080" }
if (-not $Fb2AiCenterToken) { $Fb2AiCenterToken = $env:FB2_AI_CENTER_TOKEN }
if (-not $Fb2UserToken) { $Fb2UserToken = $env:FB2_USER_TOKEN }
if (-not $Fb2Username) { $Fb2Username = $env:FB2_VISIBLE_SMOKE_USERNAME }
if (-not $Fb2Password) { $Fb2Password = $env:FB2_VISIBLE_SMOKE_PASSWORD }
if (-not $ExternalUserId) { $ExternalUserId = $env:FB2_AI_CONTEXT_EXTERNAL_USER_ID }
if (-not $VoiceDeviceEvidencePath) { $VoiceDeviceEvidencePath = $env:FB2_VOICE_DEVICE_EVIDENCE_PATH }

$MainBase = $MainBase.TrimEnd("/")
$Fb2Base = $Fb2Base.TrimEnd("/")

function Fail-FinalAcceptance {
    param([string]$Message)
    Write-Output "FAIL`tfinal acceptance`t$Message"
    exit 1
}

function Add-Arg {
    param(
        [System.Collections.Generic.List[string]]$Args,
        [string]$Name,
        [object]$Value
    )
    if ($null -ne $Value -and -not [string]::IsNullOrWhiteSpace([string]$Value)) {
        [void]$Args.Add($Name)
        [void]$Args.Add([string]$Value)
    }
}

function Add-SwitchArg {
    param(
        [System.Collections.Generic.List[string]]$Args,
        [string]$Name,
        [bool]$Enabled
    )
    if ($Enabled) {
        [void]$Args.Add($Name)
    }
}

function Invoke-JsonOrNull {
    param([string]$Url)
    try {
        Invoke-RestMethod -Uri $Url -Method Get -TimeoutSec 15
    } catch {
        $null
    }
}

function Invoke-Json {
    param(
        [string]$Url,
        [hashtable]$Headers = @{},
        [string]$Method = "GET",
        [object]$Body = $null
    )

    $params = @{
        Uri = $Url
        Method = $Method
        Headers = $Headers
        TimeoutSec = 30
    }
    if ($null -ne $Body) {
        $params.ContentType = "application/json"
        $params.Body = ($Body | ConvertTo-Json -Depth 8)
    }
    Invoke-RestMethod @params
}

function Resolve-Fb2ExternalUser {
    if ($ExternalUserId) {
        return
    }
    if (-not $Fb2Username -or -not $Fb2Password) {
        return
    }

    try {
        $login = Invoke-Json -Url "$Fb2Base/api/auth/login" -Method "POST" -Body @{
            username = $Fb2Username
            password = $Fb2Password
        }
    } catch {
        Fail-FinalAcceptance "fb2 login failed before visible smoke: $($_.Exception.Message)"
    }
    if (-not $login.success -or -not $login.data.user.id) {
        Fail-FinalAcceptance "fb2 login did not return a user id."
    }

    $script:ExternalUserId = [string]$login.data.user.id
    if (-not $script:Fb2UserToken -and $login.data.token.access_token) {
        $script:Fb2UserToken = [string]$login.data.token.access_token
    }
    Write-Output "OK`tfinal acceptance external user`t$script:ExternalUserId"
}

function Test-UserOrderContextBeforeVisibleSmoke {
    $topic = [System.Uri]::EscapeDataString("帮我分析我的票")
    $group = [System.Uri]::EscapeDataString($GroupId)
    $user = [System.Uri]::EscapeDataString($ExternalUserId)
    $headers = @{
        "X-FB2-AI-CENTER-TOKEN" = $Fb2AiCenterToken.Trim()
        "X-FB2-AI-CONTEXT-USER-ID" = $ExternalUserId
    }
    $url = "$Fb2Base/api/main-project/context/pack?group_id=$group&external_user_id=$user&topic_hint=$topic&limit=3&order_limit=1"
    try {
        $pack = Invoke-Json -Url $url -Headers $headers
    } catch {
        Fail-FinalAcceptance "fb2 user order context preflight failed before visible smoke: $($_.Exception.Message)"
    }
    if (-not $pack.success) {
        Fail-FinalAcceptance "fb2 user order context preflight failed."
    }
    $orderCount = @($pack.data.user_orders | Where-Object { $_ }).Count
    if ($orderCount -lt 1) {
        Fail-FinalAcceptance "fb2 user order context preflight found no user_orders for ExternalUserId=$ExternalUserId."
    }
    Write-Output "OK`tfinal acceptance user order preflight`torders=$orderCount audit=$($pack.data.context_audit_id)"
}

function Invoke-SmokeScript {
    param(
        [string]$Name,
        [System.Collections.Generic.List[string]]$Args,
        [string]$LogPath
    )

    Write-Output ""
    Write-Output "== $Name =="
    $lines = [System.Collections.Generic.List[string]]::new()
    & pwsh @Args 2>&1 | ForEach-Object {
        $line = [string]$_
        [void]$lines.Add($line)
        Write-Output $line
    }
    $exitCode = $LASTEXITCODE
    if ($null -eq $exitCode) { $exitCode = 0 }
    if ($LogPath) {
        Set-Content -Path $LogPath -Value $lines -Encoding UTF8
    }
    Write-Output "== $Name exit_code=$exitCode =="
    return [pscustomobject]@{
        exit_code = [int]$exitCode
        log_path = $LogPath
        output = @($lines)
    }
}

function Find-CheckDetail {
    param(
        [string[]]$Lines,
        [string]$CheckName
    )

    $prefix = "OK`t$CheckName`t"
    foreach ($line in $Lines) {
        if ($line.StartsWith($prefix)) {
            return $line.Substring($prefix.Length).Trim()
        }
    }
    return ""
}

function Find-FeedbackEvidence {
    param([string[]]$Lines)

    $items = @()
    foreach ($line in $Lines) {
        if ($line -match '^OK\t(?<scenario>.+ fb2 feedback)\t(?<request>\S+) feedback=(?<feedback>\S+)') {
            $items += [pscustomobject]@{
                scenario = $Matches.scenario
                main_request_id = $Matches.request
                feedback_id = $Matches.feedback
            }
        }
    }
    return $items
}

function Build-FeedbackCoverage {
    param([object[]]$FeedbackEvidence)

    $items = @($FeedbackEvidence | Where-Object { $_ })
    $mention = @($items | Where-Object { [string]$_.scenario -eq "visible @EL fb2 feedback" }).Count -gt 0
    $selected = @($items | Where-Object { [string]$_.scenario -eq "selected-message AI回复 fb2 feedback" }).Count -gt 0
    $summaryPost = @($items | Where-Object { [string]$_.scenario -eq "summary-post fb2 feedback" }).Count -gt 0
    $missing = @()
    if (-not $mention) { $missing += "visible @EL fb2 feedback" }
    if (-not $selected) { $missing += "selected-message AI回复 fb2 feedback" }
    if (-not $summaryPost) { $missing += "summary-post fb2 feedback" }

    [ordered]@{
        required_count = 3
        observed_count = @($items).Count
        visible_mention = $mention
        selected_message = $selected
        summary_post = $summaryPost
        missing_required = $missing
        complete = ($missing.Count -eq 0)
    }
}

function Build-AiCenterEvidence {
    param([string[]]$Lines)

    [ordered]@{
        main_version = Find-CheckDetail $Lines "main version"
        live_manifest_ready = Find-CheckDetail $Lines "live manifest ready"
        fb2_integration_discovery = Find-CheckDetail $Lines "fb2 integration discovery"
        fb2_integration_routing_mode = Find-CheckDetail $Lines "fb2 integration routing mode"
        fb2_integration_context_pack = Find-CheckDetail $Lines "fb2 integration endpoint: context_pack"
        fb2_integration_tool_manifest = Find-CheckDetail $Lines "fb2 integration endpoint: tool_manifest"
        fb2_integration_group_mapping = Find-CheckDetail $Lines "fb2 integration group mapping"
        fb2_readiness_protected = Find-CheckDetail $Lines "fb2 readiness requires service token"
        fb2_tool_manifest_protected = Find-CheckDetail $Lines "fb2 tool manifest requires service token"
        fb2_authenticated_readiness = Find-CheckDetail $Lines "fb2 authenticated readiness"
        fb2_authenticated_readiness_status = Find-CheckDetail $Lines "fb2 authenticated readiness status"
        fb2_authenticated_manifest = Find-CheckDetail $Lines "fb2 authenticated tool manifest"
        fb2_authenticated_manifest_tool_ids = Find-CheckDetail $Lines "fb2 authenticated manifest tool ids"
        fb2_apk_version = Find-CheckDetail $Lines "fb2 APK version present"
        fb2_apk_download_head = Find-CheckDetail $Lines "fb2 APK download head"
        local_voice_sdk_build = Find-CheckDetail $Lines "local voice SDK build"
        voice_evidence_schema = Find-CheckDetail $Lines "voice evidence schema"
        voice_evidence_final_ready = Find-CheckDetail $Lines "voice evidence final ready"
        voice_evidence_device = Find-CheckDetail $Lines "voice evidence device model"
        voice_evidence_apk_version = Find-CheckDetail $Lines "voice evidence APK version"
        voice_evidence_uses_composer = Find-CheckDetail $Lines "voice evidence uses VoiceComposerView"
        voice_evidence_hold_to_talk = Find-CheckDetail $Lines "voice evidence hold-to-talk"
        voice_evidence_recording_overlay = Find-CheckDetail $Lines "voice evidence recording overlay"
        voice_evidence_slide_cancel = Find-CheckDetail $Lines "voice evidence slide cancel"
        voice_evidence_too_short = Find-CheckDetail $Lines "voice evidence too short"
        voice_evidence_system_asr_success = Find-CheckDetail $Lines "voice evidence system ASR success"
        voice_evidence_asr_fallback = Find-CheckDetail $Lines "voice evidence ASR timeout fallback"
        voice_evidence_server_asr_success = Find-CheckDetail $Lines "voice evidence server ASR success"
        voice_evidence_server_asr_failure_recovery = Find-CheckDetail $Lines "voice evidence server ASR failure recovery"
        voice_evidence_tts_playback = Find-CheckDetail $Lines "voice evidence TTS playback"
        voice_evidence_asr_tts_free = Find-CheckDetail $Lines "voice evidence ASR/TTS free"
        voice_evidence_artifacts = Find-CheckDetail $Lines "voice evidence artifacts"
        voice_evidence_artifact_refs_complete = Find-CheckDetail $Lines "voice evidence artifact refs complete"
        voice_evidence_artifact_logcat = Find-CheckDetail $Lines "voice evidence artifact logcat"
        voice_evidence_artifact_visual = Find-CheckDetail $Lines "voice evidence artifact visual"
        scenario_today_context_audit = Find-CheckDetail $Lines "scenario: today matches context audit"
        scenario_my_ticket_context_audit = Find-CheckDetail $Lines "scenario: my ticket context audit"
        scenario_my_ticket_orders = Find-CheckDetail $Lines "scenario: my ticket has user orders"
        scenario_platform_order_summary = Find-CheckDetail $Lines "scenario: platform order has summary data"
        permission_total_blocks = Find-CheckDetail $Lines "permission summary total blocks"
        permission_user_blocks = Find-CheckDetail $Lines "permission summary user blocks"
        permission_platform_blocks = Find-CheckDetail $Lines "permission summary platform blocks"
        quality_feedback_count = Find-CheckDetail $Lines "quality feedback count"
        quality_matched_cited_sources = Find-CheckDetail $Lines "quality matched cited sources"
        quality_unmatched_cited_sources = Find-CheckDetail $Lines "quality unmatched cited sources"
        quality_missing_context_count = Find-CheckDetail $Lines "quality missing context count"
        quality_wrong_context_count = Find-CheckDetail $Lines "quality wrong context count"
        quality_non_synthetic_feedback_count = Find-CheckDetail $Lines "quality non-synthetic feedback count"
        quality_non_synthetic_adoption_count = Find-CheckDetail $Lines "quality non-synthetic adoption count"
        quality_non_synthetic_memory_refs = Find-CheckDetail $Lines "quality non-synthetic memory refs"
    }
}

function Build-VisibleAnswerEvidence {
    param([string[]]$Lines)

    [ordered]@{
        visible_mention_reply_text = Find-CheckDetail $Lines "visible @EL reply text present"
        visible_mention_sources = Find-CheckDetail $Lines "visible @EL reply cites sources"
        visible_mention_fact_split = Find-CheckDetail $Lines "visible @EL reply separates facts and inference"
        visible_mention_risk_boundary = Find-CheckDetail $Lines "visible @EL reply includes risk boundary"
        visible_mention_no_guarantee = Find-CheckDetail $Lines "visible @EL reply avoids betting guarantees"
        selected_message_reply_text = Find-CheckDetail $Lines "selected-message reply text present"
        selected_message_sources = Find-CheckDetail $Lines "selected-message reply cites sources"
        selected_message_fact_split = Find-CheckDetail $Lines "selected-message reply separates facts and inference"
        selected_message_risk_boundary = Find-CheckDetail $Lines "selected-message reply includes risk boundary"
        selected_message_no_guarantee = Find-CheckDetail $Lines "selected-message reply avoids betting guarantees"
        selected_message_rejects_claim = Find-CheckDetail $Lines "selected-message rejects guarantee claim"
        selected_message_references_claim = Find-CheckDetail $Lines "selected-message references reviewed claim"
        summary_post_text = Find-CheckDetail $Lines "summary-post text present"
        summary_post_sources = Find-CheckDetail $Lines "summary-post cites sources"
        summary_post_fact_split = Find-CheckDetail $Lines "summary-post separates facts and inference"
        summary_post_risk_boundary = Find-CheckDetail $Lines "summary-post includes risk boundary"
        summary_post_no_guarantee = Find-CheckDetail $Lines "summary-post avoids betting guarantees"
    }
}

function Invoke-FinalAcceptanceSelfTest {
    function Assert-SelfTest {
        param(
            [bool]$Condition,
            [string]$Name,
            [string]$Detail = ""
        )
        if ($Condition) {
            if ($Detail) {
                Write-Output "OK`tself-test $Name`t$Detail"
            } else {
                Write-Output "OK`tself-test $Name"
            }
        } else {
            $script:SelfTestFailed += 1
            if ($Detail) {
                Write-Output "FAIL`tself-test $Name`t$Detail"
            } else {
                Write-Output "FAIL`tself-test $Name"
            }
        }
    }

    $script:SelfTestFailed = 0

    $completeLines = @(
        "OK`tvisible @EL fb2 feedback`tsocial_group_message:gai_visible feedback=fb_visible",
        "OK`tselected-message AI回复 fb2 feedback`tsocial_group_selected_message:gai_selected feedback=fb_selected",
        "OK`tsummary-post fb2 feedback`tsocial_group_summary_post:gsp_summary feedback=fb_summary",
        "OK`tvisible @EL reply text present`tlen=120"
    )
    $completeEvidence = @(Find-FeedbackEvidence $completeLines)
    $completeCoverage = Build-FeedbackCoverage $completeEvidence
    Assert-SelfTest ($completeEvidence.Count -eq 3) "feedback evidence parses three required entries" "count=$($completeEvidence.Count)"
    Assert-SelfTest ([bool]$completeCoverage["complete"]) "feedback coverage complete when all entries are present"
    Assert-SelfTest ([bool]$completeCoverage["visible_mention"]) "visible mention feedback covered"
    Assert-SelfTest ([bool]$completeCoverage["selected_message"]) "selected-message feedback covered"
    Assert-SelfTest ([bool]$completeCoverage["summary_post"]) "summary-post feedback covered"
    Assert-SelfTest ([int]$completeCoverage["observed_count"] -eq 3) "feedback observed count ignores unrelated OK lines" "observed=$($completeCoverage["observed_count"])"

    $missingSummaryLines = @(
        "OK`tvisible @EL fb2 feedback`tsocial_group_message:gai_visible feedback=fb_visible",
        "OK`tselected-message AI回复 fb2 feedback`tsocial_group_selected_message:gai_selected feedback=fb_selected"
    )
    $missingSummaryCoverage = Build-FeedbackCoverage @(Find-FeedbackEvidence $missingSummaryLines)
    Assert-SelfTest (-not [bool]$missingSummaryCoverage["complete"]) "feedback coverage incomplete when summary feedback is missing"
    Assert-SelfTest (@($missingSummaryCoverage["missing_required"]) -contains "summary-post fb2 feedback") "missing summary feedback is reported" ($missingSummaryCoverage["missing_required"] -join ",")

    $missingVisibleLines = @(
        "OK`tselected-message AI回复 fb2 feedback`tsocial_group_selected_message:gai_selected feedback=fb_selected",
        "OK`tsummary-post fb2 feedback`tsocial_group_summary_post:gsp_summary feedback=fb_summary"
    )
    $missingVisibleCoverage = Build-FeedbackCoverage @(Find-FeedbackEvidence $missingVisibleLines)
    Assert-SelfTest (-not [bool]$missingVisibleCoverage["complete"]) "feedback coverage incomplete when visible feedback is missing"
    Assert-SelfTest (@($missingVisibleCoverage["missing_required"]) -contains "visible @EL fb2 feedback") "missing visible feedback is reported" ($missingVisibleCoverage["missing_required"] -join ",")

    $missingSelectedLines = @(
        "OK`tvisible @EL fb2 feedback`tsocial_group_message:gai_visible feedback=fb_visible",
        "OK`tsummary-post fb2 feedback`tsocial_group_summary_post:gsp_summary feedback=fb_summary"
    )
    $missingSelectedCoverage = Build-FeedbackCoverage @(Find-FeedbackEvidence $missingSelectedLines)
    Assert-SelfTest (-not [bool]$missingSelectedCoverage["complete"]) "feedback coverage incomplete when selected-message feedback is missing"
    Assert-SelfTest (@($missingSelectedCoverage["missing_required"]) -contains "selected-message AI回复 fb2 feedback") "missing selected-message feedback is reported" ($missingSelectedCoverage["missing_required"] -join ",")

    $noisyLines = @(
        "FAIL`tvisible @EL fb2 feedback`tsocial_group_message:gai_bad feedback=fb_bad",
        "OK`tvisible @EL reply text present`tlen=120",
        "SKIP`tsummary-post fb2 feedback`tno token",
        "OK`tvisible @EL fb2 feedback`tsocial_group_message:gai_visible feedback_id=fb_visible"
    )
    $noisyEvidence = @(Find-FeedbackEvidence $noisyLines)
    Assert-SelfTest ($noisyEvidence.Count -eq 0) "feedback parser ignores non-OK and non-feedback lines" "count=$($noisyEvidence.Count)"

    $completeSuccess = ($true -and $true -and [bool]$completeCoverage["complete"])
    $missingSuccess = ($true -and $true -and [bool]$missingSummaryCoverage["complete"])
    $visibleFailedSuccess = ($false -and $true -and [bool]$completeCoverage["complete"])
    $centerFailedSuccess = ($true -and $false -and [bool]$completeCoverage["complete"])
    Assert-SelfTest $completeSuccess "final success allows complete feedback coverage"
    Assert-SelfTest (-not $missingSuccess) "final success rejects missing feedback coverage"
    Assert-SelfTest (-not $visibleFailedSuccess) "final success rejects visible smoke failure"
    Assert-SelfTest (-not $centerFailedSuccess) "final success rejects final acceptance failure"

    $centerLines = @(
        "OK`tmain version`t0.3.592 abcdef",
        "OK`tlive manifest ready`ttool_count=30",
        "OK`tfb2 integration discovery`tproject_id=fb2",
        "OK`tfb2 integration routing mode`trouting_mode=main_project_ready",
        "OK`tfb2 integration endpoint: context_pack`tcontext_pack",
        "OK`tfb2 integration endpoint: tool_manifest`ttool_manifest",
        "OK`tfb2 integration group mapping`tofficial",
        "OK`tfb2 readiness requires service token`tstatus=401 expected=401",
        "OK`tfb2 tool manifest requires service token`tstatus=401 expected=401",
        "OK`tfb2 authenticated readiness`tsuccess=True",
        "OK`tfb2 authenticated readiness status`tstatus=ready",
        "OK`tfb2 authenticated tool manifest`tsuccess=True",
        "OK`tfb2 authenticated manifest tool ids`tcount=31 min=1",
        "OK`tfb2 APK version present`t1.1.48 code=96",
        "OK`tlocal voice SDK build`t:chat-voice-kit:assembleDebug",
        "OK`tvoice evidence schema`tfb2.voice_device_evidence.v1",
        "OK`tvoice evidence final ready`tfinalAcceptanceReady=True",
        "OK`tvoice evidence device model`tXiaomi 23116PN5BC",
        "OK`tvoice evidence APK version`t1.1.48",
        "OK`tvoice evidence uses VoiceComposerView`tusesVoiceComposerView=True",
        "OK`tvoice evidence hold-to-talk`tholdToTalkButton=True",
        "OK`tvoice evidence recording overlay`trecordingOverlay=True",
        "OK`tvoice evidence slide cancel`tslideToCancel=True",
        "OK`tvoice evidence too short`ttooShort=True",
        "OK`tvoice evidence system ASR success`tsystemAsrSuccess=True",
        "OK`tvoice evidence ASR timeout fallback`tsystemAsrTimeoutServerFallback=True",
        "OK`tvoice evidence server ASR success`tserverAsrSuccess=True",
        "OK`tvoice evidence server ASR failure recovery`tserverAsrFailureRecoversUi=True",
        "OK`tvoice evidence TTS playback`tttsPlayback=True",
        "OK`tvoice evidence ASR/TTS free`tasrTtsFreeWithZeroAiBalance=True",
        "OK`tvoice evidence artifact refs complete`tvalid=2 count=2",
        "OK`tvoice evidence artifact logcat`tlogcat,screenshot",
        "OK`tvoice evidence artifact visual`tlogcat,screenshot",
        "OK`tscenario: today matches context audit`taudit_today",
        "OK`tscenario: my ticket context audit`taudit_ticket",
        "OK`tscenario: my ticket has user orders`tcount=1 min=1",
        "OK`tscenario: platform order has summary data`tcount=1 min=1",
        "OK`tpermission summary total blocks`tvalue=3",
        "OK`tpermission summary user blocks`tvalue=2",
        "OK`tpermission summary platform blocks`tvalue=1",
        "OK`tquality feedback count`tvalue=3 min=3",
        "OK`tquality matched cited sources`tvalue=3 min=3",
        "OK`tquality unmatched cited sources`tvalue=0",
        "OK`tquality missing context count`tvalue=0",
        "OK`tquality wrong context count`tvalue=0",
        "OK`tquality non-synthetic feedback count`tvalue=2 min=1",
        "OK`tquality non-synthetic adoption count`tvalue=1 min=1",
        "OK`tquality non-synthetic memory refs`tvalue=3"
    )
    $centerEvidence = Build-AiCenterEvidence $centerLines
    foreach ($key in @(
        "main_version",
        "live_manifest_ready",
        "fb2_integration_discovery",
        "fb2_integration_routing_mode",
        "fb2_integration_context_pack",
        "fb2_integration_tool_manifest",
        "fb2_integration_group_mapping",
        "fb2_readiness_protected",
        "fb2_tool_manifest_protected",
        "fb2_authenticated_readiness",
        "fb2_authenticated_readiness_status",
        "fb2_authenticated_manifest",
        "fb2_authenticated_manifest_tool_ids",
        "fb2_apk_version",
        "local_voice_sdk_build",
        "voice_evidence_final_ready",
        "voice_evidence_artifact_refs_complete",
        "voice_evidence_artifact_logcat",
        "voice_evidence_artifact_visual",
        "scenario_today_context_audit",
        "scenario_my_ticket_context_audit",
        "scenario_platform_order_summary",
        "permission_total_blocks",
        "permission_user_blocks",
        "permission_platform_blocks",
        "quality_feedback_count",
        "quality_matched_cited_sources",
        "quality_unmatched_cited_sources",
        "quality_missing_context_count",
        "quality_wrong_context_count",
        "quality_non_synthetic_feedback_count",
        "quality_non_synthetic_adoption_count",
        "quality_non_synthetic_memory_refs"
    )) {
        Assert-SelfTest (-not [string]::IsNullOrWhiteSpace([string]$centerEvidence[$key])) "ai-center evidence maps $key" ([string]$centerEvidence[$key])
    }

    $failed = $script:SelfTestFailed
    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
    exit 0
}

if ($SelfTest) {
    Invoke-FinalAcceptanceSelfTest
}

if ($PreflightOnly -and $AllowVisibleMessages) {
    Fail-FinalAcceptance "Use either -PreflightOnly or -AllowVisibleMessages, not both."
}
if (-not $PreflightOnly -and -not $AllowVisibleMessages) {
    Fail-FinalAcceptance "Pass -AllowVisibleMessages after explicit authorization; this wrapper sends visible group messages."
}
if (-not $Fb2AiCenterToken) {
    Fail-FinalAcceptance "FB2_AI_CENTER_TOKEN or -Fb2AiCenterToken is required."
}
if (-not $VoiceDeviceEvidencePath) {
    Fail-FinalAcceptance "FB2_VOICE_DEVICE_EVIDENCE_PATH or -VoiceDeviceEvidencePath is required."
}
if (-not (Test-Path $VoiceDeviceEvidencePath)) {
    Fail-FinalAcceptance "Voice device evidence file not found: $VoiceDeviceEvidencePath"
}
if (-not $MainToken -and -not $Fb2UserToken -and (-not $Fb2Username -or -not $Fb2Password)) {
    Fail-FinalAcceptance "Set ELON_MAIN_TOKEN, FB2_USER_TOKEN, or -Fb2Username/-Fb2Password for authenticated chat flows."
}
if (-not $ExternalUserId -and (-not $Fb2Username -or -not $Fb2Password)) {
    Fail-FinalAcceptance "Set FB2_AI_CONTEXT_EXTERNAL_USER_ID or provide -Fb2Username/-Fb2Password so the wrapper can resolve the fb2 user id."
}

Resolve-Fb2ExternalUser
if (-not $ExternalUserId) {
    Fail-FinalAcceptance "Unable to resolve fb2 external user id from credentials; pass -ExternalUserId explicitly."
}
Test-UserOrderContextBeforeVisibleSmoke

$startedAt = (Get-Date).ToUniversalTime().ToString("o")
$qualitySince = $startedAt
$root = Split-Path -Parent $PSScriptRoot
$visibleScript = Join-Path $PSScriptRoot "smoke-fb2-visible-chat.ps1"
$centerScript = Join-Path $PSScriptRoot "smoke-fb2-ai-center.ps1"
$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")

if (-not $SummaryPath) {
    $summaryDir = Join-Path $root "target\fb2-ai-center"
    $SummaryPath = Join-Path $summaryDir "final-acceptance-$stamp.json"
} else {
    $summaryDir = Split-Path -Parent $SummaryPath
    if (-not $summaryDir) {
        $summaryDir = "."
    }
}
New-Item -ItemType Directory -Force -Path $summaryDir | Out-Null
$visibleLogPath = Join-Path $summaryDir "final-acceptance-$stamp-visible-chat.log"
$centerLogPath = Join-Path $summaryDir "final-acceptance-$stamp-ai-center.log"

$mainHead = ""
try { $mainHead = (& git -C $root rev-parse HEAD).Trim() } catch { $mainHead = "" }
$mainStatus = ""
try { $mainStatus = (& git -C $root status --short --branch) -join "`n" } catch { $mainStatus = "" }
$mainVersion = Invoke-JsonOrNull "$MainBase/api/server/version"
$fb2Version = Invoke-JsonOrNull "$Fb2Base/api/app-version"

if ($PreflightOnly) {
    $preflightArgs = [System.Collections.Generic.List[string]]::new()
    [void]$preflightArgs.Add("-NoProfile")
    [void]$preflightArgs.Add("-ExecutionPolicy")
    [void]$preflightArgs.Add("Bypass")
    [void]$preflightArgs.Add("-File")
    [void]$preflightArgs.Add($centerScript)
    Add-Arg $preflightArgs "-MainBase" $MainBase
    Add-Arg $preflightArgs "-MainToken" $MainToken
    Add-Arg $preflightArgs "-Fb2Base" $Fb2Base
    Add-Arg $preflightArgs "-Fb2Token" $Fb2AiCenterToken
    Add-Arg $preflightArgs "-Fb2UserToken" $Fb2UserToken
    Add-Arg $preflightArgs "-Fb2Username" $Fb2Username
    Add-Arg $preflightArgs "-Fb2Password" $Fb2Password
    Add-Arg $preflightArgs "-GroupId" $GroupId
    Add-Arg $preflightArgs "-ExternalUserId" $ExternalUserId
    Add-Arg $preflightArgs "-RequestTimeoutSec" $RequestTimeoutSec
    Add-Arg $preflightArgs "-VoiceDeviceEvidencePath" $VoiceDeviceEvidencePath
    Add-SwitchArg $preflightArgs "-RequireFb2Live" $true
    Add-SwitchArg $preflightArgs "-RequireAllScenarios" $true
    Add-SwitchArg $preflightArgs "-IncludePlatformOrderSummary" $true
    Add-SwitchArg $preflightArgs "-CheckFb2ApkVersion" $true
    Add-SwitchArg $preflightArgs "-CheckLocalVoiceSdkBuild" $true
    Add-SwitchArg $preflightArgs "-RequireVoiceDeviceEvidence" $true
    Add-SwitchArg $preflightArgs "-CheckPermissionBoundaries" $true
    Add-SwitchArg $preflightArgs "-RequireNoSkips" $true

    $preflightLogPath = Join-Path $summaryDir "final-acceptance-$stamp-preflight.log"
    $preflightResult = Invoke-SmokeScript "final preflight without visible messages" $preflightArgs $preflightLogPath
    $completedAt = (Get-Date).ToUniversalTime().ToString("o")
    $summary = [ordered]@{
        schema = "fb2.main_project.final_acceptance.v1"
        mode = "preflight_only"
        started_at = $startedAt
        completed_at = $completedAt
        quality_since = $qualitySince
        main_base = $MainBase
        fb2_base = $Fb2Base
        group_id = $GroupId
        external_user_id = $ExternalUserId
        voice_device_evidence_path = $VoiceDeviceEvidencePath
        main_project_head = $mainHead
        main_project_status = $mainStatus
        main_server_version = $mainVersion
        fb2_app_version = $fb2Version
        preflight_exit_code = $preflightResult.exit_code
        preflight_log_path = $preflightResult.log_path
        preflight_evidence = Build-AiCenterEvidence $preflightResult.output
        success = ($preflightResult.exit_code -eq 0)
    }

    $summaryJson = $summary | ConvertTo-Json -Depth 8
    Set-Content -Path $SummaryPath -Value $summaryJson -Encoding UTF8

    Write-Output ""
    Write-Output "== final acceptance summary =="
    Write-Output $summaryJson
    Write-Output "summary_path=$SummaryPath"

    if (-not $summary.success) {
        exit 1
    }
    exit 0
}

$visibleArgs = [System.Collections.Generic.List[string]]::new()
[void]$visibleArgs.Add("-NoProfile")
[void]$visibleArgs.Add("-ExecutionPolicy")
[void]$visibleArgs.Add("Bypass")
[void]$visibleArgs.Add("-File")
[void]$visibleArgs.Add($visibleScript)
Add-Arg $visibleArgs "-MainBase" $MainBase
Add-Arg $visibleArgs "-MainToken" $MainToken
Add-Arg $visibleArgs "-Fb2Base" $Fb2Base
Add-Arg $visibleArgs "-Fb2Token" $Fb2UserToken
Add-Arg $visibleArgs "-Fb2AiCenterToken" $Fb2AiCenterToken
Add-Arg $visibleArgs "-Fb2UserId" $ExternalUserId
Add-Arg $visibleArgs "-Fb2Username" $Fb2Username
Add-Arg $visibleArgs "-Fb2Password" $Fb2Password
Add-Arg $visibleArgs "-GroupId" $GroupId
Add-Arg $visibleArgs "-RequestTimeoutSec" $RequestTimeoutSec
Add-Arg $visibleArgs "-PollTimeoutSec" $PollTimeoutSec
Add-Arg $visibleArgs "-FeedbackPollTimeoutSec" $FeedbackPollTimeoutSec
Add-Arg $visibleArgs "-PollIntervalSec" $PollIntervalSec
Add-SwitchArg $visibleArgs "-AllowVisibleMessages" $true

$visibleResult = Invoke-SmokeScript "visible group chat smoke" $visibleArgs $visibleLogPath
$visibleLines = @($visibleResult.output)

$centerArgs = [System.Collections.Generic.List[string]]::new()
[void]$centerArgs.Add("-NoProfile")
[void]$centerArgs.Add("-ExecutionPolicy")
[void]$centerArgs.Add("Bypass")
[void]$centerArgs.Add("-File")
[void]$centerArgs.Add($centerScript)
Add-Arg $centerArgs "-MainBase" $MainBase
Add-Arg $centerArgs "-MainToken" $MainToken
Add-Arg $centerArgs "-Fb2Base" $Fb2Base
Add-Arg $centerArgs "-Fb2Token" $Fb2AiCenterToken
Add-Arg $centerArgs "-Fb2UserToken" $Fb2UserToken
Add-Arg $centerArgs "-Fb2Username" $Fb2Username
Add-Arg $centerArgs "-Fb2Password" $Fb2Password
Add-Arg $centerArgs "-GroupId" $GroupId
Add-Arg $centerArgs "-ExternalUserId" $ExternalUserId
Add-Arg $centerArgs "-RequestTimeoutSec" $RequestTimeoutSec
Add-Arg $centerArgs "-VoiceDeviceEvidencePath" $VoiceDeviceEvidencePath
Add-Arg $centerArgs "-QualitySince" $qualitySince
Add-Arg $centerArgs "-MinFeedbackCount" $MinFeedbackCount
Add-Arg $centerArgs "-MinMatchedCitedSourceCount" $MinMatchedCitedSourceCount
Add-Arg $centerArgs "-MinNonSyntheticFeedbackCount" $MinNonSyntheticFeedbackCount
Add-Arg $centerArgs "-MinOpinionAdoptionCount" $MinOpinionAdoptionCount
Add-Arg $centerArgs "-QualityFeedbackSampleLimit" $QualityFeedbackSampleLimit
Add-Arg $centerArgs "-MaxLargeContextPackRate" $MaxLargeContextPackRate
Add-Arg $centerArgs "-MaxCitationUnmatchedRate" $MaxCitationUnmatchedRate
Add-Arg $centerArgs "-MaxMissingContextRate" $MaxMissingContextRate
Add-Arg $centerArgs "-MaxWrongContextRate" $MaxWrongContextRate
Add-SwitchArg $centerArgs "-FinalAcceptance" $true

$centerResult = Invoke-SmokeScript "final no-skip acceptance" $centerArgs $centerLogPath
$centerLines = @($centerResult.output)

$completedAt = (Get-Date).ToUniversalTime().ToString("o")
$feedbackEvidence = @(Find-FeedbackEvidence $visibleLines)
$feedbackCoverage = Build-FeedbackCoverage $feedbackEvidence
$summary = [ordered]@{
    schema = "fb2.main_project.final_acceptance.v1"
    mode = "visible_final_acceptance"
    started_at = $startedAt
    completed_at = $completedAt
    quality_since = $qualitySince
    main_base = $MainBase
    fb2_base = $Fb2Base
    group_id = $GroupId
    external_user_id = $ExternalUserId
    voice_device_evidence_path = $VoiceDeviceEvidencePath
    main_project_head = $mainHead
    main_project_status = $mainStatus
    main_server_version = $mainVersion
    fb2_app_version = $fb2Version
    visible_chat_exit_code = $visibleResult.exit_code
    final_acceptance_exit_code = $centerResult.exit_code
    visible_chat_log_path = $visibleResult.log_path
    final_acceptance_log_path = $centerResult.log_path
    visible_mention_message_id = Find-CheckDetail $visibleLines "visible @EL sent"
    visible_mention_reply_id = Find-CheckDetail $visibleLines "visible @EL ai reply"
    selected_message_seed_id = Find-CheckDetail $visibleLines "selected-message seed sent"
    selected_message_reply_id = Find-CheckDetail $visibleLines "selected-message ai reply"
    summary_post_id = Find-CheckDetail $visibleLines "summary-post created"
    summary_post_status = Find-CheckDetail $visibleLines "summary-post ready"
    feedback_evidence = $feedbackEvidence
    feedback_coverage = $feedbackCoverage
    visible_answer_policy_evidence = Build-VisibleAnswerEvidence $visibleLines
    final_acceptance_evidence = Build-AiCenterEvidence $centerLines
    success = ($visibleResult.exit_code -eq 0 -and $centerResult.exit_code -eq 0 -and [bool]$feedbackCoverage["complete"])
}

$summaryJson = $summary | ConvertTo-Json -Depth 8
Set-Content -Path $SummaryPath -Value $summaryJson -Encoding UTF8

Write-Output ""
Write-Output "== final acceptance summary =="
Write-Output $summaryJson
Write-Output "summary_path=$SummaryPath"

if (-not $summary.success) {
    exit 1
}

exit 0
