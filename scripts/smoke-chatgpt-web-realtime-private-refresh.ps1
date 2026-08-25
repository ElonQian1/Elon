#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [Parameter(Mandatory = $true)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory = $true)]
    [ValidateSet("Start", "ObserveRoundTrip", "VerifyClosed", "StartReentry", "VerifyReentryClosed")]
    [string]$Phase,
    [switch]$UserConfirmedRealtimeVoice,
    [switch]$UserConfirmedVoiceRoundTrip,
    [switch]$UserConfirmedVoiceClosed,
    [string]$CheckpointPath = "",
    [ValidateRange(10, 180)][int]$TimeoutSec = 90,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion

$checkpointSchema = "elon.chatgpt_realtime_private_refresh_acceptance.v1"
$reportSchema = "elon.chatgpt_realtime_private_refresh_report.v1"
$checkpointMaxAge = [TimeSpan]::FromHours(12)
$voiceDumpPath = "/sdcard/elon-chatgpt-realtime-private-refresh.xml"
if (-not $CheckpointPath.Trim()) {
    $CheckpointPath = Join-Path (Split-Path $PSScriptRoot -Parent) `
        ".ai-tmp\chatgpt-realtime-private-refresh-acceptance.json"
}
$CheckpointPath = [System.IO.Path]::GetFullPath($CheckpointPath)
$reportPath = [System.IO.Path]::ChangeExtension($CheckpointPath, ".report.json")

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

function Write-SafeJson {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Value
    )

    $directory = Split-Path $Path -Parent
    if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }
    $temporary = "$Path.$PID.$([Guid]::NewGuid().ToString('N')).tmp"
    try {
        [System.IO.File]::WriteAllText(
            $temporary,
            "$(($Value | ConvertTo-Json -Depth 8))`n",
            [System.Text.UTF8Encoding]::new($false)
        )
        Move-Item -LiteralPath $temporary -Destination $Path -Force
    } finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Read-Checkpoint {
    if (-not (Test-Path -LiteralPath $CheckpointPath -PathType Leaf)) {
        throw "Realtime voice checkpoint is missing. Run -Phase Start first."
    }
    if ((Get-Item -LiteralPath $CheckpointPath).Length -gt 16384) {
        throw "Realtime voice checkpoint exceeds the safe size limit."
    }
    $value = Get-Content -LiteralPath $CheckpointPath -Raw | ConvertFrom-Json
    if ([string]$value.schema -ne $checkpointSchema) {
        throw "Realtime voice checkpoint schema is not supported."
    }
    $created = [DateTimeOffset]::Parse([string]$value.created_utc)
    if ([DateTimeOffset]::UtcNow - $created -gt $checkpointMaxAge) {
        throw "Realtime voice checkpoint expired. Run -Phase Start again."
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

function Get-ConversationBinding {
    param([Parameter(Mandatory = $true)]$State)

    $url = [string]$State.conversation.url
    if (-not $url.Trim()) { throw "Current ChatGPT conversation has no stable binding." }
    return Get-Sha256Text -Value $url
}

function Assert-AdapterState {
    param(
        [Parameter(Mandatory = $true)]$State,
        [switch]$RequireIdleComposer
    )

    Assert-ChatGptWebSmokeAdapterVersion -State $State `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    if ($State.adapter_current -ne $true) { throw "ChatGPT adapter is not current." }
    if ($State.authenticated -ne $true) { throw "ChatGPT authentication is not available." }
    if ($RequireIdleComposer) {
        if ($State.streaming -eq $true) { throw "Generation must be idle before realtime voice starts." }
        if ([int]$State.input.text_length -ne 0) { throw "Composer draft must be empty before realtime voice starts." }
        if (@($State.conversation.attachments).Count -ne 0) {
            throw "Attachments must be cleared before realtime voice starts."
        }
    }
}

function Promote-FirstTurnBinding {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)]$Checkpoint
    )

    $current = Get-ConversationBinding $State
    if ($current -eq [string]$Checkpoint.conversation_binding_sha256) { return }
    if (
        [int]$Checkpoint.baseline_message_count -ne 0 -or
        [int]$State.conversation.message_count -lt 2
    ) {
        throw "The active conversation changed during realtime voice acceptance."
    }
    $Checkpoint.conversation_binding_sha256 = $current
    $Checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
    Write-SafeJson -Path $CheckpointPath -Value $Checkpoint
}

function Assert-ConversationBinding {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)]$Checkpoint,
        [switch]$AllowFirstTurnPromotion
    )

    Assert-AdapterState -State $State
    if ($AllowFirstTurnPromotion) {
        Promote-FirstTurnBinding -State $State -Checkpoint $Checkpoint
    }
    if ((Get-ConversationBinding $State) -ne [string]$Checkpoint.conversation_binding_sha256) {
        throw "The active conversation changed during realtime voice acceptance."
    }
}

function Remove-VoiceDump {
    param([Parameter(Mandatory = $true)]$Runtime)

    try {
        Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
            -Arguments @("shell", "rm", "-f", $voiceDumpPath) `
            -TimeoutSec 5 -Label "remove transient realtime voice UI dump" | Out-Null
    } catch {
        # The dump is transient acceptance evidence; cleanup failure must not expose its content.
    }
}

function Get-VoiceSurfaceEvidence {
    param([Parameter(Mandatory = $true)]$Runtime)

    try {
        Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
            -Arguments @("shell", "uiautomator", "dump", $voiceDumpPath) `
            -TimeoutSec 12 -Label "capture realtime voice semantic surface" | Out-Null
        $dump = $null
        $readError = $null
        foreach ($attempt in 1..5) {
            try {
                $candidate = Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
                    -Arguments @("shell", "cat", $voiceDumpPath) `
                    -TimeoutSec 8 -Label "read realtime voice semantic surface"
                if ([string]$candidate -match '<hierarchy') {
                    $dump = $candidate
                    break
                }
            } catch {
                $readError = $_
            }
            Start-Sleep -Milliseconds (100 * $attempt)
        }
        if ($null -eq $dump) {
            if ($null -ne $readError) { throw $readError }
            throw "Realtime voice semantic surface dump was not readable."
        }
        $xml = [string]$dump
        return [pscustomobject]@{
            surface_visible = $xml -match 'content-desc="web-chat-realtime-voice:surface'
            close_visible = $xml -match 'content-desc="web-chat-realtime-voice:close"'
        }
    } finally {
        Remove-VoiceDump -Runtime $Runtime
    }
}

function Wait-VoiceCommand {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$ExistingRequestIds
    )

    return Wait-ChatGptWebSmokeState -Runtime $Runtime -TimeoutSec $TimeoutSec `
        -Description "realtime voice command receipt" -Predicate {
            param($state)
            @($state.command_requests | Where-Object {
                $_.request_id -notin $ExistingRequestIds -and
                    $_.expected_web_action -eq "invoke_ui_control" -and
                    $_.status -eq "succeeded"
            }).Count -gt 0
        }.GetNewClosure()
}

function Get-PrivateRefreshEvidence {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)][int64]$AfterObservedAtMs
    )

    $command = $State.last_command
    $detail = [string]$command.detail
    $match = [regex]::Match($detail, '^v1\|private_outcome\|success\|(\d{1,2})\|(\d{1,6})$')
    if (
        [string]$command.action -ne "research_network_observation" -or
        [int64]$command.observed_at_ms -le $AfterObservedAtMs -or
        -not $match.Success
    ) {
        return [pscustomobject]@{ observed = $false; elapsed_ms = 0; message_count = 0 }
    }
    return [pscustomobject]@{
        observed = $true
        elapsed_ms = [int64]$match.Groups[2].Value
        message_count = [int]$match.Groups[1].Value
    }
}

function Start-RealtimeVoice {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)]$State
    )

    $before = @($State.command_requests | ForEach-Object { [string]$_.request_id })
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    $accepted = Invoke-ChatGptWebSmokeAction -Runtime $Runtime `
        -Action "start_web_chat_realtime_voice"
    if ($accepted.control_ok -ne $true) {
        throw "Production native realtime voice entry rejected the request."
    }
    $active = Wait-VoiceCommand -Runtime $Runtime -ExistingRequestIds $before
    $surface = Get-VoiceSurfaceEvidence -Runtime $Runtime
    $timer.Stop()
    # The non-blocking voice surface starts collapsed. Its close control is only
    # exposed after the user expands the floating orb.
    if ($surface.surface_visible -ne $true) {
        throw "Realtime voice did not expose its native floating surface."
    }
    return [pscustomobject]@{
        state = $active
        elapsed_ms = [int64]$timer.ElapsedMilliseconds
        surface_visible = $true
        close_visible = [bool]$surface.close_visible
    }
}

function Write-Report {
    param(
        [Parameter(Mandatory = $true)][string]$ReportPhase,
        [Parameter(Mandatory = $true)][string]$Status,
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)]$Checkpoint,
        [int64]$LaunchElapsedMs = 0,
        [bool]$SurfaceVisible = $false,
        [bool]$CloseVisible = $false,
        [bool]$PrivateRefreshObserved = $false,
        [int64]$PrivateRefreshElapsedMs = 0
    )

    $report = [ordered]@{
        schema = $reportSchema
        phase = $ReportPhase
        status = $Status
        observed_utc = [DateTimeOffset]::UtcNow.ToString("o")
        device_binding_sha256 = [string]$Checkpoint.device_binding_sha256
        conversation_binding_sha256 = [string]$Checkpoint.conversation_binding_sha256
        adapter_version = [int]$State.adapter_version
        baseline_message_count = [int]$Checkpoint.baseline_message_count
        observed_message_count = [int]$State.conversation.message_count
        messages_added = [int]$State.conversation.message_count - [int]$Checkpoint.baseline_message_count
        launch_elapsed_ms = $LaunchElapsedMs
        voice_surface_visible = $SurfaceVisible
        close_control_visible = $CloseVisible
        private_refresh_observed = $PrivateRefreshObserved
        private_refresh_elapsed_ms = $PrivateRefreshElapsedMs
        android_microphone_permission = [string]$State.audio.android_permission
        web_microphone_permission = [string]$State.audio.request_state
        private_content_emitted = $false
        audio_content_read = $false
        raw_conversation_identifier_persisted = $false
        cookie_or_app_data_cleared = $false
    }
    Write-SafeJson -Path $reportPath -Value $report
    $report | ConvertTo-Json -Compress
}

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$readiness = Get-ChatGptWebSmokeUserReadiness -Runtime $runtime
if (-not $readiness.ready) { throw "Device must be unlocked for supervised realtime voice acceptance." }

switch ($Phase) {
    "Start" {
        if (-not $UserConfirmedRealtimeVoice) {
            throw "Use -UserConfirmedRealtimeVoice only while the user supervises the microphone flow."
        }
        Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $TimeoutSec | Out-Null
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        Assert-AdapterState -State $state -RequireIdleComposer
        $checkpoint = [ordered]@{
            schema = $checkpointSchema
            phase = "voice_started"
            created_utc = [DateTimeOffset]::UtcNow.ToString("o")
            updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
            device_binding_sha256 = Get-Sha256Text -Value $ExpectedHardwareSerial.Trim()
            conversation_binding_sha256 = Get-ConversationBinding $state
            adapter_version = [int]$state.adapter_version
            baseline_message_count = [int]$state.conversation.message_count
            voice_round_trip_confirmed = $false
            first_launch_elapsed_ms = 0
            reentry_launch_elapsed_ms = 0
            first_close_message_count = 0
            reentry_baseline_message_count = 0
            private_observation_at_ms = if (
                [string]$state.last_command.action -eq "research_network_observation"
            ) { [int64]$state.last_command.observed_at_ms } else { 0 }
            private_content_emitted = $false
            audio_content_read = $false
            raw_conversation_identifier_persisted = $false
        }
        Write-SafeJson -Path $CheckpointPath -Value $checkpoint
        $started = Start-RealtimeVoice -Runtime $runtime -State $state
        $checkpoint.first_launch_elapsed_ms = [int64]$started.elapsed_ms
        $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
        Write-SafeJson -Path $CheckpointPath -Value $checkpoint
        Write-Report -ReportPhase "voice_started" -Status "waiting_for_voice_round_trip" `
            -State $started.state -Checkpoint $checkpoint `
            -LaunchElapsedMs $started.elapsed_ms -SurfaceVisible $started.surface_visible `
            -CloseVisible $started.close_visible
    }
    "ObserveRoundTrip" {
        if (-not $UserConfirmedVoiceRoundTrip) {
            throw "Use -UserConfirmedVoiceRoundTrip after the user spoke and heard a complete reply."
        }
        $checkpoint = Read-Checkpoint
        Assert-CheckpointIdentity $checkpoint
        if ([string]$checkpoint.phase -ne "voice_started") {
            throw "Checkpoint is not waiting for realtime voice round-trip confirmation."
        }
        $state = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
            -Description "granted realtime voice permissions" -Predicate {
                param($candidate)
                [string]$candidate.audio.android_permission -eq "granted" -and
                    [string]$candidate.audio.request_state -eq "web_permission_granted"
            }
        Assert-ConversationBinding -State $state -Checkpoint $checkpoint -AllowFirstTurnPromotion
        $surface = Get-VoiceSurfaceEvidence -Runtime $runtime
        $checkpoint.phase = "voice_round_trip_observed"
        $checkpoint.voice_round_trip_confirmed = $true
        $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
        Write-SafeJson -Path $CheckpointPath -Value $checkpoint
        Write-Report -ReportPhase "voice_round_trip_observed" `
            -Status "waiting_for_user_to_close_voice" -State $state -Checkpoint $checkpoint `
            -LaunchElapsedMs ([int64]$checkpoint.first_launch_elapsed_ms) `
            -SurfaceVisible ([bool]$surface.surface_visible) `
            -CloseVisible ([bool]$surface.close_visible)
    }
    "VerifyClosed" {
        if (-not $UserConfirmedVoiceClosed) {
            throw "Close realtime voice, then use -UserConfirmedVoiceClosed."
        }
        $checkpoint = Read-Checkpoint
        Assert-CheckpointIdentity $checkpoint
        if (
            [string]$checkpoint.phase -ne "voice_round_trip_observed" -or
            $checkpoint.voice_round_trip_confirmed -ne $true
        ) {
            throw "Checkpoint is not waiting for realtime voice closure."
        }
        Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $TimeoutSec | Out-Null
        $state = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
            -Description "private transcript refresh after voice closure" -Predicate {
                param($candidate)
                $privateEvidence = Get-PrivateRefreshEvidence -State $candidate `
                    -AfterObservedAtMs ([int64]$checkpoint.private_observation_at_ms)
                $candidate.dictation_active -ne $true -and
                    $candidate.audio.local_request_pending -ne $true -and
                    $candidate.audio.web_request_pending -ne $true -and
                    [int]$candidate.conversation.message_count -ge
                        ([int]$checkpoint.baseline_message_count + 2) -and
                    $privateEvidence.observed -eq $true
            }.GetNewClosure()
        Assert-ConversationBinding -State $state -Checkpoint $checkpoint -AllowFirstTurnPromotion
        $privateEvidence = Get-PrivateRefreshEvidence -State $state `
            -AfterObservedAtMs ([int64]$checkpoint.private_observation_at_ms)
        $checkpoint.phase = "voice_closed_verified"
        $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
        $checkpoint.first_close_message_count = [int]$state.conversation.message_count
        Write-SafeJson -Path $CheckpointPath -Value $checkpoint
        Write-Report -ReportPhase "voice_closed_verified" `
            -Status "private_refresh_passed_waiting_for_reentry" -State $state `
            -Checkpoint $checkpoint -LaunchElapsedMs ([int64]$checkpoint.first_launch_elapsed_ms) `
            -PrivateRefreshObserved $privateEvidence.observed `
            -PrivateRefreshElapsedMs $privateEvidence.elapsed_ms
    }
    "StartReentry" {
        if (-not $UserConfirmedRealtimeVoice) {
            throw "Use -UserConfirmedRealtimeVoice only while the user supervises the re-entry."
        }
        $checkpoint = Read-Checkpoint
        Assert-CheckpointIdentity $checkpoint
        if ([string]$checkpoint.phase -ne "voice_closed_verified") {
            throw "Verify the first voice closure before testing re-entry."
        }
        Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $TimeoutSec | Out-Null
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        Assert-AdapterState -State $state -RequireIdleComposer
        Assert-ConversationBinding -State $state -Checkpoint $checkpoint
        $checkpoint.reentry_baseline_message_count = [int]$state.conversation.message_count
        $started = Start-RealtimeVoice -Runtime $runtime -State $state
        $checkpoint.phase = "voice_reentry_started"
        $checkpoint.reentry_launch_elapsed_ms = [int64]$started.elapsed_ms
        $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
        Write-SafeJson -Path $CheckpointPath -Value $checkpoint
        Write-Report -ReportPhase "voice_reentry_started" `
            -Status "waiting_for_reentry_close" -State $started.state -Checkpoint $checkpoint `
            -LaunchElapsedMs $started.elapsed_ms -SurfaceVisible $started.surface_visible `
            -CloseVisible $started.close_visible
    }
    "VerifyReentryClosed" {
        if (-not $UserConfirmedVoiceClosed) {
            throw "Close the re-entered voice session, then use -UserConfirmedVoiceClosed."
        }
        $checkpoint = Read-Checkpoint
        Assert-CheckpointIdentity $checkpoint
        if ([string]$checkpoint.phase -ne "voice_reentry_started") {
            throw "Checkpoint is not waiting for re-entry closure."
        }
        Open-ChatGptWebNativeChatSurface -Runtime $runtime -TimeoutSec $TimeoutSec | Out-Null
        $state = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $TimeoutSec `
            -Description "stable native chat after realtime voice re-entry" -Predicate {
                param($candidate)
                $candidate.dictation_active -ne $true -and
                    $candidate.audio.local_request_pending -ne $true -and
                    $candidate.audio.web_request_pending -ne $true
            }
        Assert-ConversationBinding -State $state -Checkpoint $checkpoint
        $checkpoint.phase = "completed"
        $checkpoint.updated_utc = [DateTimeOffset]::UtcNow.ToString("o")
        Write-SafeJson -Path $CheckpointPath -Value $checkpoint
        Write-Report -ReportPhase "completed" -Status "passed" -State $state `
            -Checkpoint $checkpoint -LaunchElapsedMs ([int64]$checkpoint.reentry_launch_elapsed_ms)
    }
}
