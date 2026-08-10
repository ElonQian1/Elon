Import-Module "$PSScriptRoot\RustCache.Paths.psm1" -Force -DisableNameChecking

function Resolve-RustCacheBuildScope {
    param(
        [Parameter(Mandatory)][bool]$Registered,
        [Parameter(Mandatory)][string]$WorkspaceHash,
        [string]$SharedBuildPartition
    )

    if (-not $Registered) {
        if (-not [string]::IsNullOrWhiteSpace($SharedBuildPartition)) {
            throw "Shared Rust build partitions require a registered rust-cache.project.json project."
        }
        return [pscustomobject]@{
            cache_scope = "quarantine"
            partition_name = $WorkspaceHash
            shared_partition = $null
        }
    }

    if ([string]::IsNullOrWhiteSpace($SharedBuildPartition)) {
        return [pscustomobject]@{
            cache_scope = "workspace"
            partition_name = $WorkspaceHash
            shared_partition = $null
        }
    }

    $requestedPartition = $SharedBuildPartition.Trim()
    $partition = ConvertTo-RustCacheSlug -Value $requestedPartition
    if ($partition -eq "unknown" -or $partition -cne $requestedPartition) {
        throw "Shared Rust build partition must already be a lowercase stable slug using letters, digits, dots, underscores, or hyphens: $SharedBuildPartition"
    }
    return [pscustomobject]@{
        cache_scope = "shared"
        partition_name = "shared-$partition"
        shared_partition = $partition
    }
}

Export-ModuleMember -Function Resolve-RustCacheBuildScope
