param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [string]$Tool = "phone_status",
    [string]$Arguments = "{}",
    [int]$Port = 8787,
    [int]$HealthTimeoutSec = 6,
    [int]$HealthPollMs = 250,
    [int]$RequestTimeoutSec = 120,
    [ValidateRange(1, 60)][int]$AdbTimeoutSec = 10,
    [switch]$EnsureMainActivity,
    [switch]$OpenAppOnFailure,
    [switch]$NoBootstrap
)

$ErrorActionPreference = "Stop"
$nativeCommandModule = Join-Path $PSScriptRoot "native-command-timeout.ps1"
if (-not (Test-Path -LiteralPath $nativeCommandModule -PathType Leaf)) {
    throw "Missing native command timeout helper: $nativeCommandModule"
}
. $nativeCommandModule

if (!(Test-Path -LiteralPath $Adb)) {
    throw "adb not found: $Adb"
}

function Invoke-Adb {
    $adbArgs = @($args)
    $serialArgs = @()
    if ($DeviceSerial.Trim()) {
        $serialArgs = @("-s", $DeviceSerial.Trim())
    }
    $script:LastAdbResult = Invoke-ElonNativeCommand -FilePath $Adb `
        -ArgumentList (@($serialArgs) + @($adbArgs)) `
        -TimeoutSeconds $AdbTimeoutSec -Label "APK MCP adb command"
    @($script:LastAdbResult.Stdout, $script:LastAdbResult.Stderr) |
        Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }
}

function Start-ApkMcpDebug {
    $serviceOutput = Invoke-Adb shell am start-foreground-service `
        -a com.elon.app.mcp.START_KEEPALIVE `
        -n com.elon.app/.mcp.McpDebugKeepAliveService
    $serviceText = ($serviceOutput | Out-String).Trim()
    if ($script:LastAdbResult.ExitCode -eq 0 -and $serviceText -notmatch "Error:") {
        return
    }

    Invoke-Adb shell am broadcast --receiver-foreground `
        -a com.elon.app.mcp.START_DEBUG `
        -n com.elon.app/.mcp.McpDebugControlReceiver | Out-Null
    Assert-ElonNativeCommand -Result $script:LastAdbResult `
        -FailureMessage "Unable to start APK MCP debug service"
}

function Wait-ApkMcpHealth {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds([Math]::Max(1, $HealthTimeoutSec))
    $lastError = $null
    do {
        try {
            return Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 2
        } catch {
            $lastError = $_.Exception.Message
            Start-Sleep -Milliseconds ([Math]::Max(50, $HealthPollMs))
        }
    } while ([DateTimeOffset]::UtcNow -lt $deadline)

    if ($OpenAppOnFailure) {
        Start-ApkMainActivity -SettleMilliseconds 600
        $retryDeadline = [DateTimeOffset]::UtcNow.AddSeconds(3)
        do {
            try {
                return Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 2
            } catch {
                $lastError = $_.Exception.Message
                Start-Sleep -Milliseconds ([Math]::Max(50, $HealthPollMs))
            }
        } while ([DateTimeOffset]::UtcNow -lt $retryDeadline)
    }

    throw "APK MCP health did not respond on port $Port within ${HealthTimeoutSec}s. Last error: $lastError"
}

function Get-ApkMcpHealthIfAvailable {
    try {
        return Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 1
    } catch {
        return $null
    }
}

function Start-ApkMainActivity {
    param([int]$SettleMilliseconds = 700)

    # Mirror McpNativeControlBridge.openMainActivity so repeated MCP bootstrap
    # reuses the existing task instead of stacking another MainActivity.
    Invoke-Adb shell am start -f 0x34000000 `
        -n com.elon.app/.MainActivity --ez mcp_open_main true | Out-Null
    Assert-ElonNativeCommand -Result $script:LastAdbResult `
        -FailureMessage "Unable to start APK MainActivity"
    Start-Sleep -Milliseconds ([Math]::Max(0, $SettleMilliseconds))
}

$health = if ($NoBootstrap -and -not $EnsureMainActivity) {
    Get-ApkMcpHealthIfAvailable
} else { $null }
if ($null -eq $health) {
    if (!$NoBootstrap) {
        Start-ApkMcpDebug
    }

    if ($EnsureMainActivity) {
        Start-ApkMainActivity
    }

    Invoke-Adb forward "tcp:$Port" "tcp:$Port" | Out-Null
    Assert-ElonNativeCommand -Result $script:LastAdbResult `
        -FailureMessage "Unable to create APK MCP adb forward"
    $health = Wait-ApkMcpHealth
}
$token = [string]$health.auth_token
if (!$token) {
    throw "MCP health endpoint did not return auth_token."
}

$argumentObject = $Arguments | ConvertFrom-Json
if ($null -eq $argumentObject) {
    $argumentObject = [pscustomobject]@{}
}
$argumentObject | Add-Member -NotePropertyName auth_token -NotePropertyValue $token -Force

$effectiveRequestTimeoutSec = [Math]::Max(1, $RequestTimeoutSec)
$waitTimeoutProperty = $argumentObject.PSObject.Properties["wait_timeout_ms"]
if ($waitTimeoutProperty -and $null -ne $waitTimeoutProperty.Value) {
    try {
        $waitTimeoutSec = [Math]::Ceiling(([double]$waitTimeoutProperty.Value) / 1000)
        $effectiveRequestTimeoutSec = [Math]::Max($effectiveRequestTimeoutSec, [int]$waitTimeoutSec + 15)
    } catch {
        throw "Invalid wait_timeout_ms: $($waitTimeoutProperty.Value)"
    }
}

$request = [ordered]@{
    jsonrpc = "2.0"
    id = "ps-$([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())"
    method = "tools/call"
    params = [ordered]@{
        name = $Tool
        arguments = $argumentObject
    }
}

$body = $request | ConvertTo-Json -Depth 30 -Compress
Invoke-RestMethod `
    -Method Post `
    -Uri "http://127.0.0.1:$Port/mcp" `
    -ContentType "application/json" `
    -Body $body `
    -TimeoutSec $effectiveRequestTimeoutSec
