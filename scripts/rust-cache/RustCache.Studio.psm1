Import-Module "$PSScriptRoot\RustCache.NetworkStorage.psm1" -DisableNameChecking

function Assert-RustCacheLocalControlPath {
    param([Parameter(Mandatory)][string]$Path)
    if (-not [IO.Path]::IsPathRooted($Path) -or $Path -match '^[A-Za-z]:($|[^\\/])|^[\\/][^\\/]' -or $Path.StartsWith('\\?\')) {
        throw 'RUST_CACHE_CONTROL_PATH: A local absolute path is required.'
    }
    $full = [IO.Path]::GetFullPath($Path)
    if ($full.TrimEnd('\','/') -eq [IO.Path]::GetPathRoot($full).TrimEnd('\','/')) {
        throw 'RUST_CACHE_CONTROL_PATH: A volume root cannot be the control directory.'
    }
    if (Test-RustCacheNetworkPath $full) { throw 'RUST_CACHE_CONTROL_PATH: Control files must be local.' }
    $parent = [IO.DirectoryInfo]::new($full)
    while ($null -ne $parent) {
        $item = Get-Item -LiteralPath $parent.FullName -Force -ErrorAction SilentlyContinue
        if ($item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw 'RUST_CACHE_CONTROL_PATH: Reparse points are not supported for control files.'
        }
        $parent = $parent.Parent
    }
    return $full
}

function Resolve-RustCacheManagementRoot {
    param([Parameter(Mandatory)][string]$CacheRoot)
    if ([string]::IsNullOrWhiteSpace($env:ELON_RUST_CACHE_CONTROL_ROOT)) {
        if ($env:ELON_RUST_CACHE_SCCACHE_TIERS) {
            $tiersPath = Assert-RustCacheLocalControlPath $env:ELON_RUST_CACHE_SCCACHE_TIERS
            $tiers = Get-Content -LiteralPath $tiersPath -Raw -Encoding UTF8 | ConvertFrom-Json -ErrorAction Stop
            if ($tiers.schema -ne 'elon.rust_cache.sccache_tiers.v1') { throw 'RUST_CACHE_CONTROL_PATH: Unknown tiers schema.' }
            $control = [string]$tiers.local.control_root
            if (-not $control) { $control = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Elon\rust-cache-control-v1' }
            return Assert-RustCacheLocalControlPath $control
        }
        return [IO.Path]::GetFullPath($CacheRoot)
    }
    return Assert-RustCacheLocalControlPath $env:ELON_RUST_CACHE_CONTROL_ROOT
}

function Get-RustCacheStudioProfile {
    if ([string]::IsNullOrWhiteSpace($env:ELON_STUDIO_CACHE_PROFILE)) { return $null }
    $path = Assert-RustCacheLocalControlPath $env:ELON_STUDIO_CACHE_PROFILE
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw 'STUDIO_CACHE_PROFILE_MISSING: Run the machine bootstrap again.' }
    $profile = Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json -ErrorAction Stop
    if ($profile.schema -ne 'elon.studio_cache.machine.v1' -or
        $profile.machine_id_sha256 -cne (Get-RustCacheMachineIdentity) -or
        $profile.role -notin @('Host','Client')) { throw 'STUDIO_CACHE_PROFILE_IDENTITY: Wrong schema, machine or role.' }
    foreach ($name in @('control_root','local_build_root','l1_cache_dir','sccache_tiers_path')) {
        if (-not $profile.$name) { throw "STUDIO_CACHE_PROFILE_INVALID: Missing $name." }
        Assert-RustCacheLocalControlPath ([string]$profile.$name) | Out-Null
    }
    if ($profile.allow_unc_build -isnot [bool] -or $profile.remote_build_ready -isnot [bool]) {
        throw 'STUDIO_CACHE_PROFILE_INVALID: Routing switches must be JSON booleans.'
    }
    if (-not $env:ELON_RUST_CACHE_CONTROL_ROOT -or -not $env:ELON_RUST_CACHE_SCCACHE_TIERS -or
        [IO.Path]::GetFullPath($profile.sccache_tiers_path) -ne [IO.Path]::GetFullPath($env:ELON_RUST_CACHE_SCCACHE_TIERS) -or
        [IO.Path]::GetFullPath($profile.control_root) -ne [IO.Path]::GetFullPath($env:ELON_RUST_CACHE_CONTROL_ROOT)) {
        throw 'STUDIO_CACHE_PROFILE_INVALID: The profile and active control directory disagree.'
    }
    try { $tiers = Get-Content -LiteralPath $profile.sccache_tiers_path -Raw -Encoding UTF8 | ConvertFrom-Json -ErrorAction Stop }
    catch { throw 'STUDIO_CACHE_PROFILE_INVALID: The local tiers configuration is missing or invalid.' }
    $tierDir = if($tiers.local.cache_dir){[string]$tiers.local.cache_dir}else{Join-Path $profile.control_root 'sccache-l1'}
    $tierSize = if($null -ne $tiers.local.max_bytes){$tiers.local.max_bytes}else{2147483648}
    if ($tiers.schema -ne 'elon.rust_cache.sccache_tiers.v1' -or
        [IO.Path]::GetFullPath($tierDir) -ne [IO.Path]::GetFullPath($profile.l1_cache_dir) -or
        [long]$tierSize -ne [long]$profile.l1_max_bytes) {
        throw 'STUDIO_CACHE_PROFILE_INVALID: Profile L1 metadata disagrees with the active tiers configuration.'
    }
    $shared = [string]$profile.shared_build_root
    if (-not [IO.Path]::IsPathRooted($shared) -or $shared -match '^[A-Za-z]:($|[^\\/])|^[\\/][^\\/]' -or $shared.StartsWith('\\?\')) {
        throw 'STUDIO_CACHE_PROFILE_INVALID: Shared build root must be an ordinary absolute path.'
    }
    if ([string]$profile.selected_cache_root -notin @([string]$profile.local_build_root,$shared)) {
        throw 'STUDIO_CACHE_PROFILE_INVALID: The default management root must match a build root in this machine profile.'
    }
    return $profile
}

function Select-RustCacheStudioBuildRoot {
    param([string]$ExplicitRoot, [string]$TargetDir, [Parameter(Mandatory)][string]$WorkspaceHash)
    $profile = Get-RustCacheStudioProfile
    if ($ExplicitRoot -or $null -eq $profile) {
        return [pscustomobject]@{ root=$ExplicitRoot; managed_target=($null -ne $profile); reason=if($ExplicitRoot){'explicit-root'}else{'legacy-root'} }
    }
    Import-Module "$PSScriptRoot\RustCache.Capacity.psm1" -DisableNameChecking
    Import-Module "$PSScriptRoot\RustCache.Policy.psm1" -DisableNameChecking
    # The host's physical shared path is its local build root. Clients try local
    # space first. A network write target is never inferred or used without opt-in.
    $candidates = @([pscustomobject]@{root=[string]$profile.local_build_root;reason='local-capacity-available'})
    if ($profile.role -eq 'Client' -and $profile.allow_unc_build -and -not $profile.remote_build_ready) {
        $candidates += [pscustomobject]@{root=[string]$profile.shared_build_root;reason='local-capacity-insufficient-exclusive-unc'}
    }
    foreach ($candidate in $candidates) {
        $root = [IO.Path]::GetFullPath($candidate.root)
        if (Test-Path -LiteralPath $root) { Assert-RustCacheNetworkRootOwner -CacheRoot $root }
        $policy = Get-DefaultRustCachePolicy
        $policyPath = Join-Path $root 'config\policy.json'
        if (Test-Path -LiteralPath $policyPath -PathType Leaf) { $policy = Get-RustCachePolicy -CacheRoot $root }
        $target = if($TargetDir){$TargetDir}else{Join-Path $root "targets\$WorkspaceHash"}
        $plan = Get-RustCacheCapacityPlan -CacheRoot $root -BuildDir (Join-Path $root 'build') -TargetDir $target -Policy $policy
        if ($plan.admissible) {
            return [pscustomobject]@{root=$root;managed_target=$true;reason=$candidate.reason}
        }
    }
    if ($profile.remote_build_ready) {
        throw 'STUDIO_REMOTE_BUILD_REQUIRED: Local capacity is insufficient. Submit an exact source revision to the configured build host; this process did not start Cargo.'
    }
    throw 'STUDIO_CACHE_CAPACITY_UNAVAILABLE: No approved build location has enough reserved space. Cargo was not started; no fallback directory was created.'
}

Export-ModuleMember -Function Assert-RustCacheLocalControlPath, Resolve-RustCacheManagementRoot, Get-RustCacheStudioProfile, Select-RustCacheStudioBuildRoot
