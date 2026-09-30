$ErrorActionPreference = 'Stop'
$modulesRoot = Join-Path $PSScriptRoot 'rust-cache'
Import-Module "$modulesRoot\RustCache.NetworkStorage.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Portability.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Runtime.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Inventory.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Paths.psm1" -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Runtime.psm1" -DisableNameChecking
Import-Module "$modulesRoot\RustCache.NetworkStorage.psm1" -DisableNameChecking
$networkModule = Get-Module RustCache.NetworkStorage
$runtimeModule = Get-Module RustCache.Runtime
$inventoryModule = Get-Module RustCache.Inventory
$script:Assertions = 0

function Assert-Equal {
    param($Expected, $Actual, [string]$Message)
    $script:Assertions++
    if ($Expected -ne $Actual) { throw "ASSERT FAILED: $Message expected='$Expected' actual='$Actual'" }
}

function Assert-True {
    param([bool]$Condition, [string]$Message)
    $script:Assertions++
    if (-not $Condition) { throw "ASSERT FAILED: $Message" }
}

function Assert-Throws {
    param([scriptblock]$Action, [string]$Pattern, [string]$Message)
    $caught = $null
    try { & $Action | Out-Null } catch { $caught = $_.Exception.Message }
    Assert-True ($null -ne $caught -and $caught -match $Pattern) "$Message (error=$caught)"
}

$tempRoot = Join-Path (Join-Path (Split-Path $PSScriptRoot -Parent) '.ai-tmp') ('network-cache-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
$tempRoot = [IO.Path]::GetFullPath($tempRoot)
$networkPredicate = & $networkModule { (Get-Command Test-RustCacheNetworkPath).ScriptBlock }
$volumeProbe = & $networkModule { (Get-Command Get-RustCacheWindowsVolumeBytes).ScriptBlock }

try {
    if ($env:OS -eq 'Windows_NT') {
        $unicodePart = ([char]0x7f13).ToString() + [char]0x5b58
        $unc = '\\cache-host\share\' + $unicodePart + '\build'
        $extendedUnc = '\\?\UNC\cache-host\share\' + $unicodePart + '\build'
        Assert-Equal $extendedUnc (ConvertTo-RustCacheExtendedPath $unc) 'UNC long paths retain Unicode and use the UNC namespace'
        Assert-Equal $extendedUnc (ConvertTo-RustCacheExtendedPath $extendedUnc) 'extended UNC paths stay idempotent'
        Assert-Equal '\\?\D:\cache\build' (ConvertTo-RustCacheExtendedPath 'D:\cache\build') 'local long paths stay compatible'
        Assert-True (Test-RustCacheNetworkPath $unc) 'UNC root is recognized without probing the share'
        Assert-True (Test-RustCacheNetworkPath $extendedUnc) 'extended UNC is recognized as network storage'

        & $networkModule {
            function script:Get-RustCacheWindowsVolumeBytes {
                param($Path)
                $script:LastVolumeProbePath = $Path
                [pscustomobject]@{ total_bytes = 1000; free_bytes = 250 }
            }
        }
        $volume = Get-RustCacheVolumeState -CacheRoot $unc
        Assert-Equal '\\cache-host\share' $volume.root 'volume probe receives the UNC share root'
        Assert-Equal 25 $volume.free_percent 'UNC quota-aware available bytes drive the waterline'
        Assert-Equal 250 $volume.free_bytes 'UNC available bytes are preserved'
        $advice = Get-RustCacheMigrationAdvice -CacheRoot $unc -LowWatermarkPercent 30 -ManagedAlternativeRoot $tempRoot
        Assert-True $advice.migration_recommended 'migration advice uses the same UNC-aware volume adapter'
        Assert-Equal '\\cache-host\share' (& $networkModule { $script:LastVolumeProbePath }) 'migration never passes UNC to DriveInfo'
        $doctor = Get-RustCacheDoctor -ProjectRoot (Split-Path $PSScriptRoot -Parent) -SourceScriptsRoot $PSScriptRoot -CacheRoot $tempRoot -UserLauncherPath (Join-Path $tempRoot 'unused-launcher.ps1')
        $diskCheck = @($doctor.checks | Where-Object { $_.id -eq 'disk-space' })
        Assert-Equal 1 $diskCheck.Count 'doctor emits one storage check'
        Assert-True ($diskCheck[0].message -match '25%') 'doctor uses the same quota-aware storage adapter'
        & $networkModule { param($Original) Set-Item Function:script:Get-RustCacheWindowsVolumeBytes -Value $Original } $volumeProbe

        $localVolume = Get-RustCacheVolumeState -CacheRoot $tempRoot
        $drive = [IO.DriveInfo]::new([IO.Path]::GetPathRoot($tempRoot))
        Assert-Equal $drive.TotalSize $localVolume.total_bytes 'native probe preserves local volume totals'
        Assert-True ($localVolume.free_bytes -gt 0) 'native local probe returns available space'
        Assert-Throws { & $networkModule { Get-RustCacheWindowsVolumeBytes -Path '?:\' } } 'RUST_CACHE_STORAGE_UNAVAILABLE' 'native failure reports an actionable storage error'
    }

    $networkFixture = Join-Path $tempRoot 'simulated-share'
    New-Item -ItemType Directory -Path $networkFixture | Out-Null
    & $networkModule {
        param($Fixture, $Original)
        $script:NetworkFixture = $Fixture
        $script:OriginalNetworkPredicate = $Original
        function script:Test-RustCacheNetworkPath {
            param($Path)
            if ([IO.Path]::GetFullPath($Path).StartsWith($script:NetworkFixture, [StringComparison]::OrdinalIgnoreCase)) { return $true }
            & $script:OriginalNetworkPredicate -Path $Path
        }
    } $networkFixture $networkPredicate

    $ownedRoot = Join-Path $networkFixture 'owned'
    $resolved = Resolve-RustCacheRoot -ExplicitRoot $ownedRoot
    Assert-Equal $ownedRoot $resolved 'empty per-PC network root can be initialized'
    $marker = Join-Path $ownedRoot '.rust-cache-owner.json'
    $owner = Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json
    Assert-Equal (Get-RustCacheMachineIdentity) $owner.machine_id_sha256 'root stores only a machine identity hash'
    $originalMarker = [IO.File]::ReadAllText($marker)
    Resolve-RustCacheRoot -ExplicitRoot $ownedRoot -NoCreate | Out-Null
    Assert-Equal $originalMarker ([IO.File]::ReadAllText($marker)) 'same-machine reads preserve the owner marker bytes'

    $unclaimed = Join-Path $networkFixture 'unclaimed'
    New-Item -ItemType Directory -Path $unclaimed | Out-Null
    Set-Content -LiteralPath (Join-Path $unclaimed 'existing.data') -Value 'preserve'
    Assert-Throws { Resolve-RustCacheRoot -ExplicitRoot $unclaimed } 'RUST_CACHE_SHARED_ROOT_UNCLAIMED' 'populated unknown roots cannot be implicitly claimed'
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $unclaimed '.rust-cache-owner.json'))) 'unclaimed root remains unmodified'
    $foreignHash = 'f' * 64
    if ($foreignHash -eq (Get-RustCacheMachineIdentity)) { $foreignHash = 'e' * 64 }
    $owner.machine_id_sha256 = $foreignHash
    $owner | ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding UTF8
    $foreignMarker = [IO.File]::ReadAllText($marker)
    Assert-Throws { Resolve-RustCacheRoot -ExplicitRoot $ownedRoot } 'RUST_CACHE_SHARED_ROOT_FOREIGN' 'another PC cannot resolve or claim this root'
    Assert-Equal $foreignMarker ([IO.File]::ReadAllText($marker)) 'foreign ownership is never overwritten'
    Set-Content -LiteralPath $marker -Value '{invalid' -Encoding UTF8
    Assert-Throws { Resolve-RustCacheRoot -ExplicitRoot $ownedRoot } 'RUST_CACHE_SHARED_OWNER_INVALID' 'malformed ownership fails closed'
    [IO.File]::WriteAllText($marker, $originalMarker)
    & $networkModule {
        param($FixtureMarker)
        $script:ReparseFixture = $FixtureMarker
        function script:Get-Item {
            [CmdletBinding()] param($LiteralPath, [switch]$Force)
            if ($LiteralPath -eq $script:ReparseFixture) {
                return [pscustomobject]@{ FullName = $LiteralPath; Attributes = [IO.FileAttributes]::ReparsePoint }
            }
            Microsoft.PowerShell.Management\Get-Item -LiteralPath $LiteralPath -Force:$Force
        }
    } $marker
    try {
        Assert-Throws { Resolve-RustCacheRoot -ExplicitRoot $ownedRoot } 'RUST_CACHE_SHARED_OWNER_INVALID' 'reparse owner marker is refused before reading its target'
        & $networkModule { param($Ancestor) $script:ReparseFixture = $Ancestor } $networkFixture
        Assert-Throws { Resolve-RustCacheRoot -ExplicitRoot $ownedRoot } 'RUST_CACHE_SHARED_REPARSE_ROOT' 'reparse ancestor cannot redirect cache ownership'
        & $networkModule { param($LocalRoot) $script:ReparseFixture = $LocalRoot } $tempRoot
        Assert-Throws { Resolve-RustCacheRoot -ExplicitRoot (Join-Path $tempRoot 'local-link') -NoCreate } 'RUST_CACHE_SHARED_REPARSE_ROOT' 'local drive aliases cannot bypass the reparse guard'
    } finally { & $networkModule { $script:ReparseFixture = $null; Remove-Item Function:\Get-Item } }

    $partition = Join-Path $ownedRoot 'build\test-partition'
    $lease = Enter-RustCacheLock -CacheRoot $ownedRoot -BuildDir $partition -WorkspaceRoot $tempRoot -TimeoutSeconds 0
    $lockOwnerPath = Join-Path $lease 'owner.json'
    $lockOwner = Get-Content -LiteralPath $lockOwnerPath -Raw | ConvertFrom-Json
    Assert-Equal (Get-RustCacheMachineIdentity) $lockOwner.machine_id_sha256 'new locks carry their machine identity'
    Assert-Equal 'active' (Get-RustCacheLockState $lease).state 'same-machine live lock remains active'
    $lockOwner.machine_id_sha256 = $foreignHash
    $lockOwner.pid = 2147483000
    $lockOwner | ConvertTo-Json | Set-Content -LiteralPath $lockOwnerPath -Encoding UTF8
    Assert-Equal 'foreign' (Get-RustCacheLockState $lease).reason 'foreign missing PID is not treated as local stale PID'
    $clock = [Diagnostics.Stopwatch]::StartNew()
    Assert-Throws { Enter-RustCacheLock -CacheRoot $ownedRoot -BuildDir $partition -WorkspaceRoot $tempRoot -TimeoutSeconds 1 } 'RUST_CACHE_LOCK_TIMEOUT.*foreign' 'foreign lock acquisition has a deadline'
    Assert-True ($clock.Elapsed.TotalSeconds -lt 3) 'foreign lock wait is bounded'
    Assert-True (Test-Path -LiteralPath $lease) 'foreign lock is preserved after timeout'
    $lockOwner.pid = $PID
    $lockOwner | ConvertTo-Json | Set-Content -LiteralPath $lockOwnerPath -Encoding UTF8
    Exit-RustCacheLock -LockPath $lease
    Assert-True (Test-Path -LiteralPath $lease) 'matching PID on another machine cannot release its lock'

    $lockOwner.PSObject.Properties.Remove('machine_id_sha256')
    $lockOwner.pid = 2147483000
    $lockOwner | ConvertTo-Json | Set-Content -LiteralPath $lockOwnerPath -Encoding UTF8
    Assert-Equal 'unknown-network-owner' (Get-RustCacheLockState $lease).reason 'legacy network locks are never guessed to be local'
    Set-Content -LiteralPath $lockOwnerPath -Value '{invalid' -Encoding UTF8
    (Get-Item -LiteralPath $lease).LastWriteTimeUtc = [DateTime]::UtcNow.AddMinutes(-5)
    Assert-Equal 'unknown-network-owner' (Get-RustCacheLockState $lease).reason 'old malformed network lock owners remain protected'
    Assert-Throws { Enter-RustCacheLock -CacheRoot $ownedRoot -BuildDir $partition -WorkspaceRoot $tempRoot -TimeoutSeconds 0 } 'RUST_CACHE_LOCK_TIMEOUT.*unknown-network-owner' 'malformed network locks cannot be reclaimed'
    Remove-Item -LiteralPath $lockOwnerPath
    (Get-Item -LiteralPath $lease).LastWriteTimeUtc = [DateTime]::UtcNow.AddMinutes(-5)
    Assert-Equal 'unknown-network-owner' (Get-RustCacheLockState $lease).reason 'old missing network lock owners remain protected'
    Assert-True (Test-Path -LiteralPath $lease) 'unidentified network locks remain intact'

    $localRoot = Join-Path $tempRoot 'local-cache'
    $localPartition = Join-Path $localRoot 'build\old-lock'
    $localLock = Join-Path $localPartition '.rust-cache.lockdir'
    New-Item -ItemType Directory -Path $localLock -Force | Out-Null
    @{ pid = 2147483000; started_utc = '2000-01-01T00:00:00Z' } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $localLock 'owner.json') -Encoding UTF8
    Assert-Equal 'stale' (Get-RustCacheLockState $localLock).state 'legacy local stale locks remain recoverable'
    $recovered = Enter-RustCacheLock -CacheRoot $localRoot -BuildDir $localPartition -WorkspaceRoot $tempRoot -TimeoutSeconds 0
    Assert-Equal $PID (Get-RustCacheLockOwner $recovered).pid 'GC zero-timeout still allows one local stale-lock recovery'
    Exit-RustCacheLock $recovered
    Assert-True (-not (Test-Path -LiteralPath $recovered)) 'own local lock releases normally'

    # Inject persistent create failure with an absent lock: this formerly bypassed the timeout forever.
    & $runtimeModule {
        function script:New-Item {
            [CmdletBinding()] param($ItemType, $Path, [switch]$Force)
            if ($Path.EndsWith('.rust-cache.lockdir')) { throw [IO.IOException]::new('simulated storage disconnect') }
            Microsoft.PowerShell.Management\New-Item -ItemType $ItemType -Path $Path -Force:$Force
        }
    }
    try {
        $clock.Restart()
        Assert-Throws { Enter-RustCacheLock -CacheRoot $localRoot -BuildDir $localPartition -WorkspaceRoot $tempRoot -TimeoutSeconds 1 } 'RUST_CACHE_LOCK_TIMEOUT.*simulated storage disconnect' 'absent-lock I/O failure cannot spin forever'
        Assert-True ($clock.Elapsed.TotalSeconds -lt 3) 'I/O retry loop respects its deadline'
    } finally { & $runtimeModule { Remove-Item Function:\New-Item } }

    # Exercise real local atomic quarantine/move/delete through the new path adapter.
    $moveSource = Join-Path $localRoot 'build\move-test'
    New-Item -ItemType Directory -Path $moveSource -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $moveSource 'artifact.bin') -Value 'fixture'
    $trashPath = & $inventoryModule { param($Root, $Source) Move-RustCachePartitionToTrash -CacheRoot $Root -Path $Source } $localRoot $moveSource
    Assert-True (-not (Test-Path -LiteralPath $moveSource) -and (Test-Path -LiteralPath (Join-Path $trashPath 'artifact.bin'))) 'local quarantine move retains its contents'
    & $inventoryModule { param($Path) Remove-RustCachePartition -Path $Path } $trashPath
    Assert-True (-not (Test-Path -LiteralPath $trashPath)) 'local quarantine deletion remains functional'

    $gcRoot = Join-Path $tempRoot 'gc-race-cache'
    $gcPartition = Join-Path $gcRoot 'build\rustc-old\fixture\agent-validation\gc-race'
    New-Item -ItemType Directory -Path $gcPartition -Force | Out-Null
    @{ workspace_root = $tempRoot; cache_scope = 'workspace'; last_used_utc = '2000-01-01T00:00:00Z' } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $gcPartition '.last-used.json') -Encoding UTF8
    $removePartition = & $inventoryModule { (Get-Command Remove-RustCachePartitionSafely).ScriptBlock }
    & $inventoryModule {
        function script:Remove-RustCachePartitionSafely {
            param($CacheRoot, $Path, $WorkspaceRoot)
            New-Item -ItemType Directory -Path (Join-Path $Path '.rust-cache.lockdir') -Force | Out-Null
            throw 'RUST_CACHE_LOCK_TIMEOUT: Timed out waiting for Rust cache lock: injected-after-plan'
        }
    }
    try {
        $gcResult = Invoke-RustCacheGc -CacheRoot $gcRoot -RepoRoot $tempRoot -ForceAged -Apply
        $raceAction = @($gcResult.actions | Where-Object { $_.path -eq $gcPartition })
        Assert-Equal 1 $raceAction.Count 'GC emits a result for the newly locked partition'
        Assert-Equal 'preserve' $raceAction[0].action 'GC preserves a lock appearing after planning'
        Assert-Equal 'lock-appeared' $raceAction[0].reason 'structured timeout retains the GC race receipt contract'
        Assert-True (Test-Path -LiteralPath $gcResult.report_path) 'partial GC still writes its audit report'
        Assert-True (Test-Path -LiteralPath $gcPartition) 'race candidate was not removed'
    } finally {
        & $inventoryModule { param($Original) Set-Item Function:script:Remove-RustCachePartitionSafely -Value $Original } $removePartition
    }
    Write-Host "PASS: Rust cache network storage ($script:Assertions assertions)."
} finally {
    & $networkModule { param($Original) Set-Item Function:script:Test-RustCacheNetworkPath -Value $Original } $networkPredicate
    & $networkModule { param($Original) Set-Item Function:script:Get-RustCacheWindowsVolumeBytes -Value $Original } $volumeProbe
    $expectedParent = [IO.Path]::GetFullPath((Join-Path (Split-Path $PSScriptRoot -Parent) '.ai-tmp')).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    if (-not $tempRoot.StartsWith($expectedParent, [StringComparison]::OrdinalIgnoreCase)) { throw 'Refusing test cleanup outside .ai-tmp.' }
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
