$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "smoke-google-web-native-chat.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$parseErrors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$parseErrors
)
if (@($parseErrors).Count -gt 0) {
    throw "Google Web native chat smoke has parse errors: $($parseErrors[0].Message)"
}

foreach ($required in @(
    '[switch]$SendProbe',
    '[switch]$RequireStreamingTransition',
    '[ValidateRange(100, 2000)][int]$StatePollIntervalMs = 250',
    'Probe arguments require -SendProbe',
    'RequireStreamingTransition requires -SendProbe',
    'Prompt and ExpectedReply must be provided together',
    'probe_kind = if ($Prompt) { "custom_exact" } else { "marker_exact" }',
    'Open-WebChatNativeChatSurface',
    '-ProviderId "google_web"',
    'web_chat_adapter_version',
    'web_chat_composer_ready',
    'start_new_web_chat_conversation',
    '-Action "set_input_text"',
    '-Action "send_input"',
    'Wait-GoogleWebProbeReply',
    'web_chat_streaming',
    '$state.social_chat.web_chat_streaming -ne $true',
    'completion_after_streaming',
    'state_poll_interval_ms = $StatePollIntervalMs',
    'Google Web AI completed without an observable native streaming transition.',
    '-ExpectedReply $probeExpectedReply',
    'Restore-WebChatNativeConversation',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    'private_content_emitted = $false',
    'GOOGLE_WEB_NATIVE_CHAT_SMOKE_STATUS=passed'
)) {
    if (-not $source.Contains($required)) {
        throw "Google Web native chat smoke contract is missing: $required"
    }
}

foreach ($forbidden in @(
    'pm clear',
    'removeAllCookies',
    'document.cookie',
    'account_email',
    'conversation_title',
    'ConvertTo-Json -Depth 100'
)) {
    if ($source.Contains($forbidden)) {
        throw "Google Web native chat smoke contains forbidden behavior: $forbidden"
    }
}

$sendCount = @([regex]::Matches($source, '-Action "send_input"')).Count
if ($sendCount -ne 1) {
    throw "Google Web native chat smoke must contain exactly one supervised send action."
}

Write-Output "GOOGLE_WEB_NATIVE_CHAT_SMOKE_CONTRACT=passed"
