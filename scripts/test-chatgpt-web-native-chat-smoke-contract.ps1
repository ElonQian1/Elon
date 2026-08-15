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
    'Register-ChatGptWebVerificationCases',
    '-CaseIds @("reversible/send_probe")',
    'Restore-WebChatNativeConversation',
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
if (-not $runtime.Contains('function Restore-WebChatNativeConversation')) {
    throw "Native web chat smoke runtime must expose background-safe conversation recovery."
}

. $runtimePath
$script:restoreMcpCalls = 0
$script:restoreActionCalls = 0
$script:restoredPath = ""
function Invoke-ChatGptWebSmokeMcp {
    param($Runtime, [string]$Tool)
    $script:restoreMcpCalls += 1
    if ($script:restoreMcpCalls -eq 1) { throw "temporary MCP interruption" }
    return [pscustomobject]@{
        social_chat = [pscustomobject]@{
            web_chat_provider_id = "chatgpt_web"
            web_chat_conversation_path = $script:restoredPath
        }
    }
}
function Invoke-ChatGptWebSmokeAction {
    param($Runtime, [string]$Action, [hashtable]$Arguments = @{})
    $script:restoreActionCalls += 1
    $script:restoredPath = [string]$Arguments.conversation_path
    return [pscustomobject]@{ control_ok = $true }
}
$fakeRuntime = [pscustomobject]@{ poll_interval_sec = 0; mcp_bootstrapped = $true }
$restored = Restore-WebChatNativeConversation -Runtime $fakeRuntime `
    -ProviderId "chatgpt_web" -ConversationPath "/c/acceptance" -TimeoutSec 5
if (-not $restored -or $script:restoreActionCalls -ne 1 -or $script:restoreMcpCalls -lt 3) {
    throw "Background-safe conversation recovery did not retry and restore exactly once."
}
if (Restore-WebChatNativeConversation -Runtime $fakeRuntime `
    -ProviderId "chatgpt_web" -ConversationPath " " -TimeoutSec 5) {
    throw "Background-safe conversation recovery must reject an empty path."
}

Write-Output "CHATGPT_WEB_NATIVE_CHAT_SMOKE_CONTRACT=passed"
