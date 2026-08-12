#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-regenerate.ps1"
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
    "function Wait-RegeneratedReply",
    'Invoke-ReceiptAction -Action "chatgpt_new_conversation"',
    'isolated blank regenerate conversation',
    'Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"',
    '-Action "chatgpt_regenerate_response"',
    'expected_web_action -eq "regenerate_response"',
    'assistant_identity_changed',
    'assistant_content_changed',
    'regenerated_assistant_completed = $true',
    'original_conversation_restored = $true',
    'original_view_mode_restored = $true',
    'sent_messages = 1',
    'regenerated_messages = 1',
    'private_content_emitted = $false',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    "CHATGPT_WEB_REGENERATE_ACCEPTANCE=passed"
)) {
    if (-not $source.Contains($required)) {
        throw "ChatGPT regenerate smoke contract is missing: $required"
    }
}

foreach ($forbidden in @(
    "chatgpt_remove_attachment",
    "chatgpt_delete",
    "removeAllCookies",
    "pm clear",
    "password",
    "access_token"
)) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT regenerate smoke contains a forbidden action: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_REGENERATE_SMOKE_CONTRACT=passed"
