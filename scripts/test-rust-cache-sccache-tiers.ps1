$ErrorActionPreference = 'Stop'
$modules = Join-Path $PSScriptRoot 'rust-cache'
Import-Module "$modules\RustCache.Install.psm1" -Force -DisableNameChecking
Import-Module "$modules\RustCache.Sccache.psm1" -Force -DisableNameChecking
Import-Module "$modules\RustCache.SccacheTiers.psm1" -Force -DisableNameChecking
Import-Module "$modules\RustCache.Studio.psm1" -DisableNameChecking
$script:Assertions = 0
function Assert-That([bool]$Condition,[string]$Message) {
    $script:Assertions++
    if (-not $Condition) { throw "ASSERT FAILED: $Message" }
}
function Assert-Rejected([scriptblock]$Action,[string]$Message) {
    $rejected=$false
    try { & $Action | Out-Null } catch { $rejected=$true }
    Assert-That $rejected $Message
}
function Write-FixtureJson($Value,[string]$Path) {
    [IO.File]::WriteAllText($Path,($Value|ConvertTo-Json -Depth 10),(New-Object Text.UTF8Encoding($false)))
}
function New-FixtureConfig {
    [pscustomobject]@{schema='elon.rust_cache.sccache_tiers.v1';cache_root=$build
        local=[pscustomobject]@{control_root=$control;cache_dir=$objects;max_bytes=67108864;server_port=$port};remote=$null}
}
$environmentNames = @(Get-RustCacheSccacheEnvironmentNames) + @('ELON_RUST_CACHE_SCCACHE_TIERS','ELON_RUST_CACHE_CONTROL_ROOT','ELON_STUDIO_CACHE_PROFILE','SCCACHE_WEBDAV_TOKEN','ELON_RUST_CACHE_ROOT')
$previousEnvironment=@{}
foreach($name in $environmentNames){$previousEnvironment[$name]=[Environment]::GetEnvironmentVariable($name,'Process');Remove-Item -LiteralPath ('Env:'+$name) -ErrorAction SilentlyContinue}
$root = Join-Path (Split-Path $PSScriptRoot -Parent) ('.ai-tmp\sccache-tiers-'+[Guid]::NewGuid().ToString('N'))
$build=Join-Path $root 'build-root';$control=Join-Path $root 'local-control';$objects=Join-Path $root 'local-objects'
New-Item -ItemType Directory -Path $root,$build -Force | Out-Null
$configPath=Join-Path $root 'tiers.json'
$reserve=[Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback,0);$reserve.Start();$port=$reserve.LocalEndpoint.Port;$reserve.Stop()
$env:ELON_RUST_CACHE_CONTROL_ROOT=$control
$sccache=(Get-Command sccache -ErrorAction Stop).Source
$daemonStarted=$false;$listener=$null
try {
    $legacy=Get-RustCacheSccacheTierLayout -CacheRoot $build -LegacyMaxSize '3G'
    Assert-That (-not $legacy.tiered -and $legacy.cache_dir -eq (Join-Path $build 'sccache') -and $legacy.max_cache_size -eq '3G') 'Legacy single-root layout remains unchanged.'
    Assert-That ((Get-RustCacheSccacheConfigContent) -notmatch '\[cache\.disk\]') 'Legacy config has only basedirs.'
    $env:ELON_RUST_CACHE_SCCACHE_TIERS=$configPath
    $config=New-FixtureConfig;Write-FixtureJson $config $configPath
    $layout=Get-RustCacheSccacheTierLayout -CacheRoot $build
    Assert-That ($layout.tiered -and $layout.cache_dir -eq $objects -and $layout.max_cache_size -eq '67108864') 'Local L1 is independently bounded.'
    Assert-That ($layout.config_path.StartsWith($control+'\') -and $layout.config_path -notlike "$objects*") 'Control files are independent of object storage.'
    $alternate=Get-RustCacheSccacheTierLayout -CacheRoot '\\fixture-host\share\build'
    Assert-That ($alternate.config_path -eq $layout.config_path -and $alternate.server_port -eq $layout.server_port) 'Build-root routing shares one stable daemon layout.'
    $env:ELON_RUST_CACHE_CONTROL_ROOT=$null
    Assert-That ((Resolve-RustCacheManagementRoot -CacheRoot $build) -eq $control) 'Tier discovery alone resolves the management root.'
    $env:ELON_RUST_CACHE_CONTROL_ROOT=$control
    foreach($case in @('credential','port','small','fraction','unc','relative','overlap','control-mismatch','bad-schema','bad-base')) {
        $bad=New-FixtureConfig
        switch($case) {
            credential { $bad|Add-Member secret 'DO-NOT-PERSIST' }
            port { $bad.local.server_port=4226 }
            small { $bad.local.max_bytes=10 }
            fraction { $bad.local.max_bytes=67108864.5 }
            unc { $bad.local.cache_dir='\\fixture-host\share\objects' }
            relative { $bad.local.cache_dir='relative-objects' }
            overlap { $bad.local.cache_dir=$control }
            control-mismatch { $bad.local.control_root=Join-Path $root 'other-control' }
            bad-schema { $bad.schema='other' }
            bad-base { $bad|Add-Member base_directories @('relative') }
        }
        Write-FixtureJson $bad $configPath
        Assert-Rejected { Get-RustCacheSccacheTierLayout -CacheRoot $build } "Reject invalid $case configuration."
    }
    foreach($kind in @('s3','webdav')) {
        $config=New-FixtureConfig
        $remote=[pscustomobject]@{kind=$kind;endpoint='https://cache.example.test';allowed_host='cache.example.test';key_prefix='studio/cache/';rw_mode='READ_WRITE'}
        if($kind -eq 's3'){$remote|Add-Member bucket 'fixture-cache';$remote|Add-Member region 'us-east-1'}
        $config.remote=$remote;Write-FixtureJson $config $configPath
        $remoteLayout=Get-RustCacheSccacheTierLayout -CacheRoot $build
        $toml=Get-RustCacheSccacheTierToml $remoteLayout
        Assert-That ($toml -match ('chain = \["disk", "'+$kind+'"\]') -and $toml -match 'write_error_policy = "l0"') "Generate official $kind multilevel chain."
        Assert-RustCacheSccacheCapabilities $remoteLayout $sccache
        Assert-That ($toml -notmatch 'DO-NOT-PERSIST|password|token|secret|username') 'Generated remote TOML contains no credentials.'
        $remoteEnvironment=Get-RustCacheSccacheTierEnvironment $remoteLayout
        if($kind -eq 's3'){Assert-That ($toml -match 'no_credentials = false') 'S3 includes its required credential-mode field.'}
        else {
            Assert-That ($remoteEnvironment.SCCACHE_WEBDAV_ENDPOINT -eq $remote.endpoint -and $remoteEnvironment.SCCACHE_WEBDAV_KEY_PREFIX -eq $remote.key_prefix) 'Validated WebDAV routing enables standard credential environment.'
            Assert-That (-not $remoteEnvironment.Contains('SCCACHE_WEBDAV_TOKEN')) 'WebDAV token is never read or persisted by tier generation.'
        }
        foreach($case in @('host','userinfo','query','plaintext','readonly','secret-field','parent-prefix')) {
            $copy=$config|ConvertTo-Json -Depth 10|ConvertFrom-Json
            switch($case){
                host{$copy.remote.allowed_host='wrong.example.test'}
                userinfo{$copy.remote.endpoint='https://user:DO-NOT-PERSIST@cache.example.test'}
                query{$copy.remote.endpoint='https://cache.example.test?token=DO-NOT-PERSIST'}
                plaintext{$copy.remote.endpoint='http://cache.example.test'}
                readonly{$copy.remote.rw_mode='READ_ONLY'}
                secret-field{$copy.remote|Add-Member password 'DO-NOT-PERSIST'}
                parent-prefix{$copy.remote.key_prefix='../outside'}
            }
            Write-FixtureJson $copy $configPath
            Assert-Rejected {Get-RustCacheSccacheTierLayout -CacheRoot $build} "Reject $kind $case."
        }
    }
    $config=New-FixtureConfig;Write-FixtureJson $config $configPath
    Initialize-RustCacheSccacheTierStorage $layout
    Assert-That (Test-Path -LiteralPath (Join-Path $objects '.elon-sccache-owner.json')) 'Local cache ownership is recorded.'
    $foreign=Join-Path $root 'foreign';New-Item -ItemType Directory -Path $foreign|Out-Null
    [IO.File]::WriteAllText((Join-Path $foreign 'preserve.txt'),'existing-user-file')
    $config.local.cache_dir=$foreign;Write-FixtureJson $config $configPath
    Assert-Rejected {Initialize-RustCacheSccacheTierStorage (Get-RustCacheSccacheTierLayout -CacheRoot $build)} 'Populated unowned cache is not adopted.'
    $config=New-FixtureConfig;Write-FixtureJson $config $configPath
    $junction=Join-Path $root 'junction'
    New-Item -ItemType Junction -Path $junction -Target $objects | Out-Null
    try { Assert-Rejected {Assert-RustCacheTierLocalPath (Join-Path $junction 'child')} 'Reject reparse traversal.' }
    finally { [IO.Directory]::Delete($junction) }
    $ownerPath=Join-Path $objects '.elon-sccache-owner.json';$ownerBytes=[IO.File]::ReadAllBytes($ownerPath)
    $owner=[IO.File]::ReadAllText($ownerPath)|ConvertFrom-Json;$owner.machine='wrong-machine';Write-FixtureJson $owner $ownerPath
    Assert-Rejected {Initialize-RustCacheSccacheTierStorage $layout} 'Foreign machine owner is rejected.'
    [IO.File]::WriteAllBytes($ownerPath,$ownerBytes)
    $userConfig=Join-Path $root 'user-sccache.toml';[IO.File]::WriteAllText($userConfig,'# preserve user config')
    $env:SCCACHE_CONF=$userConfig
    $sync=Sync-RustCacheSccacheConfiguration -CacheRoot $build
    Assert-That ($sync.changed -and $sync.restart_pending) 'Preview generation does not start a daemon.'
    Assert-That ([IO.File]::ReadAllText($userConfig) -eq '# preserve user config') 'Existing user configuration is untouched.'
    $initialHash=$sync.config_sha256
    $sync=Sync-RustCacheSccacheConfiguration -CacheRoot '\\fixture-host\share\build' -AdditionalBaseDirs @($root)
    Assert-That (-not $sync.changed -and $sync.config_sha256 -eq $initialHash) 'Task paths and alternate build roots do not churn config.'
    $configBytes=[IO.File]::ReadAllBytes($layout.config_path)
    [IO.File]::AppendAllText($layout.config_path,'# user edit')
    Assert-Rejected {Sync-RustCacheSccacheConfiguration -CacheRoot $build} 'Edited owned TOML is preserved and rejected.'
    Assert-That ([IO.File]::ReadAllText($layout.config_path).EndsWith('# user edit')) 'Generator leaves changed user content intact.'
    [IO.File]::WriteAllBytes($layout.config_path,$configBytes)
    $listener=[Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback,$port);$listener.Start()
    Assert-Rejected {Sync-RustCacheSccacheConfiguration -CacheRoot $build -ConfigureProcessEnvironment -ForceRestart} 'Unrecognized occupied daemon port blocks activation.'
    Assert-That ($listener.Server.IsBound) 'Foreign listener remains running.'
    $listener.Stop();$listener=$null
    Write-FixtureJson @{binding_sha256=('0'*64)} $layout.binding_path
    Assert-Rejected {Sync-RustCacheSccacheConfiguration -CacheRoot $build} 'Changed wrapper binding requires reinstall.'
    Remove-Item -LiteralPath $layout.binding_path -Force
    $wrapper=Install-RustCacheSccacheWrapper -SourceModulesRoot $modules -CacheRoot $build -SccachePath $sccache -MaxCacheSize '20G'
    Assert-That ($wrapper -eq (Join-Path $control 'platform\rustc-sccache-wrapper.exe') -and (Test-Path -LiteralPath $wrapper)) 'Native wrapper installs under local control root.'
    $daemonStarted=$true
    $server=Restart-RustCacheSccacheServer -CacheRoot $build -MaxCacheSize '20G'
    Assert-That ($server.status -eq 'ready' -and $server.configuration_loaded -and $server.max_cache_size -eq '67108864') 'Real dedicated local-only daemon loads bounded L1.'
    $state=[IO.File]::ReadAllText($layout.state_path)|ConvertFrom-Json
    $pidBefore=$state.server_pid
    $sync=Sync-RustCacheSccacheConfiguration -CacheRoot '\\fixture-host\share\build' -ConfigureProcessEnvironment -RestartIfChanged
    Assert-That (-not $sync.restarted -and $sync.configuration_loaded) 'Stable alternate-root sync reuses daemon.'
    $after=[IO.File]::ReadAllText($layout.state_path)|ConvertFrom-Json
    Assert-That ($after.server_pid -eq $pidBefore) 'Same owned daemon PID is retained.'
    Assert-That ($env:SCCACHE_IDLE_TIMEOUT -eq '0') 'Dedicated daemon does not expire independently of its ownership receipt.'
    & $sccache --stop-server 2>$null | Out-Null
    $sync=Sync-RustCacheSccacheConfiguration -CacheRoot $build -ConfigureProcessEnvironment -RestartIfChanged
    $after=[IO.File]::ReadAllText($layout.state_path)|ConvertFrom-Json
    Assert-That ($sync.restarted -and $sync.configuration_loaded -and $after.server_pid -ne $pidBefore) 'Platform Sync recovers a stopped daemon with a new verified ownership receipt.'
    $values=(Set-RustCacheUserEnvironment -CacheRoot $build -MaxCacheSize '20G').values
    Assert-That ($values.SCCACHE_DIR -eq $objects -and $values.SCCACHE_SERVER_PORT -eq [string]$port -and $values.SCCACHE_CACHE_SIZE -eq '67108864') 'User environment preview uses local tier bindings.'
    Assert-That ($values.PSObject.Properties.Name -notcontains 'SCCACHE_IDLE_TIMEOUT') 'Idle policy is never written to the global user environment.'
    $source=Join-Path $root 'fixture.rs';$output=Join-Path $root 'libtier_fixture.rlib'
    [IO.File]::WriteAllText($source,'pub fn cached_answer() -> u32 { 42 }')
    $env:SCCACHE_CONF=$userConfig;$env:SCCACHE_DIR=$foreign;$env:SCCACHE_SERVER_PORT='4226';$env:SCCACHE_BUCKET='unwanted-backend';$env:SCCACHE_GHA_ENABLED='true';$env:SCCACHE_WEBDAV_TOKEN='DO-NOT-PERSIST'
    $rustc=(Get-Command rustc -ErrorAction Stop).Source
    for($attempt=0;$attempt -lt 2;$attempt++) {
        if(Test-Path -LiteralPath $output){Remove-Item -LiteralPath $output -Force}
        & $wrapper $rustc --crate-name tier_fixture --crate-type rlib --emit link $source --out-dir $root
        Assert-That ($LASTEXITCODE -eq 0 -and (Test-Path -LiteralPath $output)) "Native wrapper compilation attempt $attempt succeeds with hostile routing env."
    }
    $environment=Get-RustCacheSccacheTierEnvironment $layout
    Set-RustCacheSccacheProcessEnvironment $environment
    $statistics=@(& $sccache --show-stats 2>&1)
    [IO.File]::WriteAllText((Join-Path $root 'sccache-stats.txt'),($statistics|Out-String))
    Assert-That (($statistics|Out-String) -match '(?m)^Cache hits\s+[1-9][0-9]*\s*$') 'Repeated real Rust compile hits the local L1.'
    Assert-That (@(Get-ChildItem -LiteralPath $foreign -Force).Count -eq 1) 'Wrapper never used conflicting caller object directory.'
    $toml=[IO.File]::ReadAllText($layout.config_path)
    Assert-That ($toml -notmatch 'DO-NOT-PERSIST') 'Runtime authentication never enters generated config.'
    $config.local.max_bytes=134217728;Write-FixtureJson $config $configPath
    Assert-Rejected {Sync-RustCacheSccacheConfiguration -CacheRoot $build} 'Changed L1 policy cannot silently disagree with installed wrapper.'
    Write-Output "PASS assertions=$script:Assertions PowerShell=$($PSVersionTable.PSVersion) real_local_hit=true remote_network_tested=false"
} finally {
    if($listener){$listener.Stop()}
    if($daemonStarted -and (Test-Path -LiteralPath $layout.state_path)) {
        $state=[IO.File]::ReadAllText($layout.state_path)|ConvertFrom-Json
        $process=if($state.server_pid){Get-Process -Id $state.server_pid -ErrorAction SilentlyContinue}else{$null}
        if($process -and $process.Path -ieq $sccache -and [string]$process.StartTime.ToUniversalTime().Ticks -eq [string]$state.server_started_ticks){
            $environment=Get-RustCacheSccacheTierEnvironment $layout
            Set-RustCacheSccacheProcessEnvironment $environment
            & $sccache --stop-server 2>$null | Out-Null
        }
    }
    Set-RustCacheSccacheProcessEnvironment $previousEnvironment
    # Artifacts remain isolated under .ai-tmp for evidence; no production cache is reclaimed.
}
