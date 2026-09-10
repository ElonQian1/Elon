#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(30, 300)][int]$ReadyTimeoutSec = 120,
    [ValidateRange(30, 600)][int]$ReplyTimeoutSec = 240,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [switch]$NativeRetry,
    [switch]$RequireOfficialRuntime,
    [switch]$UseCurrentNativeSurface,
    [switch]$UseExistingProbe,
    [switch]$CaptureProtocol
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-reply-state.ps1")

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

    Assert-ChatGptRegenerateForeground -Runtime $runtime
    $dispatch = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action $Action -Arguments $Arguments
    $requestId = [string]$dispatch.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action." }
    return Wait-ChatGptCommandReceipt `
        -InvokeUiState {
            Assert-ChatGptRegenerateForeground -Runtime $runtime
            Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        } `
        -RequestId $requestId -ExpectedAction $ExpectedAction `
        -TimeoutSec $ReadyTimeoutSec -PollIntervalSec $PollIntervalSec
}

function Restore-Origin {
    param([AllowEmptyString()][string]$ConversationPath)

    if ($ConversationPath) {
        Invoke-ReceiptAction -Action "chatgpt_open_conversation" `
            -ExpectedAction "open_conversation" `
            -Arguments @{ conversation_path = $ConversationPath } | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -RequireChatGptForeground `
            -Description "original ChatGPT conversation restoration" -Predicate {
                param($state)
                [string]$state.conversation.url -like "*$ConversationPath*" -and
                    $state.bridge_state -eq "ready"
            }.GetNewClosure() | Out-Null
    } else {
        Invoke-ReceiptAction -Action "chatgpt_new_conversation" `
            -ExpectedAction "new_conversation" | Out-Null
    }
}

function Wait-RegeneratedReply {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$Marker,
        [Parameter(Mandatory = $true)][string]$PreviousMessageId,
        [Parameter(Mandatory = $true)][string]$PreviousContentDigest,
        [Parameter(Mandatory = $true)][string]$ExpectedConversationUrl,
        [Parameter(Mandatory = $true)][string[]]$OriginalUserIds
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReplyTimeoutSec)
    $streamingObserved = $false
    $lastReceipt = $null
    $lastProgressAt = [DateTimeOffset]::MinValue
    do {
        Assert-ChatGptRegenerateForeground -Runtime $runtime
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        if ([string]$state.conversation.url -ne $ExpectedConversationUrl) { throw 'Retry conversation changed.' }
        $currentUserIds = @($state.conversation.messages | Where-Object { $_.role -eq 'user' } | ForEach-Object { [string]$_.id })
        if (($currentUserIds -join '|') -ne ($OriginalUserIds -join '|')) { throw 'Retry changed the original user turn.' }
        if ($state.streaming -eq $true) { $streamingObserved = $true }
        $lastReceipt = @($state.command_requests) |
            Where-Object { [string]$_.request_id -eq $RequestId } |
            Select-Object -Last 1
        if ($null -ne $lastReceipt -and [string]$lastReceipt.status -eq "failed") {
            $code = [string]$lastReceipt.result.detail
            if ($code -cnotmatch '^official_runtime_v1:regenerate_[a-z_:]{1,100}$') { $code = 'unconfirmed' }
            throw "ChatGPT regenerate command failed: $code"
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
                $officialRuntime = [string]$lastReceipt.result.detail -eq 'official_runtime_v1:regenerate_observed'
                if (!(Test-ChatGptRegeneratedReplyIdentity -Receipt $lastReceipt -IdentityChanged $identityChanged `
                    -ContentChanged $contentChanged -RequireOfficialRuntime ([bool]$RequireOfficialRuntime))) {
                    throw 'Retry did not confirm a new variant in the requested transport and native UI.'
                }
                return [pscustomobject]@{
                    state = $state
                    streaming_observed = $streamingObserved
                    assistant_identity_changed = $identityChanged
                    assistant_content_changed = $contentChanged
                    official_runtime_confirmed = $officialRuntime
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
$originRestored = $false
$originCaptured = $false
$protocolCaptureStarted = $false
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    if ($UseCurrentNativeSurface) {
        Assert-ChatGptRegenerateForeground -Runtime $runtime
        $current = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state -MainState
        if ($current.active_surface -ne 'social_ai' -or $current.social_chat.web_chat_provider_id -ne 'chatgpt_web') {
            throw 'Current native ChatGPT surface is unavailable.'
        }
    } else {
        Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    }
    $origin = Wait-ChatGptWebSmokeState -Runtime $runtime `
        -TimeoutSec $ReadyTimeoutSec -RequireChatGptForeground `
        -Description 'authenticated native regenerate surface' -Predicate {
            param($state)
            $state.surface -eq 'chatgpt_web' -and $state.bridge_state -eq 'ready' -and
                $state.adapter_current -eq $true -and $state.authenticated -eq $true
        }
    Assert-ChatGptWebSmokeAdapterVersion -State $origin `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    if ($origin.streaming -or $origin.input.text_length -ne 0 -or @($origin.conversation.attachments).Count) {
        throw 'Origin has an active reply, draft or attachment.'
    }
    $originPath = [regex]::Match(
        [string]$origin.conversation.url,
        '(?:/g/g-p-[A-Za-z0-9_-]{1,160})?/c/[A-Za-z0-9_-]{1,160}'
    ).Value
    $originCaptured = $true

    if ($UseExistingProbe) {
        if (!$originPath) { throw 'Existing probe has no server conversation.' }
        $probe = Get-ChatGptExistingRegenerateProbe -State $origin
        $prompt = $probe.prompt
        $marker = $probe.marker
        Write-Output 'CHATGPT_REGENERATE_PROGRESS phase=reuse_existing_probe'
    } else {
        Write-Output "CHATGPT_REGENERATE_PROGRESS phase=create_isolated_conversation"
        Invoke-ReceiptAction -Action "chatgpt_new_conversation" `
            -ExpectedAction "new_conversation" | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -RequireChatGptForeground `
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
        Assert-ChatGptRegenerateForeground -Runtime $runtime
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
            -Arguments @{ text = $prompt } | Out-Null
        $beforeSend = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
            -RequireChatGptForeground `
            -Description "regenerate probe draft synchronization" -Predicate {
                param($state)
                $state.bridge_state -eq "ready" -and
                    $state.adapter_current -eq $true -and
                    [string]$state.input.text -eq $prompt
            }.GetNewClosure()
        $beforeMain = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state -MainState
        $previousSendAt = [long]$beforeMain.social_chat.web_chat_last_send_command.observed_at_ms
        Assert-ChatGptRegenerateForeground -Runtime $runtime
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input" | Out-Null
        $sent = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec -MainState `
            -RequireChatGptForeground `
            -Description 'production regenerate-probe send receipt' -Predicate {
                param($s) $s.social_chat.web_chat_last_send_command.action -eq 'send_prompt' -and
                    [long]$s.social_chat.web_chat_last_send_command.observed_at_ms -gt $previousSendAt
            }.GetNewClosure()
        $sendReceipt = $sent.social_chat.web_chat_last_send_command
        if ($sendReceipt.ok -ne $true -or ($RequireOfficialRuntime -and $sendReceipt.detail -ne 'official_runtime_v1:accepted')) {
            throw 'Production regenerate-probe send was not confirmed.'
        }
    }
    $probeObservation = @{ last = $null }
    $readReplyState = ${function:Get-ChatGptRegenerateReplyState}
    try {
        $initialReply = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReplyTimeoutSec `
            -RequireChatGptForeground `
            -Description 'initial production regenerate-probe reply' -Predicate {
                param($s)
                $probeObservation.last = & $readReplyState -State $s -Prompt $prompt -Marker $marker
                $probeObservation.last.ready
            }.GetNewClosure()
    } catch {
        Write-Output ('CHATGPT_REGENERATE_INITIAL_REPLY_STATE=' + ($probeObservation.last | ConvertTo-Json -Compress))
        throw
    }
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

    if ($CaptureProtocol) {
        Invoke-ReceiptAction -Action chatgpt_private_protocol_probe -ExpectedAction private_protocol_probe `
            -Arguments @{mode='start'} | Out-Null
        $protocolCaptureStarted = $true
    }
    if ($NativeRetry) {
        Assert-ChatGptRegenerateForeground -Runtime $runtime
        $priorRetryIds = @($initialReply.command_requests | ForEach-Object { [string]$_.request_id })
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action chatgpt_reveal_message `
            -Arguments @{message_id=[string]$initialAssistant.id;target='regenerate'} | Out-Null
        $stableId = ([string]$initialAssistant.id -replace '[^A-Za-z0-9_.:-]', '_')
        $stableId = $stableId.Substring(0, [Math]::Min(160, $stableId.Length))
        & (Join-Path $PSScriptRoot 'invoke-conversation-ui-acceptance.ps1') `
            -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial `
            -Step regenerate -Selector "web-chat-message-action:chatgpt_web:${stableId}:regenerate" | Out-Null
        $dispatched = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 20 -RequireChatGptForeground -Description 'native retry dispatch' -Predicate {
            param($s) @($s.command_requests | Where-Object {
                $_.expected_web_action -eq 'regenerate_response' -and $_.request_id -notin $priorRetryIds
            }).Count -gt 0
        }.GetNewClosure()
        $regenerateRequestId = [string](@($dispatched.command_requests | Where-Object {
            $_.expected_web_action -eq 'regenerate_response' -and $_.request_id -notin $priorRetryIds
        }) | Select-Object -Last 1).request_id
    } else {
        Assert-ChatGptRegenerateForeground -Runtime $runtime
        $regenerate = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_regenerate_response"
        $regenerateRequestId = [string]$regenerate.command_receipt.request_id
    }
    if (-not $regenerateRequestId) { throw "ChatGPT regenerate did not return a receipt id." }
    Write-Output "CHATGPT_REGENERATE_PROGRESS phase=regenerate_dispatched"
    $regenerated = Wait-RegeneratedReply -RequestId $regenerateRequestId `
        -Marker $marker -PreviousMessageId ([string]$initialAssistant.id) `
        -PreviousContentDigest $initialDigest -ExpectedConversationUrl ([string]$initialReply.conversation.url) `
        -OriginalUserIds @($initialReply.conversation.messages | Where-Object { $_.role -eq 'user' } | ForEach-Object { [string]$_.id })

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
        production_surface_preserved = Test-ChatGptWebSmokeActivityForeground -Runtime $runtime
        sent_messages = if ($UseExistingProbe) { 0 } else { 1 }
        reused_initial_reply = [bool]$UseExistingProbe
        regenerated_messages = 1
        native_retry_button = [bool]$NativeRetry
        official_runtime_confirmed = [bool]$regenerated.official_runtime_confirmed
        original_user_turn_preserved = $true
        private_content_emitted = $false
        cleared_cookies = $false
        cleared_app_data = $false
    }
} finally {
    if ($protocolCaptureStarted) {
        try {
            $capture = Invoke-ReceiptAction -Action chatgpt_private_protocol_probe -ExpectedAction private_protocol_probe `
                -Arguments @{mode='read'}
            $evidence = $capture.receipt.result.detail | ConvertFrom-Json
            if ($evidence.schema -ne 'elon.private_protocol_probe.v1') { throw 'Unexpected protocol evidence.' }
            Write-Output ('CHATGPT_REGENERATE_PROTOCOL=' + ([ordered]@{
                schema = $evidence.schema; active = [bool]$evidence.active; dropped = [int]$evidence.dropped
                page_generation = [int]$capture.state.page_generation
                records = @($evidence.records | ForEach-Object { [ordered]@{
                    method = $_.method; path = $_.path; status = $_.status; response_kind = $_.responseKind
                } })
            } | ConvertTo-Json -Depth 5 -Compress))
        } catch { Write-Warning 'Regeneration protocol evidence unavailable; no retry was issued.' }
        finally {
            try { Invoke-ReceiptAction -Action chatgpt_private_protocol_probe -ExpectedAction private_protocol_probe `
                -Arguments @{mode='stop'} | Out-Null }
            catch { Write-Warning 'Protocol capture stop not confirmed; its bounded expiry remains active.' }
        }
    }
    if ($originCaptured -and -not $originRestored) {
        try {
            if (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime) {
                Write-Output "CHATGPT_REGENERATE_PROGRESS phase=restore_origin"
                Restore-Origin -ConversationPath $originPath
                $originRestored = $true
            } else {
                Write-Warning 'Original ChatGPT view restoration deferred: native_chat_not_foreground.'
            }
        } catch {
            Write-Warning "Unable to restore the original ChatGPT view after a failed regenerate smoke."
        }
    }
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}

if (!$originRestored) { throw 'Native regenerate acceptance did not restore the original conversation.' }
Register-ChatGptWebVerificationCases -Runtime $runtime `
    -CaseIds @("reversible/regenerate_response") `
    -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
$result | ConvertTo-Json -Depth 4
Write-Output "CHATGPT_WEB_REGENERATE_ACCEPTANCE=passed"
