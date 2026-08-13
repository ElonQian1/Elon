$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-anonymous-chat.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$parseErrors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$parseErrors
)
if (@($parseErrors).Count -gt 0) {
    throw "Anonymous ChatGPT Web smoke has parse errors: $($parseErrors[0].Message)"
}

foreach ($required in @(
    '[switch]$SendProbe',
    'ProbeMarker requires -SendProbe',
    '$state.authenticated -eq $false',
    '$state.login_required -eq $false',
    '$state.composer_ready -eq $true',
    '$matrix.chat_access_available -ne $true',
    'Start-ChatGptWebSmokeIsolatedConversation',
    'Wait-ChatGptProbeReply',
    'Restore-ChatGptWebSmokeOrigin',
    'Register-ChatGptWebVerificationCases',
    'reversible/anonymous_send_probe',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    'private_content_emitted = $false',
    'CHATGPT_WEB_ANONYMOUS_SMOKE_STATUS=passed'
)) {
    if (-not $source.Contains($required)) {
        throw "Anonymous ChatGPT Web smoke contract is missing: $required"
    }
}

foreach ($forbidden in @(
    'pm clear',
    'removeAllCookies',
    'chatgpt_logout',
    'logout',
    'account_email',
    'conversation_title'
)) {
    if ($source.Contains($forbidden)) {
        throw "Anonymous ChatGPT Web smoke contains forbidden behavior: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_ANONYMOUS_SMOKE_CONTRACT=passed"
