#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(30, 180)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 2,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 72
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

$pinned = @{
    Adb = $Adb
    DeviceSerial = $DeviceSerial
    ExpectedHardwareSerial = $ExpectedHardwareSerial
    ReadyTimeoutSec = $ReadyTimeoutSec
    PollIntervalSec = $PollIntervalSec
    ExpectedAdapterVersion = $ExpectedAdapterVersion
}
$cases = @(
    [pscustomobject]@{
        id = "reversible_controls"
        script = "smoke-chatgpt-web-reversible-controls.ps1"
        arguments = $pinned
    },
    [pscustomobject]@{
        id = "composer_controls"
        script = "smoke-chatgpt-web-composer-controls.ps1"
        arguments = $pinned + @{ SkipDictation = $true }
    },
    [pscustomobject]@{
        id = "message_structure"
        script = "smoke-chatgpt-web-message-structure.ps1"
        arguments = @{
            Adb = $Adb
            DeviceSerial = $DeviceSerial
            ExpectedHardwareSerial = $ExpectedHardwareSerial
            ReadyTimeoutSec = $ReadyTimeoutSec
            PollIntervalSec = $PollIntervalSec
            MaxConversations = 20
            ExpectedAdapterVersion = $ExpectedAdapterVersion
        }
    },
    [pscustomobject]@{
        id = "message_actions_and_regenerate_menu"
        script = "inspect-chatgpt-web-regenerate-menu.ps1"
        arguments = $pinned + @{ ReplyTimeoutSec = 240 }
    },
    [pscustomobject]@{
        id = "regenerate_reply"
        script = "smoke-chatgpt-web-regenerate.ps1"
        arguments = $pinned + @{ ReplyTimeoutSec = 240 }
    }
)

Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    $results = [System.Collections.Generic.List[object]]::new()
    foreach ($case in $cases) {
        $path = Join-Path $PSScriptRoot $case.script
        try {
            $caseArguments = $case.arguments
            & $path @caseArguments | Out-Null
            $results.Add([pscustomobject]@{ id = $case.id; passed = $true; detail = "" })
        } catch {
            $detail = ([string]$_.Exception.Message).Replace("`r", " ").Replace("`n", " ").Trim()
            if ($detail.Length -gt 160) { $detail = $detail.Substring(0, 160) }
            $results.Add([pscustomobject]@{ id = $case.id; passed = $false; detail = $detail })
        }
    }

    $failed = @($results | Where-Object { $_.passed -ne $true })
    [ordered]@{
        schema = "elon.chatgpt_web.reversible_acceptance_batch.v1"
        passed = $failed.Count -eq 0
        device_serial = $DeviceSerial
        case_count = $results.Count
        passed_count = $results.Count - $failed.Count
        failed_count = $failed.Count
        cases = $results
        user_supervised_remaining = @(
            "official_authentication",
            "attachment_lifecycle",
            "dictation_audio_capture",
            "realtime_voice",
            "account_mutations",
            "destructive_conversation_actions"
        )
        sent_messages = 2
        regenerated_messages = 1
        uploaded_attachments = 0
        cleared_cookies = $false
        cleared_app_data = $false
    } | ConvertTo-Json -Depth 6

    if ($failed.Count -gt 0) {
        throw "ChatGPT Web reversible acceptance batch failed: $($failed.Count) case(s)."
    }
    Write-Output "CHATGPT_WEB_REVERSIBLE_ACCEPTANCE_BATCH_STATUS=passed"
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
}
