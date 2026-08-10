[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Za-z0-9._-]+$')]
    [string]$LogName
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot 'git-path-resolution.ps1')
$repositoryPaths = Get-ElonRepositoryPathsFromScriptRoot -ScriptRoot $PSScriptRoot
$repoRoot = $repositoryPaths.RepoRoot
$gitCommonDir = $repositoryPaths.GitCommonDir
$logRoot = Join-Path $gitCommonDir "ai-command-logs"
$latest = Get-ChildItem -LiteralPath $logRoot -File -Filter "$LogName-*" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1

if (-not $latest) {
    Write-Output "AI_COMMAND_RECOVERY_STATUS=missing"
    Write-Output "AI_COMMAND_NAME=$LogName"
    exit 3
}

$basePath = $latest.FullName -replace '(?:\.stdout\.log|\.stderr\.log|\.job\.pid|\.state\.json|\.result\.json)$', ''
$stdoutLog = "$basePath.stdout.log"
$stderrLog = "$basePath.stderr.log"
$pidFile = "$basePath.job.pid"
$stateFile = "$basePath.state.json"
$resultFile = "$basePath.result.json"
$pidValue = 0
if (Test-Path -LiteralPath $pidFile -PathType Leaf) {
    [void][int]::TryParse((Get-Content -LiteralPath $pidFile -Raw).Trim(), [ref]$pidValue)
}
$process = if ($pidValue -gt 0) { Get-Process -Id $pidValue -ErrorAction SilentlyContinue } else { $null }
$result = if (Test-Path -LiteralPath $resultFile -PathType Leaf) {
    Read-ElonUtf8TextFile -Path $resultFile | ConvertFrom-Json
} else { $null }
$state = if (Test-Path -LiteralPath $stateFile -PathType Leaf) {
    try { Read-ElonUtf8TextFile -Path $stateFile | ConvertFrom-Json } catch { $null }
} else { $null }
$status = if ($result) {
    "completed"
} elseif ($process) {
    "running"
} elseif ((Test-Path -LiteralPath $pidFile -PathType Leaf) -or
    ($state -and [string]$state.status -eq "running")) {
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
$resultStatus = if ($result) { [string]$result.status } else { "unknown" }
$resultExitCode = if ($result) { [int]$result.exit_code } else { -1 }
$stateStatus = if ($state) { [string]$state.status } else { "unknown" }
$stateElapsedSeconds = if ($state) { [double]$state.elapsed_seconds } else { 0 }
$stateLastProgressAt = if ($state) { [string]$state.last_progress_at } else { "unknown" }
$stateTimeoutSeconds = if ($state) { [int]$state.timeout_seconds } else { 0 }
$stateStallTimeoutSeconds = if ($state) { [int]$state.stall_timeout_seconds } else { 0 }

Write-Output "AI_COMMAND_RECOVERY_STATUS=$status"
Write-Output "AI_COMMAND_NAME=$LogName"
Write-Output "AI_COMMAND_PID=$pidValue"
Write-Output "AI_COMMAND_STDOUT_BYTES=$stdoutBytes"
Write-Output "AI_COMMAND_STDERR_BYTES=$stderrBytes"
Write-Output "AI_COMMAND_IDLE_SECONDS=$([Math]::Round($idleSeconds, 1))"
Write-Output "AI_COMMAND_LOG_BASE=$basePath"
Write-Output "AI_COMMAND_STATE_STATUS=$stateStatus"
Write-Output "AI_COMMAND_STATE_ELAPSED_SECONDS=$([Math]::Round($stateElapsedSeconds, 1))"
Write-Output "AI_COMMAND_LAST_PROGRESS_AT=$stateLastProgressAt"
Write-Output "AI_COMMAND_TIMEOUT_SECONDS=$stateTimeoutSeconds"
Write-Output "AI_COMMAND_STALL_TIMEOUT_SECONDS=$stateStallTimeoutSeconds"
Write-Output "AI_COMMAND_RESULT_STATUS=$resultStatus"
Write-Output "AI_COMMAND_RESULT_EXIT_CODE=$resultExitCode"

if ($status -eq "stale") { exit 2 }
exit 0
