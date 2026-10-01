$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot '..\rust-cache\RustCache.NetworkStorage.psm1') -DisableNameChecking
Import-Module (Join-Path $PSScriptRoot '..\rust-cache\RustCache.SccacheTiers.psm1') -DisableNameChecking
Import-Module (Join-Path $PSScriptRoot '..\rust-cache\RustCache.ControlFiles.psm1') -DisableNameChecking

function Get-StudioBootstrapFileHash {
    param([string]$Path)
    $stream=[IO.File]::OpenRead($Path); $sha=[Security.Cryptography.SHA256]::Create()
    try { ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant() } finally { $sha.Dispose();$stream.Dispose() }
}

function Assert-StudioBootstrapPath {
    param([string]$Path, [switch]$Local, [switch]$AllowVolumeRoot)
    if ([string]::IsNullOrWhiteSpace($Path) -or $Path -match '(^\\\\[?.]\\|[*?])' -or $Path -notmatch '^(?:[A-Za-z]:\\|\\\\[^\\]+\\[^\\]+)') { throw 'STUDIO_PATH_INVALID: Absolute ordinary paths are required.' }
    $full = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    if ($full -eq [IO.Path]::GetPathRoot($full).TrimEnd('\')) {
        if (-not $AllowVolumeRoot) { throw 'STUDIO_PATH_INVALID: A volume/share root cannot be managed directly.' }
        $full=[IO.Path]::GetPathRoot([IO.Path]::GetFullPath($Path))
    }
    if ($Local -and ($full -notmatch '^[A-Za-z]:\\' -or ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($full))).DriveType -ne 'Fixed')) { throw 'STUDIO_LOCAL_REQUIRED: Select a local fixed disk.' }
    $ancestor = [IO.DirectoryInfo]::new($full)
    while ($ancestor) {
        if (Test-Path -LiteralPath $ancestor.FullName) {
            $item = Get-Item -LiteralPath $ancestor.FullName -Force
            if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'STUDIO_REPARSE_PATH: Linked/non-directory ancestors are refused.' }
        }
        $ancestor = $ancestor.Parent
    }
    return $full
}

function Resolve-StudioHostShare {
    param([string]$HostName, [string]$ShareName)
    if ($HostName -ine [Environment]::MachineName) { throw 'STUDIO_HOST_IDENTITY: Host mode must run on the named SMB host itself.' }
    try { $shares = @(Get-SmbShare -Name $ShareName -ErrorAction Stop | Where-Object { $_.Name -ceq $ShareName }) } catch { throw 'STUDIO_SHARE_MAPPING: Cannot read the local SMB share; run on the host with share-query permission.' }
    if ($shares.Count -ne 1 -or -not $shares[0].Path) { throw 'STUDIO_SHARE_MAPPING: Exactly one physical SMB mapping is required.' }
    $path = Assert-StudioBootstrapPath -Path ([string]$shares[0].Path) -Local -AllowVolumeRoot
    if (-not (Test-Path -LiteralPath $path -PathType Container)) { throw 'STUDIO_SHARE_MAPPING: The mapped physical directory does not exist.' }
    return $path
}

function Assert-StudioBootstrapOwner {
    param([string]$Path, [string]$MachineHash, [switch]$Control, [switch]$Claim)
    $path = Assert-StudioBootstrapPath -Path $Path
    $markerName = if ($Control) { '.studio-cache-owner.json' } else { '.rust-cache-owner.json' }
    $schema = if ($Control) { 'elon.studio_cache.control_owner.v1' } else { 'elon.rust_cache.root_owner.v1' }
    $marker = Join-Path $path $markerName
    if (-not (Test-Path -LiteralPath $marker)) {
        if ((Test-Path -LiteralPath $path) -and @(Get-ChildItem -LiteralPath $path -Force).Count -gt 0) { throw 'STUDIO_ROOT_UNOWNED: Preserve this populated unowned directory; choose a new root.' }
        if (-not $Claim) { return }
        [IO.Directory]::CreateDirectory($path) | Out-Null
        Write-StudioBootstrapJson -Path $marker -Value ([ordered]@{ schema=$schema; machine_id_sha256=$MachineHash; created_utc=[DateTime]::UtcNow.ToString('o') })
    }
    $item = Get-Item -LiteralPath $marker -Force
    if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'STUDIO_ROOT_OWNER: Invalid owner marker.' }
    $owner = Get-Content -LiteralPath $marker -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($owner.schema -ne $schema -or $owner.machine_id_sha256 -cne $MachineHash) { throw 'STUDIO_ROOT_FOREIGN: Do not claim another machine root or replace its owner marker.' }
}

function Write-StudioBootstrapJson {
    param([string]$Path, $Value)
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes(($Value | ConvertTo-Json -Depth 12))
    $stream = [IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try { $stream.Write($bytes,0,$bytes.Length); $stream.Flush($true) } finally { $stream.Dispose() }
}

function Get-StudioBootstrapHardware {
    $result = [ordered]@{ verified=$false; cpu=@(); gpu=@(); memory_bytes=$null; disks=@(); network=@() }
    try {
        $result.cpu = @(Get-CimInstance Win32_Processor -ErrorAction Stop | Select-Object Name,NumberOfCores,NumberOfLogicalProcessors)
        $result.gpu = @(Get-CimInstance Win32_VideoController -ErrorAction Stop | Select-Object Name,AdapterRAM)
        $result.memory_bytes = [int64](Get-CimInstance Win32_ComputerSystem -ErrorAction Stop).TotalPhysicalMemory
        $result.network = @(Get-NetAdapter -Physical -ErrorAction Stop | Select-Object InterfaceDescription,Status,LinkSpeed)
        $result.disks = @(Get-PhysicalDisk -ErrorAction Stop | Select-Object MediaType,BusType,Size)
        $result.verified = $true
    } catch { $result.verified = $false }
    return [pscustomobject]$result
}

function Assert-StudioBootstrapSource {
    param([string]$SourceRoot,[string]$SourceSha256)
    $manifestPath = Join-Path $SourceRoot 'studio-cache-bundle.json'
    if ($SourceSha256 -notmatch '^[a-fA-F0-9]{64}$' -or (Get-StudioBootstrapFileHash $manifestPath) -ne $SourceSha256) { throw 'STUDIO_BUNDLE_HASH: Reviewed bundle identity changed.' }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($manifest.schema -ne 'elon.studio_cache.bundle.v1') { throw 'STUDIO_BUNDLE_SCHEMA: Invalid bundle.' }
    $expected = @{}
    foreach ($file in $manifest.files) {
        $relative=[string]$file.path
        if ([string]::IsNullOrWhiteSpace($relative) -or $relative -match '(^/|\\|:|[*?]|(^|/)\.\.?(/|$))' -or $file.sha256 -notmatch '^[a-fA-F0-9]{64}$' -or $expected.ContainsKey($relative)) { throw 'STUDIO_BUNDLE_PATH: Invalid or duplicate file.' }
        $expected[$relative]=[string]$file.sha256
    }
    foreach ($required in @('scripts/studio-cache/StudioCache.Bootstrap.psm1','scripts/rust-cache.ps1','rust-cache.project.json',
        'scripts/rust-cache/native/rustc_sccache_wrapper.rs',
        '.agents/skills/manage-shared-build-cache/SKILL.md','.agents/skills/manage-shared-build-cache/agents/openai.yaml',
        'docs/studio-cache-ai-entry.md','docs/studio-cache-operations.md','docs/design/studio-build-cache-v1.md',
        'docs/rust-cache-platform.md','docs/rust-cache-on-demand-adoption.md','docs/rust-cache-network-storage.md','docs/rust-cache-fleet-operations.md')) {
        if (-not $expected.ContainsKey($required)) { throw "STUDIO_BUNDLE_CONTENT: Required installation file missing: $required" }
    }
    foreach($name in @('Capacity','CargoIncludeMigration','ControlFiles','Fleet','FleetQueue','GcApproval','Help','Install','Inventory','Launcher','Legacy','NetworkStorage',
        'Paths','Policy','Portability','ProjectAdoption','Registry','Run','Runtime','Sccache','SccacheTiers','Scope','Studio','TaskLifecycle')){
        if(-not $expected.ContainsKey("scripts/rust-cache/RustCache.$name.psm1")){throw "STUDIO_BUNDLE_CONTENT: Required import module missing: $name"}
    }
    $pending=New-Object 'Collections.Generic.Stack[string]'; $pending.Push($SourceRoot); $seen=0
    while ($pending.Count) {
        foreach ($item in Get-ChildItem -LiteralPath $pending.Pop() -Force) {
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'STUDIO_BUNDLE_REPARSE: Linked source.' }
            if ($item.PSIsContainer) { $pending.Push($item.FullName); continue }
            if ($item.FullName -eq $manifestPath) { continue }
            $relative=$item.FullName.Substring($SourceRoot.Length+1).Replace('\','/')
            if (-not $expected.ContainsKey($relative) -or (Get-StudioBootstrapFileHash $item.FullName) -ne $expected[$relative]) { throw 'STUDIO_BUNDLE_CONTENT: Source changed after review.' }
            $seen++
        }
    }
    if ($seen -ne $expected.Count -or $seen -eq 0) { throw 'STUDIO_BUNDLE_CONTENT: Missing source.' }
}

function Get-StudioBootstrapEnvironment {
    param([string]$Name,[string]$Scope)
    return [Environment]::GetEnvironmentVariable($Name,$Scope)
}

function Set-StudioBootstrapEnvironment {
    param([string]$Name,[AllowNull()]$Value,[string]$Scope)
    if ($null -eq $Value) {
        if ($Scope -eq 'Process') { Remove-Item -LiteralPath ('Env:' + $Name) -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable($Name,[NullString]::Value,$Scope) }
        return
    }
    [Environment]::SetEnvironmentVariable($Name,[string]$Value,$Scope)
}

function Get-StudioBootstrapControlFiles {
    param([string]$SourceRoot,[string]$CacheRoot,[string]$ControlRoot)
    $modules=Join-Path $SourceRoot 'scripts\rust-cache'
    $paths=Import-Module (Join-Path $modules 'RustCache.Paths.psm1') -PassThru -DisableNameChecking
    $launcher=Import-Module (Join-Path $modules 'RustCache.Launcher.psm1') -PassThru -DisableNameChecking
    $installer=Import-Module (Join-Path $modules 'RustCache.Install.psm1') -PassThru -DisableNameChecking
    $cargoPath=& $paths { Get-RustCacheDefaultCargoConfigPath }
    $launcherPath=& $launcher { Get-RustCacheDefaultUserLauncherPath }
    $proposal=& $installer {param($Path,$Include) Set-RustCacheParentCargoConfig -CargoConfigPath $Path -IncludeConfigPath $Include} $cargoPath (Join-Path $ControlRoot 'config\cargo-cache.toml')
    $launcherText=& $launcher {param($Root) Get-RustCacheUserLauncherContent -CacheRoot $Root} $CacheRoot
    @([pscustomobject]@{kind='cargo-config';path=$cargoPath;expected_content=$proposal.content},[pscustomobject]@{kind='user-launcher';path=$launcherPath;expected_content=$launcherText})
}

function New-StudioBootstrapPrivateDirectory {
    param([string]$Path)
    Assert-StudioBootstrapPath -Path $Path -Local | Out-Null
    if (Test-Path -LiteralPath $Path) { throw 'STUDIO_BACKUP_EXISTS: Private backup directory must be new.' }
    [IO.Directory]::CreateDirectory($Path) | Out-Null
    $sid=[Security.Principal.WindowsIdentity]::GetCurrent().User
    $acl=[Security.AccessControl.DirectorySecurity]::new();$acl.SetAccessRuleProtection($true,$false);$acl.SetOwner($sid)
    foreach ($identity in @($sid,[Security.Principal.SecurityIdentifier]::new('S-1-5-18'),[Security.Principal.SecurityIdentifier]::new('S-1-5-32-544'))) {
        $rule=[Security.AccessControl.FileSystemAccessRule]::new($identity,'FullControl','ContainerInherit, ObjectInherit','None','Allow');$acl.AddAccessRule($rule)
    }
    if ($PSVersionTable.PSVersion.Major -lt 6) { [IO.Directory]::SetAccessControl($Path,$acl) }
    else { [IO.FileSystemAclExtensions]::SetAccessControl([IO.DirectoryInfo]::new($Path),$acl) }
}

function Save-StudioBootstrapControlFiles {
    param([object[]]$Files,[string]$BackupRoot)
    $records=@()
    foreach ($file in $Files) {
        if ($file.kind -notin @('cargo-config','user-launcher')) { throw 'STUDIO_BACKUP_SCOPE: Unexpected control file kind.' }
        Assert-StudioBootstrapPath -Path (Split-Path $file.path -Parent) -Local | Out-Null
        $exists=Test-Path -LiteralPath $file.path
        $hash=$null;$backup=Join-Path $BackupRoot ($file.kind + '.before')
        if ($exists) {
            $item=Get-Item -LiteralPath $file.path -Force
            if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -or $item.Length -gt 1MB) { throw 'STUDIO_BACKUP_SOURCE: Control files must be small regular local files.' }
            $hash=Get-StudioBootstrapFileHash $file.path
            [IO.File]::Copy($file.path,$backup,$false)
            if ((Get-StudioBootstrapFileHash $backup) -ne $hash -or (Get-StudioBootstrapFileHash $file.path) -ne $hash) { throw 'STUDIO_BACKUP_DRIFT: Control file changed during snapshot.' }
        }
        $encoding=[Text.UTF8Encoding]::new($PSVersionTable.PSVersion.Major -lt 6)
        [byte[]]$expectedBytes=@($encoding.GetPreamble()) + @($encoding.GetBytes([string]$file.expected_content))
        $sha=[Security.Cryptography.SHA256]::Create()
        try { $expectedHash=([BitConverter]::ToString($sha.ComputeHash($expectedBytes))).Replace('-','').ToLowerInvariant() } finally { $sha.Dispose() }
        $records += [pscustomobject]@{kind=$file.kind;path=$file.path;existed=$exists;before_sha256=$hash;backup_path=$backup;expected_installer_sha256=$expectedHash}
    }
    $receipt=Join-Path $BackupRoot 'control-files.json'
    Write-StudioBootstrapJson -Path $receipt -Value ([ordered]@{schema='elon.studio_cache.control_rollback.v1';files=$records})
    return [pscustomobject]@{path=$receipt;sha256=(Get-StudioBootstrapFileHash $receipt);backup_root=$BackupRoot}
}

function Restore-StudioBootstrapControlFiles {
    param($Snapshot)
    if ((Get-StudioBootstrapFileHash $Snapshot.path) -ne $Snapshot.sha256) { throw 'STUDIO_BACKUP_RECEIPT: Control backup receipt changed.' }
    $receipt=Get-Content -LiteralPath $Snapshot.path -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($receipt.schema -ne 'elon.studio_cache.control_rollback.v1') { throw 'STUDIO_BACKUP_RECEIPT: Unsupported receipt.' }
    $results=@()
    foreach ($file in $receipt.files) {
        Assert-StudioBootstrapPath -Path (Split-Path $file.path -Parent) -Local | Out-Null
        $exists=Test-Path -LiteralPath $file.path
        $current=$null
        if ($exists) {
            $item=Get-Item -LiteralPath $file.path -Force
            if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'STUDIO_ROLLBACK_DRIFT: Control path was replaced.' }
            $current=Get-StudioBootstrapFileHash $file.path
        }
        if (($file.existed -and $current -eq $file.before_sha256) -or (-not $file.existed -and -not $exists)) { $results+=@{kind=$file.kind;status='unchanged'};continue }
        if (-not $exists -or $current -ne $file.expected_installer_sha256) { $results+=@{kind=$file.kind;status='preserved-drift'};continue }
        if ($file.existed) {
            $expectedBackup=Join-Path $Snapshot.backup_root ($file.kind + '.before')
            if ($file.backup_path -ne $expectedBackup) { throw 'STUDIO_BACKUP_HASH: Original control backup path changed.' }
            $backupBytes=[IO.File]::ReadAllBytes($expectedBackup)
            if ((Get-RustCacheControlBytesHash $backupBytes) -ne $file.before_sha256) { throw 'STUDIO_BACKUP_HASH: Original control backup changed.' }
            try {
                Write-RustCacheBoundControlFile -Path $file.path -ExpectedSha256 $file.expected_installer_sha256 -Bytes $backupBytes | Out-Null
            } catch {
                if($_.Exception.Message -match '^RUST_CACHE_CONTROL_(DRIFT|BUSY):'){$results+=@{kind=$file.kind;status='preserved-drift'};continue}
                throw
            }
        } else {
            $results+=@{kind=$file.kind;status='preserved-new-file-needs-review'};continue
        }
        $results+=@{kind=$file.kind;status='restored'}
    }
    return [pscustomobject]@{complete=(@($results|Where-Object {$_.status -like 'preserved-*'}).Count -eq 0);files=$results}
}

function New-StudioCacheBootstrapPlan {
    param([ValidateSet('Host','Client')][string]$Role,[string]$HostName,[string]$ShareName,[string]$CacheDirectory,[string]$LocalControlRoot,[switch]$AllowUncBuild)
    foreach ($segment in @($HostName,$ShareName,$CacheDirectory)) {
        if ([string]::IsNullOrWhiteSpace($segment) -or $segment -match '[\\/:*?"<>|]' -or $segment -in @('.','..') -or $segment.Trim() -ne $segment) { throw 'STUDIO_SHARE_ARGUMENT: Host/share/cache names must be single ordinary path components.' }
    }
    if ($Role -eq 'Host' -and $AllowUncBuild) { throw 'STUDIO_HOST_LOCAL_ONLY: Host mode uses its physical local path.' }
    $machine = Get-RustCacheMachineIdentity
    if ($machine -notmatch '^[a-f0-9]{64}$') { throw 'STUDIO_MACHINE_ID: A verified machine hash is required.' }
    if (-not $LocalControlRoot) { $LocalControlRoot = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Elon\rust-cache-control-v1' }
    $control = Assert-StudioBootstrapPath -Path $LocalControlRoot -Local
    $uncBase = '\\' + $HostName + '\' + $ShareName + '\' + $CacheDirectory
    $hostShare = $null
    if ($Role -eq 'Host') { $hostShare = Resolve-StudioHostShare -HostName $HostName -ShareName $ShareName; $base = Join-Path $hostShare $CacheDirectory } else { $base = $uncBase }
    $base = Assert-StudioBootstrapPath -Path $base
    if (-not (Test-Path -LiteralPath $base -PathType Container)) { throw 'STUDIO_SHARE_UNAVAILABLE: The exact cache directory must already exist; no drive fallback is allowed.' }
    $partition = 'pc-' + $machine.Substring(0,12)
    $ownRelative = $partition + '\rust-cache-studio-v1'
    $shared = Join-Path $uncBase $ownRelative
    $localBuild = if ($Role -eq 'Host') { Join-Path $base $ownRelative } else { Join-Path $control 'build-root' }
    $selected = if ($Role -eq 'Client' -and $AllowUncBuild) { $shared } else { $localBuild }
    if ($base.StartsWith($control + '\',[StringComparison]::OrdinalIgnoreCase) -or $control.StartsWith($base + '\',[StringComparison]::OrdinalIgnoreCase) -or $base -eq $control) { throw 'STUDIO_ROOT_OVERLAP: Control and shared storage must be separate.' }
    Assert-StudioBootstrapOwner -Path $control -MachineHash $machine -Control
    Assert-StudioBootstrapOwner -Path $selected -MachineHash $machine
    $tiersPath = Join-Path $control 'sccache-tiers.json'
    $profile = [ordered]@{
        schema='elon.studio_cache.machine.v1'; role=$Role.ToLowerInvariant(); machine_id_sha256=$machine
        control_root=$control; local_build_root=$localBuild; shared_build_root=$shared; selected_cache_root=$selected
        l1_cache_dir=(Join-Path $control 'sccache-l1'); l1_max_bytes=[int64]2147483648; sccache_tiers_path=$tiersPath
        allow_unc_build=[bool]$AllowUncBuild; remote_build_ready=$false
    }
    $tiers = [ordered]@{ schema='elon.rust_cache.sccache_tiers.v1'; cache_root=$selected; local=[ordered]@{control_root=$control;cache_dir=$profile.l1_cache_dir;max_bytes=[int64]2147483648;server_port=4227}; remote=$null }
    [pscustomobject]@{
        schema='elon.studio_cache.bootstrap_plan.v1'; mode='preview'; profile=[pscustomobject]$profile; tiers=[pscustomobject]$tiers
        profile_path=(Join-Path $control 'machine.json'); report_root=(Join-Path $base ('studio-bootstrap-reports\' + $partition))
        physical_share_path=$hostShare; shared_volume=(Get-RustCacheStorageVolume -CacheRoot $base); control_volume=(Get-RustCacheStorageVolume -CacheRoot $control)
        capacity=[pscustomobject]@{ l1_max_bytes=[int64]2147483648; minimum_local_free_bytes=[int64](22GB); minimum_shared_free_bytes=[int64](20GB); remote_build_ready=$false; max_host_builds=1 }
        opens_ports=$false; installs_remote_service=$false; resumes_supervision=$false
    }
}

function Invoke-StudioBootstrapPlatform {
    param([string]$SourceRoot,[string]$ProjectRoot,[string]$CacheRoot)
    $entry = Join-Path $SourceRoot 'scripts\rust-cache.ps1'
    $global:LASTEXITCODE=0
    & $entry install -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -Apply -InstallCodexSkill | Out-Host
    if (-not $? -or $LASTEXITCODE -ne 0) { throw 'STUDIO_PLATFORM_INSTALL: Trusted platform installation failed.' }
    $global:LASTEXITCODE=0
    & $entry doctor -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot | Out-Host
    if (-not $? -or $LASTEXITCODE -ne 0) { throw 'STUDIO_PLATFORM_DOCTOR: Installed platform health checks did not pass.' }
}

function Invoke-StudioCacheBootstrap {
    [CmdletBinding()]
    param([ValidateSet('Host','Client')][string]$Role,[string]$HostName,[string]$ShareName,[string]$CacheDirectory,[string]$LocalControlRoot,[string]$SourceRoot,[string]$SourceSha256,[string]$ProjectRoot,[switch]$AllowUncBuild,[switch]$Apply)
    $plan = New-StudioCacheBootstrapPlan -Role $Role -HostName $HostName -ShareName $ShareName -CacheDirectory $CacheDirectory -LocalControlRoot $LocalControlRoot -AllowUncBuild:$AllowUncBuild
    if (-not $Apply) { return $plan }
    $source = Assert-StudioBootstrapPath -Path $SourceRoot -Local
    if (-not $ProjectRoot) { $ProjectRoot = $source }
    Assert-StudioBootstrapSource -SourceRoot $source -SourceSha256 $SourceSha256
    foreach ($destination in @($plan.profile.control_root,$plan.profile.selected_cache_root,$plan.report_root)) {
        if ($source -eq $destination -or $source.StartsWith($destination + '\',[StringComparison]::OrdinalIgnoreCase) -or $destination.StartsWith($source + '\',[StringComparison]::OrdinalIgnoreCase)) { throw 'STUDIO_SOURCE_OVERLAP: Keep the reviewed bundle separate from mutable cache/configuration/report roots.' }
    }
    if ($plan.control_volume.free_bytes -lt $plan.capacity.minimum_local_free_bytes -or $plan.shared_volume.free_bytes -lt $plan.capacity.minimum_shared_free_bytes) { throw 'STUDIO_CAPACITY: Preserve local/shared reserve before installing; no automatic cleanup or fallback.' }
    $profile = $plan.profile
    foreach ($file in @($plan.profile_path,$profile.sccache_tiers_path)) { if (Test-Path -LiteralPath $file) { throw 'STUDIO_ALREADY_CONFIGURED: Preserve the existing machine profile; use a reviewed upgrade workflow.' } }
    $runId = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ') + '-' + [guid]::NewGuid().ToString('N')
    $report = [ordered]@{schema='elon.studio_cache.bootstrap_report.v1';run_id=$runId;role=$profile.role;machine_id_sha256=$profile.machine_id_sha256;source_sha256=$SourceSha256;created_utc=[DateTime]::UtcNow.ToString('o');status='started';stage='claim';error_code=$null;hardware=(Get-StudioBootstrapHardware);physical_share_path=$plan.physical_share_path;shared_volume=$plan.shared_volume;control_volume=$plan.control_volume;l1_max_bytes=$profile.l1_max_bytes;remote_build_ready=$false;end_to_end_network_verified=$false}
    $userNames = @('ELON_RUST_CACHE_CONTROL_ROOT','ELON_RUST_CACHE_SCCACHE_TIERS','ELON_STUDIO_CACHE_PROFILE','ELON_RUST_CACHE_ROOT','SCCACHE_DIR','SCCACHE_CACHE_SIZE','SCCACHE_CONF','SCCACHE_SERVER_PORT','SCCACHE_CACHED_CONF','ELON_DEV_CARGO_TARGET_DIR')
    $names = @($userNames) + @(Get-RustCacheSccacheEnvironmentNames | Where-Object {$_ -notin $userNames})
    $old = [ordered]@{}
    foreach ($name in $names) {
        $old[$name] = @{Process=(Get-StudioBootstrapEnvironment -Name $name -Scope Process)}
        if($name -in $userNames){$old[$name].User=Get-StudioBootstrapEnvironment -Name $name -Scope User}
    }
    $localReport = $null; $sharedReport = $null; $controlSnapshot=$null
    try {
        Assert-StudioBootstrapOwner -Path $profile.control_root -MachineHash $profile.machine_id_sha256 -Control -Claim
        Assert-StudioBootstrapOwner -Path $profile.selected_cache_root -MachineHash $profile.machine_id_sha256 -Claim
        $receipts = Join-Path $profile.control_root 'bootstrap-receipts'
        [IO.Directory]::CreateDirectory($receipts) | Out-Null
        $localReport = Join-Path $receipts ($runId + '.json')
        $privateBackup=Join-Path $receipts ($runId + '.private')
        New-StudioBootstrapPrivateDirectory -Path $privateBackup
        Write-StudioBootstrapJson -Path (Join-Path $privateBackup 'environment-rollback.json') -Value ([ordered]@{schema='elon.studio_cache.environment_rollback.v1';run_id=$runId;previous=$old})
        $report.stage='configuration'
        Write-StudioBootstrapJson -Path $plan.profile_path -Value $profile
        Write-StudioBootstrapJson -Path $profile.sccache_tiers_path -Value $plan.tiers
        $values = @($profile.control_root,$profile.sccache_tiers_path,$plan.profile_path,$profile.selected_cache_root)
        for ($i=0;$i -lt $values.Count;$i++) { Set-StudioBootstrapEnvironment -Name $names[$i] -Value $values[$i] -Scope Process; Set-StudioBootstrapEnvironment -Name $names[$i] -Value $values[$i] -Scope User }
        $controls=@(Get-StudioBootstrapControlFiles -SourceRoot $source -CacheRoot $profile.selected_cache_root -ControlRoot $profile.control_root)
        $controlSnapshot=Save-StudioBootstrapControlFiles -Files $controls -BackupRoot $privateBackup
        $report.stage='install-and-doctor'
        Assert-StudioBootstrapSource -SourceRoot $source -SourceSha256 $SourceSha256
        Invoke-StudioBootstrapPlatform -SourceRoot $source -ProjectRoot $ProjectRoot -CacheRoot $profile.selected_cache_root
        $report.status='installed'; $report.stage='completed'
    } catch {
        $report.status='failed'; $report.error_code=($_.Exception.Message -split ':',2)[0]
        if ($report.error_code -notmatch '^STUDIO_[A-Z_]+$') { $report.error_code='STUDIO_OPERATION_FAILED' }
        foreach ($name in $names) {
            Set-StudioBootstrapEnvironment -Name $name -Value $old[$name].Process -Scope Process
            if($name -in $userNames){Set-StudioBootstrapEnvironment -Name $name -Value $old[$name].User -Scope User}
        }
        $report['environment_restored']=$true
        $report['configuration_rollback_required']=($report.stage -eq 'install-and-doctor')
        if ($controlSnapshot) {
            try { $restored=Restore-StudioBootstrapControlFiles -Snapshot $controlSnapshot; $report['control_files_rollback']=$restored; $report['configuration_rollback_required']=(-not $restored.complete) }
            catch { $report['control_files_rollback']=[pscustomobject]@{complete=$false;error_code='STUDIO_CONTROL_ROLLBACK_FAILED'};$report['configuration_rollback_required']=$true }
        }
        $report['installed_cache_files_retained']=$true
        throw
    } finally {
        if ($localReport) { Write-StudioBootstrapJson -Path $localReport -Value $report }
        try {
            Assert-StudioBootstrapPath -Path $plan.report_root | Out-Null
            [IO.Directory]::CreateDirectory($plan.report_root) | Out-Null
            $sharedReport=Join-Path $plan.report_root ($runId + '.json')
            Write-StudioBootstrapJson -Path $sharedReport -Value $report
        } catch { Write-Warning 'STUDIO_REPORT_DEFERRED: Shared report could not be written; retain the local receipt.' }
        if ($localReport) { Write-Host "STUDIO_LOCAL_REPORT=$localReport" }
        if ($sharedReport -and (Test-Path -LiteralPath $sharedReport)) { Write-Host "STUDIO_SHARED_REPORT=$sharedReport" }
    }
    [pscustomobject]@{schema='elon.studio_cache.bootstrap_result.v1';status=$report.status;profile_path=$plan.profile_path;local_report=$localReport;shared_report=$sharedReport;profile=$profile}
}

Export-ModuleMember -Function Resolve-StudioHostShare,New-StudioCacheBootstrapPlan,Invoke-StudioCacheBootstrap
