$ErrorActionPreference = 'Stop'
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1'), [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw 'Native surface runtime has a parse error.' }
$function = $ast.Find({ param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -eq 'Open-WebChatNativeChatSurface'
}, $true)
if (-not $function) { throw 'Missing production native surface entry owner.' }
. ([scriptblock]::Create($function.Extent.Text))

function Invoke-ChatGptWebSmokeAction {
    param($Runtime, $Action, $Arguments, [switch]$EnsureMainActivity)
    return [pscustomobject]@{control_ok=$true}
}
function Wait-ChatGptWebSmokeState {
    param($Runtime, $TimeoutSec, [switch]$MainState, $Description, $Predicate)
    return & $Predicate $global:NativeForegroundFixtureState
}
function Test-WebChatNativeChatSurfaceForeground {
    param($Runtime)
    $global:NativeForegroundFixtureQueries++
    return $global:NativeForegroundFixtureVisible
}

$global:NativeForegroundFixtureState = [pscustomobject]@{
    active_surface='social_ai'
    social_chat=[pscustomobject]@{
        interaction_mode='chat';web_chat_provider_id='chatgpt_web'
        web_chat_state='ready';web_chat_composer_ready=$true
    }
}
$global:NativeForegroundFixtureQueries = 0
$global:NativeForegroundFixtureVisible = $false
try {
    $runtime = [pscustomobject]@{device_serial='synthetic'}
    if (Open-WebChatNativeChatSurface -Runtime $runtime -ProviderId chatgpt_web -TimeoutSec 10) {
        throw 'Background ready MCP state was mistaken for a foreground native UI.'
    }
    if ($global:NativeForegroundFixtureQueries -ne 1) { throw 'Foreground was not checked.' }
    $global:NativeForegroundFixtureVisible = $true
    if (-not (Open-WebChatNativeChatSurface -Runtime $runtime -ProviderId chatgpt_web -TimeoutSec 10)) {
        throw 'A foreground ready native chat should be accepted.'
    }
    $global:NativeForegroundFixtureState.social_chat.web_chat_composer_ready = $false
    $before = $global:NativeForegroundFixtureQueries
    if (Open-WebChatNativeChatSurface -Runtime $runtime -ProviderId chatgpt_web -TimeoutSec 10) {
        throw 'Foreground does not imply composer readiness.'
    }
    if ($global:NativeForegroundFixtureQueries -ne $before) { throw 'Unready state need not query ADB.' }
    $global:NativeForegroundFixtureState.social_chat.web_chat_composer_ready = $true
    $global:NativeForegroundFixtureState.social_chat.web_chat_provider_id = 'google_web'
    if (-not (Open-WebChatNativeChatSurface -Runtime $runtime -ProviderId google_web -TimeoutSec 10)) {
        throw 'Google native entry must use the same foreground guard.'
    }
    Write-Output 'WEB_CHAT_NATIVE_FOREGROUND=passed cases=4'
} finally {
    Remove-Variable NativeForegroundFixtureState,NativeForegroundFixtureQueries,NativeForegroundFixtureVisible `
        -Scope Global -ErrorAction SilentlyContinue
}
