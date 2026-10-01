$ErrorActionPreference='Stop'
$modules=Join-Path $PSScriptRoot 'rust-cache'
Import-Module "$modules\RustCache.Install.psm1" -Force -DisableNameChecking
Import-Module "$modules\RustCache.CargoIncludeMigration.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.Portability.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.NetworkStorage.psm1" -DisableNameChecking
Import-Module "$modules\RustCache.ControlFiles.psm1" -DisableNameChecking
$script:Assertions=0
function Assert-True([bool]$Condition,[string]$Message){$script:Assertions++;if(-not $Condition){throw "ASSERT: $Message"}}
function Assert-Rejected([scriptblock]$Action,[string]$Pattern,[string]$Message){$errorText=$null;try{& $Action|Out-Null}catch{$errorText=$_.Exception.Message};Assert-True ($errorText -match $Pattern) $Message}
function Write-Json($Value,[string]$Path){[IO.File]::WriteAllText($Path,($Value|ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false))}
$fixture=Join-Path (Split-Path $PSScriptRoot -Parent) ('.ai-tmp\cargo-include-migration-'+[Guid]::NewGuid().ToString('N'))
$oldRoot=Join-Path $fixture 'old-managed';$platform=Join-Path $oldRoot 'platform';$cargo=Join-Path $fixture 'cargo-config.toml'
$oldInclude=Join-Path $oldRoot 'config\cargo-cache.toml';$newInclude=Join-Path $fixture 'local-control\config\cargo-cache.toml'
[IO.Directory]::CreateDirectory((Join-Path $platform 'rust-cache'))|Out-Null
[IO.Directory]::CreateDirectory((Split-Path $oldInclude -Parent))|Out-Null
[IO.File]::WriteAllText((Join-Path $platform 'rust-cache.ps1'),'# fixture managed entry')
[IO.File]::WriteAllText((Join-Path $platform 'rust-cache\Fixture.psm1'),'# fixture module')
$fingerprint=Get-RustCachePlatformFingerprint -SourceScriptsRoot $platform -Installed
$manifestPath=Join-Path $platform 'platform-install.json'
$manifest=@{schema='elon.rust_cache.platform_install.v1';installed_fingerprint_mode='raw-bytes-v1';entry_relative_path='platform/rust-cache.ps1';installed_hash=$fingerprint.hash;installed_file_count=$fingerprint.file_count}
Write-Json $manifest $manifestPath
$ownerPath=Join-Path $oldRoot '.rust-cache-owner.json'
$owner=@{schema='elon.rust_cache.root_owner.v1';machine_id_sha256=(Get-RustCacheMachineIdentity)}
Write-Json $owner $ownerPath
$includeContent=Get-RustCacheCargoIncludeContent -CacheRoot $oldRoot -SccachePath (Join-Path $platform 'rustc-sccache-wrapper.exe')
[IO.File]::WriteAllText($oldInclude,$includeContent,[Text.UTF8Encoding]::new($false))
$oldToml=$oldInclude.Replace('\','/');$newToml=$newInclude.Replace('\','/')
$original="include = [`"$oldToml`"]`r`n`r`n[build]`r`njobs = 12`r`ntarget-dir = `"custom-target`"`r`nrustflags = [`"-C`", `"target-cpu=native`"]`r`n`r`n[net]`r`nretry = 3`r`n`r`n[env]`r`nFIXTURE_VALUE = `"preserve-this-value`"`r`n"
function Restore-FixtureCargo {[IO.File]::WriteAllText($cargo,$original,[Text.UTF8Encoding]::new($false))}
Restore-FixtureCargo
$originalBytes=[IO.File]::ReadAllBytes($cargo)
$proposal=Set-RustCacheParentCargoConfig -CargoConfigPath $cargo -IncludeConfigPath $newInclude
Assert-True (-not $proposal.applied -and $proposal.include_migration.schema -eq 'elon.rust_cache.include_migration.v1') 'preview proves managed origin without applying'
Assert-True ([Convert]::ToBase64String([IO.File]::ReadAllBytes($cargo)) -eq [Convert]::ToBase64String($originalBytes)) 'preview preserves original parent bytes'
Assert-True ($proposal.content -ceq $original.Replace($oldToml,$newToml)) 'migration changes only the include and preserves all other Cargo settings'
$applied=Set-RustCacheParentCargoConfig -CargoConfigPath $cargo -IncludeConfigPath $newInclude -Apply
Assert-True ($applied.applied -and [IO.File]::ReadAllText($cargo) -ceq $proposal.content) 'apply matches the reviewed content'
Assert-True ([Convert]::ToBase64String([IO.File]::ReadAllBytes($applied.backup_path)) -eq [Convert]::ToBase64String($originalBytes)) 'backup preserves exact previous bytes'
Restore-FixtureCargo
foreach($case in @('owner','manifest','module','include')){
    $path=switch($case){owner{$ownerPath}manifest{$manifestPath}module{Join-Path $platform 'rust-cache\Fixture.psm1'}include{$oldInclude}}
    $bytes=[IO.File]::ReadAllBytes($path)
    switch($case){
        owner{$bad=@{schema=$owner.schema;machine_id_sha256=('0'*64)};Write-Json $bad $path}
        manifest{$bad=$manifest.Clone();$bad.installed_hash='0'*64;Write-Json $bad $path}
        module{[IO.File]::AppendAllText($path,'# modified')}
        include{[IO.File]::AppendAllText($path,"`n[env]`nUNREVIEWED = `"x`"`n")}
    }
    Assert-Rejected {Set-RustCacheParentCargoConfig -CargoConfigPath $cargo -IncludeConfigPath $newInclude -Apply} 'RUST_CACHE_INCLUDE_' "reject changed managed $case evidence"
    Assert-True ([Convert]::ToBase64String([IO.File]::ReadAllBytes($cargo)) -eq [Convert]::ToBase64String($originalBytes)) "changed $case leaves parent untouched"
    [IO.File]::WriteAllBytes($path,$bytes)
}
$proof=Get-RustCacheManagedIncludeMigration -CargoConfigPath $cargo -PreviousIncludePath $oldInclude
[IO.File]::AppendAllText($cargo,'# user edit')
Assert-Rejected {Assert-RustCacheIncludeMigrationCurrent $proof} 'MIGRATION_DRIFT' 'previously reviewed parent hash cannot drift'
Restore-FixtureCargo
foreach($line in @("include = [`"$oldToml`", `"$newToml`"]","include = [`"$newToml/unrelated`"]","include = [`"relative.toml`"]","[build]`r`ninclude = [`"$oldToml`"]","include = [`"$oldToml`"]`r`ninclude = [`"$oldToml`"]")){
    [IO.File]::WriteAllText($cargo,$line)
    Assert-Rejected {Set-RustCacheParentCargoConfig -CargoConfigPath $cargo -IncludeConfigPath $newInclude -Apply} 'RUST_CACHE_INCLUDE_' 'unrelated, multiple, relative and nested includes remain rejected'
    Assert-True ([IO.File]::ReadAllText($cargo) -ceq $line) 'rejected include retains exact original content'
}
Restore-FixtureCargo
$linked=Join-Path $fixture 'linked';New-Item -ItemType Junction -Path $linked -Target $oldRoot|Out-Null
try{Assert-Rejected {Get-RustCacheManagedIncludeMigration -CargoConfigPath $cargo -PreviousIncludePath (Join-Path $linked 'config\cargo-cache.toml')} 'REPARSE' 'linked source is never used as ownership evidence'}finally{[IO.Directory]::Delete($linked)}
[IO.File]::WriteAllText($cargo,'[net]'+"`r`nretry = 3`r`n")
$fresh=Set-RustCacheParentCargoConfig -CargoConfigPath $cargo -IncludeConfigPath $newInclude
Assert-True ($null -eq $fresh.include_migration -and $fresh.content.Contains('retry = 3')) 'fresh installation remains supported'
[IO.File]::WriteAllText($cargo,("include = [`"$newToml`"]`r`n[net]`r`nretry = 3`r`n"))
$same=Set-RustCacheParentCargoConfig -CargoConfigPath $cargo -IncludeConfigPath $newInclude
Assert-True ($null -eq $same.include_migration) 'same-target reinstall does not need previous installation evidence'
$guarded=Join-Path $fixture 'guarded.txt';$oldBytes=[Text.Encoding]::UTF8.GetBytes('old-control');$newBytes=[Text.Encoding]::UTF8.GetBytes('new-control')
[IO.File]::WriteAllBytes($guarded,$oldBytes);$guardHash=Get-RustCacheControlBytesHash $oldBytes
$writer=[IO.File]::Open($guarded,[IO.FileMode]::Open,[IO.FileAccess]::Write,[IO.FileShare]::ReadWrite)
try{Assert-Rejected {Write-RustCacheBoundControlFile -Path $guarded -ExpectedSha256 $guardHash -Bytes $newBytes} 'CONTROL_BUSY' 'active editor handle prevents guarded update'}finally{$writer.Dispose()}
Assert-True ([IO.File]::ReadAllText($guarded) -ceq 'old-control') 'blocked writer case preserves original content'
[IO.File]::WriteAllText($guarded,'concurrent-edit')
Assert-Rejected {Write-RustCacheBoundControlFile -Path $guarded -ExpectedSha256 $guardHash -Bytes $newBytes} 'CONTROL_DRIFT' 'same-handle comparison rejects edits made after preview'
Assert-True ([IO.File]::ReadAllText($guarded) -ceq 'concurrent-edit') 'digest mismatch cannot truncate target'
Assert-Rejected {Write-RustCacheBoundControlFile -Path $guarded -ExpectedMissing -Bytes $newBytes} 'CONTROL_BUSY' 'CreateNew refuses a path created after missing-file preview'
$absent=Join-Path $fixture 'create-new.txt'
Write-RustCacheBoundControlFile -Path $absent -ExpectedMissing -Bytes $newBytes|Out-Null
Assert-True ([IO.File]::ReadAllText($absent) -ceq 'new-control') 'new-file path uses exclusive CreateNew'
[IO.File]::WriteAllBytes($guarded,$oldBytes)
Write-RustCacheBoundControlFile -Path $guarded -ExpectedSha256 $guardHash -Bytes $newBytes|Out-Null
Assert-True ([IO.File]::ReadAllText($guarded) -ceq 'new-control') 'guarded existing update writes verified bytes'
Write-Output "PASS include_migration_assertions=$script:Assertions PowerShell=$($PSVersionTable.PSVersion) production_mutations=false"
