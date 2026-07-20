$ErrorActionPreference = "Stop"

$RepoRoot = (git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($RepoRoot)) {
    throw "Unable to resolve the repository root."
}

$TempBase = (Get-Item -LiteralPath $env:TEMP).FullName
$TempRoot = Join-Path $TempBase ("elon-cargo-dev-exit-test-{0}" -f [Guid]::NewGuid().ToString("N"))
$FakeCargoBin = Join-Path $TempRoot "bin"
$CacheRoot = Join-Path $TempRoot "cache"
$TargetDir = Join-Path $TempRoot "target"
$ProbeScript = Join-Path $TempRoot "nested-probe.ps1"
$StdoutLog = Join-Path $TempRoot "stdout.log"
$StderrLog = Join-Path $TempRoot "stderr.log"
New-Item -ItemType Directory -Force -Path $FakeCargoBin, $CacheRoot, $TargetDir | Out-Null

try {
    @"
@echo off
exit /b 37
"@ | Set-Content -LiteralPath (Join-Path $FakeCargoBin "cargo.cmd") -Encoding ASCII

    $escapedFakeCargoBin = $FakeCargoBin.Replace("'", "''")
    $escapedCargoDev = (Join-Path $RepoRoot "scripts\cargo-dev.ps1").Replace("'", "''")
    $escapedCacheRoot = $CacheRoot.Replace("'", "''")
    $escapedTargetDir = $TargetDir.Replace("'", "''")
    @(
        '$ErrorActionPreference = "Stop"'
        ("`$env:PATH = '{0};' + `$env:PATH" -f $escapedFakeCargoBin)
        ("& '{0}' -BypassValidationOrchestrator -NoLock -SkipCacheGc -DisableSccache -CacheRoot '{1}' -TargetDir '{2}' -- check" -f $escapedCargoDev, $escapedCacheRoot, $escapedTargetDir)
    ) | Set-Content -LiteralPath $ProbeScript -Encoding UTF8

    $PowerShellExe = (Get-Process -Id $PID).Path
    $ProbeProcess = Start-Process -FilePath $PowerShellExe -ArgumentList @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $ProbeScript
    ) -Wait -PassThru -NoNewWindow -RedirectStandardOutput $StdoutLog -RedirectStandardError $StderrLog

    if ($ProbeProcess.ExitCode -ne 37) {
        $stdout = if (Test-Path -LiteralPath $StdoutLog) { Get-Content -LiteralPath $StdoutLog -Raw } else { "" }
        $stderr = if (Test-Path -LiteralPath $StderrLog) { Get-Content -LiteralPath $StderrLog -Raw } else { "" }
        throw "Nested cargo-dev exit code mismatch: expected=37 actual=$($ProbeProcess.ExitCode)`nstdout=$stdout`nstderr=$stderr"
    }

    Write-Host "PASS: nested cargo-dev propagated Cargo exit code 37." -ForegroundColor Green
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
