#requires -Version 5.1

$ErrorActionPreference = "Stop"
$script = Get-Content (Join-Path $PSScriptRoot "smoke-chatgpt-web-composer-controls.ps1") -Raw
$runtime = Get-Content (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1") -Raw

$required = @(
    "ExpectedHardwareSerial",
    "ExpectedAdapterVersion",
    "SkipDictation",
    "user_assisted_audio_capture",
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Assert-ChatGptWebSmokeAdapterVersion",
    "RequireChatGptForeground",
    "chatgpt_start_dictation",
    '[string]$_.semantic -eq "dictation"',
    'Open-ChatGptWebSmokeSurface -Runtime $runtime',
    'chatgpt-web-smoke-evidence.ps1',
    'chatgpt-web-smoke-supervised-runtime.ps1',
    'Start-ChatGptWebSmokeIsolatedConversation',
    'Restore-ChatGptWebSmokeOrigin',
    'Composer control smoke will not replace a non-empty ChatGPT draft.',
    '$originConversationPath',
    'finally {',
    '$restoreSearch',
    "chatgpt_cancel_dictation",
    "start_dictation",
    "cancel_dictation",
    "web_search",
    "chatgpt_select_composer_option",
    "set_input_text",
    "sent_messages = 0",
    "uploaded_attachments = 0",
    'isolated_conversation = $true',
    'origin_restored = [bool]$originRestored',
    "original_state_restored = `$true",
    '$composerToolDiscoveryCases = [ordered]@{',
    'composer_tool_discovery',
    'executed_tools = 0',
    'production_surface_preserved = Test-ChatGptWebSmokeActivityForeground',
    "CHATGPT_WEB_COMPOSER_CONTROL_SMOKE_STATUS=passed"
)
foreach ($token in $required) {
    if (-not $script.Contains($token)) { throw "Missing composer smoke contract token: $token" }
}

foreach ($token in @("function Test-ChatGptWebSmokeActivityForeground", "topResumedActivity")) {
    if (-not $runtime.Contains($token)) { throw "Missing composer smoke runtime token: $token" }
}

if ($script -match "send_input|ProbeMarker|Reply only with") {
    throw "Composer control smoke must not send ChatGPT messages."
}
Write-Output "CHATGPT_WEB_COMPOSER_CONTROL_SMOKE_CONTRACT=passed"

$kotlin = Join-Path $PSScriptRoot '../android/app/src/main/kotlin/com/elon/app'
$session = Get-Content (Join-Path $kotlin 'chatgptweb/ChatGptBackgroundSession.kt') -Raw
$controller = Get-Content (Join-Path $kotlin 'ChatGptSocialChatController.kt') -Raw
$observed = Get-Content (Join-Path $kotlin 'chatgptweb/ChatGptWebObservedState.kt') -Raw
if ($session -notmatch '(?s)private fun handleEvent\(event: ChatGptWebEvent\).*?observedMcpState.accept\(event\).*?when \(event\)') {
    throw 'Composer observers must receive the already committed canonical state.'
}
if ($session -notmatch '(?s)is ChatGptWebEvent.ComposerControls -> \{\s+composerOptionInteraction.release\(\)\s+composerOptionRequests.complete\(event.section\)\s+onComposerControls\(event.section, event.options\)\s+\}') {
    throw 'All composer sections must notify native controls without waiting for a message snapshot.'
}
if ($controller -notmatch 'onComposerControls = \{ section, options ->\s+if \(section == "model"\) showModelOptions\(options\)\s+onComposerStateChanged\(\)\s+\}') {
    throw 'Native composer notification must preserve model handling and refresh tools directly.'
}
if ($observed -notmatch 'is ChatGptWebEvent.ComposerControls -> \{\s+composerSections = composerSections \+ \(event.section to event.options\)') {
    throw 'Composer sections must retain their canonical selected state, including cleared selections.'
}
Write-Output 'CHATGPT_WEB_COMPOSER_STATE_NOTIFICATION_CONTRACT=passed'
