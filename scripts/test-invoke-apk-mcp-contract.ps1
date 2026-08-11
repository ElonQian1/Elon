$ErrorActionPreference = "Stop"

$sourcePath = Join-Path $PSScriptRoot "invoke-apk-mcp.ps1"
$source = Get-Content -LiteralPath $sourcePath -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $sourcePath,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) {
    throw "invoke-apk-mcp.ps1 has a PowerShell parse error: $($errors[0].Message)"
}

$activityIndex = $source.IndexOf('if ($EnsureMainActivity)')
$forwardIndex = $source.IndexOf('Invoke-Adb forward "tcp:$Port" "tcp:$Port"')
$healthIndex = $source.IndexOf('$health = Wait-ApkMcpHealth')
$requestIndex = $source.IndexOf('$request = [ordered]@{')
if ($activityIndex -lt 0 -or $forwardIndex -lt 0 -or $healthIndex -lt 0 -or $requestIndex -lt 0) {
    throw "APK MCP lifecycle contract is missing a required stage."
}
if (-not ($activityIndex -lt $forwardIndex -and $forwardIndex -lt $healthIndex -and $healthIndex -lt $requestIndex)) {
    throw "APK MCP must start the requested Activity before forwarding, health verification, and the MCP request."
}
if ($source.IndexOf('if ($EnsureMainActivity)', $activityIndex + 1) -ge 0) {
    throw "APK MCP must not start the Activity again after health verification."
}
foreach ($token in @(
    'native-command-timeout.ps1',
    'AdbTimeoutSec = 10',
    'Invoke-ElonNativeCommand',
    'Assert-ElonNativeCommand',
    'Unable to create APK MCP adb forward',
    'function Get-ApkMcpHealthIfAvailable',
    'if ($NoBootstrap -and -not $EnsureMainActivity)'
)) {
    if (-not $source.Contains($token)) {
        throw "APK MCP adb timeout contract is missing token: $token"
    }
}

Write-Output "APK_MCP_LIFECYCLE_CONTRACT=passed"
