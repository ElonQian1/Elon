#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "chatgpt-web-smoke-supervised-runtime.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) {
    throw "Supervised ChatGPT Web smoke runtime has PowerShell parse errors."
}

foreach ($required in @(
    "function Get-ChatGptWebSmokeConversationPath",
    "'/c/[A-Za-z0-9_-]{1,160}'",
    "function Invoke-ChatGptWebSmokeReceiptAction",
    "Wait-ChatGptCommandReceipt",
    "function Start-ChatGptWebSmokeIsolatedConversation",
    'chatgpt_new_conversation',
    '[int]$state.conversation.message_count -eq 0',
    '[int]$state.input.text_length -eq 0',
    "function Restore-ChatGptWebSmokeOrigin",
    'chatgpt_open_conversation',
    'Open-ChatGptWebSmokeSurface -Runtime $Runtime'
)) {
    if (-not $source.Contains($required)) {
        throw "Supervised ChatGPT Web smoke runtime contract is missing: $required"
    }
}

foreach ($forbidden in @(
    "chatgpt_select_view",
    "origin_view_mode",
    "pm clear",
    "removeAllCookies",
    "input tap",
    "KEYCODE_ENTER",
    ".conversation.title",
    ".conversation.messages",
    ".attachments",
    "cookie",
    "password",
    "authorization"
)) {
    if ($source.Contains($forbidden)) {
        throw "Supervised ChatGPT Web smoke runtime contains forbidden data or action: $forbidden"
    }
}

if (@($source -split "`n").Count -gt 140) {
    throw "Supervised ChatGPT Web smoke runtime exceeded its modular size budget."
}

Write-Output "CHATGPT_WEB_SUPERVISED_RUNTIME_CONTRACT=passed"
