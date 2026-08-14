#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-message-actions.ps1"
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
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Assert-ChatGptWebSmokeAdapterVersion",
    "Dismiss-VisibleOverlays",
    "Restore-OriginalViewMode",
    'view_mode = "official"',
    'view_mode = "native"',
    "native_trigger_content_description",
    "chatgpt_reveal_message",
    'target = "actions"',
    "native_message_revealed",
    "native_message_selector_found",
    'Get-Controls -Semantic "save_to_project" -Region "message"',
    "save_to_project_discovered = `$true",
    "save_to_project_context_bound = `$true",
    "save_to_project_native_selector_found",
    "save_to_project_invoked = 0",
    "native_overlay_selector_exported",
    "Wait-ContextualOverlay",
    "context_bound = `$true",
    "conversation_restored = `$true",
    "view_mode_restored = `$true",
    "sent_messages = 0",
    "copied_messages = 0",
    "started_audio = 0",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false",
    "CHATGPT_WEB_MESSAGE_ACTION_ACCEPTANCE=passed"
)
foreach ($needle in $required) {
    if (-not $source.Contains($needle)) {
        throw "ChatGPT message action smoke contract is missing: $needle"
    }
}

foreach ($forbidden in @(
    "send_input",
    "chatgpt_remove_attachment",
    "chatgpt_stop_generation",
    'control_id = [string]$saveToProject.control_id',
    "removeAllCookies",
    "pm clear"
)) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT message action smoke contains a forbidden action: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_MESSAGE_ACTION_SMOKE_CONTRACT=passed"
