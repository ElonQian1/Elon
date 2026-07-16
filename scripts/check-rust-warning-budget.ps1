<#
.SYNOPSIS
    Keep the Rust warning count from growing.

.DESCRIPTION
    Keep Rust warning output clean. CI should run this with a zero budget so
    new warnings fail fast instead of turning into background noise.
#>
param(
    [int]$MaxWarnings = 0,
    [string]$ManifestPath = "server\Cargo.toml",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Remove-AnsiEscape {
    param([AllowNull()][object]$Value)
    if ($null -eq $Value) { return "" }
    if ($Value -is [System.Management.Automation.ErrorRecord]) {
        $Value = $Value.Exception.Message
    }
    return ([string]$Value) -replace "`e\[[0-9;?]*[ -/]*[@-~]", ""
}

function Count-RustWarningLines {
    param([AllowNull()][object[]]$Lines)
    if ($null -eq $Lines) { return 0 }

    $count = 0
    foreach ($line in $Lines) {
        if ((Remove-AnsiEscape $line) -match "^\s*warning:") {
            $count += 1
        }
    }
    return $count
}

function Stop-Guard {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Invoke-SelfTest {
    $sample = @(
        "warning: unused import: ``foo``",
        "  --> src\main.rs:1:5",
        "`e[33mwarning:`e[0m dead code",
        "error: not a warning",
        "warning: ``elon-server`` generated 2 warnings"
    )
    $count = Count-RustWarningLines $sample
    if ($count -ne 3) {
        Stop-Guard "SelfTest failed: expected 3 warnings, counted $count."
    }
    Write-Host "RUST_WARNING_BUDGET_SELFTEST=passed"
}

if ($SelfTest) {
    Invoke-SelfTest
    exit 0
}

$repoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null)
if ([string]::IsNullOrWhiteSpace($repoRoot)) {
    $repoRoot = (& git rev-parse --show-toplevel 2>$null)
}
if ([string]::IsNullOrWhiteSpace($repoRoot)) {
    Stop-Guard "Current directory is not inside a git repository."
}
$repoRoot = $repoRoot.Trim()
$cargoDev = Join-Path $repoRoot "scripts\cargo-dev.ps1"
if (-not (Test-Path -LiteralPath $cargoDev)) {
    Stop-Guard "Cannot find Cargo wrapper: $cargoDev"
}

Push-Location $repoRoot
try {
    $output = New-Object System.Collections.Generic.List[string]
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & powershell -NoProfile -ExecutionPolicy Bypass -File $cargoDev check --manifest-path $ManifestPath 2>&1 | ForEach-Object {
            $line = Remove-AnsiEscape $_
            $output.Add($line)
            Write-Host $line
        }
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    if ($exitCode -ne 0) {
        Stop-Guard "Cargo check failed before warning budget could be evaluated."
    }

    $warnings = Count-RustWarningLines $output
    if ($warnings -gt $MaxWarnings) {
        Write-Host "RUST_WARNING_BUDGET=failed warnings=$warnings max=$MaxWarnings" -ForegroundColor Red
        Stop-Guard "Rust warning budget exceeded. Clean warnings or raise the budget only with explicit review."
    }

    Write-Host "RUST_WARNING_BUDGET=passed warnings=$warnings max=$MaxWarnings"
} finally {
    Pop-Location
}
