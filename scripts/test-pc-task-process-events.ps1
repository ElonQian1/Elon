param()

$ErrorActionPreference = "Stop"

$RepoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
$AuditScript = Join-Path $RepoRoot "scripts\check-pc-task-process-events.ps1"

function Invoke-Step {
    param(
        [string]$Label,
        [scriptblock]$Body
    )

    Write-Host ""
    Write-Host "== $Label ==" -ForegroundColor Cyan
    & $Body
}

Invoke-Step "Offline process-event audit selftest" {
    powershell -ExecutionPolicy Bypass -File $AuditScript -SelfTest
    if ($LASTEXITCODE -ne 0) {
        throw "check-pc-task-process-events.ps1 -SelfTest failed with exit code $LASTEXITCODE"
    }
}

Invoke-Step "TaskId guard regression" {
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = powershell -ExecutionPolicy Bypass -File $AuditScript 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($exitCode -eq 0) {
        throw "check-pc-task-process-events.ps1 without -TaskId unexpectedly succeeded"
    }
    if (-not (($output | Out-String) -match "-TaskId is required unless -SelfTest is specified")) {
        throw "check-pc-task-process-events.ps1 without -TaskId did not report the expected guard message"
    }
    Write-Host "TaskId guard regression passed."
}

Write-Host ""
Write-Host "PC task process-event regression tests passed." -ForegroundColor Green
