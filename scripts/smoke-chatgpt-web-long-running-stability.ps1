#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [Parameter(Mandatory = $true)][string]$ExpectedHardwareSerial,
    [ValidateRange(15, 480)][int]$DurationMinutes = 120,
    [ValidateRange(10, 120)][int]$PollIntervalSec = 30,
    [ValidateRange(20, 180)][int]$RecoveryTimeoutSec = 90,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [string]$CheckpointPath = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion

if (-not $CheckpointPath.Trim()) {
    $CheckpointPath = Join-Path (Split-Path $PSScriptRoot -Parent) `
        ".ai-tmp\chatgpt-web-long-running-stability.json"
}
$CheckpointPath = [System.IO.Path]::GetFullPath($CheckpointPath)
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 2
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

function Get-Sha256Text {
    param([AllowEmptyString()][string]$Value)

    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
        return ([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Write-StabilityCheckpoint {
    param([Parameter(Mandatory = $true)]$Value)

    $directory = Split-Path $CheckpointPath -Parent
    if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }
    $temporary = "$CheckpointPath.$PID.$([Guid]::NewGuid().ToString('N')).tmp"
    try {
        [System.IO.File]::WriteAllText(
            $temporary,
            "$(($Value | ConvertTo-Json -Depth 6))`n",
            [System.Text.UTF8Encoding]::new($false)
        )
        Move-Item -LiteralPath $temporary -Destination $CheckpointPath -Force
    } finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

$deadline = [DateTimeOffset]::UtcNow.AddMinutes($DurationMinutes)
$startedUtc = [DateTimeOffset]::UtcNow
$sampleCount = 0
$recoveryCount = 0
$conversationBinding = ""
$initialMessageCount = 0
$initialMode = ""

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
    $initial = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $RecoveryTimeoutSec -InitialWaitSec ([Math]::Min(15, $RecoveryTimeoutSec))
    Assert-ChatGptWebSmokeAdapterVersion -State $initial `
        -ExpectedAdapterVersion $ExpectedAdapterVersion
    $conversationBinding = Get-Sha256Text -Value ([string]$initial.conversation.url)
    $initialMessageCount = [int]$initial.conversation.message_count
    $initialMode = [string]$initial.view_mode

    do {
        if (-not (Test-ChatGptWebSmokeActivityForeground -Runtime $runtime)) {
            throw "Long-running stability acceptance stopped because another app took the foreground."
        }
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        $healthy =
            $state.surface -eq "chatgpt_web" -and
            $state.authenticated -eq $true -and
            $state.adapter_current -eq $true -and
            $state.bridge_state -eq "ready"
        if (-not $healthy) {
            $recoveryCount += 1
            $state = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
                -TimeoutSec $RecoveryTimeoutSec -InitialWaitSec 5
        }
        Assert-ChatGptWebSmokeAdapterVersion -State $state `
            -ExpectedAdapterVersion $ExpectedAdapterVersion
        if ((Get-Sha256Text -Value ([string]$state.conversation.url)) -ne $conversationBinding) {
            throw "The active conversation changed during long-running stability acceptance."
        }
        if ([int]$state.conversation.message_count -ne $initialMessageCount) {
            throw "Conversation message count changed during read-only stability acceptance."
        }
        if ([string]$state.view_mode -ne $initialMode) {
            throw "ChatGPT view mode changed during long-running stability acceptance."
        }
        $sampleCount += 1
        Write-StabilityCheckpoint -Value ([ordered]@{
            schema = "elon.chatgpt_web.long_running_stability_checkpoint.v1"
            status = "running"
            started_utc = $startedUtc.ToString("o")
            sampled_utc = [DateTimeOffset]::UtcNow.ToString("o")
            duration_minutes = $DurationMinutes
            sample_count = $sampleCount
            recovery_count = $recoveryCount
            adapter_version = $ExpectedAdapterVersion
            authenticated = $true
            conversation_binding_sha256 = $conversationBinding
            message_count = $initialMessageCount
            sent_messages = 0
            cleared_cookies = $false
            cleared_app_data = $false
        })
        if ([DateTimeOffset]::UtcNow -ge $deadline) { break }
        Start-Sleep -Seconds $PollIntervalSec
    } while ($true)

    Register-ChatGptWebVerificationCases -Runtime $runtime `
        -CaseIds @("safe/session_long_running_stability") `
        -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
    Write-StabilityCheckpoint -Value ([ordered]@{
        schema = "elon.chatgpt_web.long_running_stability_checkpoint.v1"
        status = "passed"
        started_utc = $startedUtc.ToString("o")
        completed_utc = [DateTimeOffset]::UtcNow.ToString("o")
        duration_minutes = $DurationMinutes
        sample_count = $sampleCount
        recovery_count = $recoveryCount
        adapter_version = $ExpectedAdapterVersion
        conversation_binding_sha256 = $conversationBinding
        message_count = $initialMessageCount
        sent_messages = 0
        cleared_cookies = $false
        cleared_app_data = $false
    })
    [ordered]@{
        schema = "elon.chatgpt_web.long_running_stability_smoke.v1"
        passed = $true
        duration_minutes = $DurationMinutes
        sample_count = $sampleCount
        recovery_count = $recoveryCount
        conversation_unchanged = $true
        sent_messages = 0
        private_content_emitted = $false
        cleared_cookies = $false
        cleared_app_data = $false
    } | ConvertTo-Json -Depth 5
    Write-Output "CHATGPT_WEB_LONG_RUNNING_STABILITY_STATUS=passed"
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
