<#
.SYNOPSIS
    Route Rust cross-target builds through the managed cache platform.

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-cross.ps1 -- `
      zigbuild --target x86_64-unknown-linux-musl --manifest-path server\Cargo.toml --locked
#>
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$argumentModule = Join-Path $PSScriptRoot 'validation\Validation.Arguments.psm1'
Import-Module $argumentModule -Force -DisableNameChecking
$parsed = Split-ValidationCargoArguments -Arguments $args -ValueOptions @{
    '-CacheRoot' = 'CacheRoot'
    '-LockTimeoutSeconds' = 'LockTimeoutSeconds'
} -SwitchOptions @('-DisableSccache', '-SkipCacheGc', '-PlanOnly')

$cacheRoot = if ($parsed.wrapper.ContainsKey('CacheRoot')) {
    [string]$parsed.wrapper.CacheRoot
} else {
    ''
}
$lockTimeoutSeconds = if ($parsed.wrapper.ContainsKey('LockTimeoutSeconds')) {
    [int]$parsed.wrapper.LockTimeoutSeconds
} else {
    3600
}
$disableSccache = $parsed.wrapper.ContainsKey('DisableSccache')
$skipCacheGc = $parsed.wrapper.ContainsKey('SkipCacheGc')
$planOnly = $parsed.wrapper.ContainsKey('PlanOnly')
$cargoArgs = @($parsed.cargo)

function Resolve-CrossTargetTriple {
    param([Parameter(Mandatory)][string[]]$CargoArgs)

    $targets = [Collections.Generic.List[string]]::new()
    for ($index = 0; $index -lt $CargoArgs.Count; $index++) {
        $argument = [string]$CargoArgs[$index]
        if ($argument -eq '--target') {
            if (++$index -ge $CargoArgs.Count) {
                throw 'Cargo --target requires a value.'
            }
            $targets.Add([string]$CargoArgs[$index])
            continue
        }
        if ($argument.StartsWith('--target=')) {
            $targets.Add($argument.Substring('--target='.Length))
        }
    }

    if ($targets.Count -eq 0) {
        throw 'Cross-target builds require one explicit Cargo --target <standard-triple>.'
    }
    $distinct = @($targets | ForEach-Object { $_.Trim() } | Sort-Object -Unique)
    if ($distinct.Count -ne 1) {
        throw "Cross-target builds require exactly one target triple; received: $($distinct -join ', ')."
    }
    $target = [string]$distinct[0]
    if ($target.Length -gt 128 -or $target -cnotmatch '^[a-z0-9][a-z0-9._-]*$') {
        throw "Cross-target V1 accepts only a lowercase standard target triple, not a path or custom JSON target: $target"
    }
    return $target
}

if ($cargoArgs.Count -eq 0) {
    throw 'Usage: cargo-cross.ps1 [wrapper-options] -- <cargo-subcommand> --target <standard-triple> ...'
}
if ($lockTimeoutSeconds -lt 1 -or $lockTimeoutSeconds -gt 86400) {
    throw '-LockTimeoutSeconds must be between 1 and 86400.'
}

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
& git -C $repoRoot rev-parse --is-inside-work-tree *> $null
if ($LASTEXITCODE -ne 0) {
    throw 'cargo-cross.ps1 must run from a Git-backed project checkout.'
}

$target = Resolve-CrossTargetTriple -CargoArgs $cargoArgs
$sharedBuildPartition = "cross-$target"
$targetDir = [IO.Path]::GetFullPath(
    (Join-Path $repoRoot (Join-Path '.ai-tmp\cargo-cross-target' $target))
)
$plan = [ordered]@{
    schema = 'elon.rust_cross_cache_plan.v1'
    project_root = $repoRoot
    target = $target
    cache_domain = 'agent-validation'
    shared_build_partition = $sharedBuildPartition
    target_dir = $targetDir
    target_lifecycle = 'task_temporary'
    intermediate_lifecycle = 'rust_cache_v2_managed'
    cargo_args = @($cargoArgs)
}

Write-Host "RUST_CROSS_TARGET=$target"
Write-Host 'RUST_CROSS_CACHE_DOMAIN=agent-validation'
Write-Host "RUST_CROSS_SHARED_BUILD_PARTITION=$sharedBuildPartition"
Write-Host "RUST_CROSS_TARGET_DIR=$targetDir"
Write-Host ('RUST_CROSS_PLAN_JSON=' + ($plan | ConvertTo-Json -Depth 5 -Compress))
if ($planOnly) {
    Write-Host 'RUST_CROSS_PLAN_ONLY=true'
    exit 0
}

$rustCacheRoot = Join-Path $PSScriptRoot 'rust-cache'
Import-Module (Join-Path $rustCacheRoot 'RustCache.Inventory.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $rustCacheRoot 'RustCache.Runtime.psm1') -Force -DisableNameChecking
Invoke-RustCachePreflightGc -CacheRoot $cacheRoot -RepoRoot $repoRoot -Skip:$skipCacheGc | Out-Null
Invoke-RustCacheCargo -ProjectRoot $repoRoot -Domain 'agent-validation' -TargetDir $targetDir `
    -CacheRoot $cacheRoot -DisableSccache:$disableSccache -LockTimeoutSeconds $lockTimeoutSeconds `
    -SharedBuildPartition $sharedBuildPartition -CargoArgs $cargoArgs
$cargoExitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
if ($cargoExitCode -ne 0) {
    $host.SetShouldExit($cargoExitCode)
    Write-Error "Cross-target Cargo failed with exit code $cargoExitCode." -ErrorAction Continue
}
exit $cargoExitCode
