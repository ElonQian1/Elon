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
$fixtureRoot = Join-Path $repoRoot ".ai-tmp\command-output-test"
New-Item -ItemType Directory -Path $fixtureRoot -Force | Out-Null
$successFixture = Join-Path $fixtureRoot "success.cmd"
$failureFixture = Join-Path $fixtureRoot "failure.cmd"

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
Assert-True ($successOutput.Count -le 12) "Successful summary exceeded its line budget."

Assert-True ($failureExit -eq 7) "Bounded failure command must preserve the native exit code."
Assert-True ($failureText.Contains("AI_COMMAND_STATUS=failed")) "Failure summary is missing."
Assert-True ($failureText.Contains("error: deterministic failure")) "Failure error excerpt is missing."
Assert-True ($failureOutput.Count -le 80) "Failure excerpt exceeded its line budget."

$stdoutLogLine = $successOutput |
    Where-Object { "$_".StartsWith("AI_COMMAND_STDOUT_LOG=") } |
    Select-Object -Last 1
$stdoutLog = "$stdoutLogLine".Substring("AI_COMMAND_STDOUT_LOG=".Length)
Assert-True (Test-Path -LiteralPath $stdoutLog -PathType Leaf) "Full success log was not retained."
Assert-True (@(Get-Content -LiteralPath $stdoutLog).Count -eq 300) "Retained success log is incomplete."

Write-Host "PASS bounded AI command output"
