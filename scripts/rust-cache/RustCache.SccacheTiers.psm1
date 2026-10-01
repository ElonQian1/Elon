Set-StrictMode -Version 2.0
Import-Module "$PSScriptRoot\RustCache.NetworkStorage.psm1" -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Studio.psm1" -DisableNameChecking

function Assert-RustCacheTierFields {
    param($Value, [string[]]$Allowed, [string]$Label)
    if ($null -eq $Value -or $Value -isnot [pscustomobject]) { throw "Invalid $Label object." }
    foreach ($field in $Value.PSObject.Properties.Name) {
        if ($field -notin $Allowed) { throw "Unrecognized $Label field; credentials and arbitrary configuration are not accepted." }
    }
}

function Get-RustCacheTierValue {
    param($Value, [string]$Name, $Default = $null)
    if ($Value.PSObject.Properties.Name -contains $Name) { return $Value.$Name }
    return $Default
}

function Assert-RustCacheTierLocalPath {
    param([string]$Path)
    if ($Path -notmatch '^[A-Za-z]:[\\/]' -or $Path -match '[\x00-\x1f]') { throw 'Sccache control and L1 paths require an absolute local fixed drive.' }
    $full = [IO.Path]::GetFullPath($Path).TrimEnd('\','/')
    if ($full -match '^[A-Za-z]:$') { throw 'Sccache tier paths cannot use a volume root.' }
    $drive = New-Object IO.DriveInfo([IO.Path]::GetPathRoot($full))
    if ($drive.DriveType -ne [IO.DriveType]::Fixed) { throw 'Sccache control and L1 paths require a local fixed drive.' }
    $cursor = $full
    while ($cursor) {
        if (Test-Path -LiteralPath $cursor) {
            if ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Sccache tier paths cannot traverse reparse points.' }
        }
        $parent = Split-Path -Parent $cursor
        if ($parent -eq $cursor) { break }; $cursor = $parent
    }
    return $full
}

function Get-RustCacheTierTextHash {
    param([string]$Text)
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Text)))).Replace('-','').ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function Read-RustCacheSccacheTiers {
    param([string]$CacheRoot, [string]$ConfigPath = $env:ELON_RUST_CACHE_SCCACHE_TIERS)
    if ([string]::IsNullOrWhiteSpace($ConfigPath)) { return $null }
    $path = Assert-RustCacheTierLocalPath $ConfigPath
    try { $config = [IO.File]::ReadAllText($path,[Text.Encoding]::UTF8) | ConvertFrom-Json -ErrorAction Stop }
    catch { throw 'Sccache tier configuration could not be read as JSON; its contents are not included in diagnostics.' }
    Assert-RustCacheTierFields $config @('schema','cache_root','local','remote','base_directories') 'tier'
    if ($config.schema -cne 'elon.rust_cache.sccache_tiers.v1') { throw 'Unsupported sccache tier schema.' }
    if (-not [IO.Path]::IsPathRooted([string]$config.cache_root)) { throw 'Sccache default cache root must be absolute.' }
    $defaultRoot = [IO.Path]::GetFullPath([string]$config.cache_root).TrimEnd('\','/')
    Assert-RustCacheTierFields $config.local @('control_root','cache_dir','max_bytes','server_port') 'local tier'
    $defaultControl = if ($env:ELON_RUST_CACHE_CONTROL_ROOT) { $env:ELON_RUST_CACHE_CONTROL_ROOT } else { Join-Path $env:LOCALAPPDATA 'Elon\rust-cache-control-v1' }
    $control = Assert-RustCacheTierLocalPath (Get-RustCacheTierValue $config.local 'control_root' $defaultControl)
    if ($env:ELON_RUST_CACHE_CONTROL_ROOT -and $control -ine (Assert-RustCacheTierLocalPath $env:ELON_RUST_CACHE_CONTROL_ROOT)) { throw 'Tier control root differs from the management control root.' }
    $cacheDir = Assert-RustCacheTierLocalPath (Get-RustCacheTierValue $config.local 'cache_dir' (Join-Path $control 'sccache-l1'))
    $scope = Join-Path $control 'sccache\machine-v1'
    if ($cacheDir -ieq $control -or $cacheDir -ieq $scope -or $scope.StartsWith($cacheDir+'\',[StringComparison]::OrdinalIgnoreCase) -or $cacheDir.StartsWith($scope+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Sccache L1 must be separate from its control files.' }
    $size = Get-RustCacheTierValue $config.local 'max_bytes' 2147483648
    $port = Get-RustCacheTierValue $config.local 'server_port' 4227
    if ([string]$size -notmatch '^[0-9]+$' -or [long]$size -lt 67108864 -or [long]$size -gt 1099511627776) { throw 'Sccache L1 max_bytes must be an integer from 64 MiB through 1 TiB.' }
    if ([string]$port -notmatch '^[0-9]+$' -or [int]$port -lt 1024 -or [int]$port -gt 65535 -or [int]$port -eq 4226) { throw 'Tiered sccache requires a dedicated nondefault server port.' }
    $remote = Get-RustCacheTierValue $config 'remote'
    if ($null -ne $remote) {
        Assert-RustCacheTierFields $remote @('kind','endpoint','allowed_host','bucket','region','key_prefix','rw_mode') 'remote tier'
        if ($remote.kind -cnotin @('s3','webdav')) { throw 'Supported remote tiers are s3 and webdav.' }
        $uri = $null
        if ([string]$remote.endpoint -match '[\x00-\x20\\"]' -or -not [Uri]::TryCreate([string]$remote.endpoint,[UriKind]::Absolute,[ref]$uri) -or $uri.Scheme -notin @('https','http') -or
            $uri.UserInfo -or $uri.Query -or $uri.Fragment -or $uri.DnsSafeHost -ine [string]$remote.allowed_host -or
            ($uri.Scheme -eq 'http' -and -not $uri.IsLoopback)) { throw 'Remote endpoint must use HTTPS (or loopback HTTP), match allowed_host exactly and contain no credentials/query/fragment.' }
        $prefix = [string](Get-RustCacheTierValue $remote 'key_prefix' '')
        if ($prefix -notmatch '^[A-Za-z0-9._/-]{1,160}$' -or $prefix -match '(^|/)\.\.(/|$)|^/') { throw 'A bounded relative remote key_prefix is required.' }
        $mode = [string](Get-RustCacheTierValue $remote 'rw_mode' 'READ_WRITE')
        if ($mode -cnotin @('READ_WRITE','READ_ONLY')) { throw 'Invalid remote rw_mode.' }
        if ($mode -eq 'READ_ONLY') { throw 'Read-only remote tiers are not enabled: sccache 0.16 can suppress local writes; validate a corrected release before activation.' }
        if ($remote.kind -eq 's3') {
            if ([string]$remote.bucket -notmatch '^[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]$' -or [string]$remote.region -notmatch '^[A-Za-z0-9-]{1,40}$' -or $uri.AbsolutePath -notin @('','/')) { throw 'Invalid S3 bucket, region or endpoint path.' }
        } elseif ($remote.PSObject.Properties.Name -contains 'bucket' -or $remote.PSObject.Properties.Name -contains 'region') { throw 'WebDAV cannot contain S3-only fields.' }
    }
    $profile = Get-RustCacheStudioProfile
    $defaults = if ($null -ne $profile) { @($profile.local_build_root,$profile.shared_build_root) } else { @($defaultRoot) }
    $bases = @(Get-RustCacheTierValue $config 'base_directories' $defaults)
    foreach ($base in $bases) { if ([string]$base -match '[\x00-\x1f"]' -or -not [IO.Path]::IsPathRooted([string]$base)) { throw 'Sccache base directories must be absolute paths.' } }
    $bases = @($bases | ForEach-Object { [IO.Path]::GetFullPath([string]$_).TrimEnd('\','/') } | Sort-Object -Unique)
    return [pscustomobject]@{ schema=$config.schema; config_path=$path; cache_root=$defaultRoot; control_root=$control
        scope_root=$scope; cache_dir=$cacheDir; max_bytes=[long]$size; server_port=[int]$port; remote=$remote; base_directories=$bases }
}

function Get-RustCacheSccacheTierLayout {
    param([string]$CacheRoot, [string]$LegacyMaxSize = '20G')
    $tier = Read-RustCacheSccacheTiers -CacheRoot $CacheRoot
    if ($null -eq $tier) {
        return [pscustomobject]@{ tiered=$false; tier=$null; control_root=$CacheRoot; scope_root=$CacheRoot; cache_dir=(Join-Path $CacheRoot 'sccache')
            max_cache_size=$LegacyMaxSize; config_path=(Join-Path $CacheRoot 'config\sccache-config'); state_path=(Join-Path $CacheRoot 'state\sccache-sync.json')
            lock_path=(Join-Path $CacheRoot 'config\sccache-config.lock'); cached_config_path=$null; server_port=$null; binding_path=$null }
    }
    [pscustomobject]@{ tiered=$true; tier=$tier; control_root=$tier.control_root; scope_root=$tier.scope_root; cache_dir=$tier.cache_dir
        max_cache_size=[string]$tier.max_bytes; config_path=(Join-Path $tier.scope_root 'sccache-config'); state_path=(Join-Path $tier.scope_root 'sync.json')
        lock_path=(Join-Path $tier.scope_root 'config.lock'); cached_config_path=(Join-Path $tier.scope_root 'cached-config'); server_port=[string]$tier.server_port
        binding_path=(Join-Path $tier.scope_root 'wrapper-binding.json') }
}

function Get-RustCacheSccacheEnvironmentNames {
    @('SCCACHE_CONF','SCCACHE_DIR','SCCACHE_CACHE_SIZE','SCCACHE_SERVER_PORT','SCCACHE_CACHED_CONF','SCCACHE_BASEDIRS','SCCACHE_IDLE_TIMEOUT',
      'SCCACHE_MULTILEVEL_CHAIN','SCCACHE_MULTILEVEL_WRITE_ERROR_POLICY','SCCACHE_LOCAL_RW_MODE',
      'SCCACHE_BUCKET','SCCACHE_ENDPOINT','SCCACHE_REGION','SCCACHE_S3_USE_SSL','SCCACHE_S3_KEY_PREFIX','SCCACHE_S3_RW_MODE',
      'SCCACHE_S3_NO_CREDENTIALS','SCCACHE_S3_ENABLE_VIRTUAL_HOST_STYLE','SCCACHE_S3_SERVER_SIDE_ENCRYPTION',
      'SCCACHE_WEBDAV_ENDPOINT','SCCACHE_WEBDAV_KEY_PREFIX','SCCACHE_WEBDAV_RW_MODE',
      'SCCACHE_REDIS','SCCACHE_REDIS_ENDPOINT','SCCACHE_REDIS_CLUSTER_ENDPOINTS','SCCACHE_MEMCACHED','SCCACHE_MEMCACHED_ENDPOINT',
      'SCCACHE_GCS_BUCKET','SCCACHE_AZURE_CONNECTION_STRING','SCCACHE_AZURE_BLOB_CONTAINER','SCCACHE_GHA_CACHE_URL','SCCACHE_GHA_ENABLED','SCCACHE_GHA_VERSION','SCCACHE_GHA_RW_MODE',
      'SCCACHE_OSS_BUCKET','SCCACHE_COS_BUCKET')
}

function Get-RustCacheSccacheTierEnvironment {
    param($Layout)
    $values = [ordered]@{ SCCACHE_CONF=$Layout.config_path; SCCACHE_DIR=$Layout.cache_dir; SCCACHE_CACHE_SIZE=$Layout.max_cache_size }
    if ($Layout.tiered) {
        foreach ($name in Get-RustCacheSccacheEnvironmentNames) { if (-not $values.Contains($name)) { $values[$name]=$null } }
        $values.SCCACHE_SERVER_PORT=$Layout.server_port; $values.SCCACHE_CACHED_CONF=$Layout.cached_config_path
        $values.SCCACHE_IDLE_TIMEOUT='0'
        if ($null -ne $Layout.tier.remote -and $Layout.tier.remote.kind -eq 'webdav') {
            # sccache loads WebDAV credential env vars only when its endpoint env is set.
            $values.SCCACHE_WEBDAV_ENDPOINT=$Layout.tier.remote.endpoint
            $values.SCCACHE_WEBDAV_KEY_PREFIX=$Layout.tier.remote.key_prefix
            $values.SCCACHE_WEBDAV_RW_MODE='READ_WRITE'
        }
    }
    return $values
}

function Set-RustCacheSccacheProcessEnvironment {
    param([System.Collections.IDictionary]$Values)
    foreach ($name in $Values.Keys) {
        if ($null -eq $Values[$name]) { Remove-Item -LiteralPath ('Env:'+ $name) -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable($name,[string]$Values[$name],'Process') }
    }
}

function Get-RustCacheSccacheTierToml {
    param($Layout)
    if (-not $Layout.tiered) { return '' }
    $path = $Layout.cache_dir.Replace('\','/').Replace('"','\"')
    $text = "`r`n[cache.disk]`r`ndir = `"$path`"`r`nsize = $($Layout.tier.max_bytes)`r`nrw_mode = `"READ_WRITE`"`r`n"
    $remote = $Layout.tier.remote
    if ($null -ne $remote) {
        $text += "`r`n[cache.multilevel]`r`nchain = [`"disk`", `"$($remote.kind)`"]`r`nwrite_error_policy = `"l0`"`r`n"
        $text += "`r`n[cache.$($remote.kind)]`r`nendpoint = `"$($remote.endpoint.Replace('"','\"'))`"`r`nkey_prefix = `"$($remote.key_prefix)`"`r`nrw_mode = `"READ_WRITE`"`r`n"
        if ($remote.kind -eq 's3') { $ssl=(([Uri]$remote.endpoint).Scheme -eq 'https').ToString().ToLowerInvariant(); $text += "bucket = `"$($remote.bucket)`"`r`nregion = `"$($remote.region)`"`r`nuse_ssl = $ssl`r`nno_credentials = false`r`n" }
    }
    return $text
}

function Assert-RustCacheSccacheCapabilities {
    param($Layout, [string]$SccachePath)
    if (-not $Layout.tiered -or $null -eq $Layout.tier.remote) { return }
    $version = (& $SccachePath --version 2>&1 | Out-String)
    if ($LASTEXITCODE -ne 0 -or $version -notmatch 'sccache ([0-9]+\.[0-9]+\.[0-9]+)' -or [version]$Matches[1] -lt [version]'0.16.0') { throw 'Sccache 0.16 or later with multilevel support is required.' }
    $help = (& $SccachePath --help 2>&1 | Out-String)
    $backend = if ($Layout.tier.remote.kind -eq 's3') { 'S3' } else { 'WebDAV' }
    if ($LASTEXITCODE -ne 0 -or $help -notmatch ('(?im)^\s*'+$backend+':\s*true\s*$')) { throw 'The installed sccache binary does not enable the requested backend.' }
}

function Initialize-RustCacheSccacheTierStorage {
    param($Layout)
    if (-not $Layout.tiered) { return }
    foreach ($directory in @($Layout.scope_root,$Layout.cache_dir)) {
        $full = Assert-RustCacheTierLocalPath $directory
        $marker = Join-Path $full '.elon-sccache-owner.json'
        Assert-RustCacheTierLocalPath $marker | Out-Null
        if (Test-Path -LiteralPath $marker) {
            $owner = [IO.File]::ReadAllText($marker,[Text.Encoding]::UTF8) | ConvertFrom-Json
            if ($owner.schema -ne 'elon.sccache_storage_owner.v1' -or $owner.machine -ne (Get-RustCacheMachineIdentity) -or
                $owner.control_root -ine $Layout.control_root) { throw 'Sccache tier directory belongs to another owner.' }
        } else {
            if ((Test-Path -LiteralPath $full) -and @(Get-ChildItem -LiteralPath $full -Force).Count) { throw 'Refusing to adopt a populated unowned sccache tier directory.' }
            New-Item -ItemType Directory -Path $full -Force | Out-Null
            $owner = @{schema='elon.sccache_storage_owner.v1';machine=(Get-RustCacheMachineIdentity);control_root=$Layout.control_root} | ConvertTo-Json -Compress
            $stream = [IO.File]::Open($marker,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
            try { $bytes=[Text.Encoding]::UTF8.GetBytes($owner);$stream.Write($bytes,0,$bytes.Length);$stream.Flush($true) } finally { $stream.Dispose() }
        }
    }
}

function Get-RustCacheSccacheBinding {
    param($Layout)
    $webdav = if ($Layout.tiered -and $null -ne $Layout.tier.remote -and $Layout.tier.remote.kind -eq 'webdav') { $Layout.tier.remote } else { $null }
    [pscustomobject][ordered]@{ config_path=$Layout.config_path; cache_dir=$Layout.cache_dir; max_cache_size=$Layout.max_cache_size
        server_port=$Layout.server_port; cached_config_path=$Layout.cached_config_path
        webdav_endpoint=if($webdav){$webdav.endpoint}else{$null};webdav_key_prefix=if($webdav){$webdav.key_prefix}else{$null} }
}

Export-ModuleMember -Function Read-RustCacheSccacheTiers, Get-RustCacheSccacheTierLayout, Get-RustCacheSccacheEnvironmentNames, Get-RustCacheSccacheTierEnvironment, Set-RustCacheSccacheProcessEnvironment, Get-RustCacheSccacheTierToml, Assert-RustCacheSccacheCapabilities, Initialize-RustCacheSccacheTierStorage, Get-RustCacheSccacheBinding, Assert-RustCacheTierLocalPath, Get-RustCacheTierTextHash
