[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Za-z0-9._-]+$')]
    [string]$LogName
)

$ErrorActionPreference = "Stop"
$repoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Unable to locate the repository root."
}
$logRoot = Join-Path $repoRoot.Trim() ".ai-tmp\command-logs"
$latest = Get-ChildItem -LiteralPath $logRoot -File -Filter "$LogName-*" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1

if (-not $latest) {
    Write-Output "AI_COMMAND_RECOVERY_STATUS=missing"
    Write-Output "AI_COMMAND_NAME=$LogName"
    exit 3
}

$basePath = $latest.FullName -replace '(?:\.stdout\.log|\.stderr\.log|\.job\.pid|\.result\.json)$', ''
$stdoutLog = "$basePath.stdout.log"
$stderrLog = "$basePath.stderr.log"
$pidFile = "$basePath.job.pid"
$resultFile = "$basePath.result.json"
$pidValue = 0
if (Test-Path -LiteralPath $pidFile -PathType Leaf) {
    [void][int]::TryParse((Get-Content -LiteralPath $pidFile -Raw).Trim(), [ref]$pidValue)
}
$process = if ($pidValue -gt 0) { Get-Process -Id $pidValue -ErrorAction SilentlyContinue } else { $null }
$status = if ($process) {
    "running"
} elseif (Test-Path -LiteralPath $pidFile -PathType Leaf) {
    "stale"
} else {
    "completed"
}
$logFiles = @($stdoutLog, $stderrLog) |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
    ForEach-Object { Get-Item -LiteralPath $_ }
$lastActivity = $logFiles |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1
$idleSeconds = if ($lastActivity) {
    [Math]::Max(0, ([DateTimeOffset]::UtcNow - $lastActivity.LastWriteTimeUtc).TotalSeconds)
} else {
    0
}
$stdoutBytes = if (Test-Path -LiteralPath $stdoutLog -PathType Leaf) {
    (Get-Item -LiteralPath $stdoutLog).Length
} else { 0 }
$stderrBytes = if (Test-Path -LiteralPath $stderrLog -PathType Leaf) {
    (Get-Item -LiteralPath $stderrLog).Length
} else { 0 }
$result = if (Test-Path -LiteralPath $resultFile -PathType Leaf) {
    Get-Content -LiteralPath $resultFile -Raw | ConvertFrom-Json
} else { $null }
$resultStatus = if ($result) { [string]$result.status } else { "unknown" }
$resultExitCode = if ($result) { [int]$result.exit_code } else { -1 }

Write-Output "AI_COMMAND_RECOVERY_STATUS=$status"
Write-Output "AI_COMMAND_NAME=$LogName"
Write-Output "AI_COMMAND_PID=$pidValue"
Write-Output "AI_COMMAND_STDOUT_BYTES=$stdoutBytes"
Write-Output "AI_COMMAND_STDERR_BYTES=$stderrBytes"
Write-Output "AI_COMMAND_IDLE_SECONDS=$([Math]::Round($idleSeconds, 1))"
Write-Output "AI_COMMAND_LOG_BASE=$basePath"
Write-Output "AI_COMMAND_RESULT_STATUS=$resultStatus"
Write-Output "AI_COMMAND_RESULT_EXIT_CODE=$resultExitCode"

if ($status -eq "stale") { exit 2 }
exit 0
