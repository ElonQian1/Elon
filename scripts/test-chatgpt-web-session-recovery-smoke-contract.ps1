$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-session-recovery.ps1"
$source = Get-Content -LiteralPath $path -Raw

$required = @(
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Wait-ReadySession",
    '"force-stop", "com.elon.app"',
    "ChatGPT conversation identity changed",
    "ChatGPT view mode was not restored",
    "process_recreated = `$true",
    "process_stop_observed = `$true",
    "conversation_identity_restored = `$true",
    "sent_messages = 0",
    "uploaded_attachments = 0",
    "cleared_cookies = `$false",
    "cleared_app_data = `$false",
    "CHATGPT_WEB_SESSION_RECOVERY_SMOKE_STATUS=passed"
)
foreach ($needle in $required) {
    if (-not $source.Contains($needle)) {
        throw "ChatGPT session recovery smoke contract is missing: $needle"
    }
}
if ($source.Contains("pm clear") -or $source.Contains("removeAllCookies")) {
    throw "Session recovery smoke must preserve app data and cookies."
}

Write-Output "CHATGPT_WEB_SESSION_RECOVERY_SMOKE_CONTRACT=passed"
