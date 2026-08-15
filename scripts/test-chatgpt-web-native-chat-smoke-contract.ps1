#requires -Version 5.1

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$smokePath = Join-Path $root "scripts/smoke-chatgpt-web-native-chat.ps1"
$runtimePath = Join-Path $root "scripts/chatgpt-web-smoke-runtime.ps1"
$smoke = Get-Content -LiteralPath $smokePath -Raw
$runtime = Get-Content -LiteralPath $runtimePath -Raw

foreach ($token in @(
    'Open-WebChatNativeChatSurface',
    '-ProviderId "chatgpt_web"',
    'get_web_chat_navigation',
    'start_new_web_chat_conversation',
    'set_input_text',
    'send_input',
    'Wait-ChatGptWebNativeProbeReply',
    'open_web_chat_conversation',
    'private_content_emitted = $false',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    'CHATGPT_WEB_NATIVE_CHAT_SMOKE_STATUS=passed'
)) {
    if (-not $smoke.Contains($token)) {
        throw "Native ChatGPT Web smoke is missing required token: $token"
    }
}
if ($smoke.Contains('chatgpt_web_auth')) {
    throw "Native ChatGPT Web smoke must not route through the legacy login page."
}
if (-not $runtime.Contains('function Test-WebChatNativeChatSurfaceForeground')) {
    throw "Native web chat smoke runtime must expose a production foreground guard."
}

Write-Output "CHATGPT_WEB_NATIVE_CHAT_SMOKE_CONTRACT=passed"
