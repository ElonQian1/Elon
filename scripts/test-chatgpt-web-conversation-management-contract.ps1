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
    "Wait-ConversationManagementMenu",
    "Close-FeatureNavigation",
    'Action "chatgpt_dismiss_features"',
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
    '"conversation_files", "rename", "pin", "archive", "share", "delete"',
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
$viewRestoreIndex = $source.LastIndexOf('Restore-OriginalViewMode')
if ($rollbackIndex -lt 0 -or $viewRestoreIndex -lt 0 -or $rollbackIndex -gt $viewRestoreIndex) {
    throw "Conversation pin recovery must run before view-mode restoration."
}
$viewRestoreBeforeEvidenceIndex = $source.IndexOf('Restore-OriginalViewMode', $mutationIndex)
$registerEvidenceIndex = $source.IndexOf('Register-ChatGptWebVerificationCases', $mutationIndex)
if ($viewRestoreBeforeEvidenceIndex -lt 0 -or $registerEvidenceIndex -lt 0 -or
    $viewRestoreBeforeEvidenceIndex -gt $registerEvidenceIndex) {
    throw "Conversation evidence must be registered only after view-mode restoration."
}
foreach ($forbidden in @("pm clear", "removeAllCookies", "send_input")) {
    if ($source.Contains($forbidden)) {
        throw "Conversation management smoke contains forbidden operation: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_CONVERSATION_MANAGEMENT_CONTRACT=passed"
