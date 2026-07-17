<#
.SYNOPSIS
    Manage and use the machine-wide Rust cache platform.

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 status -IncludeSizes

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 run -ProjectRoot . -Domain dev-host check --manifest-path server\Cargo.toml
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)][ValidateSet("status", "run", "gc", "install", "register-legacy", "purge-legacy")][string]$Command = "status",
    [string]$ProjectRoot,
    [string]$Domain,
    [string]$TargetDir,
    [string]$CacheRoot,
    [string]$CargoConfigPath,
    [string]$LegacyPath,
    [string]$Label,
    [switch]$Retired,
    [switch]$Apply,
    [switch]$ForceAged,
    [switch]$IncludeSizes,
    [switch]$NoLock,
    [switch]$DisableSccache,
    [switch]$SkipCacheGc,
    [int]$LockTimeoutSeconds = 3600,
    [Parameter(Position = 1, ValueFromRemainingArguments = $true)][string[]]$RemainingArgs = @()
)

$ErrorActionPreference = "Stop"
$scriptsRoot = $PSScriptRoot
$modulesRoot = Join-Path $scriptsRoot "rust-cache"
Import-Module "$modulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Inventory.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Legacy.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Install.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Runtime.psm1" -Force -DisableNameChecking
# Nested module imports are scoped to their owning module. Re-import the two
# management surfaces last so status/run and register-legacy all stay callable.
Import-Module "$modulesRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking

if ([string]::IsNullOrWhiteSpace($ProjectRoot)) {
    $gitRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null)
    if ($LASTEXITCODE -eq 0 -and $gitRoot) {
        $ProjectRoot = $gitRoot.Trim()
    } else {
        $ProjectRoot = (Get-Location).Path
    }
}
$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)

switch ($Command) {
    "status" {
        $status = Get-RustCacheStatus -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -IncludeSizes:$IncludeSizes
        Write-Host "Rust cache platform" -ForegroundColor Cyan
        Write-Host "Root: $($status.cache_root)"
        Write-Host "Toolchain epoch: $($status.toolchain_epoch)"
        Write-Host "Disk free: $([math]::Round($status.volume.free_bytes / 1GB, 2)) GiB ($($status.volume.free_percent)%)"
        Write-Host "Managed partitions: $($status.partition_count)"
        Write-Host "Registered workspaces: $(@($status.registered_workspaces).Count)"
        if (@($status.legacy_caches).Count -gt 0) {
            Write-Host "External legacy caches:" -ForegroundColor Yellow
            $status.legacy_caches | Format-Table label, retired, exists, path -AutoSize
        }
        if ($IncludeSizes -and $status.partition_count -gt 0) {
            $status.partitions | Select-Object project_id, domain, @{n="GiB";e={[math]::Round($_.size_bytes / 1GB, 2)}}, last_used_utc, path | Format-Table -AutoSize
        }
        $status
    }
    "run" {
        if ($RemainingArgs.Count -eq 0) {
            throw "run requires Cargo arguments."
        }
        Invoke-RustCachePreflightGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -Skip:$SkipCacheGc | Out-Null
        Invoke-RustCacheCargo -ProjectRoot $ProjectRoot -Domain $Domain -TargetDir $TargetDir -CacheRoot $CacheRoot -NoLock:$NoLock -DisableSccache:$DisableSccache -LockTimeoutSeconds $LockTimeoutSeconds -CargoArgs $RemainingArgs
        exit $LASTEXITCODE
    }
    "gc" {
        $report = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -Apply:$Apply -ForceAged:$ForceAged
        Write-Host "GC mode: $($report.mode)"
        Write-Host "Low disk: $($report.low_disk); critical: $($report.critical_disk)"
        Write-Host "Report: $($report.report_path)"
        $report.actions | Format-Table action, reason, project_id, domain, age_days, @{n="GiB";e={[math]::Round($_.size_bytes / 1GB, 2)}}, path -AutoSize
        $report
    }
    "install" {
        if ([string]::IsNullOrWhiteSpace($CargoConfigPath)) {
            $CargoConfigPath = "D:\rust\.cargo\config.toml"
        }
        $result = Install-RustCachePlatform -SourceScriptsRoot $scriptsRoot -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -CargoConfigPath $CargoConfigPath -ActivateCargoConfig:$Apply -ConfigureSccacheServer
        Write-Host "Installed Rust cache platform: $($result.entry_path)" -ForegroundColor Green
        Write-Host "Cargo include: $($result.cargo_include_path)"
        if (-not $Apply) {
            Write-Host "Cargo parent config was not changed. Re-run install with -Apply to activate it." -ForegroundColor Yellow
        }
        $result
    }
    "register-legacy" {
        if ([string]::IsNullOrWhiteSpace($LegacyPath) -or [string]::IsNullOrWhiteSpace($Label)) {
            throw "register-legacy requires -LegacyPath and -Label."
        }
        $root = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $ProjectRoot
        $path = Add-RustCacheLegacyRecord -CacheRoot $root -Path $LegacyPath -Label $Label -Retired:$Retired
        Write-Host "Updated external legacy cache registry: $path" -ForegroundColor Green
    }
    "purge-legacy" {
        if ([string]::IsNullOrWhiteSpace($LegacyPath)) {
            throw "purge-legacy requires -LegacyPath. The path must already be registered and retired."
        }
        $report = Invoke-RustCacheLegacyPurge -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -LegacyPath $LegacyPath -Apply:$Apply
        Write-Host "Legacy purge mode: $($report.mode); action: $($report.action)"
        Write-Host "Path: $($report.path)"
        Write-Host "Size: $([math]::Round($report.size_bytes / 1GB, 2)) GiB"
        Write-Host "Report: $($report.report_path)"
        $report
    }
}
