#requires -Version 5.1

$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$scriptPath = Join-Path $root "scripts\smoke-chatgpt-web-realtime-private-refresh.ps1"
$source = Get-Content -LiteralPath $scriptPath -Raw
$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
    $scriptPath,
    [ref]$tokens,
    [ref]$errors
) | Out-Null
if (@($errors).Count -ne 0) {
    throw "Realtime private refresh smoke script has PowerShell parse errors."
}

$required = @(
    'ValidateSet("Start", "ObserveRoundTrip", "VerifyClosed", "StartReentry", "VerifyReentryClosed")',
    'start_web_chat_realtime_voice',
    'web-chat-realtime-voice:surface',
    'web-chat-realtime-voice:close',
    'Its close control is only',
    'private transcript refresh after voice closure',
    'private_outcome\|success',
    'private_refresh_observed',
    'voice_round_trip_confirmed',
    'conversation_binding_sha256',
    'raw_conversation_identifier_persisted = $false',
    'private_content_emitted = $false',
    'audio_content_read = $false',
    'cookie_or_app_data_cleared = $false',
    'Remove-VoiceDump',
    'foreach ($attempt in 1..5)',
    'Realtime voice semantic surface dump was not readable.',
    'Open-ChatGptWebNativeChatSurface',
    'Assert-ChatGptWebSmokeTrustedDevice'
)
foreach ($item in $required) {
    if (-not $source.Contains($item)) {
        throw "Realtime private refresh smoke contract is missing: $item"
    }
}

$forbidden = @(
    'pm clear',
    'removeAllCookies',
    'CookieManager.getInstance().remove',
    'conversation_url =',
    'conversation_path =',
    'message_content =',
    'Authorization =',
    'Cookie ='
)
foreach ($item in $forbidden) {
    if ($source.Contains($item)) {
        throw "Realtime private refresh smoke contract contains forbidden behavior: $item"
    }
}

Write-Host "ChatGPT realtime private refresh smoke contract passed."
