<#
.SYNOPSIS
    Manage and use the machine-wide Rust cache platform.

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 status -IncludeSizes

.EXAMPLE
    & .\scripts\rust-cache.ps1 run -ProjectRoot . -Domain dev-windows-msvc check --manifest-path server\Cargo.toml

.EXAMPLE
    & .\scripts\rust-cache.ps1 doctor -ProjectRoot .

.EXAMPLE
    & .\scripts\rust-cache.ps1 init-project -ProjectRoot D:\work\sample -ProjectId sample
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)][ValidateSet("help", "status", "doctor", "fleet-report", "run", "gc", "install", "init-project", "register-legacy", "purge-legacy")][string]$Command = "status",
    [string]$ProjectRoot,
    [string]$Domain,
    [string]$TargetDir,
    [string]$SharedBuildPartition,
    [string]$CacheRoot,
    [string]$CargoConfigPath,
    [string]$LegacyPath,
    [string]$Label,
    [string]$ProjectId,
    [string]$DefaultDomain = "dev-windows-msvc",
    [string[]]$AllowedDomain = @(),
    [string]$UnknownDomainFallback = "agent-validation",
    [string[]]$SharedPartitionDomain = @(),
    [string]$CodexSkillsRoot,
    [string]$UserLauncherPath,
    [string]$NodeId,
    [string]$OutputPath,
    [switch]$Retired,
    [switch]$Apply,
    [switch]$ForceAged,
    [switch]$WorkspaceOnly,
    [switch]$RecoverMissingWorkspaces,
    [switch]$SharedAliasesOnly,
    [switch]$IncludeSizes,
    [switch]$NoLock,
    [switch]$DisableSccache,
    [switch]$ResetCargoSourcePolicy,
    [switch]$InstallCodexSkill,
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
Import-Module "$modulesRoot\RustCache.Sccache.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Launcher.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Install.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Portability.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Fleet.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Help.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Runtime.psm1" -Force -DisableNameChecking
# Nested module imports are scoped to their owning module. Re-import public
# management surfaces after Fleet has loaded its own dependencies.
Import-Module "$modulesRoot\RustCache.Portability.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Inventory.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Runtime.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking

if ($Command -eq "help") {
    Show-RustCacheCommandHelp
    return
}

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
        if ($status.retired_shared_alias_count -gt 0) {
            Write-Host "Retired shared aliases: $($status.retired_shared_alias_count) (run gc dry-run to review)" -ForegroundColor Yellow
        }
        if (@($status.legacy_caches).Count -gt 0) {
            Write-Host "External legacy caches:" -ForegroundColor Yellow
            $status.legacy_caches | Format-Table label, retired, exists, path -AutoSize
        }
        if ($IncludeSizes -and $status.partition_count -gt 0) {
            $status.partitions | Select-Object project_id, domain, cache_scope, retired_shared_alias, canonical_shared_domain, @{n="GiB";e={[math]::Round($_.size_bytes / 1GB, 2)}}, last_used_utc, path | Format-Table -AutoSize
        }
        $status
    }
    "doctor" {
        $sourceSkillRoot = Join-Path (Split-Path $scriptsRoot -Parent) ".agents\skills\manage-shared-build-cache"
        $doctor = Get-RustCacheDoctor -ProjectRoot $ProjectRoot -SourceScriptsRoot $scriptsRoot -CacheRoot $CacheRoot -CargoConfigPath $CargoConfigPath -SourceSkillRoot $sourceSkillRoot -CodexSkillsRoot $CodexSkillsRoot -UserLauncherPath $UserLauncherPath
        Write-Host "Rust cache doctor: $($doctor.status)" -ForegroundColor $(if ($doctor.healthy) { "Green" } else { "Yellow" })
        $doctor.checks | Format-Table status, id, message, remediation -Wrap -AutoSize
        $doctor
        if (-not $doctor.healthy) { exit 2 }
    }
    "fleet-report" {
        $sourceSkillRoot = Join-Path (Split-Path $scriptsRoot -Parent) ".agents\skills\manage-shared-build-cache"
        $report = New-RustCacheFleetReport -ProjectRoot $ProjectRoot -SourceScriptsRoot $scriptsRoot -CacheRoot $CacheRoot -CargoConfigPath $CargoConfigPath -SourceSkillRoot $sourceSkillRoot -CodexSkillsRoot $CodexSkillsRoot -UserLauncherPath $UserLauncherPath -NodeId $NodeId -IncludeSizes:$IncludeSizes
        $resolvedRoot = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $ProjectRoot
        $export = Export-RustCacheFleetReport -Report $report -CacheRoot $resolvedRoot -OutputPath $OutputPath
        Write-Host "Fleet report: $($export.report_path)" -ForegroundColor Green
        Write-Host "Report SHA-256: $($export.content_sha256)"
        Write-Host "Health: $($report.platform.health); active writers: $($report.activity.active_writer_count); managed partitions: $($report.cache.partition_count)"
        $export
    }
    "run" {
        if ($RemainingArgs.Count -eq 0) {
            throw "run requires Cargo arguments."
        }
        Invoke-RustCachePreflightGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -Skip:$SkipCacheGc | Out-Null
        Invoke-RustCacheCargo -ProjectRoot $ProjectRoot -Domain $Domain -TargetDir $TargetDir -CacheRoot $CacheRoot -NoLock:$NoLock -DisableSccache:$DisableSccache -LockTimeoutSeconds $LockTimeoutSeconds -SharedBuildPartition $SharedBuildPartition -CargoArgs $RemainingArgs
        exit $LASTEXITCODE
    }
    "gc" {
        $report = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -Apply:$Apply -ForceAged:$ForceAged -WorkspaceOnly:$WorkspaceOnly -RecoverMissingWorkspaces:$RecoverMissingWorkspaces -SharedAliasesOnly:$SharedAliasesOnly
        Write-Host "GC mode: $($report.mode)"
        Write-Host "Workspace only: $($report.workspace_only); recover missing workspaces: $($report.recover_missing_workspaces); shared aliases only: $($report.shared_aliases_only)"
        Write-Host "Low disk: $($report.low_disk); critical: $($report.critical_disk)"
        Write-Host "Report: $($report.report_path)"
        $report.actions | Format-Table action, reason, project_id, domain, age_days, @{n="GiB";e={[math]::Round($_.size_bytes / 1GB, 2)}}, path -AutoSize
        $report
    }
    "install" {
        if ([string]::IsNullOrWhiteSpace($CargoConfigPath)) {
            $CargoConfigPath = Get-RustCacheDefaultCargoConfigPath
        }
        $sourceSkillRoot = Join-Path (Split-Path $scriptsRoot -Parent) ".agents\skills\manage-shared-build-cache"
        $result = Install-RustCachePlatform -SourceScriptsRoot $scriptsRoot -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -CargoConfigPath $CargoConfigPath -SourceSkillRoot $sourceSkillRoot -CodexSkillsRoot $CodexSkillsRoot -UserLauncherPath $UserLauncherPath -ActivateCargoConfig:$Apply -InstallCodexSkill:$InstallCodexSkill -ConfigureSccacheServer -ResetCargoSourcePolicy:$ResetCargoSourcePolicy
        Write-Host "Installed Rust cache platform: $($result.entry_path)" -ForegroundColor Green
        Write-Host "Portable user launcher: $($result.user_launcher.path)"
        Write-Host "Cargo include: $($result.cargo_include_path)"
        Write-Host "Platform manifest: $($result.platform_manifest_path)"
        Write-Host "Source hash: $($result.source_hash)"
        if ($result.codex_skill) {
            Write-Host "Codex skill: $($result.codex_skill.path)" -ForegroundColor Green
        }
        if ($result.sccache_server -and $result.sccache_server.status -eq "deferred") {
            Write-Warning "SCCache reload is pending until Cargo/rustc becomes idle: $($result.sccache_server.state_path)"
        }
        if (-not $Apply) {
            Write-Host "Cargo parent config was not changed. Re-run install with -Apply to activate it." -ForegroundColor Yellow
        }
        $result
    }
    "init-project" {
        if ([string]::IsNullOrWhiteSpace($ProjectId)) {
            throw "init-project requires -ProjectId."
        }
        $result = New-RustCacheProjectManifest -ProjectRoot $ProjectRoot -ProjectId $ProjectId -DefaultDomain $DefaultDomain -AllowedDomains $AllowedDomain -UnknownDomainFallback $UnknownDomainFallback -SharedPartitionDomains $SharedPartitionDomain -Apply:$Apply
        Write-Host "Project cache manifest: $($result.action) $($result.path)" -ForegroundColor $(if ($result.applied -or $result.action -eq "unchanged") { "Green" } else { "Yellow" })
        if (-not $Apply -and $result.action -eq "would-create") {
            Write-Host "Review the preview, then repeat with -Apply." -ForegroundColor Yellow
            Write-Host $result.content
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
