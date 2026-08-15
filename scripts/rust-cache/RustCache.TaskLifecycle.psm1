Import-Module "$PSScriptRoot\RustCache.Paths.psm1" -Force -DisableNameChecking

function ConvertTo-RustCacheComparablePath {
    param([AllowNull()][string]$PathValue)

    if (-not (Test-RustCacheAbsolutePath $PathValue)) {
        return $null
    }
    try {
        $fullPath = [System.IO.Path]::GetFullPath($PathValue.Trim())
        if ($env:OS -eq "Windows_NT" -and $fullPath.StartsWith('\\?\')) {
            $fullPath = $fullPath.Substring(4)
        }
        return $fullPath.TrimEnd('\', '/')
    } catch {
        return $null
    }
}

function Test-RustCachePathWithin {
    param(
        [Parameter(Mandatory)][string]$ParentPath,
        [Parameter(Mandatory)][string]$CandidatePath
    )

    $parent = ConvertTo-RustCacheComparablePath -PathValue $ParentPath
    $candidate = ConvertTo-RustCacheComparablePath -PathValue $CandidatePath
    if ([string]::IsNullOrWhiteSpace($parent) -or [string]::IsNullOrWhiteSpace($candidate)) {
        return $false
    }
    $comparison = if ($env:OS -eq "Windows_NT") {
        [System.StringComparison]::OrdinalIgnoreCase
    } else {
        [System.StringComparison]::Ordinal
    }
    if ($candidate.Equals($parent, $comparison)) {
        return $true
    }
    $prefix = $parent + [System.IO.Path]::DirectorySeparatorChar
    return $candidate.StartsWith($prefix, $comparison)
}

function Read-RustCachePartitionMarker {
    param([Parameter(Mandatory)][string]$PartitionPath)

    $partition = ConvertTo-RustCacheComparablePath -PathValue $PartitionPath
    $markerPath = if ($partition) { Join-Path $partition ".last-used.json" } else { $null }
    $result = [ordered]@{
        valid = $false
        marker_path = $markerPath
        workspace_root = $null
        cache_scope = "unknown"
        cache_partition = $null
        last_used_utc = $null
        error = "marker-missing"
    }
    if (-not $markerPath -or -not (Test-Path -LiteralPath $markerPath -PathType Leaf)) {
        return [pscustomobject]$result
    }

    try {
        $marker = Get-Content -Raw -LiteralPath $markerPath -Encoding UTF8 | ConvertFrom-Json -ErrorAction Stop
        $workspaceRoot = ConvertTo-RustCacheComparablePath -PathValue ([string]$marker.workspace_root)
        if ([string]::IsNullOrWhiteSpace($workspaceRoot)) {
            throw "workspace_root is missing or invalid"
        }
        $lastUsed = [DateTime]::Parse([string]$marker.last_used_utc).ToUniversalTime()
        $scopeProperty = $marker.PSObject.Properties["cache_scope"]
        $scope = if ($null -eq $scopeProperty -or [string]::IsNullOrWhiteSpace([string]$scopeProperty.Value)) {
            "workspace"
        } else {
            ([string]$scopeProperty.Value).Trim().ToLowerInvariant()
        }
        if ($scope -notin @("workspace", "shared", "quarantine")) {
            $scope = "unknown"
        }
        $cachePartitionProperty = $marker.PSObject.Properties["cache_partition"]
        $cachePartition = if ($null -eq $cachePartitionProperty) { $null } else { [string]$cachePartitionProperty.Value }
        $leaf = Split-Path -Leaf $partition
        if (-not [string]::IsNullOrWhiteSpace($cachePartition) -and $cachePartition -cne $leaf) {
            throw "cache_partition does not match the partition directory"
        }

        $result.valid = $true
        $result.workspace_root = $workspaceRoot
        $result.cache_scope = $scope
        $result.cache_partition = $cachePartition
        $result.last_used_utc = $lastUsed
        $result.error = $null
    } catch {
        $result.error = $_.Exception.Message
    }
    return [pscustomobject]$result
}

function Resolve-RustCacheTaskWorktreeBase {
    param([string]$ExplicitBase)

    $candidate = if (-not [string]::IsNullOrWhiteSpace($ExplicitBase)) {
        $ExplicitBase
    } elseif (-not [string]::IsNullOrWhiteSpace($env:ELON_AI_TASK_WORKTREE_BASE)) {
        $env:ELON_AI_TASK_WORKTREE_BASE
    } else {
        "D:\wt"
    }
    return ConvertTo-RustCacheComparablePath -PathValue $candidate
}

function Get-RustCacheDisposableTaskRoot {
    param(
        [Parameter(Mandatory)][string]$WorkspaceRoot,
        [string]$TaskWorktreeBase
    )

    $workspace = ConvertTo-RustCacheComparablePath -PathValue $WorkspaceRoot
    $base = Resolve-RustCacheTaskWorktreeBase -ExplicitBase $TaskWorktreeBase
    if (-not $workspace -or -not $base -or -not (Test-RustCachePathWithin -ParentPath $base -CandidatePath $workspace)) {
        return $null
    }
    $relative = $workspace.Substring($base.Length).TrimStart('\', '/')
    if ([string]::IsNullOrWhiteSpace($relative)) {
        return $null
    }
    $taskLeaf = ($relative -split '[\\/]', 2)[0]
    if ($taskLeaf -notmatch '^\d{1,10}-[0-9a-f]{8}$') {
        return $null
    }
    return Join-Path $base $taskLeaf
}

function Test-RustCacheTaskOwnedPartition {
    param(
        [Parameter(Mandatory)]$Partition,
        [Parameter(Mandatory)][string]$TaskWorktree
    )

    if ([string]$Partition.kind -ne "registered" -or [string]$Partition.workspace_hash -notmatch '^[0-9a-f]{16}$') {
        return $false
    }
    if (-not [bool]$Partition.marker_valid -or [string]$Partition.cache_scope -ne "workspace") {
        return $false
    }
    return Test-RustCachePathWithin -ParentPath $TaskWorktree -CandidatePath ([string]$Partition.marker_workspace_root)
}

function Test-RustCacheOrphanedTaskPartition {
    param(
        [Parameter(Mandatory)]$Partition,
        [Parameter(Mandatory)][double]$GraceHours,
        [DateTime]$NowUtc = [DateTime]::UtcNow,
        [string]$TaskWorktreeBase
    )

    if ($GraceHours -le 0 -or -not (Test-RustCacheTaskOwnedPartitionShape -Partition $Partition)) {
        return $false
    }
    $taskRoot = Get-RustCacheDisposableTaskRoot -WorkspaceRoot ([string]$Partition.marker_workspace_root) -TaskWorktreeBase $TaskWorktreeBase
    if ([string]::IsNullOrWhiteSpace($taskRoot) -or (Test-Path -LiteralPath $taskRoot)) {
        return $false
    }
    $lastUsed = [DateTime]$Partition.last_used_utc
    return $lastUsed -le $NowUtc.ToUniversalTime().AddHours(-$GraceHours)
}

function Test-RustCacheTaskOwnedPartitionShape {
    param([Parameter(Mandatory)]$Partition)

    return [string]$Partition.kind -eq "registered" -and
        [string]$Partition.workspace_hash -match '^[0-9a-f]{16}$' -and
        [bool]$Partition.marker_valid -and
        [string]$Partition.cache_scope -eq "workspace" -and
        -not [string]::IsNullOrWhiteSpace([string]$Partition.marker_workspace_root)
}

Export-ModuleMember -Function ConvertTo-RustCacheComparablePath, Test-RustCachePathWithin, Read-RustCachePartitionMarker, Resolve-RustCacheTaskWorktreeBase, Get-RustCacheDisposableTaskRoot, Test-RustCacheTaskOwnedPartition, Test-RustCacheOrphanedTaskPartition
