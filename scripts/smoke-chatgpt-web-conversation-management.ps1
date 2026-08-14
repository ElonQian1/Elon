#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [Parameter(Mandatory = $true)][string]$ExpectedHardwareSerial,
    [ValidateRange(20, 180)][int]$TimeoutSec = 90,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [switch]$ConfirmPinRoundTrip
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-supervised-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$origin = $null
$menuOpened = $false
$conversationContextId = ""
$observedSemantics = @()
$mutationsInvoked = 0
$pinRoundTripVerified = $false
$originalViewMode = ""
$viewModeChanged = $false
$originalPinned = $false
$pinOriginalStateKnown = $false
$pinStateChanged = $false

function Wait-ConversationOptions {
    param([AllowEmptyString()][string]$ExpectedContextId = "")

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $page = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_find_controls" -Arguments @{
                semantic = "conversation_options"
                limit = 100
            }
        $control = @($page.controls) | Where-Object {
            $_.enabled -eq $true -and
                -not [string]::IsNullOrWhiteSpace([string]$_.context_id) -and
                (-not $ExpectedContextId -or [string]$_.context_id -eq $ExpectedContextId)
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

function Get-ConversationPinState {
    param([Parameter(Mandatory = $true)]$Control)

    $label = [string]$Control.label
    if ($label -match '(?i)unpin|取消置顶') { return $true }
    if ($label -match '(?i)pin(?:\s+chat)?|置顶') { return $false }
    throw "Conversation pin control does not expose a recognizable desired-state label."
}

function Open-ConversationManagementMenu {
    param([AllowEmptyString()][string]$ExpectedContextId = "")

    $options = Wait-ConversationOptions -ExpectedContextId $ExpectedContextId
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
    $script:menuOpened = $true
    $contextId = [string]$options.context_id
    return [pscustomobject]@{
        context_id = $contextId
        controls = @(Wait-ConversationManagementMenu -ContextId $contextId)
    }
}

function Close-ConversationManagementMenu {
    if (-not $script:menuOpened) { return }
    $arguments = @{ region = "overlay"; limit = 100 }
    if ($script:conversationContextId) {
        $arguments.context_id = $script:conversationContextId
    }
    $menu = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_find_controls" -Arguments $arguments
    if (@($menu.controls).Count -gt 0) {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "input", "keyevent", "4") `
            -TimeoutSec 8 -Label "close conversation management menu" | Out-Null
    }
    $script:menuOpened = $false
}

function Wait-ConversationManagementMenuClosed {
    param([Parameter(Mandatory = $true)][string]$ContextId)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds([Math]::Min($TimeoutSec, 20))
    do {
        Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
            -Action "chatgpt_refresh_controls" -TimeoutSec 15 | Out-Null
        $menu = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_find_controls" -Arguments @{
                context_id = $ContextId
                region = "overlay"
                limit = 100
            }
        if (@($menu.controls).Count -eq 0) { return $true }
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $false
}

function Invoke-ConversationPinToggle {
    param(
        [Parameter(Mandatory = $true)]$PinControl,
        [Parameter(Mandatory = $true)][string]$ContextId
    )

    $receipt = Invoke-ChatGptWebSmokeReceiptAction -Runtime $runtime `
        -Action "chatgpt_invoke_control" -ExpectedAction "invoke_ui_control" `
        -Arguments @{
            control_id = [string]$PinControl.control_id
            user_confirmed = $true
        } `
        -TimeoutSec $TimeoutSec
    if ($receipt.receipt.result.ok -ne $true) {
        throw "Conversation pin command did not succeed."
    }
    if (Wait-ConversationManagementMenuClosed -ContextId $ContextId) {
        $script:menuOpened = $false
    } else {
        Close-ConversationManagementMenu
    }
    $script:mutationsInvoked++
    Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $TimeoutSec -InitialWaitSec 2 | Out-Null
}

function Restore-OriginalViewMode {
    if (-not $script:viewModeChanged) { return }
    $targetMode = switch ($script:originalViewMode) {
        "native" { "native" }
        "web" { "official" }
        "quick" { "quick" }
        default { throw "Unsupported original ChatGPT view mode." }
    }
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = $targetMode } | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
        -Description "original ChatGPT view mode restoration" -Predicate {
            param($state)
            [string]$state.view_mode -eq $script:originalViewMode -and
                $state.bridge_state -eq "ready"
        } | Out-Null
    $script:viewModeChanged = $false
}

function Restore-ConversationPinState {
    if (-not $script:pinStateChanged -or -not $script:pinOriginalStateKnown) { return }
    Close-ConversationManagementMenu
    $restorationMenu = Open-ConversationManagementMenu `
        -ExpectedContextId $script:conversationContextId
    $pinControl = @($restorationMenu.controls | Where-Object {
        [string]$_.semantic -eq "pin" -and $_.enabled -eq $true
    }) | Select-Object -First 1
    if ($null -eq $pinControl) { throw "Pin control is unavailable during recovery." }
    if ((Get-ConversationPinState -Control $pinControl) -ne $script:originalPinned) {
        Invoke-ConversationPinToggle -PinControl $pinControl `
            -ContextId $script:conversationContextId
        $verificationMenu = Open-ConversationManagementMenu `
            -ExpectedContextId $script:conversationContextId
        $verificationPin = @($verificationMenu.controls | Where-Object {
            [string]$_.semantic -eq "pin" -and $_.enabled -eq $true
        }) | Select-Object -First 1
        if ($null -eq $verificationPin) {
            throw "Pin control is unavailable after recovery."
        }
        if ((Get-ConversationPinState -Control $verificationPin) -ne $script:originalPinned) {
            throw "Conversation pin state recovery could not be verified."
        }
        Close-ConversationManagementMenu
    } else {
        Close-ConversationManagementMenu
    }
    $script:pinStateChanged = $false
}

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $TimeoutSec -InitialWaitSec ([Math]::Min(15, $TimeoutSec))
    Assert-ChatGptWebSmokeAdapterVersion -State $origin `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    $originalViewMode = [string]$origin.view_mode
    if ([string]$origin.view_mode -ne "web") {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
            -Arguments @{ view_mode = "official" } | Out-Null
        $viewModeChanged = $true
        $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
            -TimeoutSec $TimeoutSec -InitialWaitSec 5
    }

    Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
        -Action "chatgpt_list_features" -TimeoutSec $TimeoutSec | Out-Null
    $openedMenu = Open-ConversationManagementMenu
    $conversationContextId = [string]$openedMenu.context_id
    $menuControls = @($openedMenu.controls)
    $observedSemantics = @(
        $menuControls |
            Where-Object { [string]$_.semantic -ne "conversation_options" } |
            ForEach-Object { [string]$_.semantic } |
            Sort-Object -Unique
    )
    $observedRoles = @(
        $menuControls |
            ForEach-Object { [string]$_.role } |
            Where-Object { $_ } |
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
        $safeSemantics = ($observedSemantics -join ",").Replace(" ", "")
        $safeRoles = ($observedRoles -join ",").Replace(" ", "")
        throw "Conversation menu contains no recognized management action; semantics=$safeSemantics roles=$safeRoles."
    }

    if ($ConfirmPinRoundTrip) {
        $pinControl = @($menuControls | Where-Object {
            [string]$_.semantic -eq "pin" -and $_.enabled -eq $true
        }) | Select-Object -First 1
        if ($null -eq $pinControl) {
            throw "The current conversation does not expose a reversible pin control."
        }
        $originalPinned = Get-ConversationPinState -Control $pinControl
        $pinOriginalStateKnown = $true
        $pinStateChanged = $true
        Invoke-ConversationPinToggle -PinControl $pinControl -ContextId $conversationContextId

        $toggledMenu = Open-ConversationManagementMenu -ExpectedContextId $conversationContextId
        $toggledPin = @($toggledMenu.controls | Where-Object {
            [string]$_.semantic -eq "pin" -and $_.enabled -eq $true
        }) | Select-Object -First 1
        if ($null -eq $toggledPin) { throw "Pin control disappeared after the first mutation." }
        $toggledPinned = Get-ConversationPinState -Control $toggledPin
        if ($toggledPinned -eq $originalPinned) {
            throw "Conversation pin state did not change after the first mutation."
        }
        Invoke-ConversationPinToggle -PinControl $toggledPin -ContextId $conversationContextId

        $restoredMenu = Open-ConversationManagementMenu -ExpectedContextId $conversationContextId
        $restoredPin = @($restoredMenu.controls | Where-Object {
            [string]$_.semantic -eq "pin" -and $_.enabled -eq $true
        }) | Select-Object -First 1
        if ($null -eq $restoredPin) { throw "Pin control disappeared during restoration." }
        if ((Get-ConversationPinState -Control $restoredPin) -ne $originalPinned) {
            throw "Conversation pin state was not restored."
        }
        $pinStateChanged = $false
        $pinRoundTripVerified = $true
    }

    Close-ConversationManagementMenu
    Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $TimeoutSec -InitialWaitSec 5 | Out-Null

    Restore-OriginalViewMode
    $caseIds = @("safe/conversation_management_structure")
    if ($pinRoundTripVerified) { $caseIds += "supervised/conversation_mutations" }
    Register-ChatGptWebVerificationCases -Runtime $runtime `
        -CaseIds $caseIds `
        -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
    [ordered]@{
        schema = "elon.chatgpt_web.conversation_management_smoke.v1"
        passed = $true
        adapter_version = $ExpectedAdapterVersion
        context_bound = $true
        stable_native_selector = $true
        observed_semantics = $observedSemantics
        pin_round_trip_requested = [bool]$ConfirmPinRoundTrip
        pin_round_trip_verified = $pinRoundTripVerified
        mutations_invoked = $mutationsInvoked
        view_mode_restored = -not $viewModeChanged
        sent_messages = 0
        cleared_cookies = $false
        cleared_app_data = $false
    } | ConvertTo-Json -Depth 6
    Write-Output "CHATGPT_WEB_CONVERSATION_MANAGEMENT_STATUS=passed"
} finally {
    $cleanupFailures = [System.Collections.Generic.List[string]]::new()
    try { Restore-ConversationPinState } catch { $cleanupFailures.Add("pin state") }
    try { Close-ConversationManagementMenu } catch { $cleanupFailures.Add("menu") }
    try { Restore-OriginalViewMode } catch { $cleanupFailures.Add("view mode") }
    try {
        Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    } catch {
        $cleanupFailures.Add("awake lease")
    }
    if ($cleanupFailures.Count -gt 0) {
        throw "Conversation management cleanup failed: $($cleanupFailures -join ', ')."
    }
}
