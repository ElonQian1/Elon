#requires -Version 5.1

$nativeCommandModule = Join-Path $PSScriptRoot "native-command-timeout.ps1"
if (-not (Test-Path -LiteralPath $nativeCommandModule -PathType Leaf)) {
    throw "Missing native command timeout helper: $nativeCommandModule"
}
. $nativeCommandModule

function New-ChatGptWebSmokeRuntime {
    param(
        [Parameter(Mandatory = $true)][string]$Adb,
        [Parameter(Mandatory = $true)][string]$DeviceSerial,
        [string]$ExpectedHardwareSerial = "",
        [ValidateRange(1, 10)][int]$PollIntervalSec = 2
    )

    if (-not (Test-Path -LiteralPath $Adb -PathType Leaf)) {
        throw "adb not found: $Adb"
    }
    $serial = $DeviceSerial.Trim()
    if (-not $serial) { throw "A device serial is required." }
    if ($serial -match '^emulator-') {
        throw "ChatGPT Web true-device smoke does not accept emulator transport."
    }
    $hardwareSerial = $ExpectedHardwareSerial.Trim()
    if ($serial -match ':\d+$' -and -not $hardwareSerial) {
        throw "Wireless true-device smoke requires an expected hardware serial; use a physical USB serial otherwise."
    }
    $invokeMcp = Join-Path $PSScriptRoot "invoke-apk-mcp.ps1"
    if (-not (Test-Path -LiteralPath $invokeMcp -PathType Leaf)) {
        throw "Missing APK MCP helper: $invokeMcp"
    }
    return [pscustomobject]@{
        adb = [System.IO.Path]::GetFullPath($Adb)
        device_serial = $serial
        expected_hardware_serial = $hardwareSerial
        invoke_mcp = $invokeMcp
        poll_interval_sec = $PollIntervalSec
    }
}

function Assert-ChatGptWebSmokeDevice {
    param([Parameter(Mandatory = $true)]$Runtime)

    $devices = Invoke-ElonNativeCommand -FilePath $Runtime.adb `
        -ArgumentList @("devices", "-l") -TimeoutSeconds 5 -Label "list adb devices"
    Assert-ElonNativeCommand -Result $devices -FailureMessage "Unable to list adb devices"
    $connected = @(
        ([string]$devices.Stdout -split "`r?`n") |
            Where-Object { $_ -match '^\S+\s+device(?:\s|$)' } |
            ForEach-Object { ($_ -split '\s+')[0] }
    )
    if ($Runtime.device_serial -notin $connected) {
        throw "Device is not connected: $($Runtime.device_serial). Verification is deferred."
    }
    $state = Invoke-ElonNativeCommand -FilePath $Runtime.adb `
        -ArgumentList @("-s", $Runtime.device_serial, "get-state") `
        -TimeoutSeconds 5 -Label "read adb device state"
    Assert-ElonNativeCommand -Result $state -FailureMessage "Unable to read device state"
    if ([string]$state.Stdout -notmatch '^device\s*$') {
        throw "Device is not ready: $($Runtime.device_serial). Verification is deferred."
    }
    $expectedHardwareSerial = [string]$Runtime.expected_hardware_serial
    if ($expectedHardwareSerial) {
        $identity = Invoke-ElonNativeCommand -FilePath $Runtime.adb `
            -ArgumentList @("-s", $Runtime.device_serial, "shell", "getprop", "ro.serialno") `
            -TimeoutSeconds 5 -Label "read adb hardware identity"
        Assert-ElonNativeCommand -Result $identity -FailureMessage "Unable to read device hardware identity"
        if ([string]$identity.Stdout.Trim() -ine $expectedHardwareSerial) {
            throw "Device hardware identity does not match the pinned target. Verification is deferred."
        }
    }
}

function Assert-ChatGptWebSmokeUsbDevice {
    param([Parameter(Mandatory = $true)]$Runtime)

    if ($Runtime.device_serial -match ':\d+$') {
        throw "ChatGPT Web USB smoke requires a physical USB serial, not wireless transport."
    }
    Assert-ChatGptWebSmokeDevice -Runtime $Runtime
}

function Assert-ChatGptWebSmokeTrustedDevice {
    param([Parameter(Mandatory = $true)]$Runtime)

    Assert-ChatGptWebSmokeDevice -Runtime $Runtime
}

function Invoke-ChatGptWebSmokeAdb {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [ValidateRange(1, 60)][int]$TimeoutSec = 10,
        [string]$Label = "ChatGPT Web adb command"
    )

    $result = Invoke-ElonNativeCommand -FilePath $Runtime.adb `
        -ArgumentList (@("-s", $Runtime.device_serial) + $Arguments) `
        -TimeoutSeconds $TimeoutSec -Label $Label
    Assert-ElonNativeCommand -Result $result -FailureMessage "$Label failed"
    return [string]$result.Stdout
}

function Invoke-ChatGptWebSmokeMcp {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string]$Tool,
        [hashtable]$Arguments = @{},
        [switch]$EnsureMainActivity
    )

    $params = @{
        Adb = $Runtime.adb
        DeviceSerial = $Runtime.device_serial
        Tool = $Tool
        Arguments = ($Arguments | ConvertTo-Json -Depth 20 -Compress)
        OpenAppOnFailure = $true
    }
    if ($EnsureMainActivity) { $params.EnsureMainActivity = $true }
    $responses = @(& $Runtime.invoke_mcp @params)
    $response = $responses | Select-Object -Last 1
    if ($null -eq $response -or $response.result.isError) {
        throw "APK MCP tool failed: $Tool"
    }
    $structured = $response.result.structuredContent
    if ($null -eq $structured) {
        throw "APK MCP tool returned no structured content: $Tool"
    }
    return $structured
}

function Invoke-ChatGptWebSmokeAction {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][string]$Action,
        [hashtable]$Arguments = @{},
        [switch]$EnsureMainActivity
    )

    $payload = @{} + $Arguments
    $payload.action = $Action
    return Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_control" `
        -Arguments $payload -EnsureMainActivity:$EnsureMainActivity
}

function Wait-ChatGptWebSmokeState {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [Parameter(Mandatory = $true)][scriptblock]$Predicate,
        [Parameter(Mandatory = $true)][ValidateRange(1, 300)][int]$TimeoutSec,
        [Parameter(Mandatory = $true)][string]$Description
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $last = $null
    do {
        $last = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state"
        if (& $Predicate $last) { return $last }
        Start-Sleep -Seconds $Runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Description. Last page=$($last.page_kind), bridge=$($last.bridge_state)."
}
