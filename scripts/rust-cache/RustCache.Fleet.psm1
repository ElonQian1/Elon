Import-Module "$PSScriptRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Inventory.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Portability.psm1" -Force -DisableNameChecking

function Get-RustCacheFleetGroupSummary {
    param(
        [Parameter(Mandatory)][AllowEmptyCollection()][object[]]$Items,
        [Parameter(Mandatory)][string]$Property,
        [switch]$IncludeSizes
    )

    return @($Items | Group-Object -Property $Property | Sort-Object Name | ForEach-Object {
        $members = @($_.Group)
        [pscustomobject]@{
            name = [string]$_.Name
            count = $members.Count
            size_bytes = if ($IncludeSizes) { [int64](($members | Measure-Object -Property size_bytes -Sum).Sum) } else { $null }
        }
    })
}

function Assert-RustCacheFleetNodeId {
    param([string]$NodeId)

    if ([string]::IsNullOrWhiteSpace($NodeId)) { return }
    if ($NodeId.Length -gt 128 -or $NodeId -notmatch '^[A-Za-z0-9][A-Za-z0-9._:-]*$') {
        throw "NodeId must be a 1-128 character stable identifier using letters, digits, dot, underscore, colon, or hyphen."
    }
}

function New-RustCacheFleetReport {
    param(
        [Parameter(Mandatory)][string]$ProjectRoot,
        [Parameter(Mandatory)][string]$SourceScriptsRoot,
        [string]$CacheRoot,
        [string]$CargoConfigPath,
        [string]$SourceSkillRoot,
        [string]$CodexSkillsRoot,
        [string]$UserLauncherPath,
        [string]$NodeId,
        [switch]$IncludeSizes
    )

    Assert-RustCacheFleetNodeId -NodeId $NodeId
    $project = [System.IO.Path]::GetFullPath($ProjectRoot)
    $doctor = Get-RustCacheDoctor -ProjectRoot $project -SourceScriptsRoot $SourceScriptsRoot -CacheRoot $CacheRoot -CargoConfigPath $CargoConfigPath -SourceSkillRoot $SourceSkillRoot -CodexSkillsRoot $CodexSkillsRoot -UserLauncherPath $UserLauncherPath
    $status = Get-RustCacheStatus -CacheRoot $CacheRoot -RepoRoot $project -IncludeSizes:$IncludeSizes
    $manifest = Get-RustCacheProjectManifest -ProjectRoot $project
    $partitions = @($status.partitions)
    $activeWriters = @($doctor.active_writers)
    $actionableChecks = @($doctor.checks | Where-Object { $_.status -ne "pass" } | ForEach-Object {
        [pscustomobject]@{
            id = [string]$_.id
            status = [string]$_.status
        }
    })
    $writerGroups = @($activeWriters | Group-Object -Property ProcessName | Sort-Object Name | ForEach-Object {
        [pscustomobject]@{ process_name = [string]$_.Name; count = @($_.Group).Count }
    })
    $managedSize = if ($IncludeSizes) { [int64](($partitions | Measure-Object -Property size_bytes -Sum).Sum) } else { $null }
    $legacySize = if ($IncludeSizes) { [int64](($status.legacy_caches | Measure-Object -Property size_bytes -Sum).Sum) } else { $null }
    $warningPercent = [double]$status.policy.warning_free_percent

    [pscustomobject][ordered]@{
        schema = "elon.rust_cache.fleet_report.v1"
        generated_at_utc = [DateTime]::UtcNow.ToString("o")
        node = [pscustomobject][ordered]@{
            node_id = if ([string]::IsNullOrWhiteSpace($NodeId)) { $null } else { $NodeId }
            os = if ($env:OS -eq "Windows_NT") { "windows" } else { "non-windows" }
            powershell_edition = [string]$PSVersionTable.PSEdition
            powershell_version = [string]$PSVersionTable.PSVersion
        }
        project = [pscustomobject][ordered]@{
            project_id = [string]$manifest.project_id
            registered = [bool]$manifest.registered
            default_domain = [string]$manifest.default_domain
            allowed_domains = @($manifest.allowed_domains)
            shared_partition_count = @($manifest.shared_partition_domains.Keys).Count
        }
        platform = [pscustomobject][ordered]@{
            health = [string]$doctor.status
            source_mode = [string]$doctor.source_mode
            source_hash = [string]$doctor.source_hash
            actionable_checks = $actionableChecks
        }
        cache = [pscustomobject][ordered]@{
            toolchain_epoch = [string]$status.toolchain_epoch
            include_sizes = [bool]$IncludeSizes
            partition_count = $partitions.Count
            managed_size_bytes = $managedSize
            locked_partition_count = @($partitions | Where-Object { $_.locked }).Count
            invalid_marker_count = @($partitions | Where-Object { -not $_.marker_valid -and $_.kind -ne "quarantine" }).Count
            quarantine_partition_count = @($partitions | Where-Object { $_.kind -eq "quarantine" }).Count
            retired_shared_alias_count = [int]$status.retired_shared_alias_count
            by_scope = Get-RustCacheFleetGroupSummary -Items $partitions -Property "cache_scope" -IncludeSizes:$IncludeSizes
            by_domain = Get-RustCacheFleetGroupSummary -Items $partitions -Property "domain" -IncludeSizes:$IncludeSizes
            legacy_cache_count = @($status.legacy_caches).Count
            retired_legacy_cache_count = @($status.legacy_caches | Where-Object { $_.retired }).Count
            legacy_size_bytes = $legacySize
        }
        volume = [pscustomobject][ordered]@{
            total_bytes = [int64]$status.volume.total_bytes
            free_bytes = [int64]$status.volume.free_bytes
            free_percent = [double]$status.volume.free_percent
            warning_free_percent = $warningPercent
            gc_review_recommended = ([double]$status.volume.free_percent -lt $warningPercent) -or ([int]$status.retired_shared_alias_count -gt 0)
        }
        activity = [pscustomobject][ordered]@{
            active_writer_count = $activeWriters.Count
            active_writers = $writerGroups
        }
        privacy = [pscustomobject][ordered]@{
            absolute_paths_included = $false
            host_name_included = $false
            user_name_included = $false
        }
        destructive_actions_taken = $false
    }
}

function Export-RustCacheFleetReport {
    param(
        [Parameter(Mandatory)]$Report,
        [Parameter(Mandatory)][string]$CacheRoot,
        [string]$OutputPath
    )

    $root = [System.IO.Path]::GetFullPath($CacheRoot)
    if ([string]::IsNullOrWhiteSpace($OutputPath)) {
        $reportRoot = Join-Path $root "reports\fleet"
        $name = "fleet-{0}-{1}.json" -f [DateTime]::UtcNow.ToString("yyyyMMdd-HHmmss"), [Guid]::NewGuid().ToString("N").Substring(0, 8)
        $path = Join-Path $reportRoot $name
    } else {
        if (-not (Test-RustCacheAbsolutePath $OutputPath)) {
            throw "OutputPath must be absolute: $OutputPath"
        }
        $path = [System.IO.Path]::GetFullPath($OutputPath)
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $path -Parent) | Out-Null
    $temporary = "$path.$PID.tmp"
    try {
        $Report | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $temporary -Encoding UTF8
        Move-Item -LiteralPath $temporary -Destination $path -Force
    } finally {
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    }
    [pscustomobject]@{
        schema = "elon.rust_cache.fleet_export.v1"
        report_path = $path
        content_sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        report = $Report
    }
}

Export-ModuleMember -Function New-RustCacheFleetReport, Export-RustCacheFleetReport
