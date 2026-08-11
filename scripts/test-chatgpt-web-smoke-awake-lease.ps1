#requires -Version 5.1

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$script:mockPower = "  mWakefulness=Awake"
$script:mockPolicy = "  showing=false`n  mIsShowing=false"
$script:mockStayAwake = "3"
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
    if ($command -eq "shell settings get global stay_on_while_plugged_in") {
        return $script:mockStayAwake
    }
    return ""
}

function New-MockRuntime {
    [pscustomobject]@{
        awake_lease_active = $false
        previous_stay_awake_setting = ""
        previous_stay_awake_setting_missing = $false
    }
}

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$runtime = New-MockRuntime
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
Assert-True $runtime.awake_lease_active "Awake lease was not activated."
Assert-True ($script:mockCommands -contains "shell settings put global stay_on_while_plugged_in 7") `
    "Awake lease did not enable the bounded stay-awake setting."
Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
Assert-True (-not $runtime.awake_lease_active) "Awake lease was not released."
Assert-True ($script:mockCommands -contains "shell settings put global stay_on_while_plugged_in 3") `
    "Awake lease did not restore the previous setting."

$script:mockCommands.Clear()
$script:mockPolicy = "  showing=true`n  mIsShowing=true"
$locked = New-MockRuntime
$lockedRejected = $false
try {
    Start-ChatGptWebSmokeAwakeLease -Runtime $locked | Out-Null
} catch {
    $lockedRejected = $_.Exception.Message -like "Device is locked.*"
}
Assert-True $lockedRejected "Locked device was not rejected before verification."
Assert-True (-not ($script:mockCommands -match "settings put")) `
    "Locked device changed stay-awake settings."

$script:mockCommands.Clear()
$script:mockPolicy = "  showing=false`n  mIsShowing=false"
$script:mockStayAwake = "null"
$missing = New-MockRuntime
Start-ChatGptWebSmokeAwakeLease -Runtime $missing | Out-Null
Stop-ChatGptWebSmokeAwakeLease -Runtime $missing | Out-Null
Assert-True ($script:mockCommands -contains "shell settings delete global stay_on_while_plugged_in") `
    "Missing prior stay-awake setting was not restored by deletion."

Write-Output "CHATGPT_WEB_SMOKE_AWAKE_LEASE_TEST=passed"
