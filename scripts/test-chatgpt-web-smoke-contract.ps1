$ErrorActionPreference = "Stop"

$sourcePath = Join-Path $PSScriptRoot "smoke-chatgpt-web-apk.ps1"
$evidencePath = Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1"
$source = Get-Content -LiteralPath $sourcePath -Raw
$evidenceSource = Get-Content -LiteralPath $evidencePath -Raw
. $evidencePath

function Assert-Contains {
    param([Parameter(Mandatory = $true)][string]$Needle)
    if (-not $source.Contains($Needle)) {
        throw "ChatGPT Web smoke contract is missing: $Needle"
    }
}

Assert-Contains 'Invoke-UiAction -Action "chatgpt_list_features"'
Assert-Contains 'Invoke-UiAction -Action "chatgpt_select_view" -Arguments @{ view_mode = "official" }'
Assert-Contains 'function Wait-NavigationReady'
Assert-Contains 'function Wait-VisibleNativeSelectors'
Assert-Contains 'function Wait-AccountMenuReady'
Assert-Contains 'function Wait-ComposerOptionsReady'
Assert-Contains 'function Wait-NewConversationReady'
Assert-Contains '$freshCollection = $command.action -eq $expectedAction'
Assert-Contains '$cachedSnapshot = $navigation.control_ok -eq $true -and $options.Count -gt 0'
Assert-Contains 'Wait-ComposerOptionsReady -Section $Section -AfterMs $afterMs'
Assert-Contains '$command.action -eq "collect_navigation"'
Assert-Contains '$navigation = Invoke-UiAction -Action "chatgpt_get_navigation"'
Assert-Contains '$features = @($navigation.features | Where-Object { $null -ne $_ })'
Assert-Contains '$overlayOpen = [int]$matrix.observed_semantics.close -gt 0'
Assert-Contains '($collected -or $cachedSnapshot) -and $overlayOpen'
Assert-Contains 'Wait-NavigationReady -AfterMs $beforeFeatures'
Assert-Contains '$navigationMatrix = Invoke-UiAction -Action "chatgpt_get_capability_matrix"'
Assert-Contains 'Add-Check "navigation_adaptation_review"'
Assert-Contains 'Where-Object { $_.semantic -eq "profile" -and $_.region -eq "overlay" }'
Assert-Contains 'Add-Check "account_menu_entry"'
Assert-Contains 'Add-Check "account_menu_settings"'
Assert-Contains 'Add-Check "account_menu_logout"'
Assert-Contains 'Add-Check "account_menu_generic_controls"'
Assert-Contains 'Add-Check "account_menu_adaptation_review"'
Assert-Contains '$navigationCloseCount = [int]$navigationMatrix.observed_semantics.close'
Assert-Contains 'Add-Check "navigation_overlay_open" ($navigationCloseCount -gt 0)'
Assert-Contains '$nativeView = Invoke-UiAction -Action "chatgpt_select_view" -Arguments @{ view_mode = "native" }'
Assert-Contains 'Add-Check "native_view_selected"'
Assert-Contains '$visibleSelectors = Wait-VisibleNativeSelectors -RequiredPrefixes $requiredSelectors'
Assert-Contains '$restoredOfficialView = Invoke-UiAction -Action "chatgpt_select_view" -Arguments @{ view_mode = "official" }'
Assert-Contains 'Add-Check "official_view_restored"'
Assert-Contains '$beforeListState = Invoke-ApkMcp -Tool "ui_state"'
Assert-Contains '$beforeList = [long]$beforeListState.last_command.observed_at_ms'
Assert-Contains 'function Get-TopResumedActivity'
Assert-Contains 'if ($null -eq $line) { return "" }'
Assert-Contains 'function Wait-ChatGptActivityForeground'
Assert-Contains '$topResumedActivity = Wait-ChatGptActivityForeground'
Assert-Contains 'Add-Check "chatgpt_target_bound"'
Assert-Contains '$opened.target_activity_bound -eq $true'
Assert-Contains 'Add-Check "chatgpt_activity_foreground"'
Assert-Contains 'com\.elon\.app/\.chatgptweb\.ChatGptWebTestActivity\b'
Assert-Contains 'Get-ComposerOptions -Section "model"'
Assert-Contains 'Get-ComposerOptions -Section "tools"'
Assert-Contains 'Get-ForeignComposerLabels -Options $modelOptions'
Assert-Contains 'Get-ForeignComposerLabels -Options $toolOptions'
Assert-Contains 'Add-Check "composer_model_scope"'
Assert-Contains 'Add-Check "composer_tool_scope"'
Assert-Contains 'Add-Check "new_conversation_ready"'
Assert-Contains '$adaptationRequired = $matrix.adaptation_review.required -eq $true'
Assert-Contains 'Add-Check "adaptation_review" (-not $adaptationRequired)'
Assert-Contains '$featureBaseline = $matrix.feature_baseline'
Assert-Contains '$baselineStatusTotal = [int]$baselineSummary.complete'
Assert-Contains 'Add-Check "capability_matrix_app_version"'
Assert-Contains 'Add-Check "feature_baseline_schema"'
Assert-Contains 'Add-Check "feature_device_evidence_current"'
Assert-Contains '$featureBaseline.device_verification_current -eq $true'
Assert-Contains 'elon.chatgpt_web.feature_baseline.v4'
Assert-Contains '$currentEvidenceInput -eq $verifiedEvidenceInput'
Assert-Contains 'Add-Check "feature_device_evidence_provenance"'
Assert-Contains 'elon.chatgpt_web.device_evidence.v1'
Assert-Contains 'Add-Check "feature_baseline_complete"'
Assert-Contains 'Add-Check "feature_code_status_complete"'
Assert-Contains 'Add-Check "feature_verification_status_complete"'
Assert-Contains '[int]$verificationSummary.offline_verified +'
Assert-Contains '[int]$verificationSummary.device_verified +'
Assert-Contains '[int]$verificationSummary.deferred +'
Assert-Contains '[int]$verificationSummary.failed -eq [int]$featureBaseline.feature_count'
Assert-Contains 'schema = "elon.chatgpt_web.apk_smoke.v2"'
Assert-Contains 'recorded_at_utc = [DateTimeOffset]::UtcNow.ToString("o")'
Assert-Contains 'feature_baseline = $featureBaseline'
Assert-Contains 'Invoke-Adb shell input keyevent 4'
Assert-Contains 'Get-ChatGptContextPagingEvidence'
Assert-Contains 'Add-Check "context_cursor_roundtrip"'
Assert-Contains 'Add-Check "context_cursor_next"'
Assert-Contains 'Add-Check "official_fullscreen_mode"'
Assert-Contains 'official_fullscreen_chrome_$chromeId'
Assert-Contains '$sendRequestId = [string]$sendDispatch.command_receipt.request_id'
Assert-Contains 'Wait-ChatGptProbeReply -RequestId $sendRequestId'
Assert-Contains 'throw "ChatGPT Web new conversation failed; the send probe was not dispatched."'

if (-not $evidenceSource.Contains('message_cursor = [string]$first.message_cursor')) {
    throw "Context paging evidence must replay the current MCP cursor."
}
if (-not $evidenceSource.Contains('message_cursor = [string]$first.next_message_cursor')) {
    throw "Context paging evidence must follow the next MCP cursor."
}

$uiXml = '<node resource-id="com.elon.app:id/chatGptWebView" content-desc="chatgpt-native:send:ready" />'
if (-not (Test-ChatGptResourceVisible -UiXml $uiXml -ResourceId "chatGptWebView")) {
    throw "Visible ChatGPT WebView resource id was not detected."
}
if (Test-ChatGptResourceVisible -UiXml $uiXml -ResourceId "chatGptWebToolbar") {
    throw "Hidden ChatGPT chrome was reported as visible."
}
$selectors = @(Get-ChatGptNativeSelectorsFromXml -UiXml $uiXml)
if ($selectors.Count -ne 1) {
    throw "Stable native selector extraction failed: count=$($selectors.Count), values=$($selectors -join ','), xml=$uiXml"
}

$calls = [System.Collections.Generic.List[object]]::new()
$paging = Get-ChatGptContextPagingEvidence -MessageOffset 5 -InvokeUiAction {
    param($action, $arguments)
    $calls.Add([pscustomobject]@{ action = $action; arguments = $arguments })
    if ($arguments.message_cursor -eq "ctx1.revision.5") {
        return [pscustomobject]@{ control_ok = $true; context_revision = "revision"; message_offset = 5 }
    }
    if ($arguments.message_cursor -eq "ctx1.revision.6") {
        return [pscustomobject]@{ control_ok = $true; context_revision = "revision"; message_offset = 6 }
    }
    return [pscustomobject]@{
        control_ok = $true
        context_revision = "revision"
        message_offset = 5
        next_message_offset = 6
        message_cursor = "ctx1.revision.5"
        next_message_cursor = "ctx1.revision.6"
        has_more = $true
    }
}
if ($calls.Count -ne 3 -or $paging.replay.message_offset -ne 5 -or $paging.next.message_offset -ne 6) {
    throw "Context cursor roundtrip evidence did not execute first, replay, and next pages."
}

$probeState = [pscustomobject]@{
    command_requests = @([pscustomobject]@{
        request_id = "request-send"
        expected_web_action = "send_prompt"
        status = "succeeded"
        completed_at_ms = 20
        result = [pscustomobject]@{ ok = $true }
    })
    conversation = [pscustomobject]@{ messages = @(
        [pscustomobject]@{ role = "user"; content = "probe" },
        [pscustomobject]@{ role = "assistant"; content = "PROBE-MARKER" }
    ) }
    streaming = $false
    last_command = [pscustomobject]@{ action = "collect_navigation" }
}
$receiptResult = Wait-ChatGptProbeReply -RequestId "request-send" -Marker "PROBE-MARKER" `
    -AfterMs 10 -TimeoutSec 1 -PollIntervalSec 1 -InvokeUiState { $probeState }
if ($receiptResult.last_command.action -ne "collect_navigation") {
    throw "Probe receipt test did not preserve the overwritten last_command condition."
}

if ($source.Contains('Wait-CommandResult -Action "collect_navigation" -AfterMs $beforeFeatures')) {
    throw "Navigation smoke must accept the already-collected snapshot path."
}
if ($source.Contains('Wait-CommandResult -Action $commandAction')) {
    throw "Composer smoke must tolerate a newer command overwriting last_command."
}
if ($source.Contains('ToUnixTimeMilliseconds()')) {
    throw "ChatGPT Web smoke must compare bridge timestamps from the same device clock."
}
if ($source -notmatch '(?s)if \(\$EnsureMainActivity\) \{\s*\$params\.EnsureMainActivity = \$true\s*\$params\.OpenAppOnFailure = \$true\s*\}') {
    throw "ChatGPT Web smoke must relaunch MainActivity only for an explicit initial bootstrap."
}

$featuresIndex = $source.IndexOf('Invoke-UiAction -Action "chatgpt_list_features"')
$openIndex = $source.IndexOf('Invoke-UiAction -Action "open_chatgpt_web"')
$officialIndex = $source.IndexOf('Invoke-UiAction -Action "chatgpt_select_view" -Arguments @{ view_mode = "official" }')
$modelIndex = $source.IndexOf('Get-ComposerOptions -Section "model"')
$toolsIndex = $source.IndexOf('Get-ComposerOptions -Section "tools"')
$nativeIndex = $source.IndexOf('$nativeView = Invoke-UiAction -Action "chatgpt_select_view"')
$selectorsIndex = $source.IndexOf('$visibleSelectors = Wait-VisibleNativeSelectors')
if (-not ($openIndex -lt $officialIndex -and $officialIndex -lt $featuresIndex)) {
    throw "ChatGPT Web smoke must select the official view before readiness and navigation checks."
}
if (-not ($featuresIndex -lt $modelIndex -and $modelIndex -lt $toolsIndex)) {
    throw "Composer contamination smoke must open the sidebar before model and tools checks."
}
if (-not ($toolsIndex -lt $nativeIndex -and $nativeIndex -lt $selectorsIndex)) {
    throw "Native selectors must be audited only after official WebView checks complete."
}

Write-Output "CHATGPT_WEB_SMOKE_CONTRACT=passed"
