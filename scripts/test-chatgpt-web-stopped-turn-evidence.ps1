#requires -Version 7.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-stopped-turn-evidence.ps1')
$partial = 'ANSWER-START ' + ('synthetic partial answer ' * 5)
$argsForCase = @{ FirstPrompt = 'first'; SecondPrompt = 'second'; PartialReply = $partial; ExpectedReply = 'OK' }
$state = @{ social_chat = @{ web_chat_streaming = $false; messages = @(
    @{ role = 'user'; content = 'first' }, @{ role = 'friend'; content = $partial + ' tail' },
    @{ role = 'user'; content = 'second' }, @{ role = 'friend'; content = 'OK.' }
) } }
if (-not (Get-ChatGptStoppedTurnEvidence -State $state @argsForCase).passed) { throw 'valid continuity rejected' }
$state.social_chat.web_chat_streaming = $true
if ((Get-ChatGptStoppedTurnEvidence -State $state @argsForCase).passed) { throw 'unfinished stream accepted' }
$state.social_chat.web_chat_streaming = $false
$state.social_chat.messages[1].content = 'replacement'
if ((Get-ChatGptStoppedTurnEvidence -State $state @argsForCase).passed) { throw 'lost partial accepted' }
$state.social_chat.messages = @(@{ role = 'user'; content = "first`n`nsecond" }, @{ role = 'friend'; content = 'OK' })
$merged = Get-ChatGptStoppedTurnEvidence -State $state @argsForCase
if ($merged.passed -or $merged.prompts_separate -or $merged.stopped_reply_retained) { throw 'merged turns accepted' }
$state.social_chat.messages = @()
if ((Get-ChatGptStoppedTurnEvidence -State $state @argsForCase).passed) { throw 'empty projection accepted' }
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile(
    (Join-Path $PSScriptRoot 'smoke-chatgpt-web-stopped-followup.ps1'), [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw 'smoke script syntax failed' }

. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
$definition = $ast.Find({ param($n)
    $n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Read-Native'
}, $true)
Invoke-Expression $definition.Extent.Text
$runtime = @{}
$script:foreground = $true
$script:native = @{ active_surface = 'social_ai'; social_chat = @{
    web_chat_provider_id = 'chatgpt_web'; interaction_mode = 'chat'
} }
function Test-WebChatNativeChatSurfaceForeground { param($Runtime) return $script:foreground }
function Invoke-ChatGptWebSmokeMcp { param($Runtime, $Tool, [switch]$MainState) return $script:native }
function Assert-RejectedNativeRead([string]$Expected) {
    try { Read-Native | Out-Null } catch {
        if ($_.Exception.Message -ceq $Expected) { return }
        throw
    }
    throw 'unsafe native state accepted'
}
Read-Native | Out-Null
$script:native.active_surface = 'main'
Assert-RejectedNativeRead 'The native social AI chat surface is not active.'
$script:native.active_surface = 'social_ai'
$script:native.social_chat.web_chat_provider_id = 'google_web'
Assert-RejectedNativeRead 'native_surface_changed'
$script:native.social_chat.web_chat_provider_id = 'chatgpt_web'
$script:native.social_chat.interaction_mode = 'work'
Assert-RejectedNativeRead 'native_surface_changed'
$script:foreground = $false
Assert-RejectedNativeRead 'foreground_changed'
$action = $ast.Find({ param($n)
    $n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Act'
}, $true)
Invoke-Expression $action.Extent.Text
function Invoke-ChatGptWebSmokeAction { throw 'unsafe_shared_write_retry' }
function Invoke-StopTestMcp {
    param($Adb, $DeviceSerial, $Tool, $Arguments, [switch]$NoBootstrap, $HealthTimeoutSec, $RequestTimeoutSec, $AdbTimeoutSec)
    $script:dispatches++
    if ($Tool -ne 'ui_control' -or -not $NoBootstrap -or ($Arguments | ConvertFrom-Json).action -ne 'send_input') {
        throw 'wrong_write_contract'
    }
    if ($script:mode -eq 'timeout') { throw 'transport_result_unknown' }
    return @{ result = @{ isError = $script:mode -eq 'error'; structuredContent = @{ control_ok = $script:mode -eq 'ok' } } }
}
$runtime = @{ invoke_mcp = 'Invoke-StopTestMcp'; adb = 'synthetic'; device_serial = 'synthetic' }
foreach ($script:mode in @('ok', 'timeout', 'error', 'rejected')) {
    $script:dispatches = 0
    $failure = ''
    try { Act 'send_input' | Out-Null } catch { $failure = $_.Exception.Message }
    $expected = switch ($script:mode) {
        'ok' { '' }; 'timeout' { 'transport_result_unknown' }; 'error' { 'action_result_unconfirmed' }; 'rejected' { 'action_not_accepted' }
    }
    if ($failure -cne $expected -or $script:dispatches -ne 1) { throw 'write_replayed_or_result_misclassified' }
}
Write-Output 'STOPPED_TURN_EVIDENCE_TESTS=passed:15'
