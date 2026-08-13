$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-session-recovery.ps1"
$source = Get-Content -LiteralPath $path -Raw

$required = @(
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Open-ChatGptWebNativeChatSurface",
    "Get-ChatGptWebNativeChatState",
    "ExpectedAdapterVersion",
    'active_surface -ne "social_ai"',
    'web_chat_provider_id -ne "chatgpt_web"',
    "web_chat_composer_ready",
    "web_chat_authenticated",
    "message_shape_sha256",
    "A non-empty native ChatGPT Web AI conversation has no safe restorable path",
    'Invoke-ElonNativeCommand -FilePath $runtime.adb',
    'if ($result.ExitCode -eq 1 -and -not $result.TimedOut) { return "" }',
    '"force-stop", "com.elon.app"',
    'Register-ChatGptWebVerificationCases',
    '-CaseIds @("safe/session_recovery")',
    'private_content_emitted = $false',
    'native_chat_surface = $true',
    'process_recreated = $true',
    'process_stop_observed = $true',
    'conversation_identity_restored = $true',
    'sent_messages = 0',
    'uploaded_attachments = 0',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    'CHATGPT_WEB_NATIVE_SESSION_RECOVERY_STATUS=passed'
)
foreach ($needle in $required) {
    if (-not $source.Contains($needle)) {
        throw "ChatGPT native session recovery smoke contract is missing: $needle"
    }
}
foreach ($forbidden in @(
    "pm clear",
    "removeAllCookies",
    "send_input",
    "request_attachment_upload",
    ".content -join",
    "ConvertTo-Json -InputObject `$State.social_chat.messages"
)) {
    if ($source.Contains($forbidden)) {
        throw "Native session recovery smoke contains a forbidden side effect or private output: $forbidden"
    }
}
if ($source.Contains('"shell", "sh", "-c"')) {
    throw "Session recovery smoke must pass pidof directly through adb on Windows."
}

$registerIndex = $source.IndexOf('-CaseIds @("safe/session_recovery")')
$recoveredIndex = $source.IndexOf('$recovered = Wait-NativeIdentity')
if ($registerIndex -le $recoveredIndex) {
    throw "Recovery evidence must be registered only after native identity restoration."
}

Write-Output "CHATGPT_WEB_NATIVE_SESSION_RECOVERY_CONTRACT=passed"
