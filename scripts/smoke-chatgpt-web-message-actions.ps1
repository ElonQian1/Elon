#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 180)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-supervised-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-conversation-sample.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$script:overlayOpened = $false
$script:nativeActionDialogOpened = $false
$script:originalViewMode = ""
$script:viewModeChanged = $false
$script:originConversationPath = ""
$script:conversationSampleOpened = $false

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

    $dispatched = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime -Action $Action `
        -Arguments $Arguments -TimeoutSec $ReadyTimeoutSec
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
        $page = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
            -Action "chatgpt_find_controls" -Arguments $arguments `
            -TimeoutSec $ReadyTimeoutSec
        @($page.controls | Where-Object { $null -ne $_ }).ForEach({ $controls.Add($_) })
        $offset = if ($null -eq $page.next_offset) { 0 } else { [int]$page.next_offset }
    } while ($page.has_more -eq $true -and $offset -gt 0)
    return @($controls)
}

function Get-BlockingOverlayControls {
    return @(Get-Controls -Region "overlay") | Where-Object {
        -not [string]::IsNullOrWhiteSpace([string]$_.context_id) -or
        [string]$_.role -in @(
            "dialog", "menuitem", "menuitemcheckbox", "menuitemradio", "option", "slider"
        )
    }
}

function Wait-ContextualOverlay {
    param([Parameter(Mandatory = $true)][string]$ContextId)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        Invoke-ReceiptAction -Action "chatgpt_refresh_controls" `
            -ExpectedAction "snapshot_ui_manifest" | Out-Null
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
        Invoke-ReceiptAction -Action "chatgpt_refresh_controls" `
            -ExpectedAction "snapshot_ui_manifest" | Out-Null
        if (@(Get-BlockingOverlayControls).Count -eq 0) { return $true }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $false
}

function Dismiss-VisibleOverlays {
    for ($attempt = 0; $attempt -lt 3; $attempt++) {
        Invoke-ReceiptAction -Action "chatgpt_refresh_controls" `
            -ExpectedAction "snapshot_ui_manifest" | Out-Null
        if (@(Get-BlockingOverlayControls).Count -eq 0) { return $true }
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

function Restore-MessageActionOrigin {
    if ($script:conversationSampleOpened) {
        Restore-ChatGptWebSmokeOrigin -Runtime $runtime `
            -ConversationPath $script:originConversationPath `
            -ViewMode $script:originalViewMode -TimeoutSec $ReadyTimeoutSec | Out-Null
        $script:conversationSampleOpened = $false
        $script:viewModeChanged = $false
        return $true
    }
    return Restore-OriginalViewMode
}

function Get-UiAutomatorNodes {
    param(
        [Parameter(Mandatory = $true)][string]$UiXml,
        [string]$ContentDescription = ""
    )

    $document = [xml]$UiXml
    $nodes = @($document.SelectNodes("//node"))
    if (-not $ContentDescription) { return $nodes }
    return @($nodes | Where-Object {
        [string]$_.GetAttribute("content-desc") -eq $ContentDescription
    })
}

function Invoke-NativeSelector {
    param([Parameter(Mandatory = $true)][string]$Selector)

    $remotePath = "/sdcard/elon-chatgpt-native-selector.xml"
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
    try {
        do {
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "uiautomator", "dump", $remotePath) -TimeoutSec 30 `
                -Label "dump native ChatGPT selector" | Out-Null
            $xml = Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "cat", $remotePath) -TimeoutSec 30 `
                -Label "read native ChatGPT selector"
            $node = @(Get-UiAutomatorNodes -UiXml $xml -ContentDescription $Selector) |
                Select-Object -First 1
            if ($null -ne $node) {
                $match = [regex]::Match([string]$node.GetAttribute("bounds"),
                    '^\[(\d+),(\d+)\]\[(\d+),(\d+)\]$')
                if (-not $match.Success) { throw "Native selector returned invalid bounds." }
                $x = [int](
                    ([int]$match.Groups[1].Value + [int]$match.Groups[3].Value) / 2
                )
                $y = [int](
                    ([int]$match.Groups[2].Value + [int]$match.Groups[4].Value) / 2
                )
                Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                    -Arguments @("shell", "input", "tap", "$x", "$y") -TimeoutSec 5 `
                    -Label "open native ChatGPT message actions" | Out-Null
                Start-Sleep -Milliseconds 400
                return $true
            }
            Start-Sleep -Milliseconds 400
        } while ([DateTimeOffset]::UtcNow -lt $deadline)
    } finally {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "rm", "-f", $remotePath) -TimeoutSec 5 `
            -Label "remove native ChatGPT selector dump" | Out-Null
    }
    throw "The requested native ChatGPT selector was not visible to ADB."
}

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $ReadyTimeoutSec -InitialWaitSec 20
    Assert-ChatGptWebSmokeAdapterVersion -State $origin `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    $script:originConversationPath = Get-ChatGptWebSmokeConversationPath `
        -Url ([string]$origin.conversation.url)
    $script:originalViewMode = [string]$origin.view_mode
    $workingState = $origin
    if ([int]$origin.conversation.message_count -lt 1) {
        $script:conversationSampleOpened = $true
        $sample = Open-ChatGptWebSmokeConversationSample -Runtime $runtime `
            -TimeoutSec $ReadyTimeoutSec -MinimumMessageCount 1
        $workingState = $sample.state
    }
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
    $workingUrl = [string]$workingState.conversation.url

    Invoke-ReceiptAction -Action "chatgpt_refresh_controls" `
        -ExpectedAction "snapshot_ui_manifest" | Out-Null
    $messageMore = @(Get-Controls -Semantic "more" -Region "message") |
        Where-Object {
            $_.enabled -eq $true -and $_.in_viewport -eq $true -and
            -not [string]::IsNullOrWhiteSpace([string]$_.context_id)
        } |
        Select-Object -Last 1
    if ($null -eq $messageMore) {
        $messageMore = @(Get-Controls -Semantic "more" -Region "message") |
            Where-Object {
                $_.enabled -eq $true -and
                -not [string]::IsNullOrWhiteSpace([string]$_.context_id)
            } |
            Select-Object -Last 1
    }
    if ($null -eq $messageMore) {
        throw "No message overflow control is available for safe verification."
    }
    $messageContextId = [string]$messageMore.context_id
    $messageActions = @(Get-Controls -Region "message" -ContextId $messageContextId) |
        Where-Object {
            $_.enabled -eq $true -and
            [string]$_.native_presentation -eq "menu" -and
            -not [string]::IsNullOrWhiteSpace([string]$_.native_adb_content_description)
        }
    $nativeMessageSelector = [string]$messageMore.native_trigger_content_description
    $nativeActionSelector = [string](@(
        $messageActions |
            Where-Object { [string]$_.semantic -ne "more" } |
            Select-Object -ExpandProperty native_adb_content_description -Unique
    ) | Select-Object -First 1)
    if (-not $nativeActionSelector) {
        $nativeActionSelector = [string](@(
            $messageActions |
                Select-Object -ExpandProperty native_adb_content_description -Unique
        ) | Select-Object -First 1)
    }
    if (-not $nativeMessageSelector -or -not $nativeActionSelector) {
        throw "The current message actions did not export stable native selectors."
    }
    $saveToProject = @($messageActions | Where-Object semantic -eq "save_to_project") |
        Select-Object -First 1
    $nativeSaveSelector = if ($null -ne $saveToProject) {
        [string]$saveToProject.native_adb_content_description
    } else { "" }
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = "native" } | Out-Null
    $script:viewModeChanged = $true
    Wait-ViewMode -ExpectedMode "native" | Out-Null
    $reveal = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_reveal_message" `
        -Arguments @{ message_id = $messageContextId; target = "actions" }
    if ($reveal.control_ok -ne $true) {
        throw "The native message action target could not be revealed."
    }
    $nativeMessageSelectorFound =
        Invoke-NativeSelector -Selector $nativeMessageSelector
    $script:nativeActionDialogOpened = $true

    $remoteDump = "/sdcard/elon-chatgpt-message-actions.xml"
    $nativeDialogItemCount = 0
    $nativeDialogActionDescriptionCount = 0
    try {
        $selectorDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        do {
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "uiautomator", "dump", $remoteDump) -TimeoutSec 30 `
                -Label "dump native ChatGPT message menu selectors" | Out-Null
            $uiXml = Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "cat", $remoteDump) -TimeoutSec 30 `
                -Label "read native ChatGPT message menu selectors"
            $uiNodes = @(Get-UiAutomatorNodes -UiXml $uiXml)
            $nativeActionSelectorFound = @($uiNodes | Where-Object {
                [string]$_.GetAttribute("content-desc") -eq $nativeActionSelector
            }).Count -gt 0
            $nativeSaveSelectorFound = $nativeSaveSelector -and @($uiNodes | Where-Object {
                [string]$_.GetAttribute("content-desc") -eq $nativeSaveSelector
            }).Count -gt 0
            $nativeDialogItems = @($uiNodes | Where-Object {
                [string]$_.GetAttribute("resource-id") -eq "android:id/text1"
            })
            $nativeDialogItemCount = $nativeDialogItems.Count
            $nativeDialogActionDescriptionCount = @($nativeDialogItems | Where-Object {
                -not [string]::IsNullOrWhiteSpace([string]$_.GetAttribute("content-desc"))
            }).Count
            if (-not $nativeActionSelectorFound) {
                Start-Sleep -Milliseconds 500
            }
        } while (
            -not $nativeActionSelectorFound -and
            [DateTimeOffset]::UtcNow -lt $selectorDeadline
        )
    } finally {
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "rm", "-f", $remoteDump) -TimeoutSec 5 `
            -Label "remove native ChatGPT message selector dump" | Out-Null
    }
    if (-not $nativeActionSelectorFound) {
        throw "No current native message action selector was visible to ADB " +
            "(dialog_items=$nativeDialogItemCount, described_items=$nativeDialogActionDescriptionCount)."
    }
    Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
        -Arguments @("shell", "input", "keyevent", "4") -TimeoutSec 5 `
        -Label "close native ChatGPT message action dialog" | Out-Null
    $script:nativeActionDialogOpened = $false

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
                [string]$state.conversation.url -eq $workingUrl
        }
    Restore-MessageActionOrigin | Out-Null

    Register-ChatGptWebVerificationCases -Runtime $runtime `
        -CaseIds @("safe/message_actions") `
        -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null

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
        native_action_selector_found = $nativeActionSelectorFound
        available_message_action_semantics = @(
            $messageActions.semantic | Sort-Object -Unique
        )
        save_to_project_discovered = $null -ne $saveToProject
        save_to_project_context_bound = $null -ne $saveToProject -and
            [string]$saveToProject.context_id -eq $messageContextId
        save_to_project_native_selector_found = $nativeSaveSelectorFound
        save_to_project_invoked = 0
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
    if ($script:nativeActionDialogOpened) {
        try {
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "input", "keyevent", "4") -TimeoutSec 5 `
                -Label "recover native ChatGPT message action dialog" | Out-Null
        } catch { }
    }
    try { Restore-MessageActionOrigin | Out-Null } catch { }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
