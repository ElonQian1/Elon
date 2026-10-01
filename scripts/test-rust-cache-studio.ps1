$ErrorActionPreference = 'Stop'
$modules = Join-Path $PSScriptRoot 'rust-cache'
Import-Module "$modules\RustCache.Runtime.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.Studio.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.Launcher.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.Portability.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.Run.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.NetworkStorage.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.Launcher.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.Runtime.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.Capacity.psm1" -DisableNameChecking
$studio = Get-Module RustCache.Studio
$capacity = Get-Module RustCache.Capacity
$runtime = Get-Module RustCache.Runtime
$saved = @{}
foreach($name in @('ELON_RUST_CACHE_ROOT','ELON_RUST_CACHE_CONTROL_ROOT','ELON_STUDIO_CACHE_PROFILE','ELON_RUST_CACHE_SCCACHE_TIERS','RUSTC_WRAPPER')) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name,'Process')
    [Environment]::SetEnvironmentVariable($name,$null,'Process')
}
$base = [IO.Path]::GetFullPath((Join-Path $env:TEMP ('studio-routing-test-' + [guid]::NewGuid().ToString('N'))))
$script:count = 0
function Assert-True([bool]$ok,[string]$why) { $script:count++; if(-not $ok){throw "Assertion failed: $why"} }
function Assert-Throws([scriptblock]$Action,[string]$Pattern) {
    $errorText = ''
    try { & $Action | Out-Null } catch { $errorText = $_.Exception.Message }
    Assert-True ($errorText -match $Pattern) "Expected $Pattern; got $errorText"
}
New-Item -ItemType Directory -Path $base | Out-Null
try {
    $legacy = Join-Path $base 'legacy'
    Assert-True ((Resolve-RustCacheManagementRoot $legacy) -eq $legacy) 'Legacy layout remains unchanged without opt-in.'
    $control = Join-Path $base 'control'
    $env:ELON_RUST_CACHE_CONTROL_ROOT = $control
    Assert-True ((Resolve-RustCacheManagementRoot $legacy) -eq $control) 'Control directory is independent of build directory.'
    Assert-True (-not (Test-Path -LiteralPath $control)) 'Path resolution does not create directories.'
    Assert-True ((Get-RustCacheUserLauncherContent $legacy).Contains((Join-Path $control 'platform\rust-cache.ps1'))) 'Portable launcher uses local platform.'
    Assert-True ((Get-RustCachePlatformInstallManifestPath $legacy) -eq (Join-Path $control 'platform\platform-install.json')) 'Doctor reads the same local install manifest.'
    Assert-Throws { Assert-RustCacheLocalControlPath '\\other-host\share\control' } 'CONTROL_PATH'
    Assert-Throws { Assert-RustCacheLocalControlPath 'C:relative' } 'CONTROL_PATH'
    Assert-Throws { Assert-RustCacheLocalControlPath 'C:' } 'CONTROL_PATH'
    Assert-Throws { Assert-RustCacheLocalControlPath '\relative' } 'CONTROL_PATH'
    $profilePath = Join-Path $base 'machine.json'
    $local = Join-Path $base 'local-build'
    $shared = Join-Path $base 'network-build-fixture'
    $profile = [pscustomobject]@{
        schema='elon.studio_cache.machine.v1';role='Client';machine_id_sha256=Get-RustCacheMachineIdentity
        control_root=$control;local_build_root=$local;shared_build_root=$shared;selected_cache_root=$local
        l1_cache_dir=(Join-Path $base 'l1');l1_max_bytes=2GB;sccache_tiers_path=(Join-Path $base 'tiers.json')
        allow_unc_build=$true;remote_build_ready=$false
    }
    $env:ELON_STUDIO_CACHE_PROFILE=$profilePath
    $env:ELON_RUST_CACHE_SCCACHE_TIERS=$profile.sccache_tiers_path
    @{schema='elon.rust_cache.sccache_tiers.v1';cache_root=$local;local=@{control_root=$control;cache_dir=$profile.l1_cache_dir;max_bytes=2GB;server_port=43219};remote=$null} | ConvertTo-Json | Set-Content -LiteralPath $profile.sccache_tiers_path -Encoding UTF8
    $profile | ConvertTo-Json | Set-Content -LiteralPath $profilePath -Encoding UTF8
    # Simulate capacity changes through the volume adapter, not through the route
    # implementation. TEMP always has space and each fixture has a distinct volume.
    & $capacity {
        param($Local,$Shared)
        $script:TestLocal=$Local; $script:TestShared=$Shared; $script:LocalFree=90GB; $script:SharedFree=90GB
        function script:Get-RustCacheCapacityVolume {
            param($Path)
            $key='temp';$free=90GB
            if($Path.StartsWith($script:TestLocal)){ $key='local';$free=$script:LocalFree }
            if($Path.StartsWith($script:TestShared)){ $key='network';$free=$script:SharedFree }
            [pscustomobject]@{storage_id=$key;total_bytes=100GB;free_bytes=$free}
        }
    } $local $shared
    $route=Select-RustCacheStudioBuildRoot -WorkspaceHash 'fixture'
    Assert-True ($route.root -eq $local) 'Space-admissible local build is preferred.'
    & $capacity {$script:LocalFree=1GB}
    $route=Select-RustCacheStudioBuildRoot -WorkspaceHash 'fixture'
    Assert-True ($route.root -eq $shared -and $route.reason -match 'exclusive-unc') 'Approved spill is selected when local cannot accept.'
    Assert-True (-not (Test-Path -LiteralPath $local) -and -not (Test-Path -LiteralPath $shared)) 'Route preview does not create build roots.'
    $profile.allow_unc_build=$false
    $profile | ConvertTo-Json | Set-Content -LiteralPath $profilePath -Encoding UTF8
    Assert-Throws { Select-RustCacheStudioBuildRoot -WorkspaceHash fixture } 'CAPACITY_UNAVAILABLE'
    $profile.remote_build_ready=$true
    $profile | ConvertTo-Json | Set-Content -LiteralPath $profilePath -Encoding UTF8
    Assert-Throws { Select-RustCacheStudioBuildRoot -WorkspaceHash fixture } 'REMOTE_BUILD_REQUIRED'
    $profile.machine_id_sha256='foreign-machine'
    $profile | ConvertTo-Json | Set-Content -LiteralPath $profilePath -Encoding UTF8
    Assert-Throws { Select-RustCacheStudioBuildRoot -WorkspaceHash fixture } 'PROFILE_IDENTITY'
    $project=Join-Path $base 'project'
    New-Item -ItemType Directory -Path $project | Out-Null
    '{"schema_version":1,"project_id":"studio-test","default_domain":"dev-host","allowed_domains":["dev-host"],"unknown_domain_fallback":"dev-host"}' | Set-Content -LiteralPath (Join-Path $project 'rust-cache.project.json')
    $profile.machine_id_sha256=Get-RustCacheMachineIdentity
    $profile.remote_build_ready=$false
    $profile | ConvertTo-Json | Set-Content -LiteralPath $profilePath -Encoding UTF8
    $env:ELON_RUST_CACHE_ROOT='\\offline-cache.invalid\missing\root'
    & $capacity {$script:LocalFree=90GB}
    Import-Module "$modules\RustCache.Paths.psm1" -DisableNameChecking
    Assert-True ((Resolve-RustCacheRoot -RepoRoot $project -NoCreate) -eq $local) 'Validation/network root resolution honors local-first before touching the offline default.'
    $run=Get-Module RustCache.Run
    & $run {
        function script:Invoke-RustCachePreflightGc {param($CacheRoot,$RepoRoot,[switch]$Skip) $script:ChosenGcRoot=$CacheRoot}
        function script:Invoke-RustCacheCargo {param($ProjectRoot,$Domain,$TargetDir,$CacheRoot,$NoLock,$DisableSccache,$LockTimeoutSeconds,$CargoCommand,$ToolchainEpoch,$SharedBuildPartition,$CargoArgs) $script:ChosenCargoRoot=$CacheRoot}
    }
    Invoke-RustCachePreparedCargo -ProjectRoot $project -CargoArgs @('check') -ToolchainEpoch fixture -SkipCacheGc
    Assert-True ((& $run {$script:ChosenGcRoot}) -eq $local) 'Offline default UNC is never passed to preflight GC before local selection.'
    Assert-True ((& $run {$script:ChosenCargoRoot}) -eq $local) 'The chosen root is fixed from GC through Cargo.'
    Remove-Item Env:ELON_RUST_CACHE_CONTROL_ROOT
    Assert-Throws { Get-RustCacheStudioProfile } 'PROFILE_INVALID'
    $env:ELON_RUST_CACHE_CONTROL_ROOT=$control
    $profile.l1_max_bytes=3GB
    $profile | ConvertTo-Json | Set-Content -LiteralPath $profilePath -Encoding UTF8
    Assert-Throws { Get-RustCacheStudioProfile } 'PROFILE_INVALID'
    Remove-Item Env:ELON_STUDIO_CACHE_PROFILE
    # Runtime admission failure must not call even a fake Cargo executable and
    # must preserve the original error and environment, including -NoLock.
    & $runtime {
        function script:Enter-RustCacheCapacityReservation { throw 'RUST_CACHE_CAPACITY_INSUFFICIENT_TEST' }
        function script:Test-FakeCargo { throw 'CARGO_MUST_NOT_RUN' }
    }
    $env:RUSTC_WRAPPER='before-test'
    Assert-Throws { Invoke-RustCacheCargo -ProjectRoot $project -CacheRoot $legacy -CargoCommand Test-FakeCargo -CargoArgs @('check') -ToolchainEpoch fixture -DisableSccache -NoLock } '^RUST_CACHE_CAPACITY_INSUFFICIENT_TEST$'
    Assert-True ($env:RUSTC_WRAPPER -eq 'before-test') 'Environment is restored on admission failure.'
    Write-Host "STUDIO_ROUTING_TEST_ASSERTIONS=$script:count"
    Write-Host 'STUDIO_ROUTING_TEST_RESULT=PASS'
} finally {
    foreach($name in $saved.Keys){[Environment]::SetEnvironmentVariable($name,$saved[$name],'Process')}
    $expectedPrefix=[IO.Path]::GetFullPath($env:TEMP).TrimEnd('\','/')+[IO.Path]::DirectorySeparatorChar
    if($base.StartsWith($expectedPrefix,[StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $base) -like 'studio-routing-test-*') {
        Remove-Item -LiteralPath $base -Recurse -Force
    }
}
