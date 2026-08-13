$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-safe-batch.ps1"
$runtimePath = Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1"
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

. $runtimePath
$adapterState = [pscustomobject]@{ adapter_version = 63 }
Assert-ChatGptWebSmokeAdapterVersion -State $adapterState -ExpectedAdapterVersion 63
$staleAdapterState = [pscustomobject]@{ adapter_version = 62 }
$mismatchRejected = $false
try {
    Assert-ChatGptWebSmokeAdapterVersion -State $staleAdapterState -ExpectedAdapterVersion 63
} catch {
    $mismatchRejected = $_.Exception.Message -match 'expected=63 actual=62'
}
if (-not $mismatchRejected) {
    throw "ChatGPT Web adapter version mismatch was not rejected."
}

$required = @(
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Get-ChatGptWebSmokeUserReadiness",
    "NotifyWhenLocked:(-not `$SkipUnlockNotification)",
    "CHATGPT_WEB_SAFE_ACCEPTANCE_STATUS=user_action_required",
    "required_action=unlock_device",
    "Start-ChatGptWebSmokeAwakeLease",
    "Stop-ChatGptWebSmokeAwakeLease",
    "ExpectedHardwareSerial = `$ExpectedHardwareSerial",
    "ExpectedAdapterVersion = `$ExpectedAdapterVersion",
    'id = "read_only_surface"',
    'AllowStaleDeviceEvidence = $true',
    'id = "feature_pages"',
    'id = "settings_structure"',
    'id = "session_recovery"',
    'id = "conversation_management_structure"',
    'TimeoutSec = $ReadyTimeoutSec',
    "user_assisted_remaining",
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
    "smoke-chatgpt-web-settings.ps1",
    "smoke-chatgpt-web-session-recovery.ps1"
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
if ($source.Contains('id = "message_actions"')) {
    throw "Safe acceptance batch must not depend on a pre-existing conversation message."
}

Write-Output "CHATGPT_WEB_SAFE_ACCEPTANCE_BATCH_CONTRACT=passed"
