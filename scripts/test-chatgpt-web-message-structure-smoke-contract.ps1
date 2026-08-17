#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-message-structure.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) {
    throw "PowerShell parse failed for $path`: $($errors[0].Message)"
}

$required = @(
    "ExpectedHardwareSerial",
    "ExpectedAdapterVersion",
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Assert-ChatGptWebSmokeAdapterVersion",
    "Get-ContextWithParts",
    "Wait-BridgeReady",
    "Wait-ConversationList",
    "Open-ConversationPath",
    "bridge_not_ready",
    "stable historical conversation structure",
    "Get-VisibleMessageSelectors",
    "Wait-VisibleMessageSelectors",
    "chatgpt-message-part:",
    "native_adb_content_description",
    "chatgpt_reveal_message",
    "part_index = 0",
    "Restore-Origin",
    "[AllowEmptyString()][string]`$OriginPath",
    "finally",
    "message_part_types",
    "matched_part_selector_count",
    "reveal_action_succeeded = `$true",
    "original_conversation_restored = `$true",
    "production_surface_preserved = Test-ChatGptWebSmokeActivityForeground",
    "sent_messages = 0",
    "uploaded_attachments = 0",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false",
    "CHATGPT_WEB_MESSAGE_STRUCTURE_SMOKE_STATUS=passed"
)
foreach ($needle in $required) {
    if (-not $source.Contains($needle)) {
        throw "ChatGPT message structure smoke contract is missing: $needle"
    }
}
foreach ($forbidden in @(
    "send_input",
    "chatgpt_remove_attachment",
    "chatgpt_stop_generation",
    "removeAllCookies",
    "pm clear"
)) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT message structure smoke contains a forbidden action: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_MESSAGE_STRUCTURE_SMOKE_CONTRACT=passed"
