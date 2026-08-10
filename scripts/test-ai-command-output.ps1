param()

$ErrorActionPreference = "Stop"

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )
    if (-not $Condition) { throw $Message }
}

$repoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Run this test inside the repository."
}
$runner = Join-Path $repoRoot "scripts\invoke-ai-logged-command.ps1"
$statusReader = Join-Path $repoRoot "scripts\get-ai-command-status.ps1"
$gitCommonDir = (& git -C $repoRoot rev-parse --git-common-dir 2>$null).Trim()
if (-not [System.IO.Path]::IsPathRooted($gitCommonDir)) {
    $gitCommonDir = Join-Path $repoRoot $gitCommonDir
}
$commandLogRoot = Join-Path ([System.IO.Path]::GetFullPath($gitCommonDir)) "ai-command-logs"
$fixtureRoot = Join-Path $repoRoot ".ai-tmp\command-output-test"
New-Item -ItemType Directory -Path $fixtureRoot -Force | Out-Null
$successFixture = Join-Path $fixtureRoot "success.cmd"
$failureFixture = Join-Path $fixtureRoot "failure.cmd"
$timeoutFixture = Join-Path $fixtureRoot "timeout.cmd"
$silentFixture = Join-Path $fixtureRoot "silent.cmd"

Set-Content -LiteralPath $successFixture -Encoding ASCII -Value @(
    "@echo off",
    "for /L %%i in (1,1,300) do echo warning: noisy success line %%i",
    "exit /b 0"
)
Set-Content -LiteralPath $failureFixture -Encoding ASCII -Value @(
    "@echo off",
    "echo error: deterministic failure 1>&2",
    "for /L %%i in (1,1,100) do echo failure tail line %%i",
    "exit /b 7"
)
Set-Content -LiteralPath $timeoutFixture -Encoding ASCII -Value @(
    "@echo off",
    "powershell -NoProfile -Command `"Start-Sleep -Seconds 30`"",
    "exit /b 0"
)
Set-Content -LiteralPath $silentFixture -Encoding ASCII -Value @(
    "@echo off",
    "exit /b 0"
)

$oldPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
    $successOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $runner `
        -LogName "bounded-success" `
        -WorkingDirectory $repoRoot `
        -CommandLine "`"$successFixture`"" 2>&1)
    $successExit = $LASTEXITCODE

    $failureOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $runner `
        -LogName "bounded-failure" `
        -WorkingDirectory $repoRoot `
        -CommandLine "`"$failureFixture`"" 2>&1)
    $failureExit = $LASTEXITCODE

    $timeoutWatch = [System.Diagnostics.Stopwatch]::StartNew()
    $timeoutOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $runner `
        -LogName "bounded-timeout" `
        -WorkingDirectory $repoRoot `
        -TimeoutSeconds 2 `
        -CommandLine "`"$timeoutFixture`"" 2>&1)
    $timeoutExit = $LASTEXITCODE
    $timeoutWatch.Stop()

    $silentOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $runner `
        -LogName "bounded-silent" `
        -WorkingDirectory $repoRoot `
        -RequireOutput `
        -CommandLine "`"$silentFixture`"" 2>&1)
    $silentExit = $LASTEXITCODE

    $stallWatch = [System.Diagnostics.Stopwatch]::StartNew()
    $stallOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $runner `
        -LogName "bounded-stall" `
        -WorkingDirectory $repoRoot `
        -StallTimeoutSeconds 2 `
        -CommandLine "`"$timeoutFixture`"" 2>&1)
    $stallExit = $LASTEXITCODE
    $stallWatch.Stop()
} finally {
    $ErrorActionPreference = $oldPreference
}

$successText = $successOutput -join "`n"
$failureText = $failureOutput -join "`n"
Assert-True ($successExit -eq 0) "Bounded success command must preserve exit code 0."
Assert-True ($successText.Contains("AI_COMMAND_STATUS=passed")) "Success summary is missing."
Assert-True ($successText.Contains("AI_COMMAND_OUTPUT_LINES=300")) "Success output line count is wrong."
Assert-True ($successText.Contains("AI_COMMAND_WARNING_LINES=300")) "Warning count is wrong."
Assert-True (-not $successText.Contains("noisy success line")) "Successful raw output leaked into AI context."
Assert-True ($successOutput.Count -le 15) "Successful summary exceeded its line budget."

Assert-True ($failureExit -eq 7) "Bounded failure command must preserve the native exit code."
Assert-True ($failureText.Contains("AI_COMMAND_STATUS=failed")) "Failure summary is missing."
Assert-True ($failureText.Contains("error: deterministic failure")) "Failure error excerpt is missing."
Assert-True ($failureOutput.Count -le 80) "Failure excerpt exceeded its line budget."

$timeoutText = $timeoutOutput -join "`n"
Assert-True ($timeoutExit -eq 124) "Timed out command must return exit code 124."
Assert-True ($timeoutText.Contains("AI_COMMAND_TIMED_OUT=true")) "Timeout summary is missing."
Assert-True ($timeoutWatch.Elapsed.TotalSeconds -lt 10) "Timed out command left its process tree running."

$silentText = $silentOutput -join "`n"
Assert-True ($silentExit -eq 125) "Required-output command with empty logs must return exit code 125."
Assert-True ($silentText.Contains("AI_COMMAND_EMPTY_OUTPUT_REJECTED=true")) "Empty-output rejection summary is missing."
Assert-True ($silentText.Contains("AI_COMMAND_FAILURE_REASON=empty_output")) "Empty-output failure reason is missing."

$stallText = $stallOutput -join "`n"
Assert-True ($stallExit -eq 125) "Stalled command must return exit code 125."
Assert-True ($stallText.Contains("AI_COMMAND_STALLED=true")) "Stall summary is missing."
Assert-True ($stallText.Contains("AI_COMMAND_FAILURE_REASON=stalled_output")) "Stall failure reason is missing."
Assert-True ($stallWatch.Elapsed.TotalSeconds -lt 10) "Stalled command left its process tree running."

$recoveryName = "bounded-recovery-$([Guid]::NewGuid().ToString('N'))"
$recoveryBase = Join-Path $commandLogRoot "$recoveryName-20000101-000000-000"
try {
    Set-Content -LiteralPath "$recoveryBase.stdout.log" -Encoding ASCII -Value "running"
    Set-Content -LiteralPath "$recoveryBase.stderr.log" -Encoding ASCII -Value ""
    Set-Content -LiteralPath "$recoveryBase.job.pid" -Encoding ASCII -Value "$PID"
    $runningRecovery = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $statusReader `
        -LogName $recoveryName 2>&1) -join "`n"
    Assert-True ($runningRecovery.Contains("AI_COMMAND_RECOVERY_STATUS=running")) "Running recovery status is wrong."

    Remove-Item -LiteralPath "$recoveryBase.job.pid" -Force
    Set-Content -LiteralPath "$recoveryBase.result.json" -Encoding UTF8 -Value `
        '{"schema":"elon.ai_command_result.v1","status":"passed","exit_code":0}'
    $completedRecovery = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $statusReader `
        -LogName $recoveryName 2>&1) -join "`n"
    Assert-True ($completedRecovery.Contains("AI_COMMAND_RECOVERY_STATUS=completed")) "Completed recovery status is wrong."
    Assert-True ($completedRecovery.Contains("AI_COMMAND_RESULT_STATUS=passed")) "Completed result status is missing."
    Assert-True ($completedRecovery.Contains("AI_COMMAND_RESULT_EXIT_CODE=0")) "Completed result exit code is missing."

    Remove-Item -LiteralPath "$recoveryBase.result.json" -Force
    Set-Content -LiteralPath "$recoveryBase.job.pid" -Encoding ASCII -Value "2147483647"
    $staleRecovery = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $statusReader `
        -LogName $recoveryName 2>&1) -join "`n"
    Assert-True ($LASTEXITCODE -eq 2) "Stale recovery status must return exit code 2."
    Assert-True ($staleRecovery.Contains("AI_COMMAND_RECOVERY_STATUS=stale")) "Stale recovery status is wrong."
} finally {
    Remove-Item -LiteralPath "$recoveryBase.stdout.log", "$recoveryBase.stderr.log", `
        "$recoveryBase.job.pid", "$recoveryBase.result.json" `
        -Force -ErrorAction SilentlyContinue
}

$stdoutLogLine = $successOutput |
    Where-Object { "$_".StartsWith("AI_COMMAND_STDOUT_LOG=") } |
    Select-Object -Last 1
$stdoutLog = "$stdoutLogLine".Substring("AI_COMMAND_STDOUT_LOG=".Length)
Assert-True ($stdoutLog.StartsWith($commandLogRoot, [StringComparison]::OrdinalIgnoreCase)) `
    "Command logs must live outside a removable task worktree."
Assert-True (Test-Path -LiteralPath $stdoutLog -PathType Leaf) "Full success log was not retained."
Assert-True (@(Get-Content -LiteralPath $stdoutLog).Count -eq 300) "Retained success log is incomplete."

$resultFileLine = $successOutput |
    Where-Object { "$_".StartsWith("AI_COMMAND_RESULT_FILE=") } |
    Select-Object -Last 1
$resultFile = "$resultFileLine".Substring("AI_COMMAND_RESULT_FILE=".Length)
$savedResult = Get-Content -LiteralPath $resultFile -Raw | ConvertFrom-Json
Assert-True ($savedResult.status -eq "passed") "Persisted success result status is wrong."
Assert-True ($savedResult.exit_code -eq 0) "Persisted success result exit code is wrong."

Write-Host "PASS bounded AI command output"
