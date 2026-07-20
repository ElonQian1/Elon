<#
.SYNOPSIS
    Run Cargo through the machine-wide Rust cache platform.

.DESCRIPTION
    Development builds keep final artifacts workspace-local while intermediate
    artifacts are routed to an isolated, toolchain-aware build partition. The
    compiler object cache is shared through sccache.

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 check --manifest-path server\Cargo.toml
#>
param(
    [string]$TargetDir,
    [string]$CacheRoot,
    [string]$Domain = "dev-windows-msvc",
    [switch]$NoLock,
    [switch]$DisableSccache,
    [switch]$SkipCacheGc,
    [switch]$BypassValidationOrchestrator,
    [switch]$RefreshValidationEvidence,
    [switch]$SkipCheapGates,
    [int]$LightSlots = 2,
    [int]$WaitTimeoutSeconds = 3600,
    [int]$LockTimeoutSeconds = 3600,
    [Parameter(Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs = @()
)

$ErrorActionPreference = "Stop"
if ($CargoArgs.Count -eq 0) {
    Write-Error "Usage: powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 <cargo-args...>"
}

$gitRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $gitRoot) {
    Write-Error "Current script is not inside a Git repository."
}
$RepoRoot = $gitRoot.Trim()

if (-not $BypassValidationOrchestrator) {
    $validationArgs = @("-Domain", $Domain)
    if ($CacheRoot) { $validationArgs += @("-CacheRoot", $CacheRoot) }
    if ($TargetDir) { $validationArgs += @("-TargetDir", $TargetDir) }
    if ($DisableSccache) { $validationArgs += "-DisableSccache" }
    if ($RefreshValidationEvidence) { $validationArgs += "-Force" }
    if ($SkipCheapGates) { $validationArgs += "-SkipCheapGates" }
    $validationArgs += @("-LightSlots", $LightSlots, "-WaitTimeoutSeconds", $WaitTimeoutSeconds)
    $validationArgs += $CargoArgs
    & (Join-Path $RepoRoot "scripts\validate-rust.ps1") @validationArgs
    exit $LASTEXITCODE
}

function Import-LocalEnvFile {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) { return }
    foreach ($line in Get-Content -LiteralPath $Path -Encoding UTF8) {
        $trimmed = $line.Trim()
        if ([string]::IsNullOrWhiteSpace($trimmed) -or $trimmed.StartsWith("#")) { continue }
        if ($trimmed -notmatch '^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)\s*$') { continue }
        $name = $Matches[1]
        $value = $Matches[2].Trim()
        if ($value.Length -ge 2) {
            $first = $value.Substring(0, 1)
            $last = $value.Substring($value.Length - 1, 1)
            if (($first -eq '"' -and $last -eq '"') -or ($first -eq "'" -and $last -eq "'")) {
                $value = $value.Substring(1, $value.Length - 2)
            }
        }
        if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name, "Process"))) {
            [Environment]::SetEnvironmentVariable($name, $value, "Process")
        }
    }
}

Import-LocalEnvFile -Path (Join-Path $RepoRoot ".env.local")
if (-not [string]::IsNullOrWhiteSpace($env:ELON_DEV_CARGO_TARGET_DIR) -and [string]::IsNullOrWhiteSpace($TargetDir)) {
    Write-Warning "ELON_DEV_CARGO_TARGET_DIR is a legacy shared-target setting and is no longer used by cargo-dev. Use -TargetDir only when a final-artifact override is required."
}

$modulePath = Join-Path $RepoRoot "scripts\rust-cache\RustCache.Runtime.psm1"
Import-Module (Join-Path $RepoRoot "scripts\rust-cache\RustCache.Inventory.psm1") -Force -DisableNameChecking
Import-Module $modulePath -Force -DisableNameChecking
Invoke-RustCachePreflightGc -CacheRoot $CacheRoot -RepoRoot $RepoRoot -Skip:$SkipCacheGc | Out-Null
Invoke-RustCacheCargo -ProjectRoot $RepoRoot -Domain $Domain -TargetDir $TargetDir -CacheRoot $CacheRoot -NoLock:$NoLock -DisableSccache:$DisableSccache -LockTimeoutSeconds $LockTimeoutSeconds -CargoArgs $CargoArgs
$cargoExitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
if ($cargoExitCode -ne 0) {
    # `exit` only leaves this script when cargo-dev.ps1 is invoked from another
    # PowerShell script. Mark the host as well so nested CI/agent callers cannot
    # finish with process exit code 0 after Cargo failed.
    $host.SetShouldExit($cargoExitCode)
    Write-Error "Cargo failed with exit code $cargoExitCode." -ErrorAction Continue
    exit $cargoExitCode
}
exit 0
