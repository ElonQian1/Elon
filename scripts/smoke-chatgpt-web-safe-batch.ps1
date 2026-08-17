#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(30, 180)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 2,
    [ValidateRange(0, 9999)][int]$ExpectedAdapterVersion = 0,
    [switch]$SkipUnlockNotification
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
$ExpectedAdapterVersion = Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$readiness = Get-ChatGptWebSmokeUserReadiness -Runtime $runtime `
    -NotifyWhenLocked:(-not $SkipUnlockNotification)
if (-not $readiness.ready) {
    $readiness | ConvertTo-Json -Compress
    throw "CHATGPT_WEB_SAFE_ACCEPTANCE_STATUS=user_action_required required_action=unlock_device notification_posted=$($readiness.notification_posted.ToString().ToLowerInvariant())"
}

$common = @{
    Adb = $Adb
    DeviceSerial = $DeviceSerial
    ExpectedHardwareSerial = $ExpectedHardwareSerial
    ReadyTimeoutSec = $ReadyTimeoutSec
    PollIntervalSec = $PollIntervalSec
    ExpectedAdapterVersion = $ExpectedAdapterVersion
}
$pinned = @{} + $common
$pinned.ExpectedHardwareSerial = $ExpectedHardwareSerial
$cases = @(
    [pscustomobject]@{
        id = "read_only_surface"
        script = "smoke-chatgpt-web-native-chat.ps1"
        arguments = $common + @{
            TimeoutSec = $ReadyTimeoutSec
        }
    },
    [pscustomobject]@{
        id = "feature_pages"
        script = "smoke-chatgpt-web-feature-pages.ps1"
        arguments = $pinned + @{ MaxFeaturePages = 8 }
    },
    [pscustomobject]@{
        id = "settings_structure"
        script = "smoke-chatgpt-web-settings.ps1"
        arguments = $pinned
    },
    [pscustomobject]@{
        id = "session_recovery"
        script = "smoke-chatgpt-web-session-recovery.ps1"
        arguments = $pinned
    },
    [pscustomobject]@{
        id = "conversation_management_structure"
        script = "smoke-chatgpt-web-conversation-management.ps1"
        arguments = @{
            Adb = $Adb
            DeviceSerial = $DeviceSerial
            ExpectedHardwareSerial = $ExpectedHardwareSerial
            TimeoutSec = $ReadyTimeoutSec
            ExpectedAdapterVersion = $ExpectedAdapterVersion
        }
    }
)

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    $results = [System.Collections.Generic.List[object]]::new()
    foreach ($case in $cases) {
        $path = Join-Path $PSScriptRoot $case.script
        $caseWatch = [System.Diagnostics.Stopwatch]::StartNew()
        Write-Output "CHATGPT_WEB_SAFE_CASE_PROGRESS=running case=$($case.id)"
        try {
            $baseline = Restore-ChatGptWebSmokeInteractiveBaseline `
                -Runtime $runtime -TimeoutSec ([Math]::Min(60, $ReadyTimeoutSec))
            Write-Output (
                "CHATGPT_WEB_SAFE_CASE_BASELINE=ready case=$($case.id) " +
                    "recovery=$($baseline.recovery)"
            )
            $caseArguments = $case.arguments
            & $path @caseArguments | Out-Null
            $caseWatch.Stop()
            $results.Add([pscustomobject]@{
                id = $case.id
                passed = $true
                detail = ""
                elapsed_seconds = [Math]::Round($caseWatch.Elapsed.TotalSeconds, 1)
            })
            Write-Output (
                "CHATGPT_WEB_SAFE_CASE_PROGRESS=passed case=$($case.id) " +
                    "elapsed_seconds=$([Math]::Round($caseWatch.Elapsed.TotalSeconds, 1))"
            )
        } catch {
            $caseWatch.Stop()
            $detail = ([string]$_.Exception.Message).Replace("`r", " ").Replace("`n", " ").Trim()
            if ($detail.Length -gt 160) { $detail = $detail.Substring(0, 160) }
            $results.Add([pscustomobject]@{
                id = $case.id
                passed = $false
                detail = $detail
                elapsed_seconds = [Math]::Round($caseWatch.Elapsed.TotalSeconds, 1)
            })
            Write-Output (
                "CHATGPT_WEB_SAFE_CASE_PROGRESS=failed case=$($case.id) " +
                    "elapsed_seconds=$([Math]::Round($caseWatch.Elapsed.TotalSeconds, 1))"
            )
        }
    }

    $failed = @($results | Where-Object { $_.passed -ne $true })
    [ordered]@{
        schema = "elon.chatgpt_web.safe_acceptance_batch.v1"
        passed = $failed.Count -eq 0
        device_serial = $DeviceSerial
        case_count = $results.Count
        passed_count = $results.Count - $failed.Count
        failed_count = $failed.Count
        cases = $results
        user_assisted_remaining = @(
            "attachment_lifecycle",
            "dictation_audio_capture",
            "realtime_voice",
            "account_mutations",
            "destructive_conversation_actions"
        )
        sent_messages = 0
        uploaded_attachments = 0
        cleared_cookies = $false
        cleared_app_data = $false
    } | ConvertTo-Json -Depth 6

    if ($failed.Count -gt 0) {
        throw "ChatGPT Web safe acceptance batch failed: $($failed.Count) case(s)."
    }
    Write-Output "CHATGPT_WEB_SAFE_ACCEPTANCE_BATCH_STATUS=passed"
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
