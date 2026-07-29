[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Za-z0-9._-]+$')]
    [string]$LogName,

    [Parameter(Mandatory = $true)]
    [string]$CommandLine,

    [string]$WorkingDirectory = ".",

    [ValidateRange(5, 300)]
    [int]$HeartbeatSeconds = 30,

    [ValidateRange(1, 30)]
    [int]$MaxErrorLines = 30,

    [ValidateRange(1, 30)]
    [int]$FailureTailLines = 30
)

$ErrorActionPreference = "Stop"

function Get-LineCount {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return 0 }
    $reader = [System.IO.File]::OpenText($Path)
    $count = 0
    try {
        while ($null -ne $reader.ReadLine()) { $count++ }
    } finally {
        $reader.Dispose()
    }
    return $count
}

function Get-MatchCount {
    param(
        [string[]]$Paths,
        [string]$Pattern
    )
    $count = 0
    foreach ($path in $Paths) {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $count += @(Select-String -LiteralPath $path -Pattern $Pattern).Count
        }
    }
    return $count
}

$repoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Unable to locate the repository root."
}
$repoRoot = [System.IO.Path]::GetFullPath($repoRoot.Trim())
$workingPath = if ([System.IO.Path]::IsPathRooted($WorkingDirectory)) {
    [System.IO.Path]::GetFullPath($WorkingDirectory)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $repoRoot $WorkingDirectory))
}
$repoPrefix = $repoRoot.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
if (-not $workingPath.StartsWith($repoPrefix, [StringComparison]::OrdinalIgnoreCase) -and
    -not $workingPath.Equals($repoRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "WorkingDirectory must stay inside the repository: $workingPath"
}
if (-not (Test-Path -LiteralPath $workingPath -PathType Container)) {
    throw "WorkingDirectory does not exist: $workingPath"
}

$logRoot = Join-Path $repoRoot ".ai-tmp\command-logs"
New-Item -ItemType Directory -Path $logRoot -Force | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
$stdoutLog = Join-Path $logRoot "$LogName-$stamp.stdout.log"
$stderrLog = Join-Path $logRoot "$LogName-$stamp.stderr.log"
$watch = [System.Diagnostics.Stopwatch]::StartNew()

$job = Start-Job -ArgumentList @(
    $workingPath,
    $CommandLine,
    $stdoutLog,
    $stderrLog
) -ScriptBlock {
    param($CommandWorkingPath, $CommandText, $StdoutPath, $StderrPath)
    Set-Location -LiteralPath $CommandWorkingPath
    $redirectedCommand = "chcp 65001>nul & $CommandText 1>`"$StdoutPath`" 2>`"$StderrPath`""
    & $env:ComSpec /d /s /c $redirectedCommand
    [pscustomobject]@{
        ExitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
    }
}

try {
    $heartbeatInterval = $HeartbeatSeconds
    $heartbeatCount = 0
    while ($job.State -in @("NotStarted", "Running")) {
        Wait-Job -Job $job -Timeout $heartbeatInterval | Out-Null
        if ($job.State -in @("NotStarted", "Running")) {
            if ($heartbeatCount -lt 10) {
                Write-Output "AI_COMMAND_PROGRESS=running name=$LogName elapsed_seconds=$([Math]::Round($watch.Elapsed.TotalSeconds, 1))"
                $heartbeatCount++
            }
            $heartbeatInterval = [Math]::Min(300, $heartbeatInterval * 2)
        }
    }
    $jobResult = @(Receive-Job -Job $job)
    $resultRecord = $jobResult |
        Where-Object { $_.PSObject.Properties.Name -contains "ExitCode" } |
        Select-Object -Last 1
    $exitCode = if ($resultRecord) { [int]$resultRecord.ExitCode } else { 1 }
    if ($job.State -eq "Failed") { $exitCode = 1 }
} finally {
    $watch.Stop()
    Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
}

$logPaths = @($stdoutLog, $stderrLog)
$stdoutLines = Get-LineCount -Path $stdoutLog
$stderrLines = Get-LineCount -Path $stderrLog
$warningCount = Get-MatchCount -Paths $logPaths -Pattern '(?i)\bwarning(?:s)?\b|警告'

Write-Output "AI_COMMAND_STATUS=$(if ($exitCode -eq 0) { 'passed' } else { 'failed' })"
Write-Output "AI_COMMAND_NAME=$LogName"
Write-Output "AI_COMMAND_EXIT_CODE=$exitCode"
Write-Output "AI_COMMAND_DURATION_SECONDS=$([Math]::Round($watch.Elapsed.TotalSeconds, 1))"
Write-Output "AI_COMMAND_OUTPUT_LINES=$($stdoutLines + $stderrLines)"
Write-Output "AI_COMMAND_WARNING_LINES=$warningCount"
Write-Output "AI_COMMAND_STDOUT_LOG=$stdoutLog"
Write-Output "AI_COMMAND_STDERR_LOG=$stderrLog"

if ($exitCode -ne 0) {
    Write-Output "AI_COMMAND_FAILURE_ERRORS_BEGIN"
    $errorPattern = '(?i)\berror\b|\bfailed\b|\bfailure\b|\bexception\b|\bpanic\b|caused by|错误|失败|异常'
    $errorLines = foreach ($path in $logPaths) {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            Select-String -LiteralPath $path -Pattern $errorPattern |
                ForEach-Object { $_.Line }
        }
    }
    @($errorLines | Select-Object -First $MaxErrorLines) |
        ForEach-Object { Write-Output $_ }
    Write-Output "AI_COMMAND_FAILURE_ERRORS_END"
    Write-Output "AI_COMMAND_FAILURE_TAIL_BEGIN"
    $tailPerFile = [Math]::Max(1, [Math]::Floor($FailureTailLines / 2))
    foreach ($path in $logPaths) {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            Get-Content -LiteralPath $path -Tail $tailPerFile |
                ForEach-Object { Write-Output $_ }
        }
    }
    Write-Output "AI_COMMAND_FAILURE_TAIL_END"
    exit $exitCode
}

exit 0
