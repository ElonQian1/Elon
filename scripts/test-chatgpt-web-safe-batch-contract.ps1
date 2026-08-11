$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-safe-batch.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$parseErrors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$parseErrors
)
if ($parseErrors.Count -gt 0) {
    throw "ChatGPT Web safe batch has PowerShell parse errors: $($parseErrors[0].Message)"
}

$required = @(
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Start-ChatGptWebSmokeAwakeLease",
    "Stop-ChatGptWebSmokeAwakeLease",
    "ExpectedHardwareSerial = `$ExpectedHardwareSerial",
    'id = "read_only_surface"',
    'id = "feature_pages"',
    'id = "session_recovery"',
    'id = "message_actions"',
    "user_assisted_remaining",
    "official_authentication",
    "attachment_lifecycle",
    "dictation_audio_capture",
    "realtime_voice",
    "sent_messages = 0",
    "uploaded_attachments = 0",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false",
    "& `$path @caseArguments",
    "CHATGPT_WEB_SAFE_ACCEPTANCE_BATCH_STATUS=passed"
)
foreach ($needle in $required) {
    if (-not $source.Contains($needle)) {
        throw "ChatGPT Web safe batch contract is missing: $needle"
    }
}
if (
    $source.Contains("-SendProbe") -or
    $source.Contains("pm clear") -or
    $source.Contains("removeAllCookies") -or
    $source.Contains("@(`$case.arguments)") -or
    $source.Contains(".Take(")
) {
    throw "Safe acceptance batch must not send messages or clear user session data."
}

foreach ($childScript in @(
    "smoke-chatgpt-web-apk.ps1",
    "smoke-chatgpt-web-feature-pages.ps1",
    "smoke-chatgpt-web-session-recovery.ps1",
    "smoke-chatgpt-web-message-actions.ps1"
)) {
    $childSource = Get-Content -LiteralPath (Join-Path $PSScriptRoot $childScript) -Raw
    if ($childSource -match '(?m)^\s*exit\s+[1-9]') {
        throw "Safe acceptance child must throw instead of terminating the batch: $childScript"
    }
}

foreach ($forbiddenCase in @('id = "reversible_controls"', 'id = "composer_controls"')) {
    if ($source.Contains($forbiddenCase)) {
        throw "Safe acceptance batch contains a reversible case: $forbiddenCase"
    }
}

Write-Output "CHATGPT_WEB_SAFE_ACCEPTANCE_BATCH_CONTRACT=passed"
