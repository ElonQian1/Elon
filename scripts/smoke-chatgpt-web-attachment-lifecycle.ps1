#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [Parameter(Mandatory = $true)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory = $true)]
    [ValidateSet(
        "Prepare",
        "OpenPickerForRemove",
        "VerifyAndRemove",
        "OpenPickerForSend",
        "SendAndVerifyReply"
    )][string]$Phase,
    [switch]$UserConfirmedAttachmentSend,
    [string]$CheckpointPath = "",
    [ValidateRange(10, 180)][int]$TimeoutSec = 90,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-supervised-runtime.ps1")

$checkpointSchema = "elon.chatgpt_web.attachment_lifecycle_checkpoint.v1"
$reportSchema = "elon.chatgpt_web.attachment_lifecycle_smoke.v1"
$checkpointMaxAge = [TimeSpan]::FromHours(12)
if (-not $CheckpointPath.Trim()) {
    $CheckpointPath = Join-Path (Split-Path $PSScriptRoot -Parent) `
        ".ai-tmp\chatgpt-web-attachment-lifecycle.json"
}
$CheckpointPath = [System.IO.Path]::GetFullPath($CheckpointPath)

function Get-Sha256Text {
    param([Parameter(Mandatory = $true)][string]$Value)

    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
        return ([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Write-Checkpoint {
    param([Parameter(Mandatory = $true)]$Value)

    $directory = Split-Path $CheckpointPath -Parent
    if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }
    $temporary = "$CheckpointPath.$PID.$([Guid]::NewGuid().ToString('N')).tmp"
    $json = $Value | ConvertTo-Json -Depth 8
    try {
        [System.IO.File]::WriteAllText(
            $temporary,
            "$json`n",
            [System.Text.UTF8Encoding]::new($false)
        )
        Move-Item -LiteralPath $temporary -Destination $CheckpointPath -Force
    } finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Read-Checkpoint {
    if (-not (Test-Path -LiteralPath $CheckpointPath -PathType Leaf)) {
        throw "Attachment lifecycle checkpoint is missing. Run -Phase Prepare first."
    }
    if ((Get-Item -LiteralPath $CheckpointPath).Length -gt 16384) {
        throw "Attachment lifecycle checkpoint exceeds the safe size limit."
    }
    $value = Get-Content -LiteralPath $CheckpointPath -Raw | ConvertFrom-Json
    if ([string]$value.schema -ne $checkpointSchema) {
        throw "Attachment lifecycle checkpoint schema is not supported."
    }
    $created = [DateTimeOffset]::Parse([string]$value.created_utc)
    if ([DateTimeOffset]::UtcNow - $created -gt $checkpointMaxAge) {
        throw "Attachment lifecycle checkpoint expired. Run -Phase Prepare again."
    }
    return $value
}

function Get-ConversationBinding {
    param([Parameter(Mandatory = $true)]$State)

    $url = [string]$State.conversation.url
    if (-not $url.Trim()) { throw "Current ChatGPT conversation has no stable binding." }
    return Get-Sha256Text -Value $url
}

function Assert-CommonState {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)]$Checkpoint
    )

    Assert-ChatGptWebSmokeAdapterVersion -State $State `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    if ($State.authenticated -ne $true -or $State.composer_ready -ne $true) {
        throw "Authenticated ChatGPT composer is not ready."
    }
    if ($State.streaming -eq $true) { throw "Generation must be idle for attachment acceptance." }
    if ([int]$State.input.text_length -ne 0) {
        throw "Composer draft must be empty for attachment acceptance."
    }
    if ([int]$State.conversation.message_count -ne [int]$Checkpoint.message_count) {
        throw "Conversation message count changed after the checkpoint. Run -Phase Prepare again."
    }
    if ((Get-ConversationBinding -State $State) -ne [string]$Checkpoint.conversation_binding_sha256) {
        throw "The active conversation changed after the checkpoint. Run -Phase Prepare again."
    }
}

function Wait-CommandReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$ExpectedAction
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
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
            return [pscustomobject]@{ state = $state; receipt = $receipt }
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
        -Arguments $Arguments -TimeoutSec $TimeoutSec
    $requestId = [string]$dispatched.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action" }
    return Wait-CommandReceipt -RequestId $requestId -ExpectedAction $ExpectedAction
}

function Wait-ExternalPicker {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds([Math]::Min(30, $TimeoutSec))
    do {
        if (-not (Test-ChatGptWebSmokeActivityForeground -Runtime $runtime)) { return }
        Start-Sleep -Milliseconds 300
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Android file picker did not take the foreground."
}

function Open-AttachmentPicker {
    Invoke-ReceiptAction -Action "chatgpt_list_composer_options" `
        -ExpectedAction "list_composer_tools" -Arguments @{ section = "tools" } | Out-Null
    $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_get_navigation" -Arguments @{ section = "tools" }
    $option = @($navigation.composer_sections.tools) |
        Where-Object { [string]$_.semantic -eq "attachment_file" } |
        Select-Object -First 1
    if ($null -eq $option) { throw "Semantic attachment_file composer option is unavailable." }
    $selected = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_select_composer_option" -Arguments @{
            section = "tools"
            option_id = [string]$option.id
        }
    if ($selected.control_ok -ne $true) { throw "Unable to open Android file picker." }
    Wait-ExternalPicker
}

function Assert-CheckpointIdentity {
    param([Parameter(Mandatory = $true)]$Checkpoint)

    $deviceBinding = Get-Sha256Text -Value $ExpectedHardwareSerial.Trim()
    if ([string]$Checkpoint.device_binding_sha256 -ne $deviceBinding) {
        throw "Checkpoint belongs to a different physical device."
    }
    if ([int]$Checkpoint.adapter_version -ne $ExpectedAdapterVersion) {
        throw "Checkpoint adapter version does not match this acceptance run."
    }
}

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    # Preserve the isolated conversation between supervised phases. Reopening the
    # entry action can navigate back to the previously persisted conversation.
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $ready = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $TimeoutSec -InitialWaitSec ([Math]::Min(15, $TimeoutSec))
    Assert-ChatGptWebSmokeAdapterVersion -State $ready `
        -ExpectedAdapterVersion $ExpectedAdapterVersion

    switch ($Phase) {
        "Prepare" {
            if ($ready.composer_ready -ne $true -or $ready.streaming -eq $true) {
                throw "An idle authenticated ChatGPT composer is required."
            }
            if ([int]$ready.input.text_length -ne 0) {
                throw "Composer draft must be empty before creating a checkpoint."
            }
            if (@($ready.conversation.attachments).Count -ne 0) {
                throw "Remove existing attachments before creating a checkpoint."
            }
            $isolation = Start-ChatGptWebSmokeIsolatedConversation -Runtime $runtime `
                -OriginState $ready -TimeoutSec $TimeoutSec
            $isolated = $isolation.isolated_state
            $checkpoint = [ordered]@{
                schema = $checkpointSchema
                phase = "prepared"
                created_utc = [DateTimeOffset]::UtcNow.ToString("o")
                updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
                device_binding_sha256 = Get-Sha256Text -Value $ExpectedHardwareSerial.Trim()
                origin_conversation_path = [string]$isolation.origin_conversation_path
                origin_view_mode = [string]$isolation.origin_view_mode
                conversation_binding_sha256 = Get-ConversationBinding -State $isolated
                adapter_version = [int]$isolated.adapter_version
                message_count = 0
                marker = "ELON-CHATGPT-ATTACHMENT-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
                send_request_id = ""
                send_after_ms = 0
                sent_messages = 0
                cleared_cookies = $false
                cleared_app_data = $false
            }
            Write-Checkpoint -Value $checkpoint
            [ordered]@{
                schema = $reportSchema
                phase = "prepared"
                passed = $true
                adapter_version = [int]$isolated.adapter_version
                message_count = 0
                isolated_conversation = $true
                attachment_count = 0
                sent_messages = 0
                private_content_emitted = $false
            } | ConvertTo-Json -Depth 6
            Write-Output "CHATGPT_WEB_ATTACHMENT_LIFECYCLE_STATUS=prepared"
        }
        "OpenPickerForRemove" {
            $checkpoint = Read-Checkpoint
            Assert-CheckpointIdentity -Checkpoint $checkpoint
            Assert-CommonState -State $ready -Checkpoint $checkpoint
            if (@($ready.conversation.attachments).Count -ne 0) {
                throw "Attachment already exists; use -Phase VerifyAndRemove or prepare again."
            }

            Open-AttachmentPicker

            $checkpoint.phase = "picker_opened_for_remove"
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            Write-Checkpoint -Value $checkpoint
            [ordered]@{
                schema = $reportSchema
                phase = "picker_opened"
                passed = $true
                adapter_version = [int]$ready.adapter_version
                picker_foreground = $true
                selected_local_files = 0
                sent_messages = 0
                private_content_emitted = $false
            } | ConvertTo-Json -Depth 6
            Write-Output "CHATGPT_WEB_ATTACHMENT_LIFECYCLE_STATUS=waiting_for_user_selection"
        }
        "VerifyAndRemove" {
            $checkpoint = Read-Checkpoint
            Assert-CheckpointIdentity -Checkpoint $checkpoint
            if ([string]$checkpoint.phase -ne "picker_opened_for_remove") {
                throw "Checkpoint is not waiting for attachment verification."
            }
            Assert-CommonState -State $ready -Checkpoint $checkpoint
            $attached = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                -RequireChatGptForeground -Description "one ready ChatGPT attachment" -Predicate {
                    param($state)
                    $items = @($state.conversation.attachments)
                    $items.Count -eq 1 -and [string]$items[0].state -eq "ready"
                }
            Assert-CommonState -State $attached -Checkpoint $checkpoint
            $attachmentId = [string]@($attached.conversation.attachments)[0].id
            if (-not $attachmentId.Trim()) { throw "Ready attachment has no semantic ID." }

            Invoke-ReceiptAction -Action "chatgpt_remove_attachment" `
                -ExpectedAction "remove_attachment" -Arguments @{ attachment_id = $attachmentId } | Out-Null
            $restored = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                -RequireChatGptForeground -Description "empty ChatGPT attachment list" -Predicate {
                    param($state)
                    @($state.conversation.attachments).Count -eq 0
                }
            Assert-CommonState -State $restored -Checkpoint $checkpoint
            $checkpoint.phase = "attachment_removed"
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            Write-Checkpoint -Value $checkpoint
            [ordered]@{
                schema = $reportSchema
                phase = "attachment_removed"
                passed = $true
                adapter_version = [int]$restored.adapter_version
                selected_local_files = 1
                attachment_ready_count = 1
                attachment_removed_count = 1
                final_attachment_count = 0
                message_count_unchanged = $true
                input_empty = $true
                sent_messages = 0
                cleared_cookies = $false
                cleared_app_data = $false
                private_content_emitted = $false
            } | ConvertTo-Json -Depth 6
            Write-Output "CHATGPT_WEB_ATTACHMENT_LIFECYCLE_STATUS=waiting_for_send_picker"
        }
        "OpenPickerForSend" {
            $checkpoint = Read-Checkpoint
            Assert-CheckpointIdentity -Checkpoint $checkpoint
            if ([string]$checkpoint.phase -ne "attachment_removed") {
                throw "Complete attachment removal before opening the send picker."
            }
            Assert-CommonState -State $ready -Checkpoint $checkpoint
            Open-AttachmentPicker
            $checkpoint.phase = "picker_opened_for_send"
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            Write-Checkpoint -Value $checkpoint
            Write-Output "CHATGPT_WEB_ATTACHMENT_LIFECYCLE_STATUS=waiting_for_send_selection"
        }
        "SendAndVerifyReply" {
            if (-not $UserConfirmedAttachmentSend) {
                throw "Run this phase with -UserConfirmedAttachmentSend only while the user supervises file upload and message send."
            }
            $checkpoint = Read-Checkpoint
            Assert-CheckpointIdentity -Checkpoint $checkpoint
            if ([string]$checkpoint.phase -eq "picker_opened_for_send") {
                Assert-CommonState -State $ready -Checkpoint $checkpoint
                $attached = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                    -RequireChatGptForeground -Description "one ready attachment for send" -Predicate {
                        param($state)
                        $items = @($state.conversation.attachments)
                        $items.Count -eq 1 -and [string]$items[0].state -eq "ready"
                    }
                Assert-CommonState -State $attached -Checkpoint $checkpoint
                $draft = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "set_input_text" `
                    -Arguments @{ text = "Read the attached test file and reply only with: $($checkpoint.marker)" }
                Wait-ChatGptCommandReceipt `
                    -InvokeUiState { Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" } `
                    -RequestId ([string]$draft.command_receipt.request_id) `
                    -ExpectedAction "set_draft" -TimeoutSec $TimeoutSec `
                    -PollIntervalSec $runtime.poll_interval_sec | Out-Null
                $beforeSend = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
                $send = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"
                $requestId = [string]$send.command_receipt.request_id
                if (-not $requestId) { throw "Attachment send did not return a receipt id." }
                $checkpoint.phase = "reply_requested"
                $checkpoint.send_request_id = $requestId
                $checkpoint.send_after_ms = [long]$beforeSend.last_command.observed_at_ms
                $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
                Write-Checkpoint -Value $checkpoint
            } elseif ([string]$checkpoint.phase -ne "reply_requested") {
                throw "Checkpoint is not waiting for attachment send verification."
            }

            Wait-ChatGptProbeReply `
                -InvokeUiState { Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state" } `
                -RequestId ([string]$checkpoint.send_request_id) `
                -Marker ([string]$checkpoint.marker) -AfterMs ([long]$checkpoint.send_after_ms) `
                -TimeoutSec $TimeoutSec -PollIntervalSec $runtime.poll_interval_sec | Out-Null
            Restore-ChatGptWebSmokeOrigin -Runtime $runtime `
                -ConversationPath ([string]$checkpoint.origin_conversation_path) `
                -ViewMode ([string]$checkpoint.origin_view_mode) -TimeoutSec $TimeoutSec | Out-Null
            $checkpoint.phase = "passed"
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            Write-Checkpoint -Value $checkpoint
            [ordered]@{
                schema = $reportSchema
                phase = "passed"
                passed = $true
                isolated_conversation = $true
                selected_local_files = 2
                attachment_removed_count = 1
                attachment_message_sent = $true
                assistant_completed = $true
                original_view_restored = $true
                evidence_registered = $false
                diagnostic_only = $true
                sent_messages = 1
                cleared_cookies = $false
                cleared_app_data = $false
                private_content_emitted = $false
            } | ConvertTo-Json -Depth 6
            Write-Output "CHATGPT_WEB_ATTACHMENT_LIFECYCLE_STATUS=passed"
        }
    }
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
