$ErrorActionPreference = "Stop"

$sourcePath = Join-Path $PSScriptRoot "smoke-chatgpt-web-apk.ps1"
$evidencePath = Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1"
$composerPath = Join-Path $PSScriptRoot "chatgpt-web-smoke-composer.ps1"
$navigationPath = Join-Path $PSScriptRoot "chatgpt-web-smoke-navigation.ps1"
$source = Get-Content -LiteralPath $sourcePath -Raw
$evidenceSource = Get-Content -LiteralPath $evidencePath -Raw
$composerSource = Get-Content -LiteralPath $composerPath -Raw
. $evidencePath
. $composerPath
. $navigationPath

function Assert-Contains {
    param([Parameter(Mandatory = $true)][string]$Needle)
    if (-not $source.Contains($Needle)) {
        throw "ChatGPT Web smoke contract is missing: $Needle"
    }
}

function Assert-EvidenceContains {
    param([Parameter(Mandatory = $true)][string]$Needle)
    if (-not $evidenceSource.Contains($Needle)) {
        throw "ChatGPT Web smoke evidence contract is missing: $Needle"
    }
}

Assert-Contains 'Invoke-UiAction -Action "chatgpt_list_features"'
Assert-Contains 'ExpectedHardwareSerial'
Assert-Contains 'ExpectedAdapterVersion'
Assert-Contains 'Assert-ChatGptWebSmokeAdapterVersion -State $state'
Assert-Contains 'Start-ChatGptWebSmokeAwakeLease'
Assert-Contains 'Stop-ChatGptWebSmokeAwakeLease'
Assert-Contains '$state = Open-ChatGptWebSmokeSurface -Runtime $smokeRuntime'
Assert-Contains 'function Wait-NavigationReady'
Assert-Contains '$nextOpenAttemptAt = [DateTimeOffset]::UtcNow.AddSeconds(3)'
Assert-Contains 'Invoke-UiAction -Action "chatgpt_list_features" | Out-Null'
Assert-Contains 'function Wait-VisibleProductionSelectors'
Assert-Contains 'foreach ($attempt in 1..3)'
Assert-Contains 'UIAutomator dump failed after 3 attempts.'
Assert-Contains 'function Wait-AccountMenuReady'
Assert-Contains 'function Wait-AccountMenuClosed'
Assert-EvidenceContains 'function Wait-ChatGptConversationCollectionCoverage'
Assert-Contains '$conversationPage = Wait-ChatGptConversationCollectionCoverage'
Assert-EvidenceContains 'if ($last.collection.timed_out -eq $true) { return $last }'
Assert-Contains 'Invoke-ChatGptWebSmokeComposerOptions -Section $Section'
Assert-Contains 'function Wait-NewConversationReady'
Assert-Contains '$command.action -eq "collect_navigation"'
Assert-Contains '$navigation = Invoke-UiAction -Action "chatgpt_get_navigation"'
Assert-Contains '$features = @($navigation.features | Where-Object { $null -ne $_ })'
Assert-Contains '$expandedNavigation = @($last.ui_manifest.controls | Where-Object { $_.semantic -eq "navigation" -and $_.expanded -eq $true }).Count -gt 0'
Assert-Contains '$overlayOpen = [int]$matrix.observed_semantics.close -gt 0 -or $expandedNavigation'
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
Assert-Contains 'Add-Check "account_menu_close"'
Assert-Contains 'control_id = [string]$profileControls[0].control_id'
Assert-Contains 'Wait-AccountMenuClosed -TimeoutSec $ReadyTimeoutSec'
Assert-Contains 'Close-ChatGptWebSmokeNavigation -TimeoutSec $ReadyTimeoutSec'
Assert-Contains '$navigationHelper = Join-Path $PSScriptRoot "chatgpt-web-smoke-navigation.ps1"'
Assert-Contains 'Invoke-UiAction -Action "chatgpt_dismiss_features"'
Assert-Contains 'Add-Check "navigation_overlay_close"'
Assert-Contains '$navigationCloseCount = [int]$navigationMatrix.observed_semantics.close'
Assert-Contains '$navigationExpandedCount = @($featuresState.command_state.ui_manifest.controls | Where-Object { $_.semantic -eq "navigation" -and $_.expanded -eq $true }).Count'
Assert-Contains 'Add-Check "navigation_overlay_open" ($navigationCloseCount + $navigationExpandedCount -gt 0)'
Assert-Contains '$visibleSelectors = Wait-VisibleProductionSelectors -RequiredPrefixes $requiredSelectors'
Assert-Contains 'web-chat-composer-input:chatgpt_web'
Assert-Contains 'web-chat-composer-command:chatgpt_web:'
Assert-Contains 'Add-Check "production_selector_composer_entry"'
Assert-Contains 'Add-Check "production_selector_composer_action"'
Assert-Contains 'web-chat-page-actions:chatgpt_web'
Assert-Contains '$beforeListState = Invoke-ApkMcp -Tool "ui_state"'
Assert-Contains '$beforeList = [long]$beforeListState.last_command.observed_at_ms'
Assert-Contains 'function Get-TopResumedActivity'
Assert-Contains 'if ($null -eq $line) { return "" }'
Assert-Contains 'function Wait-ChatGptActivityForeground'
Assert-Contains '$topResumedActivity = Wait-ChatGptActivityForeground'
Assert-Contains 'Add-Check "chatgpt_activity_foreground"'
Assert-Contains 'com\.elon\.app/\.chatgptweb\.ChatGptWebOfficialActivity\b'
Assert-Contains 'Add-Check "production_activity_foreground"'
Assert-Contains 'Get-ComposerOptions -Section "model"'
Assert-Contains 'Get-ComposerOptions -Section "model" -TimeoutSec $initialComposerTimeoutSec'
Assert-Contains 'if ($null -eq $modelResult -and $composerOptionsOriginPath)'
Assert-Contains 'Get-ComposerOptions -Section "tools"'
Assert-Contains 'Get-ForeignComposerLabels -Options $modelOptions'
Assert-Contains 'Get-ForeignComposerLabels -Options $toolOptions'
Assert-Contains 'Add-Check "composer_model_scope"'
Assert-Contains 'Add-Check "composer_tool_scope"'
Assert-Contains 'Add-Check "composer_options_origin_restored"'
Assert-Contains '$temporaryComposerConversation = $true'
Assert-Contains 'Invoke-UiAction -Action "chatgpt_new_conversation"'
Assert-Contains 'Invoke-UiAction -Action "chatgpt_open_conversation"'
Assert-Contains 'restored ChatGPT conversation after composer inspection'
Assert-Contains 'Add-Check "new_conversation_ready"'
Assert-Contains '$adaptationRequired = $matrix.adaptation_review.required -eq $true'
Assert-Contains 'Add-Check "adaptation_review" (-not $adaptationRequired)'
Assert-Contains '$featureBaseline = $matrix.feature_baseline'
Assert-Contains '$baselineStatusTotal = [int]$baselineSummary.complete'
Assert-Contains 'Add-Check "capability_matrix_app_version"'
Assert-Contains 'Add-Check "feature_baseline_schema"'
Assert-Contains 'Add-Check "feature_device_evidence_current"'
Assert-Contains '[switch]$AllowStaleDeviceEvidence'
Assert-Contains '$evidenceCurrent -or $staleCandidateAccepted'
Assert-Contains '$AllowStaleDeviceEvidence -and'
Assert-Contains '$evidenceHashesValid'
Assert-Contains '$registeredEvidenceAdapterCurrent'
Assert-Contains '[int]$state.adapter_version -eq $ExpectedAdapterVersion'
Assert-Contains '[int]$featureBaseline.device_verification_adapter_version -le [int]$state.adapter_version'
Assert-Contains '$currentEvidenceInput -ne $verifiedEvidenceInput'
Assert-Contains 'elon.chatgpt_web.feature_baseline.v9'
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
Assert-Contains 'Close-ChatGptWebSmokeComposerOptions -TimeoutSec $TimeoutSec'
Assert-Contains 'Get-ChatGptContextPagingEvidence'
Assert-Contains 'Add-Check "context_cursor_roundtrip"'
Assert-Contains 'Add-Check "context_cursor_next"'
Assert-Contains 'Get-ChatGptConversationCollectionCoverage'
Assert-Contains 'official_fullscreen_chrome_$chromeId'
Assert-Contains '$sendRequestId = [string]$sendDispatch.command_receipt.request_id'
Assert-Contains 'Wait-ChatGptProbeReply -RequestId $sendRequestId'
Assert-Contains 'throw "ChatGPT Web new conversation failed; the send probe was not dispatched."'
Assert-Contains 'conversation_route_observed = -not [string]::IsNullOrWhiteSpace('
Assert-Contains 'model_observed = -not [string]::IsNullOrWhiteSpace('
Assert-Contains 'private_content_emitted = $false'
Assert-Contains '[switch]$VerifyStop'
Assert-Contains 'VerifyStop requires -SendProbe'
Assert-Contains 'Wait-ChatGptStreamingState -Expected $true'
Assert-Contains '-PrivateRevisionGreaterThan $privateRevisionBefore'
Assert-Contains 'Add-Check "private_stream_observed"'
Assert-Contains 'Test-ChatGptPrivateStreamState -State $streamingState'
Assert-Contains 'Invoke-UiAction -Action "chatgpt_stop_generation"'
Assert-Contains 'Wait-ChatGptCommandReceipt -RequestId $stopRequestId'
Assert-Contains '-ExpectedAction "stop_generation"'
Assert-Contains 'Wait-ChatGptStreamingState -Expected $false'
Assert-Contains 'streaming_stop = $streamingStop'

if ($source.Contains('conversation_url =')) {
    throw "ChatGPT Web smoke must not emit private conversation routes."
}

if (-not $evidenceSource.Contains('message_cursor = [string]$first.message_cursor')) {
    throw "Context paging evidence must replay the current MCP cursor."
}
if (-not $evidenceSource.Contains('message_cursor = [string]$first.next_message_cursor')) {
    throw "Context paging evidence must follow the next MCP cursor."
}
foreach ($required in @(
    'function Wait-ChatGptWebSmokeComposerBaseline',
    '$stableSamples -ge 2',
    '$state.composer_ready -eq $true',
    'function Wait-ChatGptWebSmokeComposerOptions',
    '$receipt = @($state.command_requests)',
    '[string]$_.request_id -eq $RequestId',
    'function Invoke-ChatGptWebSmokeComposerOptions',
    'function Close-ChatGptWebSmokeComposerOptions',
    '"chatgpt_dismiss_composer_options"',
    'Wait-ChatGptCommandReceipt -RequestId $requestId'
)) {
    if (-not $composerSource.Contains($required)) {
        throw "ChatGPT Web composer smoke helper is missing: $required"
    }
}

$script:composerBaselinePolls = 0
$stableComposer = Wait-ChatGptWebSmokeComposerBaseline -TimeoutSec 1 `
    -PollIntervalMilliseconds 10 -InvokeUiState {
        $script:composerBaselinePolls++
        [pscustomobject]@{
            bridge_state = "ready"
            composer_ready = $true
            ui_manifest = [pscustomobject]@{ controls = @() }
        }
    }
if ($null -eq $stableComposer -or $script:composerBaselinePolls -ne 2) {
    throw "Composer baseline helper did not require two stable samples."
}

$completeCollection = Get-ChatGptConversationCollectionCoverage `
    -Collection ([pscustomobject]@{ observed_count = 94; reached_end = $true; truncated = $false; timed_out = $false }) `
    -SourceCount 94
if ($completeCollection.passed -ne $true -or $completeCollection.required_count -ne 94) {
    throw "Complete conversation history was not accepted."
}
$boundedCollection = Get-ChatGptConversationCollectionCoverage `
    -Collection ([pscustomobject]@{ observed_count = 100; reached_end = $false; truncated = $true; timed_out = $false }) `
    -SourceCount 110
if ($boundedCollection.passed -ne $true -or $boundedCollection.required_count -ne 100) {
    throw "Bounded conversation history was not accepted."
}
$timedOutCollection = Get-ChatGptConversationCollectionCoverage `
    -Collection ([pscustomobject]@{ observed_count = 84; reached_end = $false; truncated = $false; timed_out = $true }) `
    -SourceCount 94
if ($timedOutCollection.passed -eq $true) {
    throw "Timed-out conversation history was accepted."
}
$nonTerminalCollection = Get-ChatGptConversationCollectionCoverage `
    -Collection ([pscustomobject]@{ observed_count = 94; reached_end = $false; truncated = $false; timed_out = $false }) `
    -SourceCount 94
if ($nonTerminalCollection.passed -eq $true) {
    throw "Non-terminal conversation history was accepted."
}
$privateWindowCollection = Get-ChatGptConversationCollectionCoverage `
    -Collection ([pscustomobject]@{
        observed_count = 161
        reached_end = $false
        truncated = $false
        timed_out = $false
        source = "official_private"
    }) `
    -SourceCount 161
if (
    $privateWindowCollection.passed -ne $true -or
    $privateWindowCollection.source_window_complete -ne $true
) {
    throw "Complete private conversation source window was not accepted."
}
$script:conversationCoverageAttempts = 0
$settledConversationPage = Wait-ChatGptConversationCollectionCoverage `
    -TimeoutSec 1 -PollIntervalSec 0 -InvokePage {
        $script:conversationCoverageAttempts++
        $observed = if ($script:conversationCoverageAttempts -eq 1) { 28 } else { 116 }
        [pscustomobject]@{
            control_ok = $true
            source_count = 116
            collection = [pscustomobject]@{
                observed_count = $observed
                reached_end = $false
                truncated = $false
                timed_out = $false
                source = "official_private"
            }
        }
    }
if (
    $script:conversationCoverageAttempts -ne 2 -or
    [int]$settledConversationPage.collection.observed_count -ne 116
) {
    throw "Conversation coverage wait did not settle the asynchronous private directory."
}

$uiXml = '<node resource-id="com.elon.app:id/chatGptWebView" content-desc="chatgpt-native:send:ready" />' +
    '<node content-desc="web-chat-composer-command:chatgpt_web:start-realtime-voice" />'
if (-not (Test-ChatGptResourceVisible -UiXml $uiXml -ResourceId "chatGptWebView")) {
    throw "Visible ChatGPT WebView resource id was not detected."
}
if (Test-ChatGptResourceVisible -UiXml $uiXml -ResourceId "chatGptWebToolbar") {
    throw "Hidden ChatGPT chrome was reported as visible."
}
$selectors = @(Get-ChatGptNativeSelectorsFromXml -UiXml $uiXml)
if (
    "web-chat-composer-command:chatgpt_web:start-realtime-voice" -notin $selectors
) {
    throw "Production Web Chat selector was not extracted."
}
if ($selectors.Count -ne 2 -or "chatgpt-native:send:ready" -notin $selectors) {
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

$streamingState = Wait-ChatGptStreamingState -Expected $true -TimeoutSec 1 `
    -PollIntervalMilliseconds 1 -InvokeUiState { [pscustomobject]@{ streaming = $true } }
if ($streamingState.streaming -ne $true) {
    throw "Streaming state evidence did not observe the requested state."
}
$privateStreamingState = Wait-ChatGptStreamingState -Expected $true -TimeoutSec 1 `
    -PrivateRevisionGreaterThan 3 -PollIntervalMilliseconds 1 -InvokeUiState {
        [pscustomobject]@{
            streaming = $true
            private_stream_observer = [pscustomobject]@{
                observed = $true
                revision = 4
                state = "streaming"
            }
        }
    }
if ([long]$privateStreamingState.private_stream_observer.revision -ne 4L) {
    throw "Private streaming evidence did not require an advanced observer revision."
}
if (-not (Test-ChatGptPrivateStreamState -State $privateStreamingState -RevisionGreaterThan 3)) {
    throw "Private streaming evidence did not validate its structural state."
}
$streamingStopEvidence = New-ChatGptStreamingStopEvidence `
    -StopResult ([pscustomobject]@{ receipt = [pscustomobject]@{ status = "succeeded" } }) `
    -StoppedState ([pscustomobject]@{ streaming = $false })
if (-not $streamingStopEvidence.stop_receipt_succeeded -or -not $streamingStopEvidence.streaming_stopped) {
    throw "Streaming stop evidence did not preserve its structural result."
}
$navigationPolls = 0
$closedNavigation = Close-ChatGptWebSmokeNavigation -TimeoutSec 1 -PollIntervalMilliseconds 10 `
    -InvokeAction { [pscustomobject]@{ control_ok = $true; action = "dismiss_features" } } `
    -InvokeUiState {
        $script:navigationPolls++
        [pscustomobject]@{ ui_manifest = [pscustomobject]@{ controls = @(
            [pscustomobject]@{
                semantic = "navigation"
                expanded = $script:navigationPolls -lt 2
            }
        ) } }
    }
if ($closedNavigation.passed -ne $true -or $navigationPolls -ne 2) {
    throw "Navigation cleanup helper did not wait for the expanded sidebar to close."
}
$rejectedNavigation = Close-ChatGptWebSmokeNavigation -TimeoutSec 1 `
    -InvokeAction { [pscustomobject]@{ control_ok = $false; action = "dismiss_features" } } `
    -InvokeUiState { throw "must not poll after a rejected dismiss" }
if ($rejectedNavigation.passed -eq $true) {
    throw "Navigation cleanup helper accepted a rejected dismiss action."
}
$commandReceipt = Wait-ChatGptCommandReceipt -RequestId "request-stop" `
    -ExpectedAction "stop_generation" -TimeoutSec 1 -PollIntervalSec 1 -InvokeUiState {
        [pscustomobject]@{
            command_requests = @([pscustomobject]@{
                request_id = "request-stop"
                expected_web_action = "stop_generation"
                status = "succeeded"
                result = [pscustomobject]@{ ok = $true }
            })
        }
    }
if ($commandReceipt.receipt.status -ne "succeeded") {
    throw "Streaming stop evidence did not correlate the command receipt."
}

if ($source.Contains('Wait-CommandResult -Action "collect_navigation" -AfterMs $beforeFeatures')) {
    throw "Navigation smoke must accept the already-collected snapshot path."
}
if ($source.Contains('Wait-CommandResult -Action $commandAction')) {
    throw "Composer smoke must tolerate a newer command overwriting last_command."
}
if ($source.Contains('Wait-ComposerOptionsReady')) {
    throw "Composer receipt polling must remain in the focused smoke helper."
}
if ($source.Contains('ToUnixTimeMilliseconds()')) {
    throw "ChatGPT Web smoke must compare bridge timestamps from the same device clock."
}
if ([regex]::Matches($source, 'Invoke-Adb shell input keyevent 4').Count -ne 1) {
    throw "Android back must only return from the official fallback, never dismiss hidden composer menus."
}
if ($source -notmatch '(?s)if \(\$EnsureMainActivity\) \{\s*\$params\.EnsureMainActivity = \$true\s*\$params\.OpenAppOnFailure = \$true\s*\}') {
    throw "ChatGPT Web smoke must relaunch MainActivity only for an explicit initial bootstrap."
}

$featuresFlowIndex = $source.IndexOf('$beforeFeaturesState = Invoke-ApkMcp -Tool "ui_state"')
$featuresIndex = $source.IndexOf(
    'Invoke-UiAction -Action "chatgpt_list_features"',
    $featuresFlowIndex
)
$openIndex = $source.IndexOf('Invoke-UiAction -Action "open_chatgpt_official_fallback"')
$returnIndex = $source.IndexOf('Invoke-Adb shell input keyevent 4', $openIndex)
$productionIndex = $source.IndexOf('$state = Open-ChatGptWebSmokeSurface -Runtime $smokeRuntime', $returnIndex)
$modelIndex = $source.IndexOf('Get-ComposerOptions -Section "model"')
$dismissNavigationIndex = $source.IndexOf('Invoke-UiAction -Action "chatgpt_dismiss_features"')
$toolsIndex = $source.IndexOf('Get-ComposerOptions -Section "tools"')
$selectorsIndex = $source.IndexOf('$visibleSelectors = Wait-VisibleProductionSelectors')
if (-not ($openIndex -lt $returnIndex -and $returnIndex -lt $productionIndex -and $productionIndex -lt $featuresIndex)) {
    throw "ChatGPT Web smoke must verify the official fallback, then return to the production friend-chat surface."
}
if (-not ($featuresIndex -lt $modelIndex -and $modelIndex -lt $toolsIndex)) {
    throw "Composer contamination smoke must open the sidebar before model and tools checks."
}
if (-not ($featuresIndex -lt $dismissNavigationIndex -and $dismissNavigationIndex -lt $modelIndex)) {
    throw "Composer smoke must close the official sidebar before opening model options."
}
if (-not ($toolsIndex -lt $selectorsIndex)) {
    throw "Production selectors must be audited only after adapter checks complete."
}

Write-Output "CHATGPT_WEB_SMOKE_CONTRACT=passed"
