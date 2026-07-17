Import-Module "$PSScriptRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Registry.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Runtime.psm1" -Force -DisableNameChecking

function Get-RustCacheDirectorySize {
    param([Parameter(Mandatory)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return [int64]0
    }
    $sum = (Get-ChildItem -LiteralPath $Path -File -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
    if ($null -eq $sum) { return [int64]0 }
    return [int64]$sum
}

function Get-RustCacheVolumeState {
    param([Parameter(Mandatory)][string]$CacheRoot)

    $rootPath = [System.IO.Path]::GetPathRoot([System.IO.Path]::GetFullPath($CacheRoot))
    $drive = [System.IO.DriveInfo]::new($rootPath)
    $freePercent = if ($drive.TotalSize -gt 0) { [math]::Round(($drive.AvailableFreeSpace * 100.0) / $drive.TotalSize, 2) } else { 0 }
    [pscustomobject]@{
        root = $rootPath
        total_bytes = [int64]$drive.TotalSize
        free_bytes = [int64]$drive.AvailableFreeSpace
        free_percent = $freePercent
    }
}

function Get-RustCachePartitionLastUsed {
    param([Parameter(Mandatory)][System.IO.DirectoryInfo]$Directory)

    $markerPath = Join-Path $Directory.FullName ".last-used.json"
    if (Test-Path -LiteralPath $markerPath) {
        try {
            $marker = Get-Content -Raw -LiteralPath $markerPath -Encoding UTF8 | ConvertFrom-Json
            return [DateTime]::Parse($marker.last_used_utc).ToUniversalTime()
        } catch {
        }
    }
    return $Directory.LastWriteTimeUtc
}

function Get-RustCachePartitions {
    param([Parameter(Mandatory)][string]$CacheRoot)

    $items = New-Object System.Collections.Generic.List[object]
    $buildRoot = Join-Path $CacheRoot "build"
    if (Test-Path -LiteralPath $buildRoot) {
        foreach ($epoch in Get-ChildItem -LiteralPath $buildRoot -Directory -Force -ErrorAction SilentlyContinue) {
            foreach ($project in Get-ChildItem -LiteralPath $epoch.FullName -Directory -Force -ErrorAction SilentlyContinue) {
                foreach ($domain in Get-ChildItem -LiteralPath $project.FullName -Directory -Force -ErrorAction SilentlyContinue) {
                    foreach ($workspace in Get-ChildItem -LiteralPath $domain.FullName -Directory -Force -ErrorAction SilentlyContinue) {
                        $items.Add([pscustomobject]@{
                            kind = "registered"
                            toolchain_epoch = $epoch.Name
                            project_id = $project.Name
                            domain = $domain.Name
                            workspace_hash = $workspace.Name
                            path = $workspace.FullName
                            last_used_utc = Get-RustCachePartitionLastUsed -Directory $workspace
                            locked = Test-Path -LiteralPath (Join-Path $workspace.FullName ".rust-cache.lockdir")
                        })
                    }
                }
            }
        }
    }
    $quarantineRoot = Join-Path $CacheRoot "quarantine"
    if (Test-Path -LiteralPath $quarantineRoot) {
        foreach ($shard in Get-ChildItem -LiteralPath $quarantineRoot -Directory -Force -ErrorAction SilentlyContinue) {
            $leaves = @(Get-ChildItem -LiteralPath $shard.FullName -Directory -Force -ErrorAction SilentlyContinue)
            if ($leaves.Count -eq 0) { $leaves = @($shard) }
            foreach ($workspace in $leaves) {
                $workspaceHash = if ($workspace.FullName -eq $shard.FullName) { $workspace.Name } else { "$($shard.Name)$($workspace.Name)" }
                $items.Add([pscustomobject]@{
                    kind = "quarantine"
                    toolchain_epoch = "unknown"
                    project_id = "unregistered"
                    domain = "unregistered"
                    workspace_hash = $workspaceHash
                    path = $workspace.FullName
                    last_used_utc = Get-RustCachePartitionLastUsed -Directory $workspace
                    locked = Test-Path -LiteralPath (Join-Path $workspace.FullName ".rust-cache.lockdir")
                })
            }
        }
    }
    return @($items | ForEach-Object { $_ })
}

function Get-RustCacheStatus {
    param(
        [string]$CacheRoot,
        [string]$RepoRoot,
        [switch]$IncludeSizes
    )

    $root = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $RepoRoot
    $policy = Get-RustCachePolicy -CacheRoot $root
    $volume = Get-RustCacheVolumeState -CacheRoot $root
    $partitions = @(Get-RustCachePartitions -CacheRoot $root)
    if ($IncludeSizes) {
        foreach ($partition in $partitions) {
            $partition | Add-Member -NotePropertyName size_bytes -NotePropertyValue (Get-RustCacheDirectorySize -Path $partition.path)
        }
    }
    $registry = Read-RustCacheRegistry -CacheRoot $root
    $legacy = @($policy.legacy_caches | ForEach-Object {
        $record = [pscustomobject]@{
            path = $_.path
            label = $_.label
            retired = [bool]$_.retired
            managed = $false
            exists = Test-Path -LiteralPath $_.path
        }
        if ($IncludeSizes -and $record.exists) {
            $record | Add-Member -NotePropertyName size_bytes -NotePropertyValue (Get-RustCacheDirectorySize -Path $record.path)
        }
        $record
    })
    [pscustomobject]@{
        schema_version = 1
        cache_root = $root
        policy = $policy
        volume = $volume
        toolchain_epoch = Get-RustCacheToolchainEpoch
        partition_count = $partitions.Count
        partitions = $partitions
        registered_workspaces = @($registry.workspaces)
        legacy_caches = $legacy
    }
}

function Test-RustCacheBuildProcesses {
    $active = @(Get-Process -Name cargo, rustc -ErrorAction SilentlyContinue)
    return $active
}

function Write-RustCacheGcReport {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)]$Report
    )

    $reportRoot = Join-Path $CacheRoot "reports"
    New-Item -ItemType Directory -Force -Path $reportRoot | Out-Null
    $stamp = [DateTime]::UtcNow.ToString("yyyyMMdd-HHmmss")
    $path = Join-Path $reportRoot "gc-$stamp.json"
    $Report | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $path -Encoding UTF8
    return $path
}

function Invoke-RustCacheGc {
    param(
        [string]$CacheRoot,
        [string]$RepoRoot,
        [switch]$Apply,
        [switch]$ForceAged
    )

    $root = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $RepoRoot
    $policy = Get-RustCachePolicy -CacheRoot $root
    $volumeBefore = Get-RustCacheVolumeState -CacheRoot $root
    $currentEpoch = Get-RustCacheToolchainEpoch
    $now = [DateTime]::UtcNow
    $lowDisk = $volumeBefore.free_percent -lt [double]$policy.warning_free_percent
    $criticalDisk = $volumeBefore.free_percent -lt [double]$policy.critical_free_percent
    $partitions = @(Get-RustCachePartitions -CacheRoot $root)

    foreach ($partition in $partitions) {
        $ageDays = ($now - ([DateTime]$partition.last_used_utc)).TotalDays
        $oldEpoch = $partition.kind -eq "registered" -and $partition.toolchain_epoch -ne $currentEpoch
        $expired = $ageDays -ge [double]$policy.partition_ttl_days
        $oldEpochExpired = $oldEpoch -and $ageDays -ge [double]$policy.old_epoch_ttl_days
        $partition | Add-Member -NotePropertyName age_days -NotePropertyValue ([math]::Round($ageDays, 2))
        $partition | Add-Member -NotePropertyName old_epoch -NotePropertyValue $oldEpoch
        $partition | Add-Member -NotePropertyName selected -NotePropertyValue ($oldEpochExpired -or (($lowDisk -or $ForceAged) -and $expired))
        $partition | Add-Member -NotePropertyName size_bytes -NotePropertyValue ([int64]0)
        $partition | Add-Member -NotePropertyName action -NotePropertyValue "preserve"
        $partition | Add-Member -NotePropertyName reason -NotePropertyValue "active-or-recent"
        if ($partition.locked) {
            $partition.selected = $false
            $partition.reason = "lock-present"
        } elseif ($oldEpochExpired) {
            $partition.reason = "old-toolchain-epoch"
        } elseif ($partition.selected) {
            $partition.reason = if ($lowDisk) { "disk-watermark" } else { "forced-aged-cleanup" }
        }
    }

    if ($lowDisk) {
        foreach ($partition in $partitions | Where-Object { -not $_.locked }) {
            $partition.selected = $true
            if ($partition.reason -eq "active-or-recent") {
                $partition.reason = "disk-watermark-lru"
            }
        }
    }

    $selected = @($partitions | Where-Object { $_.selected } | Sort-Object @{ Expression = { if ($_.old_epoch) { 0 } elseif ($_.kind -eq "quarantine") { 1 } else { 2 } } }, last_used_utc)
    foreach ($partition in $selected) {
        $partition.size_bytes = Get-RustCacheDirectorySize -Path $partition.path
    }

    $activeBuilds = @(Test-RustCacheBuildProcesses)
    if ($Apply -and $activeBuilds.Count -gt 0) {
        $summary = ($activeBuilds | ForEach-Object { "$($_.ProcessName):$($_.Id)" }) -join ", "
        throw "Refusing Rust cache GC while Cargo/rustc processes are active: $summary"
    }

    $estimatedFree = [int64]$volumeBefore.free_bytes
    $targetFreeBytes = [int64][math]::Ceiling($volumeBefore.total_bytes * ([double]$policy.recovery_free_percent / 100.0))
    foreach ($partition in $selected) {
        if ($lowDisk -and -not $ForceAged -and $estimatedFree -ge $targetFreeBytes) {
            $partition.selected = $false
            $partition.reason = "recovery-watermark-reached"
            continue
        }
        $partition.action = if ($Apply) { "delete" } else { "would-delete" }
        if ($Apply) {
            Assert-RustCacheManagedPath -CacheRoot $root -CandidatePath $partition.path
            Remove-Item -LiteralPath $partition.path -Recurse -Force -ErrorAction Stop
        }
        $estimatedFree += [int64]$partition.size_bytes
    }

    $volumeAfter = if ($Apply) { Get-RustCacheVolumeState -CacheRoot $root } else { $volumeBefore }
    $report = [pscustomobject]@{
        schema_version = 1
        generated_utc = [DateTime]::UtcNow.ToString("o")
        mode = if ($Apply) { "apply" } else { "dry-run" }
        force_aged = [bool]$ForceAged
        low_disk = $lowDisk
        critical_disk = $criticalDisk
        volume_before = $volumeBefore
        volume_after = $volumeAfter
        estimated_free_bytes_after_plan = $estimatedFree
        actions = @($partitions | Where-Object { $_.action -ne "preserve" -or $_.reason -ne "active-or-recent" })
        legacy_caches = @($policy.legacy_caches | ForEach-Object {
            [pscustomobject]@{ path = $_.path; label = $_.label; retired = [bool]$_.retired; action = "external-report-only" }
        })
    }
    $reportPath = Write-RustCacheGcReport -CacheRoot $root -Report $report
    $report | Add-Member -NotePropertyName report_path -NotePropertyValue $reportPath
    return $report
}

function Invoke-RustCachePreflightGc {
    param(
        [string]$CacheRoot,
        [string]$RepoRoot,
        [switch]$Skip
    )

    if ($Skip) { return $null }
    $root = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $RepoRoot
    $policy = Get-RustCachePolicy -CacheRoot $root
    if (-not [bool]$policy.auto_gc_on_run) { return $null }
    $volume = Get-RustCacheVolumeState -CacheRoot $root
    if ($volume.free_percent -ge [double]$policy.warning_free_percent) { return $null }
    try {
        $report = Invoke-RustCacheGc -CacheRoot $root -RepoRoot $RepoRoot -Apply
    } catch {
        if ($volume.free_percent -lt [double]$policy.critical_free_percent) {
            throw "Rust cache preflight cannot recover critical disk space ($($volume.free_percent)% free). $($_.Exception.Message)"
        }
        Write-Warning "Rust cache preflight GC was skipped: $($_.Exception.Message)"
        return $null
    }
    $after = Get-RustCacheVolumeState -CacheRoot $root
    if ($after.free_percent -lt [double]$policy.critical_free_percent) {
        throw "Rust cache preflight completed but disk remains critical: $($after.free_percent)% free. Retired external caches require explicit cleanup."
    }
    return $report
}

Export-ModuleMember -Function Get-RustCacheDirectorySize, Get-RustCacheVolumeState, Get-RustCachePartitions, Get-RustCacheStatus, Test-RustCacheBuildProcesses, Invoke-RustCacheGc, Invoke-RustCachePreflightGc
