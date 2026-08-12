#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "inspect-chatgpt-web-regenerate-menu.ps1"
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

foreach ($required in @(
    "ExpectedHardwareSerial",
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Assert-ChatGptWebSmokeAdapterVersion",
    'Invoke-ReceiptAction -Action "chatgpt_new_conversation"',
    'isolated regenerate-menu conversation',
    'regenerate-menu probe draft synchronization',
    'Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"',
    "function Convert-SafeControl",
    "function Dismiss-Overlays",
    'Get-Controls -Region "message"',
    'message_controls = $safeMessageControls',
    'Get-Controls -Semantic "more" -Region "message"',
    'Get-Controls -Semantic "model" -Region "message"',
    'CHATGPT_REGENERATE_MENU_PHASE phase=open_model_menu',
    'model_overlay_controls = $safeModelControls',
    'Invoke-ReceiptAction -Action "chatgpt_invoke_control"',
    'Get-Controls -Region "overlay"',
    "ConvertTo-ChatGptWebSmokeSafeDiagnostic",
    '$safe.context_bound = [string]$_.context_id -eq [string]$messageMore.context_id',
    'original_conversation_restored = $true',
    'original_view_mode_restored = $true',
    'sent_messages = 1',
    'private_content_emitted = $false',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    "CHATGPT_WEB_REGENERATE_MENU_DIAGNOSTIC=passed"
)) {
    if (-not $source.Contains($required)) {
        throw "ChatGPT regenerate-menu diagnostic contract is missing: $required"
    }
}

foreach ($forbidden in @(
    "removeAllCookies",
    "pm clear",
    "password",
    "access_token",
    "conversation.messages.content"
)) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT regenerate-menu diagnostic contains a forbidden action: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_REGENERATE_MENU_DIAGNOSTIC_CONTRACT=passed"
