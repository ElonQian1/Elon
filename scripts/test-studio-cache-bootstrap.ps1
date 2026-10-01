$ErrorActionPreference='Stop'
Import-Module Microsoft.PowerShell.Utility -ErrorAction Stop
Import-Module (Join-Path $PSScriptRoot 'studio-cache\StudioCache.Bootstrap.psm1') -Force -DisableNameChecking
$module=Get-Module StudioCache.Bootstrap
$script:Assertions=0
function Assert-True { param([bool]$Condition,[string]$Message) $script:Assertions++; if (-not $Condition) { throw "ASSERT: $Message" } }
function Assert-Equal { param($Expected,$Actual,[string]$Message) Assert-True ($Expected -ceq $Actual) "$Message (expected=$Expected actual=$Actual)" }
function Assert-Throws { param([scriptblock]$Action,[string]$Pattern,[string]$Message) $errorText=$null; try { & $Action | Out-Null } catch { $errorText=$_.Exception.Message }; Assert-True ($errorText -match $Pattern) "$Message (error=$errorText)" }
function Get-TestFileHash { param([string]$Path) $stream=[IO.File]::OpenRead($Path);$sha=[Security.Cryptography.SHA256]::Create();try {([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant()} finally {$sha.Dispose();$stream.Dispose()} }
$tempParent=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\.ai-tmp'))
$fixture=Join-Path $tempParent ('studio-bootstrap-test-' + [guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($fixture) | Out-Null
$physical=Join-Path $fixture 'physical-share'; $cache=Join-Path $physical 'cache'
[IO.Directory]::CreateDirectory($cache) | Out-Null
$control=Join-Path $fixture 'control'; $hostName=[Environment]::MachineName
$fixtureUser=Join-Path $fixture 'user-controls';[IO.Directory]::CreateDirectory($fixtureUser)|Out-Null
$fixtureCargo=Join-Path $fixtureUser 'cargo-config.toml';$fixtureLauncher=Join-Path $fixtureUser 'launcher.ps1'
[byte[]]$originalCargoBytes=65,66,67,13,10
[IO.File]::WriteAllBytes($fixtureCargo,$originalCargoBytes)
$arguments=@{Role='Host';HostName=$hostName;ShareName='fixture-share';CacheDirectory='cache';LocalControlRoot=$control}
try {
    $temporaryEnvironment='ELON_STUDIO_BOOTSTRAP_TEST_' + [guid]::NewGuid().ToString('N')
    try {
        & $module {param($n) Set-StudioBootstrapEnvironment -Name $n -Value 'fixture' -Scope Process;Set-StudioBootstrapEnvironment -Name $n -Value $null -Scope Process} $temporaryEnvironment
        Assert-True ($null -eq [Environment]::GetEnvironmentVariable($temporaryEnvironment,'Process')) 'null process values are removed rather than restored as empty strings'
    } finally { Remove-Item -LiteralPath ('Env:'+$temporaryEnvironment) -ErrorAction SilentlyContinue }
    & $module {
        param($Physical,$Cargo,$Launcher)
        $script:FixtureShare=$Physical; $script:InstallerCalls=0; $script:EnvironmentValues=@{}; $script:FailInstall=$false
        $script:ControlFiles=@([pscustomobject]@{kind='cargo-config';path=$Cargo;expected_content="new-cargo`r`n"},[pscustomobject]@{kind='user-launcher';path=$Launcher;expected_content="new-launcher`r`n"})
        $script:FreeBytes=[long](200GB)
        function script:Get-SmbShare { param($Name) [pscustomobject]@{Name=$Name;Path=$script:FixtureShare} }
        function script:Get-RustCacheMachineIdentity { 'a' * 64 }
        function script:Get-RustCacheStorageVolume { param($CacheRoot) [pscustomobject]@{root=[IO.Path]::GetPathRoot($CacheRoot);total_bytes=[int64](500GB);free_bytes=$script:FreeBytes;free_percent=40} }
        function script:Get-StudioBootstrapHardware { [pscustomobject]@{verified=$false;fixture=$true} }
        function script:Get-StudioBootstrapControlFiles { $script:ControlFiles }
        function script:Invoke-StudioBootstrapPlatform { param($SourceRoot,$ProjectRoot,$CacheRoot) $script:InstallerCalls++; foreach($file in $script:ControlFiles){[IO.File]::WriteAllText($file.path,$file.expected_content,[Text.UTF8Encoding]::new($PSVersionTable.PSVersion.Major -lt 6))}; if ($script:FailInstall) { throw 'STUDIO_TEST_INSTALL: Deliberate failure.' } }
        function script:Get-StudioBootstrapEnvironment { param($Name,$Scope) if ($script:EnvironmentValues.ContainsKey("$Scope/$Name")) { return $script:EnvironmentValues["$Scope/$Name"] }; return 'old-value' }
        function script:Set-StudioBootstrapEnvironment { param($Name,$Value,$Scope) $script:EnvironmentValues["$Scope/$Name"]=$Value }
    } $physical $fixtureCargo $fixtureLauncher
    $before=@(Get-ChildItem -LiteralPath $fixture -Recurse -Force).Count
    $plan=Invoke-StudioCacheBootstrap @arguments
    Assert-Equal 'preview' $plan.mode 'default invocation previews'
    Assert-Equal $before @(Get-ChildItem -LiteralPath $fixture -Recurse -Force).Count 'preview writes no files'
    Assert-Equal 0 (& $module {$script:InstallerCalls}) 'preview never installs'
    Assert-True (-not (Test-Path -LiteralPath $control)) 'preview does not create local control root'
    Assert-Equal (Join-Path $cache 'pc-aaaaaaaaaaaa\rust-cache-studio-v1') $plan.profile.selected_cache_root 'host resolves exact physical share'
    Assert-Equal ('\\' + $hostName + '\fixture-share\cache\pc-aaaaaaaaaaaa\rust-cache-studio-v1') $plan.profile.shared_build_root 'UNC and local roots bind same partition'
    Assert-Equal ([int64]2147483648) $plan.profile.l1_max_bytes 'L1 is two GiB'
    Assert-Equal ([int64](22GB)) $plan.capacity.minimum_local_free_bytes 'bootstrap reserves twenty GiB plus the full two GiB L1'
    Assert-Equal $plan.profile.selected_cache_root $plan.tiers.cache_root 'tier configuration binds selected cache root'
    Assert-True (-not $plan.profile.remote_build_ready) 'bootstrap does not invent remote execution'
    Assert-Throws { New-StudioCacheBootstrapPlan @arguments -AllowUncBuild } 'STUDIO_HOST_LOCAL_ONLY' 'host never loops back through UNC'
    Assert-Throws { Resolve-StudioHostShare -HostName 'different-host' -ShareName 'fixture-share' } 'STUDIO_HOST_IDENTITY' 'wrong host is rejected'
    Assert-Throws { New-StudioCacheBootstrapPlan -Role Host -HostName $hostName -ShareName '..\escape' -CacheDirectory cache -LocalControlRoot $control } 'STUDIO_SHARE_ARGUMENT' 'share traversal rejected'
    & $module { $script:FixtureShare='Q:\path-that-does-not-exist' }
    Assert-Throws { New-StudioCacheBootstrapPlan @arguments } 'STUDIO_(SHARE_MAPPING|LOCAL_REQUIRED)' 'unprovable mapping never falls back'
    & $module {param($p) $script:FixtureShare=$p} $physical
    [IO.Directory]::CreateDirectory($control) | Out-Null
    [IO.File]::WriteAllText((Join-Path $control 'unknown.txt'),'preserve')
    Assert-Throws { New-StudioCacheBootstrapPlan @arguments } 'STUDIO_ROOT_UNOWNED' 'unknown populated control root preserved'
    Remove-Item -LiteralPath (Join-Path $control 'unknown.txt')
    [IO.File]::WriteAllText((Join-Path $control '.studio-cache-owner.json'),('{"schema":"elon.studio_cache.control_owner.v1","machine_id_sha256":"' + ('b'*64) + '"}'))
    Assert-Throws { New-StudioCacheBootstrapPlan @arguments } 'STUDIO_ROOT_FOREIGN' 'foreign control owner rejected'
    Remove-Item -LiteralPath (Join-Path $control '.studio-cache-owner.json')
    & $module {
        function script:Test-Path {
            param($LiteralPath,$Path,$PathType)
            $candidate=if ($LiteralPath) {$LiteralPath} else {$Path}
            if ($candidate -like '\\fixture-host\fixture-share*') { return $candidate -eq '\\fixture-host\fixture-share\cache' }
            if ($PathType) { return Microsoft.PowerShell.Management\Test-Path -LiteralPath $candidate -PathType $PathType }
            return Microsoft.PowerShell.Management\Test-Path -LiteralPath $candidate
        }
        function script:Get-Item { param($LiteralPath,[switch]$Force) if ($LiteralPath -like '\\fixture-host\fixture-share*') { return [pscustomobject]@{PSIsContainer=$true;Attributes=[IO.FileAttributes]::Directory} }; Microsoft.PowerShell.Management\Get-Item -LiteralPath $LiteralPath -Force:$Force }
    }
    $client=New-StudioCacheBootstrapPlan -Role Client -HostName fixture-host -ShareName fixture-share -CacheDirectory cache -LocalControlRoot $control
    Assert-Equal (Join-Path $control 'build-root') $client.profile.selected_cache_root 'client defaults local'
    $uncClient=New-StudioCacheBootstrapPlan -Role Client -HostName fixture-host -ShareName fixture-share -CacheDirectory cache -LocalControlRoot $control -AllowUncBuild
    Assert-Equal '\\fixture-host\fixture-share\cache\pc-aaaaaaaaaaaa\rust-cache-studio-v1' $uncClient.profile.selected_cache_root 'explicit client UNC uses own identity'
    # Keep these two module-scoped UNC stubs for this isolated process; local calls delegate unchanged.
    $source=Join-Path $fixture 'bundle'; [IO.Directory]::CreateDirectory($source) | Out-Null
    $sourceFile=Join-Path $source 'trusted.txt'; [IO.File]::WriteAllText($sourceFile,'reviewed')
    $manifest=Join-Path $source 'studio-cache-bundle.json'
    [IO.File]::WriteAllText($manifest,([ordered]@{schema='elon.studio_cache.bundle.v1';files=@(@{path='trusted.txt';sha256=(Get-TestFileHash $sourceFile)})} | ConvertTo-Json -Depth 4))
    $sourceHash=Get-TestFileHash $manifest
    & $module { $script:FreeBytes=[long](21GB) }
    Assert-Throws { Invoke-StudioCacheBootstrap @arguments -Apply -SourceRoot $source -SourceSha256 $sourceHash } 'STUDIO_CAPACITY' 'twenty-one GiB does not fit floor plus L1'
    & $module { $script:FreeBytes=[long](200GB) }
    Assert-Throws { Invoke-StudioCacheBootstrap @arguments -Apply -SourceRoot $source -SourceSha256 ('0'*64) } 'STUDIO_BUNDLE_HASH' 'source digest cannot be invented'
    [IO.File]::WriteAllText($sourceFile,'changed')
    Assert-Throws { Invoke-StudioCacheBootstrap @arguments -Apply -SourceRoot $source -SourceSha256 $sourceHash } 'STUDIO_BUNDLE_CONTENT' 'source file drift rejected before mutations'
    [IO.File]::WriteAllText($sourceFile,'reviewed')
    & $module { $script:FailInstall=$true }
    Assert-Throws { Invoke-StudioCacheBootstrap @arguments -Apply -SourceRoot $source -SourceSha256 $sourceHash } 'STUDIO_TEST_INSTALL' 'installer failure propagates'
    Assert-Equal 1 (& $module {$script:InstallerCalls}) 'only Apply invokes installer'
    Assert-Equal ([Convert]::ToBase64String($originalCargoBytes)) ([Convert]::ToBase64String([IO.File]::ReadAllBytes($fixtureCargo))) 'failed install restores original Cargo bytes'
    Assert-True (-not(Test-Path -LiteralPath $fixtureLauncher)) 'failed install removes only its newly created known launcher'
    $envValues=& $module {$script:EnvironmentValues}
    Assert-True (@($envValues.Values | Where-Object { $_ -cne 'old-value' }).Count -eq 0) 'all managed environment restored on failure'
    $localReceipts=@(Get-ChildItem -LiteralPath (Join-Path $control 'bootstrap-receipts') -Filter '*.json')
    Assert-Equal 1 $localReceipts.Count 'failure report durable alongside private rollback directory'
    $privateReceipts=@(Get-ChildItem -LiteralPath (Join-Path $control 'bootstrap-receipts') -Directory)
    Assert-Equal 1 $privateReceipts.Count 'control backups are in a dedicated restricted directory'
    $privateAcl=if($PSVersionTable.PSVersion.Major -lt 6){[IO.Directory]::GetAccessControl($privateReceipts[0].FullName)}else{[IO.FileSystemAclExtensions]::GetAccessControl([IO.DirectoryInfo]::new($privateReceipts[0].FullName))}
    Assert-True $privateAcl.AreAccessRulesProtected 'private backup ACL is protected'
    Assert-True (Test-Path -LiteralPath (Join-Path $privateReceipts[0].FullName 'environment-rollback.json')) 'environment receipt remains local'
    $reportFile=$localReceipts[0]
    $report=Get-Content -LiteralPath $reportFile.FullName -Raw | ConvertFrom-Json
    Assert-Equal 'failed' $report.status 'failure is not success'
    Assert-True $report.environment_restored 'restoration recorded'
    Assert-True (-not $report.configuration_rollback_required) 'known Cargo and launcher changes restored'
    Assert-True $report.control_files_rollback.complete 'both control files restore verified'
    Assert-True $report.installed_cache_files_retained 'installation files remain for diagnosis'
    $sharedReports=@(Get-ChildItem -LiteralPath (Join-Path $cache 'studio-bootstrap-reports') -Recurse -File)
    Assert-Equal 1 $sharedReports.Count 'safe shared machine report written'
    Assert-True ((Get-Content $sharedReports[0].FullName -Raw) -notmatch 'old-value|previous|credential|password') 'shared report excludes private rollback state'
    Assert-Throws { Invoke-StudioCacheBootstrap @arguments -Apply -SourceRoot $source -SourceSha256 $sourceHash } 'STUDIO_ALREADY_CONFIGURED' 'failed configuration is preserved for reviewed recovery'
    $successControl=Join-Path $fixture 'success-control'; $arguments.LocalControlRoot=$successControl
    & $module { $script:FailInstall=$false }
    $applied=Invoke-StudioCacheBootstrap @arguments -Apply -SourceRoot $source -SourceSha256 $sourceHash
    Assert-Equal 'installed' $applied.status 'successful fixture installs and reports'
    Assert-Equal 2 (& $module {$script:InstallerCalls}) 'successful Apply invokes trusted adapter once'
    $profile=Get-Content -LiteralPath $applied.profile_path -Raw | ConvertFrom-Json
    Assert-Equal 'elon.studio_cache.machine.v1' $profile.schema 'profile schema is versioned'
    Assert-True (-not $profile.remote_build_ready) 'success still does not claim remote compilation'
    $successBackup=@(Get-ChildItem -LiteralPath (Join-Path $successControl 'bootstrap-receipts') -Directory)[0].FullName
    $controlReceipt=Join-Path $successBackup 'control-files.json'
    $snapshot=[pscustomobject]@{path=$controlReceipt;sha256=(Get-TestFileHash $controlReceipt);backup_root=$successBackup}
    [IO.File]::WriteAllText($fixtureCargo,'unrelated-user-edit')
    $restoration=& $module {param($s) Restore-StudioBootstrapControlFiles -Snapshot $s} $snapshot
    Assert-True (-not $restoration.complete) 'concurrent user edits prevent automatic rollback'
    Assert-Equal 'unrelated-user-edit' ([IO.File]::ReadAllText($fixtureCargo)) 'rollback preserves unrelated user content'
    [IO.File]::AppendAllText($controlReceipt,' ')
    Assert-Throws { & $module {param($s) Restore-StudioBootstrapControlFiles -Snapshot $s} $snapshot } 'STUDIO_BACKUP_RECEIPT' 'backup receipt identity is bound to SHA'
    $cliBundle=Join-Path $fixture 'cli-bundle'; $cliScripts=Join-Path $cliBundle 'scripts'
    [IO.Directory]::CreateDirectory((Join-Path $cliScripts 'studio-cache')) | Out-Null
    $cliEntry=Join-Path $cliScripts 'initialize-studio-cache.ps1'
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'initialize-studio-cache.ps1') -Destination $cliEntry
    $fakeModule=Join-Path $cliScripts 'studio-cache\StudioCache.Bootstrap.psm1'
    [IO.File]::WriteAllText($fakeModule,"function Invoke-StudioCacheBootstrap { [pscustomobject]@{mode='fixture-preview'} }; Export-ModuleMember -Function Invoke-StudioCacheBootstrap")
    [IO.File]::WriteAllText((Join-Path $cliScripts 'rust-cache.ps1'),'throw "Preview must not execute installer"')
    [IO.File]::WriteAllText((Join-Path $cliBundle 'rust-cache.project.json'),'{"schema_version":1,"project_id":"fixture"}')
    $cliFiles=@(Get-ChildItem -LiteralPath $cliBundle -Recurse -File | ForEach-Object { @{path=$_.FullName.Substring($cliBundle.Length+1).Replace('\','/');sha256=(Get-TestFileHash $_.FullName)} })
    $cliManifest=Join-Path $cliBundle 'studio-cache-bundle.json'
    [IO.File]::WriteAllText($cliManifest,(@{schema='elon.studio_cache.bundle.v1';files=$cliFiles} | ConvertTo-Json -Depth 5))
    $cliHash=Get-TestFileHash $cliManifest
    $cliResult=& $cliEntry -Role Host -ExpectedSourceSha256 $cliHash | ConvertFrom-Json
    Assert-Equal 'fixture-preview' $cliResult.mode 'entry verifies bundle before dispatch'
    $extra=Join-Path $cliBundle 'unreviewed.ps1'; [IO.File]::WriteAllText($extra,'throw "must not execute"')
    Assert-Throws { & $cliEntry -Role Host -ExpectedSourceSha256 $cliHash } 'STUDIO_BUNDLE_CONTENT' 'entry refuses unlisted scripts'
    Remove-Item -LiteralPath $extra
    [IO.File]::AppendAllText($fakeModule,'; throw "tampered module executed"')
    Assert-Throws { & $cliEntry -Role Host -ExpectedSourceSha256 $cliHash } 'STUDIO_BUNDLE_CONTENT' 'entry rejects tampered module before import'
    Assert-Throws { & $cliEntry -Role Host -ExpectedSourceSha256 ('0'*64) } 'STUDIO_BUNDLE_HASH' 'entry requires externally pinned bundle identity'
    Write-Host "STUDIO_BOOTSTRAP_TESTS=passed assertions=$script:Assertions powershell=$($PSVersionTable.PSVersion)"
} finally {
    Remove-Module StudioCache.Bootstrap -Force -ErrorAction SilentlyContinue
    $full=[IO.Path]::GetFullPath($fixture)
    if (-not $full.StartsWith($tempParent + '\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Fixture cleanup escaped .ai-tmp.' }
    if (Test-Path -LiteralPath $full) { Remove-Item -LiteralPath $full -Recurse -Force }
}
