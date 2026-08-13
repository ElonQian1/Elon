#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [Parameter(Mandatory = $true)][string]$ExpectedHardwareSerial,
    [ValidateRange(20, 180)][int]$TimeoutSec = 90,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 88
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-supervised-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$origin = $null
$menuOpened = $false
$conversationContextId = ""
$observedSemantics = @()

function Wait-ConversationOptions {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $page = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_find_controls" -Arguments @{
                semantic = "conversation_options"
                limit = 100
            }
        $control = @($page.controls) | Where-Object {
            $_.enabled -eq $true -and
                -not [string]::IsNullOrWhiteSpace([string]$_.context_id)
        } | Select-Object -First 1
        if ($null -ne $control) { return $control }
        Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
            -Action "chatgpt_refresh_controls" -TimeoutSec 15 | Out-Null
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "No conversation-scoped options control is available."
}

function Wait-ConversationManagementMenu {
    param([Parameter(Mandatory = $true)][string]$ContextId)

    $managementSemantics = @(
        "conversation_files",
        "rename",
        "pin",
        "archive",
        "share",
        "delete"
    )
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $lastControls = @()
    do {
        Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
            -Action "chatgpt_refresh_controls" -TimeoutSec 15 | Out-Null
        $menu = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_find_controls" -Arguments @{
                context_id = $ContextId
                region = "overlay"
                limit = 100
            }
        $lastControls = @($menu.controls)
        $recognized = @($lastControls | Where-Object {
            [string]$_.semantic -in $managementSemantics
        })
        if ($recognized.Count -gt 0) {
            return $lastControls
        }
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $lastControls
}

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $TimeoutSec -InitialWaitSec ([Math]::Min(15, $TimeoutSec))
    Assert-ChatGptWebSmokeAdapterVersion -State $origin `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    if ([string]$origin.view_mode -ne "web") {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
            -Arguments @{ view_mode = "official" } | Out-Null
        $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
            -TimeoutSec $TimeoutSec -InitialWaitSec 5
    }

    Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
        -Action "chatgpt_list_features" -TimeoutSec $TimeoutSec | Out-Null
    $options = Wait-ConversationOptions
    $conversationContextId = [string]$options.context_id
    if ([string]$options.native_trigger_content_description -notlike "chatgpt-conversation-actions:*") {
        throw "Conversation options do not expose a stable conversation-scoped native selector."
    }

    $receipt = Invoke-ChatGptWebSmokeReceiptAction -Runtime $runtime `
        -Action "chatgpt_invoke_control" -ExpectedAction "invoke_ui_control" `
        -Arguments @{ control_id = [string]$options.control_id } `
        -TimeoutSec $TimeoutSec
    if ($receipt.receipt.result.ok -ne $true) {
        throw "Conversation options command did not succeed."
    }
    $menuOpened = $true

    $menuControls = @(Wait-ConversationManagementMenu -ContextId $conversationContextId)
    $observedSemantics = @(
        $menuControls |
            Where-Object { [string]$_.semantic -ne "conversation_options" } |
            ForEach-Object { [string]$_.semantic } |
            Sort-Object -Unique
    )
    if ($menuControls.Count -eq 0) { throw "Conversation menu exposed no scoped controls." }
    if (@($menuControls | Where-Object { [string]$_.context_id -ne $conversationContextId }).Count -gt 0) {
        throw "Conversation menu controls escaped their triggering conversation context."
    }
    if (@($menuControls | Where-Object { [string]$_.semantic -eq "action" }).Count -gt 0) {
        throw "Conversation menu contains unknown generic controls."
    }
    if (@($observedSemantics | Where-Object {
        $_ -in @("conversation_files", "rename", "pin", "archive", "share", "delete")
    }).Count -eq 0) {
        throw "Conversation menu contains no recognized management action."
    }

    Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
        -Arguments @("shell", "input", "keyevent", "4") `
        -TimeoutSec 8 -Label "close conversation management menu" | Out-Null
    $menuOpened = $false
    Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $TimeoutSec -InitialWaitSec 5 | Out-Null

    Register-ChatGptWebVerificationCases -Runtime $runtime `
        -CaseIds @("safe/conversation_management_structure") `
        -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
    [ordered]@{
        schema = "elon.chatgpt_web.conversation_management_smoke.v1"
        passed = $true
        adapter_version = $ExpectedAdapterVersion
        context_bound = $true
        stable_native_selector = $true
        observed_semantics = $observedSemantics
        mutations_invoked = 0
        sent_messages = 0
        cleared_cookies = $false
        cleared_app_data = $false
    } | ConvertTo-Json -Depth 6
    Write-Output "CHATGPT_WEB_CONVERSATION_MANAGEMENT_STATUS=passed"
} finally {
    if ($menuOpened) {
        try {
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "input", "keyevent", "4") `
                -TimeoutSec 8 -Label "restore conversation menu state" | Out-Null
        } catch { }
    }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
