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
    [int]$FailureTailLines = 30,

    [ValidateRange(0, 86400)]
    [int]$TimeoutSeconds = 0,

    [ValidateRange(0, 86400)]
    [int]$StallTimeoutSeconds = 0,

    [switch]$RequireOutput
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

function Get-LogByteCount {
    param([string[]]$Paths)

    [long]$total = 0
    foreach ($path in $Paths) {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $total += (Get-Item -LiteralPath $path).Length
        }
    }
    return $total
}

function Stop-CommandProcessTree {
    param([int]$RootPid)

    if ($RootPid -le 0) { return }
    $processes = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue)
    $frontier = @($RootPid)
    $descendants = [System.Collections.Generic.List[int]]::new()
    while ($frontier.Count -gt 0) {
        $children = @(
            $processes |
                Where-Object { $_.ParentProcessId -in $frontier } |
                ForEach-Object { [int]$_.ProcessId }
        )
        foreach ($child in $children) { $descendants.Add($child) }
        $frontier = $children
    }

    $killer = Start-Process -FilePath "taskkill.exe" `
        -ArgumentList @("/PID", "$RootPid", "/T", "/F") `
        -WindowStyle Hidden `
        -PassThru
    if (-not $killer.WaitForExit(15000)) {
        Stop-Process -Id $killer.Id -Force -ErrorAction SilentlyContinue
    }
    if (Get-Process -Id $RootPid -ErrorAction SilentlyContinue) {
        for ($index = $descendants.Count - 1; $index -ge 0; $index--) {
            Stop-Process -Id $descendants[$index] -Force -ErrorAction SilentlyContinue
        }
        Stop-Process -Id $RootPid -Force -ErrorAction SilentlyContinue
    }
}

$repoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Unable to locate the repository root."
}
$repoRoot = [System.IO.Path]::GetFullPath($repoRoot.Trim())
$gitCommonDir = (& git -C $repoRoot rev-parse --git-common-dir 2>$null)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($gitCommonDir)) {
    throw "Unable to locate the shared Git metadata directory."
}
$gitCommonDir = $gitCommonDir.Trim()
if (-not [System.IO.Path]::IsPathRooted($gitCommonDir)) {
    $gitCommonDir = Join-Path $repoRoot $gitCommonDir
}
$gitCommonDir = [System.IO.Path]::GetFullPath($gitCommonDir)
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

$logRoot = Join-Path $gitCommonDir "ai-command-logs"
New-Item -ItemType Directory -Path $logRoot -Force | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
$stdoutLog = Join-Path $logRoot "$LogName-$stamp.stdout.log"
$stderrLog = Join-Path $logRoot "$LogName-$stamp.stderr.log"
$pidFile = Join-Path $logRoot "$LogName-$stamp.job.pid"
$resultFile = Join-Path $logRoot "$LogName-$stamp.result.json"
$commandRoot = Join-Path $logRoot "command-files"
New-Item -ItemType Directory -Path $commandRoot -Force | Out-Null
$commandToken = "$stamp-$([Guid]::NewGuid().ToString('N'))"
$commandFile = Join-Path $commandRoot "$commandToken.cmd"
$commandExitFile = Join-Path $commandRoot "$commandToken.exit"
[System.IO.File]::WriteAllText(
    $commandFile,
    "@echo off`r`nchcp 65001>nul`r`n" +
        "call $CommandLine 1>`"$stdoutLog`" 2>`"$stderrLog`"`r`n" +
        "set `"ELON_COMMAND_EXIT=%errorlevel%`"`r`n" +
        ">`"$commandExitFile`" echo %ELON_COMMAND_EXIT%`r`n" +
        "exit /b %ELON_COMMAND_EXIT%`r`n",
    [System.Text.UTF8Encoding]::new($false)
)
$watch = [System.Diagnostics.Stopwatch]::StartNew()
$logPaths = @($stdoutLog, $stderrLog)
$lastLogBytes = [long]0
$lastLogActivityAt = [DateTimeOffset]::UtcNow

$process = Start-Process -FilePath $env:ComSpec `
    -ArgumentList @("/d", "/s", "/c", "`"`"$commandFile`"`"") `
    -WorkingDirectory $workingPath `
    -WindowStyle Hidden `
    -PassThru
[System.IO.File]::WriteAllText($pidFile, [string]$process.Id, [System.Text.Encoding]::ASCII)

try {
    $heartbeatInterval = $HeartbeatSeconds
    $heartbeatCount = 0
    $timedOut = $false
    $stalled = $false
    while (-not $process.HasExited) {
        $now = [DateTimeOffset]::UtcNow
        $currentLogBytes = Get-LogByteCount -Paths $logPaths
        if ($currentLogBytes -ne $lastLogBytes) {
            $lastLogBytes = $currentLogBytes
            $lastLogActivityAt = $now
        }
        $idleSeconds = [Math]::Max(0, ($now - $lastLogActivityAt).TotalSeconds)
        if ($StallTimeoutSeconds -gt 0 -and $idleSeconds -ge $StallTimeoutSeconds) {
            $stalled = $true
            break
        }

        $waitSeconds = $heartbeatInterval
        if ($TimeoutSeconds -gt 0) {
            $remaining = $TimeoutSeconds - $watch.Elapsed.TotalSeconds
            if ($remaining -le 0) {
                $timedOut = $true
                break
            }
            $waitSeconds = [Math]::Max(1, [Math]::Min($waitSeconds, [Math]::Ceiling($remaining)))
        }
        if ($StallTimeoutSeconds -gt 0) {
            $stallRemaining = $StallTimeoutSeconds - $idleSeconds
            $waitSeconds = [Math]::Max(1, [Math]::Min($waitSeconds, [Math]::Ceiling($stallRemaining)))
        }
        [void]$process.WaitForExit([int]($waitSeconds * 1000))
        $process.Refresh()
        if (-not $process.HasExited) {
            $now = [DateTimeOffset]::UtcNow
            $currentLogBytes = Get-LogByteCount -Paths $logPaths
            if ($currentLogBytes -ne $lastLogBytes) {
                $lastLogBytes = $currentLogBytes
                $lastLogActivityAt = $now
            }
            $idleSeconds = [Math]::Max(0, ($now - $lastLogActivityAt).TotalSeconds)
            if ($heartbeatCount -lt 10) {
                Write-Output "AI_COMMAND_PROGRESS=running name=$LogName elapsed_seconds=$([Math]::Round($watch.Elapsed.TotalSeconds, 1)) log_bytes=$lastLogBytes idle_seconds=$([Math]::Round($idleSeconds, 1))"
                $heartbeatCount++
            }
            $heartbeatInterval = [Math]::Min(60, $heartbeatInterval * 2)
        }
    }
    if ($timedOut -or $stalled) {
        Stop-CommandProcessTree -RootPid $process.Id
        [void]$process.WaitForExit(5000)
        $exitCode = if ($timedOut) { 124 } else { 125 }
    } else {
        $process.WaitForExit()
        $recordedExitCode = 0
        $hasRecordedExitCode = (Test-Path -LiteralPath $commandExitFile -PathType Leaf) -and
            [int]::TryParse(
                (Get-Content -LiteralPath $commandExitFile -Raw).Trim(),
                [ref]$recordedExitCode
            )
        $exitCode = if ($hasRecordedExitCode) { $recordedExitCode } else { [int]$process.ExitCode }
    }
} finally {
    $watch.Stop()
    if ($null -ne $process -and -not $process.HasExited) {
        Stop-CommandProcessTree -RootPid $process.Id
    }
    Remove-Item -LiteralPath $pidFile -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $commandFile, $commandExitFile -Force -ErrorAction SilentlyContinue
}

$stdoutLines = Get-LineCount -Path $stdoutLog
$stderrLines = Get-LineCount -Path $stderrLog
$warningCount = Get-MatchCount -Paths $logPaths -Pattern '(?i)\bwarning(?:s)?\b|警告'
$emptyOutputRejected = $RequireOutput -and $exitCode -eq 0 -and ($stdoutLines + $stderrLines) -eq 0
if ($emptyOutputRejected) { $exitCode = 125 }
$resultStatus = if ($exitCode -eq 0) { "passed" } else { "failed" }
$result = [ordered]@{
    schema = "elon.ai_command_result.v1"
    name = $LogName
    status = $resultStatus
    exit_code = $exitCode
    timed_out = $timedOut
    stalled = $stalled
    empty_output_rejected = $emptyOutputRejected
    duration_seconds = [Math]::Round($watch.Elapsed.TotalSeconds, 1)
    output_lines = $stdoutLines + $stderrLines
    stdout_log = $stdoutLog
    stderr_log = $stderrLog
    completed_at = [DateTimeOffset]::UtcNow.ToString("o")
}
[System.IO.File]::WriteAllText(
    $resultFile,
    ($result | ConvertTo-Json -Depth 4 -Compress),
    [System.Text.UTF8Encoding]::new($false)
)

Write-Output "AI_COMMAND_STATUS=$resultStatus"
Write-Output "AI_COMMAND_NAME=$LogName"
Write-Output "AI_COMMAND_EXIT_CODE=$exitCode"
Write-Output "AI_COMMAND_TIMED_OUT=$($timedOut.ToString().ToLowerInvariant())"
Write-Output "AI_COMMAND_STALLED=$($stalled.ToString().ToLowerInvariant())"
Write-Output "AI_COMMAND_EMPTY_OUTPUT_REJECTED=$($emptyOutputRejected.ToString().ToLowerInvariant())"
Write-Output "AI_COMMAND_DURATION_SECONDS=$([Math]::Round($watch.Elapsed.TotalSeconds, 1))"
Write-Output "AI_COMMAND_OUTPUT_LINES=$($stdoutLines + $stderrLines)"
Write-Output "AI_COMMAND_WARNING_LINES=$warningCount"
Write-Output "AI_COMMAND_STDOUT_LOG=$stdoutLog"
Write-Output "AI_COMMAND_STDERR_LOG=$stderrLog"
Write-Output "AI_COMMAND_RESULT_FILE=$resultFile"

if ($exitCode -ne 0) {
    $failureReason = if ($timedOut) {
        "timeout"
    } elseif ($stalled) {
        "stalled_output"
    } elseif ($emptyOutputRejected) {
        "empty_output"
    } else {
        "command_exit"
    }
    Write-Output "AI_COMMAND_FAILURE_REASON=$failureReason"
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
