Import-Module "$PSScriptRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Registry.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Sccache.psm1" -DisableNameChecking

function Resolve-RustCacheWorkspaceRoot {
    param(
        [Parameter(Mandatory)][string]$ProjectRoot,
        [string[]]$CargoArgs = @()
    )

    $manifestValue = $null
    for ($index = 0; $index -lt $CargoArgs.Count; $index++) {
        if ($CargoArgs[$index] -eq "--manifest-path" -and $index + 1 -lt $CargoArgs.Count) {
            $manifestValue = $CargoArgs[$index + 1]
            break
        }
        if ($CargoArgs[$index] -match '^--manifest-path=(.+)$') {
            $manifestValue = $Matches[1]
            break
        }
    }

    if ([string]::IsNullOrWhiteSpace($manifestValue)) {
        return [System.IO.Path]::GetFullPath($ProjectRoot)
    }
    $manifestPath = if ([System.IO.Path]::IsPathRooted($manifestValue)) {
        $manifestValue
    } else {
        Join-Path $ProjectRoot $manifestValue
    }
    return [System.IO.Path]::GetFullPath((Split-Path $manifestPath -Parent))
}

function Test-RustCacheReleaseInvocation {
    param([string[]]$CargoArgs = @())

    if ($CargoArgs -contains "--release") {
        return $true
    }
    for ($index = 0; $index -lt $CargoArgs.Count; $index++) {
        if ($CargoArgs[$index] -eq "--profile" -and $index + 1 -lt $CargoArgs.Count -and $CargoArgs[$index + 1] -eq "release") {
            return $true
        }
        if ($CargoArgs[$index] -eq "--profile=release") {
            return $true
        }
    }
    return $false
}

function Resolve-RustCacheInvocation {
    param(
        [Parameter(Mandatory)][string]$ProjectRoot,
        [string]$Domain,
        [string]$TargetDir,
        [string]$CacheRoot,
        [string[]]$CargoArgs = @(),
        [string]$ToolchainEpoch
    )

    $project = [System.IO.Path]::GetFullPath($ProjectRoot)
    $root = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $project
    $manifest = Get-RustCacheProjectManifest -ProjectRoot $project
    $resolvedDomain = if ([string]::IsNullOrWhiteSpace($Domain)) { $manifest.default_domain } else { ConvertTo-RustCacheSlug $Domain }
    $workspace = Resolve-RustCacheWorkspaceRoot -ProjectRoot $project -CargoArgs $CargoArgs
    $workspaceHash = Get-RustCacheWorkspaceHash -WorkspaceRoot $workspace
    $epoch = if ([string]::IsNullOrWhiteSpace($ToolchainEpoch)) { Get-RustCacheToolchainEpoch } else { ConvertTo-RustCacheSlug $ToolchainEpoch }
    if ($manifest.registered) {
        $buildDir = Join-Path $root "build\$epoch\$($manifest.project_id)\$resolvedDomain\$workspaceHash"
    } else {
        $buildDir = Join-Path $root "quarantine\$workspaceHash"
    }
    $resolvedTarget = if ([string]::IsNullOrWhiteSpace($TargetDir)) {
        Join-Path $workspace "target"
    } else {
        if (-not (Test-RustCacheAbsolutePath $TargetDir)) {
            throw "Cargo target directory must be absolute: $TargetDir"
        }
        [System.IO.Path]::GetFullPath($TargetDir)
    }
    [pscustomobject]@{
        cache_root = $root
        project_id = $manifest.project_id
        project_root = $project
        workspace_root = $workspace
        workspace_hash = $workspaceHash
        domain = $resolvedDomain
        toolchain_epoch = $epoch
        build_dir = $buildDir
        target_dir = $resolvedTarget
        registered = $manifest.registered
        release = Test-RustCacheReleaseInvocation -CargoArgs $CargoArgs
    }
}

function Get-RustCacheLockOwner {
    param([Parameter(Mandatory)][string]$LockPath)

    $ownerPath = Join-Path $LockPath "owner.json"
    if (-not (Test-Path -LiteralPath $ownerPath)) {
        return $null
    }
    try {
        Get-Content -Raw -LiteralPath $ownerPath -Encoding UTF8 | ConvertFrom-Json
    } catch {
        $null
    }
}

function Test-RustCacheOwnerProcessAlive {
    param([AllowNull()]$Owner)

    if ($null -eq $Owner -or $null -eq $Owner.pid) {
        return $false
    }
    $process = Get-Process -Id ([int]$Owner.pid) -ErrorAction SilentlyContinue
    return $null -ne $process
}

function Enter-RustCacheLock {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)][string]$BuildDir,
        [Parameter(Mandatory)][string]$WorkspaceRoot,
        [int]$TimeoutSeconds = 3600
    )

    Assert-RustCacheManagedPath -CacheRoot $CacheRoot -CandidatePath $BuildDir
    New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null
    $lockPath = Join-Path $BuildDir ".rust-cache.lockdir"
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ($true) {
        try {
            New-Item -ItemType Directory -Path $lockPath -ErrorAction Stop | Out-Null
            $owner = [ordered]@{
                pid = $PID
                started_utc = [DateTime]::UtcNow.ToString("o")
                workspace_root = $WorkspaceRoot
            }
            $owner | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $lockPath "owner.json") -Encoding UTF8
            return $lockPath
        } catch {
            $existingOwner = Get-RustCacheLockOwner -LockPath $lockPath
            if (-not (Test-RustCacheOwnerProcessAlive -Owner $existingOwner)) {
                Remove-Item -LiteralPath $lockPath -Recurse -Force -ErrorAction SilentlyContinue
                continue
            }
            if ([DateTime]::UtcNow -ge $deadline) {
                $ownerText = if ($existingOwner) { $existingOwner | ConvertTo-Json -Compress } else { "unknown" }
                throw "Timed out waiting for Rust cache lock: $lockPath owner=$ownerText"
            }
            Start-Sleep -Seconds 2
        }
    }
}

function Exit-RustCacheLock {
    param([AllowNull()][string]$LockPath)

    if ([string]::IsNullOrWhiteSpace($LockPath)) {
        return
    }
    $owner = Get-RustCacheLockOwner -LockPath $LockPath
    if ($owner -and [int]$owner.pid -eq $PID) {
        Remove-Item -LiteralPath $LockPath -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Save-RustCacheEnvironment {
    param([string[]]$Names)

    $snapshot = @{}
    foreach ($name in $Names) {
        $snapshot[$name] = [pscustomobject]@{
            exists = Test-Path "Env:$name"
            value = [Environment]::GetEnvironmentVariable($name, "Process")
        }
    }
    return $snapshot
}

function Restore-RustCacheEnvironment {
    param([hashtable]$Snapshot)

    foreach ($name in $Snapshot.Keys) {
        if ($Snapshot[$name].exists) {
            [Environment]::SetEnvironmentVariable($name, $Snapshot[$name].value, "Process")
        } else {
            Remove-Item "Env:$name" -ErrorAction SilentlyContinue
        }
    }
}

function Get-RustCacheSccacheBaseDirs {
    param(
        [Parameter(Mandatory)][string]$ProjectRoot,
        [Parameter(Mandatory)][string]$WorkspaceRoot
    )

    $paths = New-Object System.Collections.Generic.List[string]
    foreach ($candidate in @($WorkspaceRoot, $ProjectRoot)) {
        $fullPath = [System.IO.Path]::GetFullPath($candidate).TrimEnd('\', '/')
        if (-not ($paths | Where-Object { $_ -ieq $fullPath })) {
            $paths.Add($fullPath)
        }
    }
    return $paths -join [System.IO.Path]::PathSeparator
}

function Set-RustCacheBuildEnvironment {
    param(
        [Parameter(Mandatory)][string]$ProjectRoot,
        [string]$Domain,
        [string]$TargetDir,
        [string]$CacheRoot,
        [switch]$DisableSccache,
        [string]$ToolchainEpoch,
        [string[]]$CargoArgs = @()
    )

    $context = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -Domain $Domain -TargetDir $TargetDir -CacheRoot $CacheRoot -CargoArgs $CargoArgs -ToolchainEpoch $ToolchainEpoch
    New-Item -ItemType Directory -Force -Path $context.build_dir, $context.target_dir | Out-Null
    $env:CARGO_BUILD_BUILD_DIR = $context.build_dir
    $env:CARGO_TARGET_DIR = $context.target_dir
    if ($context.release -or $context.domain -match '(^|-)validation($|-)|agent-validation') {
        $env:CARGO_INCREMENTAL = "0"
    }
    Update-RustCacheRegistry -CacheRoot $context.cache_root -ProjectId $context.project_id -ProjectRoot $context.project_root -WorkspaceRoot $context.workspace_root -WorkspaceHash $context.workspace_hash -Domain $context.domain -ToolchainEpoch $context.toolchain_epoch -BuildDir $context.build_dir -TargetDir $context.target_dir -Registered $context.registered
    $sccache = if ($DisableSccache) { $null } else { Get-Command sccache -ErrorAction SilentlyContinue }
    if ($sccache) {
        $sync = Sync-RustCacheSccacheConfiguration -CacheRoot $context.cache_root -AdditionalBaseDirs @($context.build_dir, $context.target_dir, $context.workspace_root, $context.project_root) -ConfigureProcessEnvironment -RestartIfChanged
        if ($sync.restart_pending) {
            Write-Warning "sccache base-directory configuration changed but restart is deferred while another Cargo/rustc process is active."
        }
        if ([string]::IsNullOrWhiteSpace($env:RUSTC_WRAPPER)) {
            $managedWrapper = Get-RustCacheSccacheWrapperPath -CacheRoot $context.cache_root
            $env:RUSTC_WRAPPER = if (Test-Path -LiteralPath $managedWrapper) { $managedWrapper } else { $sccache.Source }
        }
    }
    $marker = [ordered]@{
        project_id = $context.project_id
        workspace_root = $context.workspace_root
        domain = $context.domain
        toolchain_epoch = $context.toolchain_epoch
        last_used_utc = [DateTime]::UtcNow.ToString("o")
        pid = $PID
    }
    $marker | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $context.build_dir ".last-used.json") -Encoding UTF8
    return $context
}

function Get-RustCacheSccacheReadiness {
    param([switch]$Disabled)
    if ($Disabled) {
        return [pscustomobject]@{ status="unavailable"; path=$null; reason="disabled_by_caller"; stats=$null }
    }
    $command = Get-Command sccache -ErrorAction SilentlyContinue
    if (-not $command) {
        return [pscustomobject]@{ status="unavailable"; path=$null; reason="not_installed; install sccache explicitly, then run scripts/rust-cache.ps1 install -Apply"; stats=$null }
    }
    $stats = $null
    try {
        $raw = (& $command.Source --show-stats --stats-format json 2>$null) -join "`n"
        if ($LASTEXITCODE -eq 0 -and $raw) { $stats = $raw | ConvertFrom-Json }
    } catch { }
    return [pscustomobject]@{ status="ready"; path=$command.Source; reason=if ($stats) { $null } else { "statistics_unavailable" }; stats=$stats }
}

function Invoke-RustCacheCargo {
    param(
        [Parameter(Mandatory)][string]$ProjectRoot,
        [string]$Domain,
        [string]$TargetDir,
        [string]$CacheRoot,
        [switch]$NoLock,
        [switch]$DisableSccache,
        [int]$LockTimeoutSeconds = 3600,
        [string]$CargoCommand = "cargo",
        [string]$ToolchainEpoch,
        [Parameter(Mandatory)][string[]]$CargoArgs
    )

    $context = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -Domain $Domain -TargetDir $TargetDir -CacheRoot $CacheRoot -CargoArgs $CargoArgs -ToolchainEpoch $ToolchainEpoch
    New-Item -ItemType Directory -Force -Path $context.build_dir | Out-Null
    $lockPath = $null
    $locationPushed = $false
    $envNames = @("CARGO_BUILD_BUILD_DIR", "CARGO_TARGET_DIR", "CARGO_INCREMENTAL", "RUSTC_WRAPPER", "SCCACHE_DIR", "SCCACHE_CONF", "SCCACHE_CACHE_SIZE")
    $environment = Save-RustCacheEnvironment -Names $envNames
    try {
        if (-not $NoLock) {
            Write-Host "Waiting for Rust cache partition lock: $($context.build_dir)"
            $lockPath = Enter-RustCacheLock -CacheRoot $context.cache_root -BuildDir $context.build_dir -WorkspaceRoot $context.workspace_root -TimeoutSeconds $LockTimeoutSeconds
        }

        $context = Set-RustCacheBuildEnvironment -ProjectRoot $ProjectRoot -Domain $Domain -TargetDir $TargetDir -CacheRoot $CacheRoot -DisableSccache:$DisableSccache -ToolchainEpoch $ToolchainEpoch -CargoArgs $CargoArgs
        $sccache = if ($DisableSccache) { $null } else { Get-Command sccache -ErrorAction SilentlyContinue }
        $readiness = Get-RustCacheSccacheReadiness -Disabled:$DisableSccache

        Write-Host "RUST_CACHE_PROJECT=$($context.project_id)"
        Write-Host "RUST_CACHE_DOMAIN=$($context.domain)"
        Write-Host "CARGO_BUILD_BUILD_DIR=$($context.build_dir)"
        Write-Host "CARGO_TARGET_DIR=$($context.target_dir)"
        $alternative = if ($env:ELON_NODE_DATA_ROOT) { Join-Path $env:ELON_NODE_DATA_ROOT "cache\rust-cache-v2" } else { $null }
        $migration = Get-RustCacheMigrationAdvice -CacheRoot $context.cache_root -ManagedAlternativeRoot $alternative
        Write-Host ("RUST_CACHE_MIGRATION_ADVICE=" + ($migration | ConvertTo-Json -Compress))
        Write-Host "SCCACHE_STATUS=$($readiness.status)"
        Write-Host "SCCACHE_PATH=$($readiness.path)"
        if ($readiness.reason) { Write-Host "SCCACHE_DEGRADED_REASON=$($readiness.reason)" }
        if ($readiness.stats) {
            if ($null -ne $readiness.stats.cache_hits) { Write-Host "SCCACHE_CACHE_HITS=$($readiness.stats.cache_hits)" }
            if ($null -ne $readiness.stats.cache_misses) { Write-Host "SCCACHE_CACHE_MISSES=$($readiness.stats.cache_misses)" }
        }
        Write-Host "CARGO_INCREMENTAL_EFFECTIVE=$env:CARGO_INCREMENTAL"
        if ($sccache) {
            Write-Host "RUSTC_WRAPPER=$env:RUSTC_WRAPPER"
            Write-Host "SCCACHE_DIR=$env:SCCACHE_DIR"
        } else {
            Write-Warning "sccache is unavailable or disabled; Cargo will still use the isolated build-dir."
        }
        Write-Host "$CargoCommand $($CargoArgs -join ' ')"
        Push-Location -LiteralPath $context.project_root
        $locationPushed = $true
        & $CargoCommand @CargoArgs
    } finally {
        if ($locationPushed) { Pop-Location }
        Exit-RustCacheLock -LockPath $lockPath
        Restore-RustCacheEnvironment -Snapshot $environment
    }
}

Export-ModuleMember -Function Resolve-RustCacheWorkspaceRoot, Test-RustCacheReleaseInvocation, Resolve-RustCacheInvocation, Get-RustCacheLockOwner, Test-RustCacheOwnerProcessAlive, Enter-RustCacheLock, Exit-RustCacheLock, Get-RustCacheSccacheBaseDirs, Get-RustCacheSccacheReadiness, Set-RustCacheBuildEnvironment, Invoke-RustCacheCargo
