function Test-RustCacheAbsolutePath {
    param([AllowNull()][string]$PathValue)

    if ([string]::IsNullOrWhiteSpace($PathValue)) {
        return $false
    }
    if (-not [System.IO.Path]::IsPathRooted($PathValue)) {
        return $false
    }
    if ($PathValue -match '^[A-Za-z]:($|[^\\/])') {
        return $false
    }
    if ($PathValue -match '^[\\/][^\\/]') {
        return $false
    }
    return $true
}

function Get-RustCachePersistedNodeDataRoot {
    if ([string]::IsNullOrWhiteSpace($env:APPDATA)) {
        return $null
    }

    $configPath = Join-Path $env:APPDATA "elon-node-agent\node.json"
    if (-not (Test-Path -LiteralPath $configPath -PathType Leaf)) {
        return $null
    }

    try {
        $config = Get-Content -Raw -LiteralPath $configPath -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop
        $nodeDataRoot = [string]$config.node_data_root
        if (-not (Test-RustCacheAbsolutePath $nodeDataRoot)) {
            return $null
        }

        $fullRoot = [System.IO.Path]::GetFullPath($nodeDataRoot.Trim())
        if (-not (Test-Path -LiteralPath $fullRoot -PathType Container)) {
            return $null
        }
        if (-not (Test-Path -LiteralPath (Join-Path $fullRoot ".elon-node-data-root.json") -PathType Leaf)) {
            return $null
        }
        return $fullRoot
    } catch {
        return $null
    }
}

function Resolve-RustCacheRoot {
    param(
        [string]$ExplicitRoot,
        [string]$RepoRoot
    )

    if (-not [string]::IsNullOrWhiteSpace($ExplicitRoot)) {
        $candidate = $ExplicitRoot.Trim()
        $source = "explicit"
    } elseif (-not [string]::IsNullOrWhiteSpace($env:ELON_RUST_CACHE_ROOT)) {
        $candidate = $env:ELON_RUST_CACHE_ROOT.Trim()
        $source = "ELON_RUST_CACHE_ROOT"
    } elseif (-not [string]::IsNullOrWhiteSpace($env:ELON_NODE_DATA_ROOT)) {
        $candidate = Join-Path $env:ELON_NODE_DATA_ROOT.Trim() "cache\rust-cache-v2"
        $source = "ELON_NODE_DATA_ROOT"
    } elseif ($persistedNodeDataRoot = Get-RustCachePersistedNodeDataRoot) {
        $candidate = Join-Path $persistedNodeDataRoot "cache\rust-cache-v2"
        $source = "persisted node_data_root"
    } elseif (-not [string]::IsNullOrWhiteSpace($env:RUST_SHARED_BUILD_ROOT)) {
        $candidate = Join-Path $env:RUST_SHARED_BUILD_ROOT.Trim() "rust-cache-v2"
        $source = "RUST_SHARED_BUILD_ROOT"
    } elseif (Test-Path -LiteralPath "D:\rust\shared") {
        $candidate = "D:\rust\shared\rust-cache-v2"
        $source = "D drive convention"
    } elseif (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        $candidate = Join-Path $env:LOCALAPPDATA "Elon\rust-cache-v2"
        $source = "LOCALAPPDATA"
    } elseif (-not [string]::IsNullOrWhiteSpace($RepoRoot)) {
        $candidate = Join-Path (Split-Path $RepoRoot -Parent) ".rust-cache-v2"
        $source = "repo parent fallback"
    } else {
        throw "Cannot resolve Rust cache root without an explicit path, LOCALAPPDATA, or repo root."
    }

    if (-not (Test-RustCacheAbsolutePath $candidate)) {
        throw "$source must resolve to an absolute Rust cache path. Current value: $candidate"
    }

    $fullPath = [System.IO.Path]::GetFullPath($candidate)
    $driveRoot = [System.IO.Path]::GetPathRoot($fullPath)
    if ($driveRoot -and -not (Test-Path -LiteralPath $driveRoot)) {
        throw "Rust cache drive/root does not exist: $fullPath"
    }
    New-Item -ItemType Directory -Force -Path $fullPath | Out-Null
    return $fullPath
}

function Get-RustCacheMigrationAdvice {
    param([Parameter(Mandatory)][string]$CacheRoot, [int]$LowWatermarkPercent=15, [string]$ManagedAlternativeRoot)
    $full = [IO.Path]::GetFullPath($CacheRoot)
    $drive = New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($full))
    $freePercent = if ($drive.TotalSize -gt 0) { [math]::Round(100 * $drive.AvailableFreeSpace / $drive.TotalSize, 2) } else { 0 }
    $recommended = $freePercent -lt $LowWatermarkPercent -and -not [string]::IsNullOrWhiteSpace($ManagedAlternativeRoot)
    return [pscustomobject]@{
        schema="elon.rust_cache.migration_advice.v1"; current_root=$full; free_percent=$freePercent
        low_watermark_percent=$LowWatermarkPercent; migration_recommended=$recommended
        suggested_root=if ($recommended) { [IO.Path]::GetFullPath($ManagedAlternativeRoot) } else { $null }
        destructive_actions_taken=$false
        safe_command=if ($recommended) { "Set ELON_RUST_CACHE_ROOT after reviewing the destination; register legacy cache, then use purge-legacy dry-run before -Apply." } else { $null }
    }
}

function ConvertTo-RustCacheSlug {
    param(
        [Parameter(Mandatory)][string]$Value,
        [int]$MaxLength = 64
    )

    $slug = $Value.Trim().ToLowerInvariant() -replace '[^a-z0-9._-]+', '-'
    $slug = $slug.Trim('-', '.', '_')
    if ([string]::IsNullOrWhiteSpace($slug)) {
        $slug = "unknown"
    }
    if ($slug.Length -gt $MaxLength) {
        $slug = $slug.Substring(0, $MaxLength).TrimEnd('-', '.', '_')
    }
    return $slug
}

function Get-RustCacheWorkspaceHash {
    param([Parameter(Mandatory)][string]$WorkspaceRoot)

    if (-not (Test-RustCacheAbsolutePath $WorkspaceRoot)) {
        throw "Workspace root must be absolute: $WorkspaceRoot"
    }
    $normalized = [System.IO.Path]::GetFullPath($WorkspaceRoot).Replace('/', '\')
    if ($env:OS -eq "Windows_NT") {
        $normalized = $normalized.ToLowerInvariant()
    }
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($normalized)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hashBytes = $sha.ComputeHash($bytes)
    } finally {
        $sha.Dispose()
    }
    $hex = -join ($hashBytes | ForEach-Object { $_.ToString("x2") })
    return $hex.Substring(0, 16)
}

function Get-RustCacheToolchainEpoch {
    param([string]$RustcCommand = "rustc")

    $lines = @(& $RustcCommand -Vv 2>$null)
    if ($LASTEXITCODE -ne 0 -or $lines.Count -eq 0) {
        throw "Unable to query Rust toolchain with '$RustcCommand -Vv'."
    }
    $releaseLine = $lines | Where-Object { $_ -match '^release:\s*(.+)$' } | Select-Object -First 1
    $commitLine = $lines | Where-Object { $_ -match '^commit-hash:\s*(.+)$' } | Select-Object -First 1
    $release = if ($releaseLine -match '^release:\s*(.+)$') { $Matches[1].Trim() } else { "unknown" }
    $commit = if ($commitLine -match '^commit-hash:\s*(.+)$') { $Matches[1].Trim() } else { "unknown" }
    if ($commit.Length -gt 12) {
        $commit = $commit.Substring(0, 12)
    }
    return "rustc-$(ConvertTo-RustCacheSlug $release)-$(ConvertTo-RustCacheSlug $commit)"
}

function Assert-RustCacheManagedPath {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)][string]$CandidatePath
    )

    $root = [System.IO.Path]::GetFullPath($CacheRoot).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    $candidate = [System.IO.Path]::GetFullPath($CandidatePath).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    $comparison = if ($env:OS -eq "Windows_NT") { [System.StringComparison]::OrdinalIgnoreCase } else { [System.StringComparison]::Ordinal }
    if (-not $candidate.StartsWith($root, $comparison)) {
        throw "Refusing to manage path outside Rust cache root. Root=$root Candidate=$candidate"
    }
    if ($candidate -eq $root) {
        throw "Refusing to treat the Rust cache root itself as a removable partition: $CacheRoot"
    }
}

Export-ModuleMember -Function Test-RustCacheAbsolutePath, Resolve-RustCacheRoot, Get-RustCacheMigrationAdvice, ConvertTo-RustCacheSlug, Get-RustCacheWorkspaceHash, Get-RustCacheToolchainEpoch, Assert-RustCacheManagedPath
