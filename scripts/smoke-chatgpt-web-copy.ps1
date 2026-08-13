#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(30, 300)][int]$ReadyTimeoutSec = 120,
    [ValidateRange(30, 600)][int]$ReplyTimeoutSec = 240,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 86
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
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
            -Arguments @{ view_mode = $ViewMode } | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -Description "original ChatGPT view mode restoration" -Predicate {
                param($state)
                [string]$state.view_mode -eq $ViewMode
            }.GetNewClosure() | Out-Null
    }
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

    Invoke-ReceiptAction -Action "chatgpt_new_conversation" `
        -ExpectedAction "new_conversation" | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "isolated blank copy conversation" -Predicate {
            param($state)
            [int]$state.conversation.message_count -eq 0 -and
                $state.composer_ready -eq $true -and
                $state.streaming -eq $false
        } | Out-Null

    $marker = "ELON-CHATGPT-COPY-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
        -Arguments @{ text = "Reply only with this exact test marker: $marker" } | Out-Null
    $beforeSend = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
    $send = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"
    $sendRequestId = [string]$send.command_receipt.request_id
    if (-not $sendRequestId) { throw "ChatGPT copy prompt did not return a receipt id." }
    $reply = Wait-ChatGptProbeReply `
        -InvokeUiState { Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" } `
        -RequestId $sendRequestId -Marker $marker `
        -AfterMs ([long]$beforeSend.last_command.observed_at_ms) `
        -TimeoutSec $ReplyTimeoutSec -PollIntervalSec $PollIntervalSec

    $copy = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_copy_last_response"
    if ($copy.control_ok -ne $true) { throw "ChatGPT native copy action failed." }
    if ([string]$copy.receipt.schema -ne "elon.chatgpt_web.clipboard_receipt.v1") {
        throw "Unexpected ChatGPT clipboard receipt schema."
    }
    if ($copy.receipt.copied -ne $true -or [int]$copy.receipt.item_count -lt 1) {
        throw "ChatGPT clipboard receipt did not confirm a copied item."
    }
    if (@($copy.receipt.mime_types) -notcontains "text/plain") {
        throw "ChatGPT clipboard receipt did not report text/plain."
    }
    if ($copy.receipt.content_exported -ne $false) {
        throw "ChatGPT clipboard receipt exported message content."
    }
    if (($copy | ConvertTo-Json -Depth 8 -Compress) -like "*$marker*") {
        throw "ChatGPT clipboard receipt leaked the synthetic reply."
    }

    Restore-Origin -ConversationPath $originPath -ViewMode $originMode
    $originRestored = $true
    $result = [ordered]@{
        schema = "elon.chatgpt_web.copy_acceptance.v1"
        passed = $true
        adapter_version = [int]$reply.adapter_version
        isolated_conversation = $true
        assistant_completed = $true
        clipboard_receipt_observed = $true
        clipboard_item_count = [int]$copy.receipt.item_count
        clipboard_mime_types = @($copy.receipt.mime_types)
        clipboard_content_read_back = $false
        original_conversation_restored = $true
        original_view_mode_restored = $true
        private_content_emitted = $false
        cleared_cookies = $false
        cleared_app_data = $false
    }
    Register-ChatGptWebVerificationCases -Runtime $runtime `
        -CaseIds @("reversible/copy_receipt_without_content_readback") `
        -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
} finally {
    if (-not $originRestored -and $originMode) {
        try {
            Restore-Origin -ConversationPath $originPath -ViewMode $originMode
        } catch {
            Write-Warning "Unable to restore the original ChatGPT view after a failed copy smoke."
        }
    }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}

$result | ConvertTo-Json -Depth 4
Write-Output "CHATGPT_WEB_COPY_ACCEPTANCE=passed"
