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
        mcp_bootstrapped = $false
        awake_lease_active = $false
        previous_stay_awake_setting = ""
        previous_stay_awake_setting_missing = $false
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

function Get-ChatGptWebSmokeDisplayState {
    param([Parameter(Mandatory = $true)]$Runtime)

    $power = Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
        -Arguments @("shell", "dumpsys", "power") -TimeoutSec 8 `
        -Label "read device power state"
    $windowPolicy = Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
        -Arguments @("shell", "dumpsys", "window", "policy") -TimeoutSec 8 `
        -Label "read device keyguard state"
    return [pscustomobject]@{
        awake = $power -match '(?m)^\s*mWakefulness=Awake\s*$'
        keyguard_showing =
            $windowPolicy -match '(?m)^\s*showing=true\s*$' -or
            $windowPolicy -match '(?m)^\s*mIsShowing=true\s*$' -or
            $windowPolicy -match '(?m)\bisStatusBarKeyguard=true\b'
    }
}

function Start-ChatGptWebSmokeAwakeLease {
    param([Parameter(Mandatory = $true)]$Runtime)

    if ($Runtime.awake_lease_active) { return $Runtime }
    $display = Get-ChatGptWebSmokeDisplayState -Runtime $Runtime
    if (-not $display.awake) {
        Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
            -Arguments @("shell", "input", "keyevent", "KEYCODE_WAKEUP") `
            -TimeoutSec 5 -Label "wake ChatGPT Web acceptance device" | Out-Null
        Start-Sleep -Milliseconds 500
        $display = Get-ChatGptWebSmokeDisplayState -Runtime $Runtime
    }
    if (-not $display.awake) {
        throw "Device screen is not awake. Verification is deferred."
    }
    if ($display.keyguard_showing) {
        throw "Device is locked. Unlock it before ChatGPT Web verification; no credential input was attempted."
    }

    $previous = (Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
        -Arguments @("shell", "settings", "get", "global", "stay_on_while_plugged_in") `
        -TimeoutSec 5 -Label "read device stay-awake setting").Trim()
    $Runtime.previous_stay_awake_setting = $previous
    $Runtime.previous_stay_awake_setting_missing = -not $previous -or $previous -eq "null"
    Invoke-ChatGptWebSmokeAdb -Runtime $Runtime `
        -Arguments @("shell", "settings", "put", "global", "stay_on_while_plugged_in", "7") `
        -TimeoutSec 5 -Label "enable bounded ChatGPT Web stay-awake setting" | Out-Null
    $Runtime.awake_lease_active = $true
    return $Runtime
}

function Stop-ChatGptWebSmokeAwakeLease {
    param([Parameter(Mandatory = $true)]$Runtime)

    if (-not $Runtime.awake_lease_active) { return $true }
    try {
        $arguments = if ($Runtime.previous_stay_awake_setting_missing) {
            @("shell", "settings", "delete", "global", "stay_on_while_plugged_in")
        } else {
            @(
                "shell", "settings", "put", "global", "stay_on_while_plugged_in",
                [string]$Runtime.previous_stay_awake_setting
            )
        }
        Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments $arguments `
            -TimeoutSec 5 -Label "restore device stay-awake setting" | Out-Null
        $Runtime.awake_lease_active = $false
        return $true
    } catch {
        $detail = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message
        Write-Warning "Unable to restore ChatGPT Web stay-awake setting: $detail"
        return $false
    }
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

function ConvertTo-ChatGptWebSmokeSafeDiagnostic {
    param(
        [AllowNull()]$Value,
        [ValidateRange(16, 240)][int]$MaxLength = 120
    )

    $text = ([string]$Value).Replace("`r", " ").Replace("`n", " ").Trim()
    if (-not $text) { return "" }
    $text = [regex]::Replace($text, 'https?://\S+', '<url>', 'IgnoreCase')
    $text = [regex]::Replace(
        $text,
        '(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b',
        '<email>'
    )
    $text = [regex]::Replace(
        $text,
        '(?i)\b(cookie|token|password|authorization)\s*[:=]\s*\S+',
        '$1=<redacted>'
    )
    if ($text.Length -gt $MaxLength) { $text = $text.Substring(0, $MaxLength) }
    return $text
}

function Get-ChatGptWebSmokeMcpFailureDetail {
    param(
        [AllowNull()]$Response,
        [Parameter(Mandatory = $true)][string]$Tool
    )

    $structured = if ($null -ne $Response) { $Response.result.structuredContent } else { $null }
    $action = if ($null -ne $structured) {
        ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $structured.action -MaxLength 64
    } else { "" }
    $errorValue = if ($null -ne $structured -and $structured.PSObject.Properties["error_code"]) {
        $structured.error_code
    } elseif ($null -ne $structured) {
        $structured.error
    } else { "" }
    $error = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $errorValue -MaxLength 120
    $parts = [System.Collections.Generic.List[string]]::new()
    $parts.Add("tool=$Tool")
    if ($action) { $parts.Add("action=$action") }
    if ($error) { $parts.Add("error=$error") }
    return $parts -join " "
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
        HealthTimeoutSec = 15
        RequestTimeoutSec = 30
        AdbTimeoutSec = 8
    }
    if ($EnsureMainActivity) {
        $params.EnsureMainActivity = $true
        $params.OpenAppOnFailure = $true
    }
    if ($Runtime.mcp_bootstrapped) { $params.NoBootstrap = $true }
    try {
        $responses = @(& $Runtime.invoke_mcp @params)
    } catch {
        if (-not $Runtime.mcp_bootstrapped) { throw }
        $Runtime.mcp_bootstrapped = $false
        $params.Remove("NoBootstrap")
        $params.Remove("OpenAppOnFailure")
        $responses = @(& $Runtime.invoke_mcp @params)
    }
    $response = $responses | Select-Object -Last 1
    if ($null -eq $response -or $response.result.isError) {
        $detail = Get-ChatGptWebSmokeMcpFailureDetail -Response $response -Tool $Tool
        throw "APK MCP tool failed: $detail"
    }
    $structured = $response.result.structuredContent
    if ($null -eq $structured) {
        throw "APK MCP tool returned no structured content: $Tool"
    }
    $Runtime.mcp_bootstrapped = $true
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

function Open-ChatGptWebSmokeSurface {
    param([Parameter(Mandatory = $true)]$Runtime)

    $state = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state"
    if ($state.activity_bound -eq $true -and $state.surface -eq "chatgpt_web") {
        return $state
    }
    if ($state.activity_bound -eq $true) {
        return Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action "open_chatgpt_web"
    }
    return Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action "open_chatgpt_web" `
        -EnsureMainActivity
}

function Wait-ChatGptWebSmokeAuthenticatedReady {
    param(
        [Parameter(Mandatory = $true)]$Runtime,
        [ValidateRange(10, 300)][int]$TimeoutSec = 90,
        [ValidateRange(1, 60)][int]$InitialWaitSec = 15
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $ready = {
        param($state)
        $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.adapter_current -eq $true -and
            $state.authenticated -eq $true
    }
    try {
        return Wait-ChatGptWebSmokeState -Runtime $Runtime `
            -TimeoutSec ([Math]::Min($InitialWaitSec, $TimeoutSec)) `
            -Description "authenticated ChatGPT Web bridge" -Predicate $ready
    } catch {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool "ui_state"
        $recoverable = $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "connecting" -and
            $state.adapter_current -eq $true -and
            $state.authenticated -eq $true
        if (-not $recoverable) { throw }

        Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action "chatgpt_refresh" | Out-Null
        $remaining = [int][Math]::Ceiling(
            ($deadline - [DateTimeOffset]::UtcNow).TotalSeconds
        )
        if ($remaining -lt 1) {
            throw "Timed out refreshing the authenticated ChatGPT Web bridge."
        }
        return Wait-ChatGptWebSmokeState -Runtime $Runtime -TimeoutSec $remaining `
            -Description "refreshed authenticated ChatGPT Web bridge" -Predicate $ready
    }
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
