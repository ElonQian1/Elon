[CmdletBinding()]
param([string]$WorkerRoot, [string]$WorkerName)
$ErrorActionPreference = 'Stop'
Import-Module "$PSScriptRoot\rust-cache\RustCache.Capacity.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\rust-cache\RustCache.NetworkStorage.psm1" -DisableNameChecking
Import-Module "$PSScriptRoot\rust-cache\RustCache.Policy.psm1" -DisableNameChecking
$capacity = Get-Module RustCache.Capacity
$previousTierPath = $env:ELON_RUST_CACHE_SCCACHE_TIERS
$previousControlRoot = $env:ELON_RUST_CACHE_CONTROL_ROOT
$env:ELON_RUST_CACHE_SCCACHE_TIERS = $null
$env:ELON_RUST_CACHE_CONTROL_ROOT = $null

function Set-FixtureVolume {
    param([long]$Free = 30GB, [long]$Total = 100GB, [switch]$Separate, [switch]$Unknown, [switch]$SeparateL1, [long]$L1Free = 30GB)
    & $capacity {
        param($Free, $Total, $Separate, $Unknown, $SeparateL1, $L1Free)
        $script:FixtureFree = $Free; $script:FixtureTotal = $Total; $script:FixtureSeparate = $Separate; $script:FixtureUnknown = $Unknown
        $script:FixtureSeparateL1 = $SeparateL1; $script:FixtureL1Free = $L1Free
        function script:Get-RustCacheCapacityVolume {
            param([string]$Path)
            $key = if ($script:FixtureSeparate -and $Path.EndsWith('temp')) { 'fixture-temp-volume' } else { 'fixture-build-volume' }
            $freeValue = if ($script:FixtureUnknown) { $null } else { $script:FixtureFree }
            if ($script:FixtureSeparateL1 -and $Path.EndsWith('sccache-l1')) { $key = 'fixture-l1-volume'; $freeValue = $script:FixtureL1Free }
            [pscustomobject]@{ storage_id = $key; free_bytes = $freeValue; total_bytes = $script:FixtureTotal }
        }
    } $Free $Total ([bool]$Separate) ([bool]$Unknown) ([bool]$SeparateL1) $L1Free
}

if ($WorkerRoot) {
    Set-FixtureVolume
    $workerArgs = @{ CacheRoot = (Join-Path $WorkerRoot 'cache'); BuildDir = (Join-Path $WorkerRoot 'build'); TargetDir = (Join-Path $WorkerRoot 'target'); TempDir = (Join-Path $WorkerRoot 'temp'); ControlRoot = (Join-Path $WorkerRoot 'parallel-control') }
    [IO.File]::WriteAllText((Join-Path $WorkerRoot "$WorkerName.ready"), 'ready')
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    while (-not (Test-Path -LiteralPath (Join-Path $WorkerRoot 'go'))) { if ([DateTime]::UtcNow -gt $deadline) { throw 'worker start timeout' }; Start-Sleep -Milliseconds 50 }
    $lease = $null
    try {
        $lease = Enter-RustCacheCapacityReservation @workerArgs
        $result = [pscustomobject]@{ admitted = $true; message = 'admitted' }
    } catch { $result = [pscustomobject]@{ admitted = $false; message = $_.Exception.Message } }
    [IO.File]::WriteAllText((Join-Path $WorkerRoot "$WorkerName.result.json"), ($result | ConvertTo-Json -Compress))
    if ($lease) {
        try { while (-not (Test-Path -LiteralPath (Join-Path $WorkerRoot 'release'))) { if ([DateTime]::UtcNow -gt $deadline) { throw 'worker release timeout' }; Start-Sleep -Milliseconds 50 } }
        finally { Exit-RustCacheCapacityReservation -Reservation $lease }
    }
    exit 0
}

$testCount = 0
function Assert-True { param([bool]$Condition, [string]$Message) if (-not $Condition) { throw "ASSERT FAILED: $Message" }; $script:testCount++ }
function Assert-Throws {
    param([scriptblock]$Action, [string]$Pattern)
    try { & $Action | Out-Null } catch { if ($_.Exception.Message -like "*$Pattern*") { $script:testCount++; return }; throw "Unexpected failure (expected $Pattern): $($_.Exception.Message)" }
    throw "Expected failure: $Pattern"
}
function Write-FixtureJson { param([string]$Path, $Value) [IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 8 -Compress)) }
function Wait-FixtureFiles {
    param([string[]]$Paths)
    $deadline = [DateTime]::UtcNow.AddSeconds(25)
    while (@($Paths | Where-Object { -not (Test-Path -LiteralPath $_) }).Count -gt 0) {
        if ([DateTime]::UtcNow -gt $deadline) { throw "Fixture timed out waiting for $($Paths -join ', ')" }
        Start-Sleep -Milliseconds 50
    }
}

$fixtureParent = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\.ai-tmp'))
$fixture = Join-Path $fixtureParent ('capacity-fixture-' + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($fixture) | Out-Null
$children = @()
try {
    $argsForPlan = @{ CacheRoot = (Join-Path $fixture 'cache'); BuildDir = (Join-Path $fixture 'build'); TargetDir = (Join-Path $fixture 'target'); TempDir = (Join-Path $fixture 'temp'); ControlRoot = (Join-Path $fixture 'control') }
    $defaultPolicy = Get-DefaultRustCachePolicy
    Assert-True ($defaultPolicy.capacity_floor_bytes -eq 10GB -and $defaultPolicy.capacity_build_growth_bytes -eq 8GB -and $defaultPolicy.capacity_target_growth_bytes -eq 2GB -and $defaultPolicy.capacity_temp_growth_bytes -eq 1GB) 'unmeasured default growth budgets'
    $legacy = [pscustomobject]@{ critical_free_percent = 8 }
    $backfill = Complete-RustCacheCapacityPolicy -Policy $legacy
    Assert-True ($backfill.capacity_build_growth_bytes -eq 8GB) 'old policy receives capacity defaults'
    foreach ($bad in @($null, '', $true, -1, 0, 0.5, 'NaN', 'Infinity', '9223372036854775808')) {
        $policy = Get-DefaultRustCachePolicy; $policy.capacity_build_growth_bytes = $bad
        Assert-Throws { Complete-RustCacheCapacityPolicy -Policy $policy } 'CAPACITY_POLICY_INVALID'
    }
    foreach ($bad in @($null, '', $false, -1, 100, 'NaN', 'Infinity')) {
        $policy = Get-DefaultRustCachePolicy; $policy.critical_free_percent = $bad
        Assert-Throws { Complete-RustCacheCapacityPolicy -Policy $policy } 'CAPACITY_POLICY_INVALID'
    }
    Set-FixtureVolume
    $unknownPolicy = Get-DefaultRustCachePolicy; $unknownPolicy.schema_version = 99
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan -Policy $unknownPolicy } 'CAPACITY_POLICY_INVALID'
    $plan = Get-RustCacheCapacityPlan @argsForPlan
    Assert-True ($plan.admissible -and $plan.volumes.Count -eq 1 -and $plan.volumes[0].estimated_growth_bytes -eq 11GB -and $plan.volumes[0].floor_bytes -eq 10GB) 'same physical volume combines all growth'
    Assert-True (-not (Test-Path -LiteralPath $argsForPlan.ControlRoot) -and -not (Test-Path -LiteralPath $argsForPlan.CacheRoot)) 'planning does not create cache or control root'
    Set-FixtureVolume -Separate
    $plan = Get-RustCacheCapacityPlan @argsForPlan
    Assert-True ($plan.volumes.Count -eq 2 -and ($plan.volumes | Where-Object storage_id -eq fixture-build-volume).estimated_growth_bytes -eq 10GB) 'different volumes receive separate reservations'
    Set-FixtureVolume -Free 60GB -Total 500GB
    $plan = Get-RustCacheCapacityPlan @argsForPlan
    Assert-True ($plan.volumes[0].floor_bytes -eq 40GB) 'percentage floor wins on large volumes'
    Set-FixtureVolume -Free 21GB
    Assert-True ((Get-RustCacheCapacityPlan @argsForPlan).admissible) 'exact free-space boundary is admitted'
    Set-FixtureVolume -Free (21GB - 1)
    Assert-True (-not (Get-RustCacheCapacityPlan @argsForPlan).admissible) 'one byte below boundary is refused'
    Assert-Throws { Enter-RustCacheCapacityReservation @argsForPlan } 'CAPACITY_INSUFFICIENT'
    $custom = Get-DefaultRustCachePolicy; $custom.capacity_build_growth_bytes = 1GB; $custom.capacity_target_growth_bytes = 1GB; $custom.capacity_temp_growth_bytes = 1GB; $custom.auto_gc_on_run = $false
    Assert-True ((Get-RustCacheCapacityPlan @argsForPlan -Policy $custom).admissible) 'machine policy can explicitly tune growth'
    Assert-True (-not (Get-RustCacheCapacityPlan @argsForPlan).admissible) 'auto GC switch cannot bypass independent capacity floor'
    Set-FixtureVolume -Unknown
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_VOLUME_UNKNOWN'
    Set-FixtureVolume -Total 0
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_VOLUME_UNKNOWN'
    Set-FixtureVolume
    [IO.Directory]::CreateDirectory((Join-Path $argsForPlan.ControlRoot 'config')) | Out-Null
    [IO.File]::WriteAllText((Join-Path $argsForPlan.ControlRoot 'config\registry.json'), 'unrelated management metadata')
    $lease = Enter-RustCacheCapacityReservation @argsForPlan
    $plan = Get-RustCacheCapacityPlan @argsForPlan
    Assert-True ($plan.volumes[0].reserved_bytes -eq 11GB -and -not $plan.admissible) 'live reservation reduces all-project admission'
    $forged = $lease | ConvertTo-Json -Depth 8 | ConvertFrom-Json; $forged.lease_path = Join-Path $fixture 'outside.json'
    Assert-Throws { Exit-RustCacheCapacityReservation -Reservation $forged } 'CAPACITY_RELEASE_INVALID'
    Assert-True (Test-Path -LiteralPath $lease.lease_path) 'invalid release preserves lease'
    Exit-RustCacheCapacityReservation -Reservation $lease
    Assert-True (-not (Test-Path -LiteralPath $lease.lease_path) -and (Get-RustCacheCapacityPlan @argsForPlan).admissible) 'finally release restores admission'
    Assert-True ([IO.File]::ReadAllText((Join-Path $argsForPlan.ControlRoot 'config\registry.json')) -eq 'unrelated management metadata') 'private reservation namespace coexists with local management files'

    $capacityOwnerPath = Join-Path $lease.reservation_root '.capacity-owner.json'
    $capacityOwnerOriginal = [IO.File]::ReadAllText($capacityOwnerPath)
    $foreignOwner = $capacityOwnerOriginal | ConvertFrom-Json; $foreignOwner.user_id_sha256 = ('0' * 64)
    Write-FixtureJson -Path $capacityOwnerPath -Value $foreignOwner
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_CONTROL_FOREIGN'
    [IO.File]::WriteAllText($capacityOwnerPath, $capacityOwnerOriginal)

    $gatePath = Join-Path $lease.reservation_root '.capacity-gate.lock'
    [IO.File]::Delete($gatePath)
    [IO.Directory]::CreateDirectory($gatePath) | Out-Null
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_CONTROL_INVALID'
    [IO.Directory]::Delete($gatePath)
    [IO.File]::WriteAllText($gatePath, '')

    $stale = $lease | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $stale.process_started_ticks = 1
    Write-FixtureJson -Path $lease.lease_path -Value $stale
    $orphanPlan = Get-RustCacheCapacityPlan @argsForPlan
    Assert-True ($orphanPlan.volumes[0].reserved_bytes -eq 11GB -and $orphanPlan.orphan_reservation_count -eq 1) 'PID reuse keeps orphan budget because descendants may still write'
    $stale.process_started_ticks = $lease.process_started_ticks; $stale.pid = 2147483647
    Write-FixtureJson -Path $lease.lease_path -Value $stale
    Assert-True ((Get-RustCacheCapacityPlan @argsForPlan).volumes[0].reserved_bytes -eq 11GB) 'absent parent alone cannot release descendant write budget'
    $stale.machine_id_sha256 = ('0' * 64)
    Write-FixtureJson -Path $lease.lease_path -Value $stale
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_LEASE_INVALID'
    [IO.File]::WriteAllText($lease.lease_path, '{broken')
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_EVIDENCE_INVALID'
    [IO.File]::Delete($lease.lease_path)
    $unknown = Join-Path (Split-Path $lease.lease_path -Parent) 'unrecognized.txt'; [IO.File]::WriteAllText($unknown, 'unknown')
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_LEASE_UNKNOWN'
    [IO.File]::Delete($unknown)

    $tiersPath = Join-Path $fixture 'tiers.json'
    $tier = [pscustomobject]@{ schema = 'elon.rust_cache.sccache_tiers.v1'; cache_root = $argsForPlan.CacheRoot; local = [pscustomobject]@{ control_root = $argsForPlan.ControlRoot; cache_dir = (Join-Path $argsForPlan.ControlRoot 'sccache-l1') } }
    Write-FixtureJson -Path $tiersPath -Value $tier
    $env:ELON_RUST_CACHE_SCCACHE_TIERS = $tiersPath
    Set-FixtureVolume -Free 40GB
    $l1Plan = Get-RustCacheCapacityPlan @argsForPlan
    Assert-True ($l1Plan.volumes.Count -eq 1 -and $l1Plan.volumes[0].floor_extra_bytes -eq 2GB -and $l1Plan.volumes[0].estimated_growth_bytes -eq 11GB) 'same-volume L1 default quota is charged once as extra floor'
    $l1Lease = Enter-RustCacheCapacityReservation @argsForPlan
    try {
        $withLease = Get-RustCacheCapacityPlan @argsForPlan
        Assert-True ($withLease.volumes[0].reserved_bytes -eq 11GB -and $withLease.volumes[0].floor_extra_bytes -eq 2GB -and $withLease.volumes[0].remaining_bytes -eq 6GB) 'another task does not multiply the L1 machine quota'
    } finally { Exit-RustCacheCapacityReservation -Reservation $l1Lease }
    Set-FixtureVolume -SeparateL1 -L1Free 11GB
    $l1Plan = Get-RustCacheCapacityPlan @argsForPlan
    Assert-True (-not $l1Plan.admissible -and $l1Plan.volumes.Count -eq 2) 'independent L1 volume must retain floor plus full quota'
    Assert-Throws { Enter-RustCacheCapacityReservation @argsForPlan } 'CAPACITY_INSUFFICIENT'
    Set-FixtureVolume -SeparateL1 -L1Free 12GB
    $l1Lease = Enter-RustCacheCapacityReservation @argsForPlan
    try {
        Assert-True ($l1Lease.volumes.Count -eq 1 -and $l1Lease.volumes[0].estimated_growth_bytes -eq 11GB) 'L1-only volume has no per-task growth reservation'
        Assert-True ((Get-RustCacheCapacityPlan @argsForPlan).volumes | Where-Object storage_id -eq 'fixture-l1-volume' | ForEach-Object { $_.reserved_bytes -eq 0 -and $_.floor_extra_bytes -eq 2GB }) 'L1 quota is not duplicated by a live lease'
    } finally { Exit-RustCacheCapacityReservation -Reservation $l1Lease }
    $tier.local | Add-Member -NotePropertyName max_bytes -NotePropertyValue 4GB
    Write-FixtureJson -Path $tiersPath -Value $tier
    Assert-True (-not (Get-RustCacheCapacityPlan @argsForPlan).admissible) 'explicit L1 quota changes required free margin'
    foreach ($bad in @($null, $true, 0, 1.5, -1, 1, 'NaN', '9223372036854775808', 1099511627777)) {
        $tier.local.max_bytes = $bad; Write-FixtureJson -Path $tiersPath -Value $tier
        Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_L1_INVALID'
    }
    $tier.local.max_bytes = 2GB; $tier.schema = 'unknown'; Write-FixtureJson -Path $tiersPath -Value $tier
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_L1_INVALID'
    $tier.schema = 'elon.rust_cache.sccache_tiers.v1'; $tier.local.cache_dir = '\\host\share\l1'; Write-FixtureJson -Path $tiersPath -Value $tier
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_L1_INVALID'
    $tier.local.cache_dir = $null; Write-FixtureJson -Path $tiersPath -Value $tier
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_L1_INVALID'
    [IO.File]::Delete($tiersPath)
    Assert-Throws { Get-RustCacheCapacityPlan @argsForPlan } 'CAPACITY_L1_INVALID'
    $env:ELON_RUST_CACHE_SCCACHE_TIERS = $null
    Set-FixtureVolume

    Assert-Throws { Resolve-RustCacheCapacityControlRoot -ControlRoot '\\host\share\control' } 'CAPACITY_CONTROL_NETWORK'
    Assert-Throws { Resolve-RustCacheCapacityControlRoot -ControlRoot 'relative\control' } 'CAPACITY_PATH_INVALID'
    Assert-Throws { Resolve-RustCacheCapacityControlRoot -ControlRoot ([IO.Path]::GetPathRoot($fixture)) } 'CAPACITY_CONTROL_INVALID'
    $populated = Join-Path $fixture 'populated'; [IO.Directory]::CreateDirectory((Join-Path $populated 'capacity-v1')) | Out-Null; [IO.File]::WriteAllText((Join-Path $populated 'capacity-v1\valuable.txt'), 'preserve')
    Assert-Throws { Get-RustCacheCapacityPlan -CacheRoot $argsForPlan.CacheRoot -BuildDir $argsForPlan.BuildDir -TargetDir $argsForPlan.TargetDir -ControlRoot $populated } 'CAPACITY_CONTROL_UNKNOWN'
    $junction = Join-Path $fixture 'junction'
    New-Item -ItemType Junction -Path $junction -Target $populated | Out-Null
    try { Assert-Throws { Resolve-RustCacheCapacityControlRoot -ControlRoot $junction } 'CAPACITY_PATH_INVALID' }
    finally { [IO.Directory]::Delete($junction) }

    $ownerRoot = Join-Path $fixture 'owner-root'; [IO.Directory]::CreateDirectory($ownerRoot) | Out-Null
    Assert-RustCacheNetworkRootOwner -CacheRoot $ownerRoot
    Assert-True (@(Get-ChildItem -LiteralPath $ownerRoot -Force).Count -eq 0) 'unmarked local legacy root remains unclaimed'
    $marker = Join-Path $ownerRoot '.rust-cache-owner.json'
    $owner = [pscustomobject]@{ schema = 'elon.rust_cache.root_owner.v1'; machine_id_sha256 = Get-RustCacheMachineIdentity }
    Write-FixtureJson -Path $marker -Value $owner
    Assert-RustCacheNetworkRootOwner -CacheRoot $ownerRoot
    $owner.machine_id_sha256 = ('0' * 64); Write-FixtureJson -Path $marker -Value $owner
    Assert-Throws { Assert-RustCacheNetworkRootOwner -CacheRoot $ownerRoot } 'SHARED_ROOT_FOREIGN'
    [IO.File]::WriteAllText($marker, '{bad')
    Assert-Throws { Assert-RustCacheNetworkRootOwner -CacheRoot $ownerRoot } 'SHARED_OWNER_INVALID'
    [IO.File]::Delete($marker); [IO.Directory]::CreateDirectory($marker) | Out-Null
    Assert-Throws { Assert-RustCacheNetworkRootOwner -CacheRoot $ownerRoot } 'SHARED_OWNER_INVALID'
    [IO.Directory]::Delete($marker)
    $linkTarget = Join-Path $fixture 'owner-target.json'; Write-FixtureJson -Path $linkTarget -Value $owner
    New-Item -ItemType SymbolicLink -Path $marker -Target $linkTarget | Out-Null
    try {
        Assert-Throws { Assert-RustCacheNetworkRootOwner -CacheRoot $ownerRoot } 'SHARED_OWNER_INVALID'
        [IO.File]::Delete($linkTarget)
        Assert-Throws { Assert-RustCacheNetworkRootOwner -CacheRoot $ownerRoot } 'SHARED_OWNER_INVALID'
    } finally { [IO.File]::Delete($marker) }

    $exe = (Get-Process -Id $PID).Path
    foreach ($name in @('A', 'B')) {
        $arguments = '-NoProfile -ExecutionPolicy Bypass -File "' + $PSCommandPath + '" -WorkerRoot "' + $fixture + '" -WorkerName ' + $name
        $childProcess = Start-Process -FilePath $exe -ArgumentList $arguments -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $fixture "$name.stdout") -RedirectStandardError (Join-Path $fixture "$name.stderr")
        # Keep the process handle open before it exits (Windows PowerShell 5.1).
        [void]$childProcess.Handle
        $children += $childProcess
    }
    Wait-FixtureFiles -Paths @((Join-Path $fixture 'A.ready'), (Join-Path $fixture 'B.ready'))
    [IO.File]::WriteAllText((Join-Path $fixture 'go'), 'go')
    Wait-FixtureFiles -Paths @((Join-Path $fixture 'A.result.json'), (Join-Path $fixture 'B.result.json'))
    $results = @('A', 'B') | ForEach-Object { Get-Content -LiteralPath (Join-Path $fixture "$_.result.json") -Raw | ConvertFrom-Json }
    Assert-True (@($results | Where-Object admitted).Count -eq 1) 'exclusive gate admits exactly one real concurrent process'
    Assert-True (($results | Where-Object { -not $_.admitted }).message -like '*CAPACITY_INSUFFICIENT*') 'competing process sees the live reservation'
    [IO.File]::WriteAllText((Join-Path $fixture 'release'), 'release')
    foreach ($child in $children) {
        $finished = $child.WaitForExit(10000)
        if (-not $finished -or $child.ExitCode -ne 0) { Get-ChildItem -LiteralPath $fixture -Filter '*.stderr' | ForEach-Object { Get-Content -LiteralPath $_.FullName }; throw "Worker failure: finished=$finished exit=$($child.ExitCode)" }
        Assert-True ($finished -and $child.ExitCode -eq 0) 'worker finally releases reservation'
    }
    Assert-True (@(Get-ChildItem -LiteralPath (Join-Path $fixture 'parallel-control\capacity-v1\leases') -Force).Count -eq 0) 'parallel release leaves no live reservation metadata'
    Write-Output "PASS rust-cache-capacity: $testCount assertions; PS $($PSVersionTable.PSVersion); fixture-only, no Cargo/GC."
} finally {
    $env:ELON_RUST_CACHE_SCCACHE_TIERS = $previousTierPath
    $env:ELON_RUST_CACHE_CONTROL_ROOT = $previousControlRoot
    foreach ($child in $children) { if (-not $child.HasExited) { Stop-Process -Id $child.Id -Force -ErrorAction SilentlyContinue } }
    $resolved = [IO.Path]::GetFullPath($fixture)
    if (-not $resolved.StartsWith($fixtureParent.TrimEnd('\') + '\capacity-fixture-', [StringComparison]::OrdinalIgnoreCase)) { throw 'Refusing cleanup outside own fixture.' }
    Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction Stop
}
