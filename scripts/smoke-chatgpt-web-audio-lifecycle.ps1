#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [Parameter(Mandatory = $true)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory = $true)]
    [ValidateSet(
        "Prepare",
        "StartDictation",
        "VerifyAndClearDictation",
        "StartRealtimeVoice",
        "VerifyRealtimeVoice",
        "VerifyRestored"
    )][string]$Phase,
    [switch]$UserConfirmedMicrophone,
    [switch]$UserConfirmedRealtimeVoice,
    [switch]$UserConfirmedVoiceRoundTrip,
    [switch]$UserConfirmedVoiceClosed,
    [string]$CheckpointPath = "",
    [ValidateRange(10, 180)][int]$TimeoutSec = 90,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 90
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-supervised-runtime.ps1")

$checkpointSchema = "elon.chatgpt_web.audio_lifecycle_checkpoint.v1"
$reportSchema = "elon.chatgpt_web.audio_lifecycle_smoke.v1"
$audioSchema = "elon.chatgpt_web.audio_permission.v1"
$checkpointMaxAge = [TimeSpan]::FromHours(12)
if (-not $CheckpointPath.Trim()) {
    $CheckpointPath = Join-Path (Split-Path $PSScriptRoot -Parent) `
        ".ai-tmp\chatgpt-web-audio-lifecycle.json"
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
    try {
        [System.IO.File]::WriteAllText(
            $temporary,
            "$(($Value | ConvertTo-Json -Depth 8))`n",
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
        throw "Audio lifecycle checkpoint is missing. Run -Phase Prepare first."
    }
    if ((Get-Item -LiteralPath $CheckpointPath).Length -gt 16384) {
        throw "Audio lifecycle checkpoint exceeds the safe size limit."
    }
    $value = Get-Content -LiteralPath $CheckpointPath -Raw | ConvertFrom-Json
    if ([string]$value.schema -ne $checkpointSchema) {
        throw "Audio lifecycle checkpoint schema is not supported."
    }
    $created = [DateTimeOffset]::Parse([string]$value.created_utc)
    if ([DateTimeOffset]::UtcNow - $created -gt $checkpointMaxAge) {
        throw "Audio lifecycle checkpoint expired. Run -Phase Prepare again."
    }
    return $value
}

function Assert-CheckpointIdentity {
    param([Parameter(Mandatory = $true)]$Checkpoint)

    if (
        [string]$Checkpoint.device_binding_sha256 -ne
        (Get-Sha256Text -Value $ExpectedHardwareSerial.Trim())
    ) {
        throw "Checkpoint belongs to a different physical device."
    }
    if ([int]$Checkpoint.adapter_version -ne $ExpectedAdapterVersion) {
        throw "Checkpoint adapter version does not match this acceptance run."
    }
}

function Assert-AudioContract {
    param([Parameter(Mandatory = $true)]$State)

    if ([string]$State.audio.schema -ne $audioSchema) {
        throw "ChatGPT audio permission state schema is unavailable."
    }
    if ([string]$State.audio.audio_capture_state -ne "unobserved") {
        throw "Audio acceptance must not inspect microphone content."
    }
}

function Get-ConversationBinding {
    param([Parameter(Mandatory = $true)]$State)

    $url = [string]$State.conversation.url
    if (-not $url.Trim()) { throw "Current ChatGPT conversation has no stable binding." }
    return Get-Sha256Text -Value $url
}

function Assert-ConversationUnchanged {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)]$Checkpoint,
        [switch]$AllowNonEmptyDraft
    )

    Assert-ChatGptWebSmokeAdapterVersion -State $State -ExpectedAdapterVersion $ExpectedAdapterVersion
    Assert-AudioContract -State $State
    if ($State.authenticated -ne $true) { throw "ChatGPT authentication is not available." }
    if ($State.streaming -eq $true) { throw "Generation must remain idle during audio acceptance." }
    if (-not $AllowNonEmptyDraft -and [int]$State.input.text_length -ne 0) {
        throw "Composer draft changed during audio acceptance. Do not submit captured speech."
    }
    if ([int]$State.conversation.message_count -ne [int]$Checkpoint.message_count) {
        throw "Conversation message count changed during audio acceptance."
    }
    if ((Get-ConversationBinding $State) -ne [string]$Checkpoint.conversation_binding_sha256) {
        throw "The active conversation changed during audio acceptance."
    }
    if (@($State.conversation.attachments).Count -ne 0) {
        throw "Audio acceptance requires an empty attachment list."
    }
}

function Assert-AudioSessionBinding {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)]$Checkpoint
    )

    Assert-ChatGptWebSmokeAdapterVersion -State $State -ExpectedAdapterVersion $ExpectedAdapterVersion
    Assert-AudioContract -State $State
    if ($State.authenticated -ne $true) { throw "ChatGPT authentication is not available." }
    if ((Get-ConversationBinding $State) -ne [string]$Checkpoint.conversation_binding_sha256) {
        throw "The isolated audio conversation changed during acceptance."
    }
    if (@($State.conversation.attachments).Count -ne 0) {
        throw "Audio acceptance requires an empty attachment list."
    }
}

function Assert-IdleComposer {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)]$Checkpoint
    )

    Assert-ConversationUnchanged -State $State -Checkpoint $Checkpoint
    if ($State.composer_ready -ne $true) { throw "ChatGPT composer is not ready." }
    if ($State.dictation_active -eq $true) { throw "ChatGPT dictation is already active." }
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
        [Parameter(Mandatory = $true)][string]$ExpectedAction
    )

    $dispatched = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime -Action $Action `
        -TimeoutSec $TimeoutSec
    $requestId = [string]$dispatched.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action" }
    return [pscustomobject]@{
        request_id = $requestId
        result = Wait-CommandReceipt -RequestId $requestId -ExpectedAction $ExpectedAction
    }
}

function Write-Report {
    param(
        [Parameter(Mandatory = $true)][string]$ReportPhase,
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)][string]$Status,
        [bool]$MessageCountUnchanged = $true
    )

    [ordered]@{
        schema = $reportSchema
        phase = $ReportPhase
        passed = $true
        adapter_version = [int]$State.adapter_version
        view_mode = [string]$State.view_mode
        dictation_active = [bool]$State.dictation_active
        android_permission = [string]$State.audio.android_permission
        audio_request_state = [string]$State.audio.request_state
        audio_capture_state = [string]$State.audio.audio_capture_state
        message_count_unchanged = $MessageCountUnchanged
        input_empty = $true
        sent_messages = 0
        uploaded_attachments = 0
        cleared_cookies = $false
        cleared_app_data = $false
        audio_content_read = $false
        private_content_emitted = $false
    } | ConvertTo-Json -Depth 6
    Write-Output "CHATGPT_WEB_AUDIO_LIFECYCLE_STATUS=$Status"
}

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "open_chatgpt_web" `
        -EnsureMainActivity | Out-Null
    $ready = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $TimeoutSec -InitialWaitSec ([Math]::Min(15, $TimeoutSec))
    Assert-AudioContract -State $ready

    switch ($Phase) {
        "Prepare" {
            if ($ready.composer_ready -ne $true -or $ready.streaming -eq $true) {
                throw "An idle authenticated ChatGPT composer is required."
            }
            if ([int]$ready.input.text_length -ne 0 -or $ready.dictation_active -eq $true) {
                throw "Clear the draft and stop dictation before preparing audio acceptance."
            }
            if (@($ready.conversation.attachments).Count -ne 0) {
                throw "Remove attachments before preparing audio acceptance."
            }
            $isolation = Start-ChatGptWebSmokeIsolatedConversation -Runtime $runtime `
                -OriginState $ready -TimeoutSec $TimeoutSec
            $isolated = $isolation.isolated_state
            $checkpoint = [ordered]@{
                schema = $checkpointSchema
                phase = "prepared"
                created_utc = [DateTimeOffset]::UtcNow.ToString("o")
                updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
                device_binding_sha256 = Get-Sha256Text $ExpectedHardwareSerial.Trim()
                origin_conversation_path = [string]$isolation.origin_conversation_path
                origin_view_mode = [string]$isolation.origin_view_mode
                conversation_binding_sha256 = Get-ConversationBinding $isolated
                adapter_version = [int]$isolated.adapter_version
                initial_view_mode = [string]$isolated.view_mode
                message_count = 0
                dictation_request_id = ""
                realtime_voice_request_id = ""
                voice_round_trip_confirmed = $false
                voice_message_count_added = 0
                sent_messages = 0
                cleared_cookies = $false
                cleared_app_data = $false
            }
            Write-Checkpoint $checkpoint
            Write-Report -ReportPhase "prepared" -State $isolated -Status "prepared"
        }
        "StartDictation" {
            if (-not $UserConfirmedMicrophone) {
                throw "Run this phase with -UserConfirmedMicrophone only while the user supervises microphone access."
            }
            $checkpoint = Read-Checkpoint
            Assert-CheckpointIdentity $checkpoint
            if ([string]$checkpoint.phase -ne "prepared") {
                throw "Checkpoint is not ready to start dictation."
            }
            Assert-IdleComposer -State $ready -Checkpoint $checkpoint
            $started = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
                -Action "chatgpt_start_dictation" -TimeoutSec $TimeoutSec
            $checkpoint.phase = "dictation_requested"
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            $checkpoint.dictation_request_id = [string]$started.command_receipt.request_id
            Write-Checkpoint $checkpoint
            $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
            Write-Report -ReportPhase "dictation_requested" -State $state `
                -Status "waiting_for_user_microphone_permission"
        }
        "VerifyAndClearDictation" {
            if (-not $UserConfirmedMicrophone) {
                throw "Run this phase with -UserConfirmedMicrophone only while the user supervises active dictation."
            }
            $checkpoint = Read-Checkpoint
            Assert-CheckpointIdentity $checkpoint
            if ([string]$checkpoint.phase -ne "dictation_requested") {
                throw "Checkpoint is not waiting for dictation verification."
            }
            Wait-CommandReceipt -RequestId ([string]$checkpoint.dictation_request_id) `
                -ExpectedAction "start_dictation" | Out-Null
            $active = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                -RequireChatGptForeground -Description "active ChatGPT dictation" -Predicate {
                    param($state)
                    $state.dictation_active -eq $true -and
                        [string]$state.audio.android_permission -eq "granted" -and
                        [string]$state.audio.request_state -in @("local_action_ready", "web_permission_granted")
                }
            Assert-ConversationUnchanged -State $active -Checkpoint $checkpoint
            Invoke-ReceiptAction -Action "chatgpt_submit_dictation" `
                -ExpectedAction "submit_dictation" | Out-Null
            $transcribed = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                -RequireChatGptForeground -Description "non-empty ChatGPT dictation draft" -Predicate {
                    param($state)
                    $state.dictation_active -ne $true -and
                        [int]$state.input.text_length -gt 0 -and
                        [string]$state.view_mode -eq [string]$checkpoint.initial_view_mode
                }
            Assert-ConversationUnchanged -State $transcribed -Checkpoint $checkpoint `
                -AllowNonEmptyDraft
            $cleared = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime `
                -Action "set_input_text" -Arguments @{ text = "" } -TimeoutSec $TimeoutSec
            Wait-CommandReceipt -RequestId ([string]$cleared.command_receipt.request_id) `
                -ExpectedAction "set_draft" | Out-Null
            $restored = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                -RequireChatGptForeground -Description "cleared dictation draft" -Predicate {
                    param($state)
                    $state.dictation_active -ne $true -and [int]$state.input.text_length -eq 0
                }
            Assert-IdleComposer -State $restored -Checkpoint $checkpoint
            Register-ChatGptWebVerificationCases -Runtime $runtime `
                -CaseIds @("supervised/dictation_transcription") `
                -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
            $checkpoint.phase = "dictation_cleared"
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            Write-Checkpoint $checkpoint
            Write-Report -ReportPhase "dictation_cleared" -State $restored `
                -Status "dictation_passed"
        }
        "StartRealtimeVoice" {
            if (-not $UserConfirmedRealtimeVoice) {
                throw "Run this phase with -UserConfirmedRealtimeVoice only while the user supervises realtime voice."
            }
            $checkpoint = Read-Checkpoint
            Assert-CheckpointIdentity $checkpoint
            if ([string]$checkpoint.phase -ne "dictation_cleared") {
                throw "Complete, verify, and clear dictation before starting realtime voice."
            }
            Assert-IdleComposer -State $ready -Checkpoint $checkpoint
            $voice = Invoke-ReceiptAction -Action "chatgpt_start_realtime_voice" `
                -ExpectedAction "invoke_ui_control"
            $official = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                -Description "official ChatGPT realtime voice surface" -Predicate {
                    param($state)
                    [string]$state.view_mode -eq "web"
                }
            Assert-ConversationUnchanged -State $official -Checkpoint $checkpoint
            $checkpoint.phase = "realtime_voice_started"
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            $checkpoint.realtime_voice_request_id = [string]$voice.request_id
            Write-Checkpoint $checkpoint
            Write-Report -ReportPhase "realtime_voice_started" -State $official `
                -Status "waiting_for_user_realtime_voice_confirmation"
        }
        "VerifyRealtimeVoice" {
            if (-not $UserConfirmedVoiceRoundTrip) {
                throw "Run this phase with -UserConfirmedVoiceRoundTrip only after the user spoke and heard a ChatGPT voice reply."
            }
            $checkpoint = Read-Checkpoint
            Assert-CheckpointIdentity $checkpoint
            if ([string]$checkpoint.phase -ne "realtime_voice_started") {
                throw "Checkpoint is not waiting for realtime voice verification."
            }
            $activeVoice = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                -RequireChatGptForeground -Description "granted ChatGPT realtime voice permission" -Predicate {
                    param($state)
                    [string]$state.view_mode -eq "web" -and
                        [string]$state.audio.android_permission -eq "granted" -and
                        [string]$state.audio.request_state -eq "web_permission_granted"
                }
            Assert-AudioSessionBinding -State $activeVoice -Checkpoint $checkpoint
            $checkpoint.phase = "realtime_voice_observed"
            $checkpoint.voice_round_trip_confirmed = $true
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            Write-Checkpoint $checkpoint
            Write-Report -ReportPhase "realtime_voice_observed" -State $activeVoice `
                -Status "waiting_for_user_to_close_realtime_voice"
        }
        "VerifyRestored" {
            if (-not $UserConfirmedVoiceClosed) {
                throw "Close the official voice UI, then run this phase with -UserConfirmedVoiceClosed."
            }
            $checkpoint = Read-Checkpoint
            Assert-CheckpointIdentity $checkpoint
            if ([string]$checkpoint.phase -ne "realtime_voice_observed") {
                throw "Checkpoint is not waiting for realtime voice closure."
            }
            if ($checkpoint.voice_round_trip_confirmed -ne $true) {
                throw "Realtime voice round-trip confirmation is missing."
            }
            $targetMode = switch ([string]$checkpoint.initial_view_mode) {
                "native" { "native" }
                "web" { "official" }
                "quick" { "quick" }
                default { throw "Checkpoint contains an unsupported initial view mode." }
            }
            Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
                -Arguments @{ view_mode = $targetMode } | Out-Null
            $restored = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
                -RequireChatGptForeground -Description "restored ChatGPT view" -Predicate {
                    param($state)
                    [string]$state.view_mode -eq [string]$checkpoint.initial_view_mode -and
                        $state.dictation_active -ne $true -and
                        $state.audio.local_request_pending -ne $true -and
                        $state.audio.web_request_pending -ne $true
                }
            Assert-AudioSessionBinding -State $restored -Checkpoint $checkpoint
            $messagesAdded = [int]$restored.conversation.message_count - [int]$checkpoint.message_count
            if ($messagesAdded -lt 2) {
                throw "Realtime voice did not persist a complete user and assistant round trip."
            }
            Restore-ChatGptWebSmokeOrigin -Runtime $runtime `
                -ConversationPath ([string]$checkpoint.origin_conversation_path) `
                -ViewMode ([string]$checkpoint.origin_view_mode) -TimeoutSec $TimeoutSec | Out-Null
            Register-ChatGptWebVerificationCases -Runtime $runtime `
                -CaseIds @("supervised/realtime_voice_round_trip") `
                -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
            $checkpoint.phase = "passed"
            $checkpoint.voice_message_count_added = $messagesAdded
            $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            Write-Checkpoint $checkpoint
            Write-Report -ReportPhase "passed" -State $restored -Status "passed" `
                -MessageCountUnchanged:$false
        }
    }
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
