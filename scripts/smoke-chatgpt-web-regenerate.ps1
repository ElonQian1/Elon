#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(30, 300)][int]$ReadyTimeoutSec = 120,
    [ValidateRange(30, 600)][int]$ReplyTimeoutSec = 240,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 71
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

function Get-ContentDigest {
    param([AllowEmptyString()][string]$Value)

    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        return [BitConverter]::ToString($sha.ComputeHash($bytes)).Replace("-", "")
    } finally {
        $sha.Dispose()
    }
}

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

function Wait-RegeneratedReply {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$Marker,
        [Parameter(Mandatory = $true)][string]$PreviousMessageId,
        [Parameter(Mandatory = $true)][string]$PreviousContentDigest
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReplyTimeoutSec)
    $streamingObserved = $false
    $lastReceipt = $null
    $lastProgressAt = [DateTimeOffset]::MinValue
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        if ($state.streaming -eq $true) { $streamingObserved = $true }
        $lastReceipt = @($state.command_requests) |
            Where-Object { [string]$_.request_id -eq $RequestId } |
            Select-Object -Last 1
        if ($null -ne $lastReceipt -and [string]$lastReceipt.status -eq "failed") {
            throw "ChatGPT regenerate command failed."
        }
        $assistant = @($state.conversation.messages) |
            Where-Object { [string]$_.role -eq "assistant" } |
            Select-Object -Last 1
        if ($null -ne $assistant) {
            $normalized = Normalize-ChatGptProbeReply ([string]$assistant.content)
            $identityChanged = [string]$assistant.id -ne $PreviousMessageId
            $contentChanged = (Get-ContentDigest -Value $normalized) -ne $PreviousContentDigest
            if (
                $null -ne $lastReceipt -and
                [string]$lastReceipt.expected_web_action -eq "regenerate_response" -and
                [string]$lastReceipt.status -eq "succeeded" -and
                $lastReceipt.result.ok -eq $true -and
                $state.streaming -eq $false -and
                [string]$assistant.state -eq "completed" -and
                $normalized -like "*$Marker*" -and
                ($streamingObserved -or $identityChanged -or $contentChanged)
            ) {
                return [pscustomobject]@{
                    state = $state
                    streaming_observed = $streamingObserved
                    assistant_identity_changed = $identityChanged
                    assistant_content_changed = $contentChanged
                }
            }
        }
        if (([DateTimeOffset]::UtcNow - $lastProgressAt).TotalSeconds -ge 20) {
            Write-Output "CHATGPT_REGENERATE_PROGRESS phase=await_regenerated_reply streaming=$([bool]$state.streaming) messages=$([int]$state.conversation.message_count) receipt=$([string]$lastReceipt.status)"
            $lastProgressAt = [DateTimeOffset]::UtcNow
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for a structurally new regenerated reply. Receipt=$($lastReceipt.status)."
}

$result = $null
$originPath = ""
$originMode = ""
$originRestored = $false
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

    Write-Output "CHATGPT_REGENERATE_PROGRESS phase=create_isolated_conversation"
    Invoke-ReceiptAction -Action "chatgpt_new_conversation" `
        -ExpectedAction "new_conversation" | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "isolated blank regenerate conversation" -Predicate {
            param($state)
            $state.page_kind -eq "home" -and
                (-not $originPath -or [string]$state.conversation.url -notlike "*$originPath*") -and
                $state.composer_ready -eq $true -and
                $state.streaming -eq $false
        }.GetNewClosure() | Out-Null

    Write-Output "CHATGPT_REGENERATE_PROGRESS phase=send_probe"
    $marker = "ELON-CHATGPT-REGENERATE-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
    $prompt = "Reply with a fresh 12-character lowercase hexadecimal token, one space, then exactly: $marker"
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
        -Arguments @{ text = $prompt } | Out-Null
    $beforeSend = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "regenerate probe draft synchronization" -Predicate {
            param($state)
            $state.bridge_state -eq "ready" -and
                $state.adapter_current -eq $true -and
                [string]$state.input.text -eq $prompt
        }.GetNewClosure()
    $send = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"
    $sendRequestId = [string]$send.command_receipt.request_id
    if (-not $sendRequestId) { throw "ChatGPT regenerate probe did not return a send receipt id." }
    $initialReply = Wait-ChatGptProbeReply `
        -InvokeUiState { Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" } `
        -RequestId $sendRequestId -Marker $marker `
        -AfterMs ([long]$beforeSend.last_command.observed_at_ms) `
        -TimeoutSec $ReplyTimeoutSec -PollIntervalSec $PollIntervalSec
    Write-Output "CHATGPT_REGENERATE_PROGRESS phase=initial_reply_complete"
    $initialAssistant = @($initialReply.conversation.messages) |
        Where-Object { [string]$_.role -eq "assistant" } |
        Select-Object -Last 1
    if ($null -eq $initialAssistant -or [string]$initialAssistant.state -ne "completed") {
        throw "Initial ChatGPT regenerate probe did not produce a completed assistant message."
    }
    $initialDigest = Get-ContentDigest -Value (
        Normalize-ChatGptProbeReply ([string]$initialAssistant.content)
    )

    $regenerate = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_regenerate_response"
    $regenerateRequestId = [string]$regenerate.command_receipt.request_id
    if (-not $regenerateRequestId) { throw "ChatGPT regenerate did not return a receipt id." }
    Write-Output "CHATGPT_REGENERATE_PROGRESS phase=regenerate_dispatched"
    $regenerated = Wait-RegeneratedReply -RequestId $regenerateRequestId `
        -Marker $marker -PreviousMessageId ([string]$initialAssistant.id) `
        -PreviousContentDigest $initialDigest

    Write-Output "CHATGPT_REGENERATE_PROGRESS phase=restore_origin"
    Restore-Origin -ConversationPath $originPath -ViewMode $originMode
    $originRestored = $true
    $result = [ordered]@{
        schema = "elon.chatgpt_web.regenerate_acceptance.v1"
        passed = $true
        adapter_version = [int]$regenerated.state.adapter_version
        isolated_conversation = $true
        initial_assistant_completed = $true
        regenerate_receipt_observed = $true
        streaming_observed = [bool]$regenerated.streaming_observed
        assistant_identity_changed = [bool]$regenerated.assistant_identity_changed
        assistant_content_changed = [bool]$regenerated.assistant_content_changed
        regenerated_assistant_completed = $true
        original_conversation_restored = $true
        original_view_mode_restored = $true
        sent_messages = 1
        regenerated_messages = 1
        private_content_emitted = $false
        cleared_cookies = $false
        cleared_app_data = $false
    }
} finally {
    if (-not $originRestored -and $originMode) {
        try {
            Restore-Origin -ConversationPath $originPath -ViewMode $originMode
        } catch {
            Write-Warning "Unable to restore the original ChatGPT view after a failed regenerate smoke."
        }
    }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}

$result | ConvertTo-Json -Depth 4
Write-Output "CHATGPT_WEB_REGENERATE_ACCEPTANCE=passed"
