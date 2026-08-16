#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-conversation-management.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) { throw "Conversation management smoke has parse errors." }

foreach ($required in @(
    "Assert-ChatGptWebSmokeTrustedDevice",
    "ExpectedHardwareSerial",
    "ExpectedAdapterVersion",
    '[switch]$ConfirmPinRoundTrip',
    'semantic = "conversation_options"',
    "Test-ConversationManagementControls",
    "Wait-ConversationManagementMenu",
    'for ($attempt = 1; $attempt -le 2; $attempt++)',
    '-WaitTimeoutSec $menuWaitSec',
    'Start-Sleep -Milliseconds 1200',
    "Close-FeatureNavigation",
    'Action "chatgpt_dismiss_features"',
    "Open-ConversationManagementSample",
    'Action "chatgpt_get_conversations"',
    'Action "chatgpt_open_conversation"',
    "Restore-ConversationManagementOrigin",
    "Restore-ChatGptWebSmokeOrigin",
    "Get-ConversationPinState",
    "Open-ConversationManagementMenu",
    "Wait-ConversationManagementMenuClosed",
    "Invoke-ConversationPinToggle",
    'user_confirmed = $true',
    "Restore-ConversationPinState",
    'throw "Conversation pin state recovery could not be verified."',
    "Restore-OriginalViewMode",
    'Action "chatgpt_refresh_controls"',
    '"conversation_files",',
    '"rename",',
    'context_id = $ContextId',
    'region = "overlay"',
    'chatgpt-conversation-actions:*',
    'if ((Test-ConversationManagementControls -Controls $controls) -or $attempt -eq 2)',
    '"safe/conversation_management_structure"',
    '"supervised/conversation_mutations"',
    'pin_round_trip_verified = $pinRoundTripVerified',
    'mutations_invoked = $mutationsInvoked',
    'view_mode_restored = -not $viewModeChanged',
    "sent_messages = 0",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false",
    "CHATGPT_WEB_CONVERSATION_MANAGEMENT_STATUS=passed"
)) {
    if (-not $source.Contains($required)) {
        throw "Conversation management smoke is missing: $required"
    }
}
$confirmationIndex = $source.IndexOf('if ($ConfirmPinRoundTrip)')
$mutationIndex = if ($confirmationIndex -ge 0) {
    $source.IndexOf('Invoke-ConversationPinToggle -PinControl $pinControl', $confirmationIndex)
} else {
    -1
}
if ($confirmationIndex -lt 0 -or $mutationIndex -lt 0 -or $confirmationIndex -gt $mutationIndex) {
    throw "Conversation mutation must remain behind explicit confirmation."
}
$rollbackIndex = $source.LastIndexOf('Restore-ConversationPinState')
$conversationRestoreIndex = $source.LastIndexOf('Restore-ConversationManagementOrigin')
$viewRestoreIndex = $source.LastIndexOf('Restore-OriginalViewMode')
if ($rollbackIndex -lt 0 -or $conversationRestoreIndex -lt 0 -or $viewRestoreIndex -lt 0 -or
    $rollbackIndex -gt $conversationRestoreIndex -or $conversationRestoreIndex -gt $viewRestoreIndex) {
    throw "Conversation and pin recovery must run before view-mode restoration."
}
$viewRestoreBeforeEvidenceIndex = $source.IndexOf('Restore-OriginalViewMode', $mutationIndex)
$registerEvidenceIndex = $source.IndexOf('Register-ChatGptWebVerificationCases', $mutationIndex)
if ($viewRestoreBeforeEvidenceIndex -lt 0 -or $registerEvidenceIndex -lt 0 -or
    $viewRestoreBeforeEvidenceIndex -gt $registerEvidenceIndex) {
    throw "Conversation evidence must be registered only after view-mode restoration."
}
$sampleIndex = $source.LastIndexOf('Open-ConversationManagementSample')
$menuIndex = $source.IndexOf('$openedMenu = Open-ConversationManagementMenu', $sampleIndex)
if ($sampleIndex -lt 0 -or $menuIndex -lt 0 -or $sampleIndex -gt $menuIndex) {
    throw "Conversation management must open a deterministic history sample before the menu."
}
foreach ($forbidden in @("pm clear", "removeAllCookies", "send_input")) {
    if ($source.Contains($forbidden)) {
        throw "Conversation management smoke contains forbidden operation: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_CONVERSATION_MANAGEMENT_CONTRACT=passed"
