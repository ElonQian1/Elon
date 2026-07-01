param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [string]$Tool = "phone_status",
    [string]$Arguments = "{}",
    [int]$Port = 8787,
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

if (!$NoBootstrap) {
    Invoke-Adb shell am broadcast `
        -a com.elon.app.mcp.START_DEBUG `
        -n com.elon.app/.mcp.McpDebugControlReceiver | Out-Null
}

Invoke-Adb forward "tcp:$Port" "tcp:$Port" | Out-Null

$health = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 10
$token = [string]$health.auth_token
if (!$token) {
    throw "MCP health endpoint did not return auth_token."
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
