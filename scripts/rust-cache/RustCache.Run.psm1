Import-Module "$PSScriptRoot\RustCache.Inventory.psm1" -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Runtime.psm1" -DisableNameChecking

# Choose exactly once before GC. This keeps an offline default UNC root from
# blocking an admissible local route, and prevents GC against a different root.
function Invoke-RustCachePreparedCargo {
    param(
        [Parameter(Mandatory)][string]$ProjectRoot,
        [string]$Domain, [string]$TargetDir, [string]$CacheRoot,
        [switch]$NoLock, [switch]$DisableSccache, [switch]$SkipCacheGc,
        [int]$LockTimeoutSeconds=3600, [string]$CargoCommand='cargo',
        [string]$ToolchainEpoch, [string]$SharedBuildPartition,
        [Parameter(Mandatory)][string[]]$CargoArgs
    )
    $context = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -Domain $Domain -TargetDir $TargetDir -CacheRoot $CacheRoot -CargoArgs $CargoArgs -ToolchainEpoch $ToolchainEpoch -SharedBuildPartition $SharedBuildPartition
    Invoke-RustCachePreflightGc -CacheRoot $context.cache_root -RepoRoot $ProjectRoot -Skip:$SkipCacheGc | Out-Null
    Write-Host "RUST_CACHE_PLANNED_ROUTE=$($context.route_reason)"
    Invoke-RustCacheCargo -ProjectRoot $ProjectRoot -Domain $Domain -TargetDir $context.target_dir -CacheRoot $context.cache_root -NoLock:$NoLock -DisableSccache:$DisableSccache -LockTimeoutSeconds $LockTimeoutSeconds -CargoCommand $CargoCommand -ToolchainEpoch $context.toolchain_epoch -SharedBuildPartition $SharedBuildPartition -CargoArgs $CargoArgs
}

Export-ModuleMember -Function Invoke-RustCachePreparedCargo
