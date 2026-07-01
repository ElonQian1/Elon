param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [string]$Tool = "phone_status",
    [string]$Arguments = "{}",
    [int]$Port = 8787,
    [int]$HealthTimeoutSec = 6,
    [int]$HealthPollMs = 250,
    [switch]$EnsureMainActivity,
    [switch]$OpenAppOnFailure,
    [switch]$NoBootstrap
)

$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $Adb)) {
    throw "adb not found: $Adb"
}

function Invoke-Adb {
    $adbArgs = @($args)
    $serialArgs = @()
    if ($DeviceSerial.Trim()) {
        $serialArgs = @("-s", $DeviceSerial.Trim())
    }
    & $Adb @serialArgs @AdbArgs
}

function Start-ApkMcpDebug {
    $serviceOutput = Invoke-Adb shell am start-foreground-service `
        -a com.elon.app.mcp.START_KEEPALIVE `
        -n com.elon.app/.mcp.McpDebugKeepAliveService 2>&1
    $serviceText = ($serviceOutput | Out-String).Trim()
    if ($LASTEXITCODE -eq 0 -and $serviceText -notmatch "Error:") {
        return
    }

    Invoke-Adb shell am broadcast --receiver-foreground `
        -a com.elon.app.mcp.START_DEBUG `
        -n com.elon.app/.mcp.McpDebugControlReceiver | Out-Null
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
        Invoke-Adb shell am start -n com.elon.app/.MainActivity | Out-Null
        Start-Sleep -Milliseconds 600
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

function Start-ApkMainActivity {
    Invoke-Adb shell am start -n com.elon.app/.MainActivity | Out-Null
    Start-Sleep -Milliseconds 700
}

if (!$NoBootstrap) {
    Start-ApkMcpDebug
}

Invoke-Adb forward "tcp:$Port" "tcp:$Port" | Out-Null

$health = Wait-ApkMcpHealth
$token = [string]$health.auth_token
if (!$token) {
    throw "MCP health endpoint did not return auth_token."
}

if ($EnsureMainActivity) {
    Start-ApkMainActivity
}

$argumentObject = $Arguments | ConvertFrom-Json
if ($null -eq $argumentObject) {
    $argumentObject = [pscustomobject]@{}
}
$argumentObject | Add-Member -NotePropertyName auth_token -NotePropertyValue $token -Force

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
    -TimeoutSec 120
