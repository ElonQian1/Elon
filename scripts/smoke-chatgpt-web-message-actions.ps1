#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 180)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 54
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$script:overlayOpened = $false

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

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $ReadyTimeoutSec -InitialWaitSec 20
    if ([int]$origin.adapter_version -ne $ExpectedAdapterVersion) {
        throw "Unexpected ChatGPT adapter version."
    }
    if ([int]$origin.conversation.message_count -lt 1) {
        throw "A conversation with at least one rendered message is required."
    }
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

    Invoke-ReceiptAction -Action "chatgpt_invoke_control" `
        -ExpectedAction "invoke_ui_control" `
        -Arguments @{ control_id = [string]$messageMore.control_id } | Out-Null
    $script:overlayOpened = $true
    $overlayControls = @(Wait-ContextualOverlay -ContextId ([string]$messageMore.context_id))
    if (@($overlayControls | Where-Object { $_.semantic -eq "action" }).Count -gt 0) {
        throw "The message action menu still contains unclassified controls."
    }
    $expectedSelector = [string](@(
        $overlayControls |
            Where-Object {
                $_.native_presentation -eq "menu" -and
                -not [string]::IsNullOrWhiteSpace([string]$_.native_trigger_content_description)
            } |
            Select-Object -ExpandProperty native_trigger_content_description -Unique
    ) | Select-Object -First 1)
    if (-not $expectedSelector) {
        throw "The message action menu did not export a native trigger selector."
    }

    Start-Sleep -Seconds 1
    $remoteDump = "/sdcard/elon-chatgpt-message-actions.xml"
    try {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "uiautomator", "dump", $remoteDump) -TimeoutSec 30 `
            -Label "dump ChatGPT message action selectors" | Out-Null
        $uiXml = Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "cat", $remoteDump) -TimeoutSec 30 `
            -Label "read ChatGPT message action selectors"
        $nativeSelectorFound = $uiXml.Contains($expectedSelector)
    } finally {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "rm", "-f", $remoteDump) -TimeoutSec 5 `
            -Label "remove ChatGPT message action selector dump" | Out-Null
    }
    if (-not $nativeSelectorFound) {
        throw "The native message overlay selector was not visible to ADB."
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

    [ordered]@{
        schema = "elon.chatgpt_web.message_action_acceptance.v1"
        passed = $true
        adapter_version = [int]$restored.adapter_version
        overlay_control_count = $overlayControls.Count
        generic_control_count = @($overlayControls | Where-Object { $_.semantic -eq "action" }).Count
        generic_control_labels = @(
            $overlayControls |
                Where-Object { $_.semantic -eq "action" } |
                ForEach-Object { ([string]$_.label).Trim().Substring(0, [Math]::Min(80, ([string]$_.label).Trim().Length)) }
        )
        context_bound = $true
        native_selector_found = $nativeSelectorFound
        conversation_restored = $true
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
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
