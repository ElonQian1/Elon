#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-long-running-stability.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) { throw "Long-running stability smoke has parse errors." }

foreach ($required in @(
    "DurationMinutes = 120",
    "PollIntervalSec = 30",
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Test-ChatGptWebSmokeActivityForeground",
    "Wait-ChatGptWebSmokeAuthenticatedReady",
    "conversation_binding_sha256",
    "message_count = `$initialMessageCount",
    '"safe/session_long_running_stability"',
    'status = "running"',
    'status = "passed"',
    "sent_messages = 0",
    "private_content_emitted = `$false",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false",
    "CHATGPT_WEB_LONG_RUNNING_STABILITY_STATUS=passed"
)) {
    if (-not $source.Contains($required)) {
        throw "Long-running stability smoke is missing: $required"
    }
}
foreach ($forbidden in @("pm clear", "removeAllCookies", "send_input")) {
    if ($source.Contains($forbidden)) {
        throw "Long-running stability smoke contains forbidden operation: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_LONG_RUNNING_STABILITY_CONTRACT=passed"
