#requires -Version 5.1

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$script:mockPower = "  mWakefulness=Awake"
$script:mockPolicy = "  showing=true`n  mIsShowing=true"
$script:mockCommands = [System.Collections.Generic.List[string]]::new()

function Invoke-ChatGptWebSmokeAdb {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [int]$TimeoutSec = 10,
        [string]$Label = ""
    )

    $command = $Arguments -join " "
    $script:mockCommands.Add($command)
    if ($command -eq "shell dumpsys power") { return $script:mockPower }
    if ($command -eq "shell dumpsys window policy") { return $script:mockPolicy }
    return ""
}

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$runtime = [pscustomobject]@{}
$locked = Get-ChatGptWebSmokeUserReadiness -Runtime $runtime -NotifyWhenLocked
Assert-True (-not $locked.ready) "Locked device was reported ready."
Assert-True ($locked.status -eq "user_action_required") "Locked status is not actionable."
Assert-True ($locked.required_action -eq "unlock_device") "Unlock action was not reported."
Assert-True $locked.notification_posted "Locked-device notification was not posted."
Assert-True (@($script:mockCommands -match "shell cmd notification post").Count -gt 0) `
    "Locked-device gate did not post the bounded notification."
foreach ($forbidden in @("KEYCODE_WAKEUP", "settings put", "am start", "pm clear")) {
    Assert-True (@($script:mockCommands -match [regex]::Escape($forbidden)).Count -eq 0) `
        "Locked-device gate invoked forbidden command: $forbidden"
}

$script:mockCommands.Clear()
$script:mockPolicy = "  showing=false`n  mIsShowing=false"
$ready = Get-ChatGptWebSmokeUserReadiness -Runtime $runtime -NotifyWhenLocked
Assert-True $ready.ready "Unlocked device was not reported ready."
Assert-True ($ready.status -eq "ready") "Unlocked status is not ready."
Assert-True (-not $ready.notification_posted) "Unlocked device posted a notification."
Assert-True (@($script:mockCommands -match "shell cmd notification post").Count -eq 0) `
    "Unlocked-device gate posted an unnecessary notification."

Write-Output "CHATGPT_WEB_USER_READINESS_GATE_TEST=passed"
