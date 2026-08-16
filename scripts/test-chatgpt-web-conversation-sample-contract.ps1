#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "chatgpt-web-smoke-conversation-sample.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) {
    throw "Conversation sample helper has PowerShell parse errors."
}

foreach ($required in @(
    "function Open-ChatGptWebSmokeConversationSample",
    'Action "chatgpt_list_conversations"',
    'Action "chatgpt_get_conversations"',
    'Action "chatgpt_open_conversation"',
    "Invoke-ChatGptWebSmokeReceiptAction",
    '[int]$candidate.conversation.message_count -ge $MinimumMessageCount',
    "read-only ChatGPT conversation sample"
)) {
    if (-not $source.Contains($required)) {
        throw "Conversation sample helper is missing: $required"
    }
}

foreach ($forbidden in @(
    "send_input",
    "pm clear",
    "removeAllCookies",
    ".conversation.title",
    ".conversation.messages"
)) {
    if ($source.Contains($forbidden)) {
        throw "Conversation sample helper contains forbidden behavior: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_CONVERSATION_SAMPLE_CONTRACT=passed"
