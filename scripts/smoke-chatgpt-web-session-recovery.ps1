#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(15, 180)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 2
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

function Wait-ReadySession {
    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "restored authenticated ChatGPT session" -Predicate {
            param($state)
            $state.surface -eq "chatgpt_web" -and
                $state.bridge_state -eq "ready" -and
                $state.adapter_current -eq $true -and
                $state.authenticated -eq $true -and
                $state.composer_ready -eq $true
        }
}

function Get-AppPid {
    $result = Invoke-ElonNativeCommand -FilePath $runtime.adb `
        -ArgumentList @("-s", $runtime.device_serial, "shell", "pidof", "com.elon.app") `
        -TimeoutSeconds 10 -Label "read Elon app pid"
    if ($result.ExitCode -eq 1 -and -not $result.TimedOut) { return "" }
    Assert-ElonNativeCommand -Result $result -FailureMessage "read Elon app pid failed"
    return ([string]$result.Stdout).Trim()
}

function Get-ContextIdentity {
    $context = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_get_context"
    if (
        [string]::IsNullOrWhiteSpace([string]$context.context_revision) -or
        [int]$context.message_count -lt 0
    ) {
        throw "ChatGPT context identity is unavailable."
    }
    return [pscustomobject]@{
        revision = [string]$context.context_revision
        message_count = [int]$context.message_count
        available_message_count = [int]$context.available_message_count
    }
}

function Wait-ContextIdentity {
    param([Parameter(Mandatory = $true)]$Expected)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $current = Get-ContextIdentity
        if (
            $current.revision -eq $Expected.revision -and
            $current.message_count -eq $Expected.message_count -and
            $current.available_message_count -eq $Expected.available_message_count
        ) {
            return $current
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "ChatGPT conversation window did not recover before timeout."
}

Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "open_chatgpt_web" `
    -EnsureMainActivity | Out-Null
$before = Wait-ReadySession
$beforePid = Get-AppPid
if ([string]::IsNullOrWhiteSpace($beforePid)) { throw "Elon app pid is unavailable before restart." }
$beforeContext = Get-ContextIdentity
$beforeMode = [string]$before.view_mode

$restartRequested = $false
$processStopObserved = $false
try {
    Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
        -Arguments @("shell", "am", "force-stop", "com.elon.app") -TimeoutSec 15 `
        -Label "force-stop Elon app for session recovery" | Out-Null
    $restartRequested = $true
    $stoppedDeadline = [DateTimeOffset]::UtcNow.AddSeconds(15)
    do {
        if ([string]::IsNullOrWhiteSpace((Get-AppPid))) { break }
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $stoppedDeadline)
    if (-not [string]::IsNullOrWhiteSpace((Get-AppPid))) {
        throw "Elon app process did not stop before recovery."
    }
    $processStopObserved = $true

    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "open_chatgpt_web" `
        -EnsureMainActivity | Out-Null
    $after = Wait-ReadySession
    $afterPid = Get-AppPid
    $afterContext = Wait-ContextIdentity -Expected $beforeContext

    if ([string]::IsNullOrWhiteSpace($afterPid) -or -not $processStopObserved) {
        throw "Elon app process was not recreated."
    }
    if ([string]$after.view_mode -ne $beforeMode) {
        throw "ChatGPT view mode was not restored after process recreation."
    }
    if ($afterContext.revision -ne $beforeContext.revision) {
        throw "ChatGPT conversation identity changed after process recreation."
    }
    if (
        $afterContext.message_count -ne $beforeContext.message_count -or
        $afterContext.available_message_count -ne $beforeContext.available_message_count
    ) {
        throw "ChatGPT conversation window changed after process recreation."
    }

    [ordered]@{
        schema = "elon.chatgpt_web.session_recovery_smoke.v1"
        passed = $true
        device_serial = $DeviceSerial
        process_recreated = $true
        process_stop_observed = $true
        authenticated_restored = $true
        composer_restored = $true
        adapter_current = $after.adapter_current -eq $true
        view_mode_restored = $true
        conversation_identity_restored = $true
        context_window_restored = $true
        sent_messages = 0
        uploaded_attachments = 0
        cleared_cookies = $false
        cleared_app_data = $false
    } | ConvertTo-Json -Depth 5
    Write-Output "CHATGPT_WEB_SESSION_RECOVERY_SMOKE_STATUS=passed"
} finally {
    if ($restartRequested -and [string]::IsNullOrWhiteSpace((Get-AppPid))) {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "open_chatgpt_web" `
            -EnsureMainActivity | Out-Null
    }
}
