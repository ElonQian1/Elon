#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(30, 300)][int]$ReadyTimeoutSec = 120,
    [ValidateRange(30, 600)][int]$ReplyTimeoutSec = 240,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 81
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

function Invoke-ReceiptAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [Parameter(Mandatory = $true)][string]$ExpectedAction,
        [hashtable]$Arguments = @{}
    )

    $dispatch = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
        -Action $Action -Arguments $Arguments -TimeoutSec $ReadyTimeoutSec
    $requestId = [string]$dispatch.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action." }
    return Wait-ChatGptCommandReceipt `
        -InvokeUiState { Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" } `
        -RequestId $requestId -ExpectedAction $ExpectedAction `
        -TimeoutSec $ReadyTimeoutSec -PollIntervalSec $PollIntervalSec
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

function Convert-SafeControl {
    param(
        [Parameter(Mandatory = $true)]$Control,
        [AllowEmptyString()][string]$UserMessageId = "",
        [AllowEmptyString()][string]$AssistantMessageId = ""
    )

    $contextId = [string]$Control.context_id
    $contextRole = if ($contextId -and $contextId -eq $UserMessageId) {
        "user"
    } elseif ($contextId -and $contextId -eq $AssistantMessageId) {
        "assistant"
    } else {
        "unknown"
    }
    return [ordered]@{
        role = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $Control.role -MaxLength 40
        semantic = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $Control.semantic -MaxLength 40
        label = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $Control.label -MaxLength 80
        context_role = $contextRole
        enabled = [bool]$Control.enabled
        in_viewport = [bool]$Control.in_viewport
    }
}

function Wait-OverlayControls {
    param([AllowEmptyString()][string]$ContextId = "")

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $controls = @(Get-Controls -Region "overlay")
        if ($controls.Count -gt 0) {
            $foreign = @($controls | Where-Object {
                -not [string]::IsNullOrWhiteSpace([string]$_.context_id) -and
                    [string]$_.context_id -ne $ContextId
            })
            if ($foreign.Count -eq 0) { return $controls }
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for the synthetic assistant message menu."
}

function Wait-OverlayClosed {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(20)
    do {
        if (@(Get-Controls -Region "overlay").Count -eq 0) { return $true }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $false
}

function Dismiss-Overlays {
    param([Parameter(Mandatory = $true)][string]$Label)

    for ($attempt = 0; $attempt -lt 3; $attempt++) {
        if (Wait-OverlayClosed) { return $true }
        Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
            -Arguments @("shell", "input", "keyevent", "4") -TimeoutSec 5 `
            -Label $Label | Out-Null
    }
    return Wait-OverlayClosed
}

function Restore-Origin {
    param(
        [AllowEmptyString()][string]$ConversationPath,
        [Parameter(Mandatory = $true)][string]$ViewMode
    )

    if ($ConversationPath) {
        Invoke-ReceiptAction -Action "chatgpt_open_conversation" `
            -ExpectedAction "open_conversation" `
            -Arguments @{ conversation_path = $ConversationPath } | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -Description "original ChatGPT conversation restoration" -Predicate {
                param($state)
                [string]$state.conversation.url -like "*$ConversationPath*" -and
                    $state.bridge_state -eq "ready"
            }.GetNewClosure() | Out-Null
    } else {
        Invoke-ReceiptAction -Action "chatgpt_new_conversation" `
            -ExpectedAction "new_conversation" | Out-Null
    }
    if ($ViewMode -in @("web", "native")) {
        $requestedMode = if ($ViewMode -eq "web") { "official" } else { "native" }
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
            -Arguments @{ view_mode = $requestedMode } | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -Description "original ChatGPT view mode restoration" -Predicate {
                param($state)
                [string]$state.view_mode -eq $ViewMode
            }.GetNewClosure() | Out-Null
    }
}

$originPath = ""
$originMode = ""
$originRestored = $false
$overlayOpened = $false
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $ReadyTimeoutSec -InitialWaitSec 20
    Assert-ChatGptWebSmokeAdapterVersion -State $origin `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    $originMode = [string]$origin.view_mode
    $originPath = [regex]::Match(
        [string]$origin.conversation.url,
        '/c/[A-Za-z0-9_-]{1,160}'
    ).Value

    Write-Output "CHATGPT_REGENERATE_MENU_PHASE phase=create_isolated_conversation"
    Invoke-ReceiptAction -Action "chatgpt_new_conversation" `
        -ExpectedAction "new_conversation" | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "isolated regenerate-menu conversation" -Predicate {
            param($state)
            $state.page_kind -eq "home" -and
                (-not $originPath -or [string]$state.conversation.url -notlike "*$originPath*") -and
                $state.composer_ready -eq $true -and
                $state.streaming -eq $false
        }.GetNewClosure() | Out-Null

    Write-Output "CHATGPT_REGENERATE_MENU_PHASE phase=send_synthetic_probe"
    $marker = "ELON-CHATGPT-MENU-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
    $prompt = "Reply with exactly: $marker"
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
        -Arguments @{ text = $prompt } | Out-Null
    $beforeSend = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "regenerate-menu probe draft synchronization" -Predicate {
            param($state)
            [string]$state.input.text -eq $prompt -and $state.bridge_state -eq "ready"
        }.GetNewClosure()
    $send = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"
    $sendRequestId = [string]$send.command_receipt.request_id
    if (-not $sendRequestId) { throw "Synthetic menu probe did not return a send receipt id." }
    $reply = Wait-ChatGptProbeReply `
        -InvokeUiState { Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" } `
        -RequestId $sendRequestId -Marker $marker `
        -AfterMs ([long]$beforeSend.last_command.observed_at_ms) `
        -TimeoutSec $ReplyTimeoutSec -PollIntervalSec $PollIntervalSec
    $assistant = @($reply.conversation.messages) |
        Where-Object { [string]$_.role -eq "assistant" } |
        Select-Object -Last 1
    $user = @($reply.conversation.messages) |
        Where-Object { [string]$_.role -eq "user" } |
        Select-Object -Last 1
    if ($null -eq $assistant) { throw "Synthetic menu probe returned no assistant message." }
    if ($null -eq $user) { throw "Synthetic menu probe returned no user message." }

    Write-Output "CHATGPT_REGENERATE_MENU_PHASE phase=open_message_menu"
    Invoke-ReceiptAction -Action "chatgpt_refresh_controls" `
        -ExpectedAction "snapshot_ui_manifest" | Out-Null
    $messageControls = @(Get-Controls -Region "message")
    $safeMessageControls = @($messageControls | ForEach-Object {
        Convert-SafeControl -Control $_ `
            -UserMessageId ([string]$user.id) `
            -AssistantMessageId ([string]$assistant.id)
    })
    $messageMore = @(Get-Controls -Semantic "more" -Region "message") |
        Where-Object {
            $_.enabled -eq $true -and $_.in_viewport -eq $true -and
                [string]$_.context_id -eq [string]$assistant.id
        } |
        Select-Object -Last 1
    if ($null -eq $messageMore) {
        $messageMore = @(Get-Controls -Semantic "more" -Region "message") |
            Where-Object { $_.enabled -eq $true -and $_.in_viewport -eq $true } |
            Select-Object -Last 1
    }
    if ($null -eq $messageMore) {
        throw "Synthetic assistant message exported no visible overflow control."
    }
    $messageModel = @($messageControls) |
        Where-Object {
            [string]$_.semantic -eq "model" -and
                $_.enabled -eq $true -and $_.in_viewport -eq $true -and
                [string]$_.context_id -eq [string]$assistant.id
        } |
        Select-Object -Last 1
    Invoke-ReceiptAction -Action "chatgpt_invoke_control" `
        -ExpectedAction "invoke_ui_control" `
        -Arguments @{ control_id = [string]$messageMore.control_id } | Out-Null
    $overlayOpened = $true
    $overlayControls = @(Wait-OverlayControls -ContextId ([string]$messageMore.context_id))
    $safeControls = @($overlayControls | ForEach-Object {
        $safe = Convert-SafeControl -Control $_ `
            -UserMessageId ([string]$user.id) `
            -AssistantMessageId ([string]$assistant.id)
        $safe.context_bound = [string]$_.context_id -eq [string]$messageMore.context_id
        $safe
    })

    if (-not (Dismiss-Overlays -Label "close synthetic ChatGPT message menu")) {
        throw "Synthetic message menu did not close cleanly."
    }
    $overlayOpened = $false

    $safeModelControls = @()
    if ($null -ne $messageModel) {
        Write-Output "CHATGPT_REGENERATE_MENU_PHASE phase=open_model_menu"
        Invoke-ReceiptAction -Action "chatgpt_refresh_controls" `
            -ExpectedAction "snapshot_ui_manifest" | Out-Null
        $messageModel = @(Get-Controls -Semantic "model" -Region "message") |
            Where-Object {
                $_.enabled -eq $true -and $_.in_viewport -eq $true -and
                    [string]$_.context_id -eq [string]$assistant.id
            } |
            Select-Object -Last 1
        if ($null -ne $messageModel) {
            Invoke-ReceiptAction -Action "chatgpt_invoke_control" `
                -ExpectedAction "invoke_ui_control" `
                -Arguments @{ control_id = [string]$messageModel.control_id } | Out-Null
            $overlayOpened = $true
            $modelControls = @(Wait-OverlayControls)
            $safeModelControls = @($modelControls | ForEach-Object {
                Convert-SafeControl -Control $_ `
                    -UserMessageId ([string]$user.id) `
                    -AssistantMessageId ([string]$assistant.id)
            })
            if (-not (Dismiss-Overlays -Label "close synthetic ChatGPT message model menu")) {
                throw "Synthetic message model menu did not close cleanly."
            }
            $overlayOpened = $false
        }
    }

    Write-Output "CHATGPT_REGENERATE_MENU_PHASE phase=restore_origin"
    Restore-Origin -ConversationPath $originPath -ViewMode $originMode
    $originRestored = $true
    [ordered]@{
        schema = "elon.chatgpt_web.regenerate_menu_diagnostic.v1"
        passed = $true
        adapter_version = [int]$reply.adapter_version
        isolated_conversation = $true
        assistant_completed = $true
        overflow_control_found = $true
        message_control_count = $safeMessageControls.Count
        message_controls = $safeMessageControls
        overlay_control_count = $safeControls.Count
        overlay_controls = $safeControls
        model_control_found = $null -ne $messageModel
        model_overlay_control_count = $safeModelControls.Count
        model_overlay_controls = $safeModelControls
        original_conversation_restored = $true
        original_view_mode_restored = $true
        sent_messages = 1
        private_content_emitted = $false
        cleared_cookies = $false
        cleared_app_data = $false
    } | ConvertTo-Json -Depth 5
    Write-Output "CHATGPT_WEB_REGENERATE_MENU_DIAGNOSTIC=passed"
} finally {
    if ($overlayOpened) {
        try {
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "input", "keyevent", "4") -TimeoutSec 5 `
                -Label "recover synthetic ChatGPT message menu" | Out-Null
        } catch { }
    }
    if (-not $originRestored -and $originMode) {
        try { Restore-Origin -ConversationPath $originPath -ViewMode $originMode } catch { }
    }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
