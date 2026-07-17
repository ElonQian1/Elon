function Get-RustCacheRegistryPath {
    param([Parameter(Mandatory)][string]$CacheRoot)
    Join-Path $CacheRoot "state\registry.json"
}

function Read-RustCacheRegistry {
    param([Parameter(Mandatory)][string]$CacheRoot)

    $path = Get-RustCacheRegistryPath -CacheRoot $CacheRoot
    if (-not (Test-Path -LiteralPath $path)) {
        return [pscustomobject]@{ schema_version = 1; workspaces = @() }
    }
    try {
        $registry = Get-Content -Raw -LiteralPath $path -Encoding UTF8 | ConvertFrom-Json
    } catch {
        throw "Invalid Rust cache registry JSON at $path. $($_.Exception.Message)"
    }
    if ($registry.schema_version -ne 1) {
        throw "Unsupported Rust cache registry schema_version '$($registry.schema_version)' at $path."
    }
    if ($null -eq $registry.workspaces) {
        $registry | Add-Member -NotePropertyName workspaces -NotePropertyValue @()
    }
    return $registry
}

function Write-RustCacheRegistry {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)]$Registry
    )

    $path = Get-RustCacheRegistryPath -CacheRoot $CacheRoot
    $parent = Split-Path $path -Parent
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    $temporary = Join-Path $parent ("registry.{0}.{1}.tmp" -f $PID, [Guid]::NewGuid().ToString("N"))
    try {
        $Registry | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $temporary -Encoding UTF8
        Move-Item -LiteralPath $temporary -Destination $path -Force
    } finally {
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    }
    return $path
}

function Enter-RustCacheRegistryLock {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [int]$TimeoutSeconds = 30
    )

    $lockPath = Join-Path $CacheRoot "state\registry.lock"
    New-Item -ItemType Directory -Force -Path (Split-Path $lockPath -Parent) | Out-Null
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        try {
            return [System.IO.File]::Open($lockPath, [System.IO.FileMode]::OpenOrCreate, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
        } catch [System.IO.IOException] {
            Start-Sleep -Milliseconds 100
        }
    }
    throw "Timed out waiting for Rust cache registry lock: $lockPath"
}

function Update-RustCacheRegistry {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)][string]$ProjectId,
        [Parameter(Mandatory)][string]$ProjectRoot,
        [Parameter(Mandatory)][string]$WorkspaceRoot,
        [Parameter(Mandatory)][string]$WorkspaceHash,
        [Parameter(Mandatory)][string]$Domain,
        [Parameter(Mandatory)][string]$ToolchainEpoch,
        [Parameter(Mandatory)][string]$BuildDir,
        [Parameter(Mandatory)][string]$TargetDir,
        [bool]$Registered
    )

    $lock = Enter-RustCacheRegistryLock -CacheRoot $CacheRoot
    try {
        $registry = Read-RustCacheRegistry -CacheRoot $CacheRoot
        $items = @($registry.workspaces)
        $existing = $items | Where-Object { $_.workspace_hash -eq $WorkspaceHash -and $_.domain -eq $Domain } | Select-Object -First 1
        $values = [ordered]@{
            project_id = $ProjectId
            project_root = $ProjectRoot
            workspace_root = $WorkspaceRoot
            workspace_hash = $WorkspaceHash
            domain = $Domain
            toolchain_epoch = $ToolchainEpoch
            build_dir = $BuildDir
            target_dir = $TargetDir
            registered = $Registered
            last_seen_utc = [DateTime]::UtcNow.ToString("o")
        }
        if ($existing) {
            foreach ($key in $values.Keys) { $existing.$key = $values[$key] }
        } else {
            $items += [pscustomobject]$values
        }
        $registry.workspaces = @($items | Sort-Object project_id, domain, workspace_hash)
        Write-RustCacheRegistry -CacheRoot $CacheRoot -Registry $registry | Out-Null
    } finally {
        if ($lock) { $lock.Dispose() }
    }
}

Export-ModuleMember -Function Get-RustCacheRegistryPath, Read-RustCacheRegistry, Write-RustCacheRegistry, Enter-RustCacheRegistryLock, Update-RustCacheRegistry
