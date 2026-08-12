#requires -Version 5.1

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = [pscustomobject]@{ poll_interval_sec = 1 }
$script:stateReads = 0
$script:actionAttempts = 0

function script:Invoke-ChatGptWebSmokeMcp {
    param($Runtime, [string]$Tool)

    if ($Tool -ne "ui_state") { throw "Unexpected tool: $Tool" }
    $script:stateReads += 1
    [pscustomobject]@{
        surface = "chatgpt_web"
        bridge_state = "ready"
        adapter_current = $true
    }
}

function script:Invoke-ChatGptWebSmokeAction {
    param($Runtime, [string]$Action, [hashtable]$Arguments, [switch]$EnsureMainActivity)

    $script:actionAttempts += 1
    if ($script:actionAttempts -eq 1) {
        throw "APK MCP tool failed: tool=ui_control action=$Action error=bridge_not_ready"
    }
    [pscustomobject]@{
        control_ok = $true
        action = $Action
        marker = [string]$Arguments.marker
    }
}

$result = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
    -Action "chatgpt_refresh_controls" -Arguments @{ marker = "synthetic" } `
    -TimeoutSec 5
if ($result.control_ok -ne $true -or $result.marker -ne "synthetic") {
    throw "Ready action did not return the successful dispatch result."
}
if ($script:stateReads -ne 2 -or $script:actionAttempts -ne 2) {
    throw "Ready action did not retry exactly one pre-dispatch bridge race."
}

$script:stateReads = 0
$script:actionAttempts = 0
function script:Invoke-ChatGptWebSmokeAction {
    param($Runtime, [string]$Action, [hashtable]$Arguments, [switch]$EnsureMainActivity)

    $script:actionAttempts += 1
    if ($script:actionAttempts -eq 1) {
        throw "APK MCP tool failed: tool=ui_control action=$Action error=adapter_generation_not_ready"
    }
    [pscustomobject]@{ control_ok = $true; action = $Action }
}

$generationResult = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
    -Action "chatgpt_list_features" -TimeoutSec 5
if (
    $generationResult.control_ok -ne $true -or
    $script:stateReads -ne 2 -or
    $script:actionAttempts -ne 2
) {
    throw "Ready action did not retry exactly one pre-dispatch adapter race."
}

$script:actionAttempts = 0
function script:Invoke-ChatGptWebSmokeAction {
    param($Runtime, [string]$Action, [hashtable]$Arguments, [switch]$EnsureMainActivity)

    $script:actionAttempts += 1
    throw "APK MCP tool failed: tool=ui_control action=$Action error=stale_control_id"
}

$nonRetryableRejected = $false
try {
    Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
        -Action "chatgpt_invoke_control" -TimeoutSec 5 | Out-Null
} catch {
    $nonRetryableRejected = $_.Exception.Message -match "stale_control_id"
}
if (-not $nonRetryableRejected -or $script:actionAttempts -ne 1) {
    throw "Ready action retried a failure that may follow command dispatch."
}

Write-Output "CHATGPT_WEB_SMOKE_READY_ACTION_TEST=passed"
