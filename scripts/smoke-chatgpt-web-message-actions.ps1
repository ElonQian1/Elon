#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 180)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 63
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$script:overlayOpened = $false
$script:originalViewMode = ""
$script:viewModeChanged = $false

function Wait-CommandReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$ExpectedAction
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        $receipt = @($state.command_requests) |
            Where-Object { [string]$_.request_id -eq $RequestId } |
            Select-Object -Last 1
        if ($null -ne $receipt -and [string]$receipt.status -eq "failed") {
            throw "ChatGPT command failed: $ExpectedAction"
        }
        if (
            $null -ne $receipt -and
            [string]$receipt.status -eq "succeeded" -and
            [string]$receipt.expected_web_action -eq $ExpectedAction -and
            $receipt.result.ok -eq $true
        ) {
            return $receipt
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT command: $ExpectedAction"
}

function Invoke-ReceiptAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [Parameter(Mandatory = $true)][string]$ExpectedAction,
        [hashtable]$Arguments = @{}
    )

    $dispatched = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action `
        -Arguments $Arguments
    $requestId = [string]$dispatched.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action" }
    return Wait-CommandReceipt -RequestId $requestId -ExpectedAction $ExpectedAction
}

function Get-Controls {
    param(
        [string]$Semantic = "",
        [string]$Region = "",
        [string]$ContextId = ""
    )

    $controls = [System.Collections.Generic.List[object]]::new()
    $offset = 0
    do {
        $arguments = @{ offset = $offset; limit = 50 }
        if ($Semantic) { $arguments.semantic = $Semantic }
        if ($Region) { $arguments.region = $Region }
        if ($ContextId) { $arguments.context_id = $ContextId }
        $page = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_find_controls" -Arguments $arguments
        @($page.controls | Where-Object { $null -ne $_ }).ForEach({ $controls.Add($_) })
        $offset = if ($null -eq $page.next_offset) { 0 } else { [int]$page.next_offset }
    } while ($page.has_more -eq $true -and $offset -gt 0)
    return @($controls)
}

function Wait-ContextualOverlay {
    param([Parameter(Mandatory = $true)][string]$ContextId)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $controls = @(Get-Controls -Region "overlay")
        if (
            $controls.Count -gt 0 -and
            @($controls | Where-Object { [string]$_.context_id -ne $ContextId }).Count -eq 0
        ) {
            return $controls
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for message-owned ChatGPT overlay controls."
}

function Wait-OverlayClosed {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds([Math]::Min($ReadyTimeoutSec, 30))
    do {
        if (@(Get-Controls -Region "overlay").Count -eq 0) { return $true }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $false
}

function Dismiss-VisibleOverlays {
    for ($attempt = 0; $attempt -lt 3; $attempt++) {
        if (@(Get-Controls -Region "overlay").Count -eq 0) { return $true }
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "input", "keyevent", "4") -TimeoutSec 5 `
            -Label "dismiss pre-existing ChatGPT overlay" | Out-Null
        if (Wait-OverlayClosed) { return $true }
    }
    throw "Pre-existing ChatGPT overlays could not be dismissed safely."
}

function Wait-ViewMode {
    param([Parameter(Mandatory = $true)][string]$ExpectedMode)

    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 20 `
        -Description "ChatGPT view mode $ExpectedMode" -Predicate {
            param($state)
            [string]$state.view_mode -eq $ExpectedMode -and $state.bridge_state -eq "ready"
        }.GetNewClosure()
}

function Restore-OriginalViewMode {
    if (-not $script:viewModeChanged) { return $true }
    $requestedMode = if ($script:originalViewMode -eq "web") { "official" } else { "native" }
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = $requestedMode } | Out-Null
    Wait-ViewMode -ExpectedMode $script:originalViewMode | Out-Null
    $script:viewModeChanged = $false
    return $true
}

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $ReadyTimeoutSec -InitialWaitSec 20
    Assert-ChatGptWebSmokeAdapterVersion -State $origin `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    if ([int]$origin.conversation.message_count -lt 1) {
        throw "A conversation with at least one rendered message is required."
    }
    $script:originalViewMode = [string]$origin.view_mode
    if ($script:originalViewMode -notin @("native", "web")) {
        throw "ChatGPT returned an unsupported view mode."
    }
    if ($script:originalViewMode -ne "web") {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
            -Arguments @{ view_mode = "official" } | Out-Null
        $script:viewModeChanged = $true
        Wait-ViewMode -ExpectedMode "web" | Out-Null
    }
    Dismiss-VisibleOverlays | Out-Null
    $originUrl = [string]$origin.conversation.url

    Invoke-ReceiptAction -Action "chatgpt_refresh_controls" `
        -ExpectedAction "snapshot_ui_manifest" | Out-Null
    $messageMore = @(Get-Controls -Semantic "more" -Region "message") |
        Where-Object {
            $_.enabled -eq $true -and $_.in_viewport -eq $true -and
            -not [string]::IsNullOrWhiteSpace([string]$_.context_id)
        } |
        Select-Object -Last 1
    if ($null -eq $messageMore) {
        throw "No visible message overflow control is available for safe verification."
    }
    $nativeMessageSelector = [string]$messageMore.native_trigger_content_description
    if (-not $nativeMessageSelector) {
        throw "The message overflow control did not export a native menu selector."
    }

    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = "native" } | Out-Null
    $script:viewModeChanged = $true
    Wait-ViewMode -ExpectedMode "native" | Out-Null
    $reveal = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_reveal_message" `
        -Arguments @{ message_id = [string]$messageMore.context_id; target = "actions" }
    if ($reveal.control_ok -ne $true) {
        throw "The native message action target could not be revealed."
    }

    $remoteDump = "/sdcard/elon-chatgpt-message-actions.xml"
    try {
        $selectorDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        do {
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "uiautomator", "dump", $remoteDump) -TimeoutSec 30 `
                -Label "dump native ChatGPT message menu selectors" | Out-Null
            $uiXml = Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "cat", $remoteDump) -TimeoutSec 30 `
                -Label "read native ChatGPT message menu selectors"
            $nativeMessageSelectorFound = $uiXml.Contains($nativeMessageSelector)
            if (-not $nativeMessageSelectorFound) { Start-Sleep -Milliseconds 500 }
        } while (-not $nativeMessageSelectorFound -and [DateTimeOffset]::UtcNow -lt $selectorDeadline)
    } finally {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "rm", "-f", $remoteDump) -TimeoutSec 5 `
            -Label "remove native ChatGPT message selector dump" | Out-Null
    }
    if (-not $nativeMessageSelectorFound) {
        throw "The native message menu selector was not visible to ADB."
    }

    Invoke-ReceiptAction -Action "chatgpt_invoke_control" `
        -ExpectedAction "invoke_ui_control" `
        -Arguments @{ control_id = [string]$messageMore.control_id } | Out-Null
    Wait-ViewMode -ExpectedMode "web" | Out-Null
    $script:overlayOpened = $true
    $overlayControls = @(Wait-ContextualOverlay -ContextId ([string]$messageMore.context_id))
    $unclassifiedLabels = @(
        $overlayControls |
            Where-Object { $_.semantic -eq "action" } |
            ForEach-Object {
                ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.label -MaxLength 80
            }
    )
    if ($unclassifiedLabels.Count -gt 0) {
        throw "The message action menu contains unclassified controls: $($unclassifiedLabels -join ' | ')"
    }
    $nativeOverlaySelector = [string](@(
        $overlayControls |
            Where-Object {
                $_.native_presentation -eq "menu" -and
                -not [string]::IsNullOrWhiteSpace([string]$_.native_trigger_content_description)
            } |
            Select-Object -ExpandProperty native_trigger_content_description -Unique
    ) | Select-Object -First 1)
    if (-not $nativeOverlaySelector) {
        throw "The message action menu did not export a native trigger selector."
    }

    Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
        -Arguments @("shell", "input", "keyevent", "4") -TimeoutSec 5 `
        -Label "close ChatGPT message action overlay" | Out-Null
    if (-not (Wait-OverlayClosed)) { throw "Message action overlay did not close cleanly." }
    $script:overlayOpened = $false

    $restored = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 20 `
        -Description "message action conversation restoration" -Predicate {
            param($state)
            $state.bridge_state -eq "ready" -and
                $state.adapter_current -eq $true -and
                [string]$state.conversation.url -eq $originUrl
        }
    Restore-OriginalViewMode | Out-Null

    [ordered]@{
        schema = "elon.chatgpt_web.message_action_acceptance.v1"
        passed = $true
        adapter_version = [int]$restored.adapter_version
        overlay_control_count = $overlayControls.Count
        generic_control_count = $unclassifiedLabels.Count
        generic_control_labels = $unclassifiedLabels
        context_bound = $true
        native_message_revealed = $true
        native_message_selector_found = $nativeMessageSelectorFound
        native_overlay_selector_exported = $true
        conversation_restored = $true
        view_mode_restored = $true
        sent_messages = 0
        copied_messages = 0
        started_audio = 0
        cleared_cookies = $false
        cleared_app_data = $false
    } | ConvertTo-Json -Depth 4
    Write-Output "CHATGPT_WEB_MESSAGE_ACTION_ACCEPTANCE=passed"
} finally {
    if ($script:overlayOpened) {
        try {
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "input", "keyevent", "4") -TimeoutSec 5 `
                -Label "recover ChatGPT message action overlay" | Out-Null
        } catch { }
    }
    try { Restore-OriginalViewMode | Out-Null } catch { }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
