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
    'semantic = "conversation_options"',
    'context_id = $conversationContextId',
    'region = "overlay"',
    'chatgpt-conversation-actions:*',
    '"conversation_files", "pin", "archive", "share", "delete"',
    '"safe/conversation_management_structure"',
    "mutations_invoked = 0",
    "sent_messages = 0",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false",
    "CHATGPT_WEB_CONVERSATION_MANAGEMENT_STATUS=passed"
)) {
    if (-not $source.Contains($required)) {
        throw "Conversation management smoke is missing: $required"
    }
}
foreach ($forbidden in @("pm clear", "removeAllCookies", "send_input")) {
    if ($source.Contains($forbidden)) {
        throw "Conversation management smoke contains forbidden operation: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_CONVERSATION_MANAGEMENT_CONTRACT=passed"
