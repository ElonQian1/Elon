Import-Module "$PSScriptRoot\RustCache.NetworkStorage.psm1" -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Policy.psm1" -DisableNameChecking

# Admission scope: all projects of this Windows user on this PC. This is not a
# distributed quota. Budgets are conservative, unmeasured initial estimates.
function Assert-RustCacheCapacityPath {
    param([Parameter(Mandatory)][string]$Path, [switch]$LocalOnly)
    if (-not [IO.Path]::IsPathRooted($Path) -or $Path -match '^[A-Za-z]:($|[^\\/])|^[\\/][^\\/]' -or $Path.StartsWith('\\?\')) {
        throw "RUST_CACHE_CAPACITY_PATH_INVALID: Expected an absolute ordinary path: $Path"
    }
    $full = [IO.Path]::GetFullPath($Path)
    if ($full -ne [IO.Path]::GetPathRoot($full)) { $full = $full.TrimEnd('\', '/') }
    if ($LocalOnly -and (Test-RustCacheNetworkPath -Path $full)) { throw 'RUST_CACHE_CAPACITY_CONTROL_NETWORK: Control root must be local.' }
    $ancestor = [IO.DirectoryInfo]::new($full)
    while ($null -ne $ancestor) {
        $item = Get-Item -LiteralPath $ancestor.FullName -Force -ErrorAction SilentlyContinue
        if ($item -and (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or -not $item.PSIsContainer)) {
            throw "RUST_CACHE_CAPACITY_PATH_INVALID: Directory or ancestor is not an ordinary directory: $($ancestor.FullName)"
        }
        $ancestor = $ancestor.Parent
    }
    return $full
}

function Resolve-RustCacheCapacityControlRoot {
    param([string]$ControlRoot)
    if ([string]::IsNullOrWhiteSpace($ControlRoot)) { $ControlRoot = $env:ELON_RUST_CACHE_CONTROL_ROOT }
    if ([string]::IsNullOrWhiteSpace($ControlRoot)) {
        $localData = [Environment]::GetFolderPath('LocalApplicationData')
        if (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) { $localData = $env:LOCALAPPDATA }
        if ([string]::IsNullOrWhiteSpace($localData)) { throw 'RUST_CACHE_CAPACITY_CONTROL_UNKNOWN: LocalApplicationData is unavailable.' }
        $ControlRoot = Join-Path $localData 'Elon\rust-cache-control-v1'
    }
    $root = Assert-RustCacheCapacityPath -Path $ControlRoot -LocalOnly
    if ($root.TrimEnd('\', '/') -eq [IO.Path]::GetPathRoot($root).TrimEnd('\', '/')) { throw 'RUST_CACHE_CAPACITY_CONTROL_INVALID: A volume root cannot be the control root.' }
    return $root
}

function Get-RustCacheCapacityOwner {
    $userIdentity = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $userHash = ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($userIdentity)))).Replace('-', '').ToLowerInvariant() } finally { $sha.Dispose() }
    $process = Get-Process -Id $PID -ErrorAction Stop
    return [pscustomobject]@{ machine_id_sha256 = Get-RustCacheMachineIdentity; user_id_sha256 = $userHash; pid = $PID; process_started_ticks = $process.StartTime.ToUniversalTime().Ticks }
}

function Read-RustCacheCapacityJson {
    param([string]$Path)
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
    if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or $item.Length -gt 65536) {
        throw "RUST_CACHE_CAPACITY_EVIDENCE_INVALID: Not a bounded ordinary metadata file: $Path"
    }
    try { return ([IO.File]::ReadAllText($Path) | ConvertFrom-Json -ErrorAction Stop) }
    catch { throw "RUST_CACHE_CAPACITY_EVIDENCE_INVALID: Cannot parse capacity metadata: $Path" }
}

function Write-RustCacheCapacityJson {
    param([string]$Path, $Value)
    $stream = [IO.File]::Open($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $bytes = [Text.Encoding]::UTF8.GetBytes(($Value | ConvertTo-Json -Depth 8 -Compress)); $stream.Write($bytes, 0, $bytes.Length); $stream.Flush() }
    finally { $stream.Dispose() }
}

function Assert-RustCacheCapacityControl {
    param([string]$Root, $Owner, [switch]$Initialize)
    if (-not (Test-Path -LiteralPath $Root)) { if (-not $Initialize) { return }; [IO.Directory]::CreateDirectory($Root) | Out-Null }
    [void](Assert-RustCacheCapacityPath -Path $Root -LocalOnly)
    foreach ($entry in @(Get-ChildItem -LiteralPath $Root -Force -ErrorAction Stop)) {
        if ($entry.Name -notin @('.capacity-owner.json', '.capacity-gate.lock', 'leases') -or ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "RUST_CACHE_CAPACITY_CONTROL_UNKNOWN: Unknown or redirected control entry: $($entry.FullName)"
        }
        if ($entry.PSIsContainer -ne ($entry.Name -eq 'leases')) { throw "RUST_CACHE_CAPACITY_CONTROL_INVALID: Unexpected entry type: $($entry.FullName)" }
    }
    $marker = Join-Path $Root '.capacity-owner.json'
    if (-not (Test-Path -LiteralPath $marker)) {
        if (@(Get-ChildItem -LiteralPath $Root -Force | Where-Object { $_.Name -ne '.capacity-gate.lock' }).Count -gt 0) { throw 'RUST_CACHE_CAPACITY_CONTROL_UNCLAIMED: Populated control root has no verified owner.' }
        if (-not $Initialize) { return }
        Write-RustCacheCapacityJson -Path $marker -Value ([ordered]@{ schema = 'elon.rust_cache.capacity_owner.v1'; machine_id_sha256 = $Owner.machine_id_sha256; user_id_sha256 = $Owner.user_id_sha256 })
    }
    $record = Read-RustCacheCapacityJson -Path $marker
    if ($record.schema -ne 'elon.rust_cache.capacity_owner.v1' -or $record.machine_id_sha256 -cne $Owner.machine_id_sha256 -or $record.user_id_sha256 -cne $Owner.user_id_sha256) {
        throw 'RUST_CACHE_CAPACITY_CONTROL_FOREIGN: Control owner identity is invalid or belongs to a different machine/user.'
    }
}

function Enter-RustCacheCapacityGate {
    param([string]$Root, [switch]$ReadOnly)
    $path = Join-Path $Root '.capacity-gate.lock'
    if ($ReadOnly -and -not (Test-Path -LiteralPath $path)) { return $null }
    [void](Assert-RustCacheCapacityPath -Path $Root -LocalOnly)
    $gateItem = Get-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
    if ($gateItem -and ($gateItem.PSIsContainer -or ($gateItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'RUST_CACHE_CAPACITY_CONTROL_INVALID: Capacity gate must be an ordinary file.' }
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        try {
            $mode = if ($ReadOnly) { [IO.FileMode]::Open } else { [IO.FileMode]::OpenOrCreate }
            return [IO.File]::Open($path, $mode, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
        } catch [IO.IOException] {
            if ([DateTime]::UtcNow -ge $deadline) { throw 'RUST_CACHE_CAPACITY_BUSY: Timed out acquiring the machine/user capacity gate.' }
            Start-Sleep -Milliseconds 100
        }
    } while ($true)
}

function Get-RustCacheCapacityVolume {
    param([Parameter(Mandatory)][string]$Path)
    $full = Assert-RustCacheCapacityPath -Path $Path
    $existing = [IO.DirectoryInfo]::new($full)
    while (-not $existing.Exists) { $existing = $existing.Parent; if ($null -eq $existing) { throw 'RUST_CACHE_CAPACITY_VOLUME_UNKNOWN: No accessible ancestor.' } }
    if (-not ('ElonRustCache.CapacityNativeVolume' -as [type])) {
        Add-Type -TypeDefinition @'
using System; using System.Text; using System.Runtime.InteropServices;
namespace ElonRustCache { public static class CapacityNativeVolume {
 [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
 public static extern bool GetVolumePathName(string path, StringBuilder volumePath, uint length);
 [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
 public static extern bool GetVolumeNameForVolumeMountPoint(string path, StringBuilder name, uint length);
}}
'@
    }
    if (Test-RustCacheNetworkPath -Path $full) {
        if (-not $full.StartsWith('\\')) { throw 'RUST_CACHE_CAPACITY_VOLUME_UNKNOWN: Use canonical UNC instead of a mapped drive for capacity accounting.' }
        $volumePath = [IO.Path]::GetPathRoot($full)
        $storageId = 'unc:' + $volumePath.TrimEnd('\').ToLowerInvariant()
    } else {
        $volumeBuffer = [Text.StringBuilder]::new(1024); $nameBuffer = [Text.StringBuilder]::new(1024)
        if (-not [ElonRustCache.CapacityNativeVolume]::GetVolumePathName($existing.FullName, $volumeBuffer, 1024) -or
            -not [ElonRustCache.CapacityNativeVolume]::GetVolumeNameForVolumeMountPoint($volumeBuffer.ToString(), $nameBuffer, 1024)) {
            throw 'RUST_CACHE_CAPACITY_VOLUME_UNKNOWN: Cannot establish physical local volume identity.'
        }
        $volumePath = $volumeBuffer.ToString(); $storageId = $nameBuffer.ToString().ToLowerInvariant()
    }
    $bytes = Get-RustCacheStorageVolume -CacheRoot $volumePath
    if ($null -eq $bytes.total_bytes -or $null -eq $bytes.free_bytes -or [long]$bytes.total_bytes -le 0 -or [long]$bytes.free_bytes -lt 0 -or [long]$bytes.free_bytes -gt [long]$bytes.total_bytes) {
        throw 'RUST_CACHE_CAPACITY_VOLUME_UNKNOWN: Free/total space measurement is unavailable or inconsistent.'
    }
    return [pscustomobject]@{ storage_id = $storageId; total_bytes = [long]$bytes.total_bytes; free_bytes = [long]$bytes.free_bytes }
}

function Get-RustCacheCapacityLeases {
    param([string]$Root, $Owner)
    $leasesRoot = Join-Path $Root 'leases'
    if (-not (Test-Path -LiteralPath $leasesRoot)) { return }
    [void](Assert-RustCacheCapacityPath -Path $leasesRoot -LocalOnly)
    foreach ($file in @(Get-ChildItem -LiteralPath $leasesRoot -Force -ErrorAction Stop)) {
        if ($file.Name -cnotmatch '^[0-9a-f]{32}\.json$') { throw 'RUST_CACHE_CAPACITY_LEASE_UNKNOWN: Unknown reservation evidence must be preserved.' }
        $lease = Read-RustCacheCapacityJson -Path $file.FullName
        if ($lease.schema -ne 'elon.rust_cache.capacity_lease.v1' -or $lease.id -cne $file.BaseName -or
            $lease.machine_id_sha256 -cne $Owner.machine_id_sha256 -or $lease.user_id_sha256 -cne $Owner.user_id_sha256 -or
            $null -eq $lease.pid -or $lease.pid -is [bool] -or [decimal]$lease.pid -ne [int]$lease.pid -or [int]$lease.pid -le 0 -or
            $null -eq $lease.process_started_ticks -or $lease.process_started_ticks -is [bool] -or [decimal]$lease.process_started_ticks -ne [long]$lease.process_started_ticks -or [long]$lease.process_started_ticks -le 0 -or @($lease.volumes).Count -eq 0) {
            throw 'RUST_CACHE_CAPACITY_LEASE_INVALID: Reservation identity is incomplete or foreign.'
        }
        $seen = @{}
        foreach ($volume in @($lease.volumes)) {
            if ([string]::IsNullOrWhiteSpace([string]$volume.storage_id) -or $seen.ContainsKey([string]$volume.storage_id) -or $null -eq $volume.estimated_growth_bytes -or
                $volume.estimated_growth_bytes -is [bool] -or [decimal]$volume.estimated_growth_bytes -ne [long]$volume.estimated_growth_bytes -or [long]$volume.estimated_growth_bytes -le 0) {
                throw 'RUST_CACHE_CAPACITY_LEASE_INVALID: Invalid volume reservation budget.'
            }
            $seen[[string]$volume.storage_id] = $true
        }
        # A dead parent does not prove Cargo/rustc descendants stopped writing.
        # Keep orphan/unknown reservations until reviewed recovery; only the
        # owning invocation's normal finally releases its byte budget.
        $ownerState = 'active'
        $process = $null
        try { $process = Get-Process -Id ([int]$lease.pid) -ErrorAction Stop }
        catch { $ownerState = if ($_.FullyQualifiedErrorId -like 'NoProcessFoundForGivenId*') { 'orphan' } else { 'unknown' } }
        if ($process) { try { if ($process.StartTime.ToUniversalTime().Ticks -ne [long]$lease.process_started_ticks) { $ownerState = 'orphan' } } catch { $ownerState = 'unknown' } }
        $lease | Add-Member -NotePropertyName owner_state -NotePropertyValue $ownerState -Force
        $lease
    }
}

function Get-RustCacheCapacityL1Requirement {
    # Read only the local L1 portion, without importing Sccache/Install. Those
    # modules validate remote backend configuration before starting a writer.
    # Omitted local settings use the same control/L1/2-GiB defaults as SccacheTiers.
    $path = $env:ELON_RUST_CACHE_SCCACHE_TIERS
    if ([string]::IsNullOrWhiteSpace($path)) { return $null }
    try {
        if ($path -notmatch '^[A-Za-z]:[\\/]' -or $path -match '[\x00-\x1f]') { throw 'nonlocal configuration path' }
        [void](Assert-RustCacheCapacityPath -Path (Split-Path -Parent $path) -LocalOnly)
        $config = Read-RustCacheCapacityJson -Path $path
        if ($config.schema -cne 'elon.rust_cache.sccache_tiers.v1' -or $config.local -isnot [pscustomobject]) { throw 'unknown local tier schema' }
        foreach ($name in $config.local.PSObject.Properties.Name) {
            if ($name -notin @('control_root','cache_dir','max_bytes','server_port')) { throw 'unknown local tier property' }
        }
        $control = if ($config.local.PSObject.Properties['control_root']) { [string]$config.local.control_root } else { Resolve-RustCacheCapacityControlRoot }
        if ([string]::IsNullOrWhiteSpace($control)) { throw 'missing local control root' }
        $control = Resolve-RustCacheCapacityControlRoot -ControlRoot $control
        if ($env:ELON_RUST_CACHE_CONTROL_ROOT -and $control -ine (Resolve-RustCacheCapacityControlRoot -ControlRoot $env:ELON_RUST_CACHE_CONTROL_ROOT)) { throw 'control root mismatch' }
        $cacheDir = if ($config.local.PSObject.Properties['cache_dir']) { [string]$config.local.cache_dir } else { Join-Path $control 'sccache-l1' }
        if ($cacheDir -notmatch '^[A-Za-z]:[\\/]' -or $cacheDir -match '[\x00-\x1f]') { throw 'nonlocal L1 cache path' }
        $cacheDir = Resolve-RustCacheCapacityControlRoot -ControlRoot $cacheDir
        if ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($cacheDir)).DriveType -ne [IO.DriveType]::Fixed) { throw 'L1 must use a fixed local drive' }
        $scope = Join-Path $control 'sccache\machine-v1'
        if ($cacheDir -ieq $control -or $cacheDir -ieq $scope -or $scope.StartsWith($cacheDir+'\',[StringComparison]::OrdinalIgnoreCase) -or $cacheDir.StartsWith($scope+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'L1 overlaps sccache control files' }
        $size = if ($config.local.PSObject.Properties['max_bytes']) { $config.local.max_bytes } else { [long]2147483648 }
        if ($size -is [bool] -or [string]$size -notmatch '^[0-9]+$' -or [long]$size -lt 67108864 -or [long]$size -gt 1099511627776) { throw 'invalid L1 byte quota' }
        return @{ path = $cacheDir; bytes = [long]0; floor_extra = [long]$size }
    } catch { throw 'RUST_CACHE_CAPACITY_L1_INVALID: Enabled local tier metadata, quota, or path is missing or invalid; preserve it and repair the machine configuration.' }
}

function Get-RustCacheCapacityPlanCore {
    param([string]$CacheRoot, [string]$BuildDir, [string]$TargetDir, [string]$TempDir, [string]$Root, $Owner, $Policy)
    if ($null -eq $Policy) {
        $policyPath = Get-RustCachePolicyPath -CacheRoot $CacheRoot
        $Policy = if (Test-Path -LiteralPath $policyPath) { Read-RustCacheCapacityJson -Path $policyPath } else { Get-DefaultRustCachePolicy }
    }
    if ($Policy.schema_version -ne 1) { throw 'RUST_CACHE_CAPACITY_POLICY_INVALID: Unknown policy schema_version.' }
    $Policy = Complete-RustCacheCapacityPolicy -Policy $Policy
    $leases = @(Get-RustCacheCapacityLeases -Root $Root -Owner $Owner)
    $volumes = @{}
    $requirements = @(@{ path = $BuildDir; bytes = $Policy.capacity_build_growth_bytes; floor_extra = 0 }, @{ path = $TargetDir; bytes = $Policy.capacity_target_growth_bytes; floor_extra = 0 }, @{ path = $TempDir; bytes = $Policy.capacity_temp_growth_bytes; floor_extra = 0 })
    $l1Requirement = Get-RustCacheCapacityL1Requirement
    if ($null -ne $l1Requirement) { $requirements += $l1Requirement }
    foreach ($requirement in $requirements) {
        $measurement = Get-RustCacheCapacityVolume -Path $requirement.path
        $key = [string]$measurement.storage_id
        if ([string]::IsNullOrWhiteSpace($key) -or $null -eq $measurement.free_bytes -or $null -eq $measurement.total_bytes -or [long]$measurement.total_bytes -le 0 -or [long]$measurement.free_bytes -lt 0 -or [long]$measurement.free_bytes -gt [long]$measurement.total_bytes) {
            throw 'RUST_CACHE_CAPACITY_VOLUME_UNKNOWN: A reliable capacity measurement is required.'
        }
        if (-not $volumes.ContainsKey($key)) {
            [long]$reserved = 0
            foreach ($lease in $leases) { foreach ($prior in @($lease.volumes)) { if ($prior.storage_id -eq $key) { $reserved = [long]([decimal]$reserved + [decimal]$prior.estimated_growth_bytes) } } }
            $volumes[$key] = [pscustomobject]@{ storage_id = $key; paths = @(); estimated_growth_bytes = [long]0; reserved_bytes = $reserved; floor_bytes = [long][math]::Max([double]$Policy.capacity_floor_bytes, [math]::Ceiling([double]$measurement.total_bytes * [double]$Policy.critical_free_percent / 100)); floor_extra_bytes = [long]0; available_bytes = [long]$measurement.free_bytes; remaining_bytes = [long]0 }
        } else { $volumes[$key].available_bytes = [math]::Min($volumes[$key].available_bytes, [long]$measurement.free_bytes) }
        $volumes[$key].paths += $requirement.path
        $volumes[$key].estimated_growth_bytes = [long]([decimal]$volumes[$key].estimated_growth_bytes + [decimal]$requirement.bytes)
        # The bounded L1 quota is one machine-level floor, not a task reservation.
        # Counting its entire quota even if already occupied is conservative.
        $volumes[$key].floor_extra_bytes = [long]([decimal]$volumes[$key].floor_extra_bytes + [decimal]$requirement.floor_extra)
    }
    $admissible = $true
    foreach ($volume in $volumes.Values) {
        $volume.remaining_bytes = [long]([decimal]$volume.available_bytes - [decimal]$volume.reserved_bytes - [decimal]$volume.floor_bytes - [decimal]$volume.floor_extra_bytes - [decimal]$volume.estimated_growth_bytes)
        if ($volume.remaining_bytes -lt 0) { $admissible = $false }
    }
    return [pscustomobject]@{ schema = 'elon.rust_cache.capacity_plan.v1'; scope = 'machine-user'; control_root = $Root; admissible = $admissible; orphan_reservation_count = @($leases | Where-Object owner_state -eq 'orphan').Count; unknown_owner_reservation_count = @($leases | Where-Object owner_state -eq 'unknown').Count; volumes = @($volumes.Values | Sort-Object storage_id) }
}

function Get-RustCacheCapacityPlan {
    param([Parameter(Mandatory)][string]$CacheRoot, [Parameter(Mandatory)][string]$BuildDir, [Parameter(Mandatory)][string]$TargetDir, [string]$TempDir = [IO.Path]::GetTempPath(), [string]$ControlRoot, $Policy)
    $control = Resolve-RustCacheCapacityControlRoot -ControlRoot $ControlRoot; $root = Join-Path $control 'capacity-v1'; $owner = Get-RustCacheCapacityOwner
    $gate = Enter-RustCacheCapacityGate -Root $root -ReadOnly
    try {
        Assert-RustCacheCapacityControl -Root $root -Owner $owner
        $plan = Get-RustCacheCapacityPlanCore -CacheRoot $CacheRoot -BuildDir $BuildDir -TargetDir $TargetDir -TempDir $TempDir -Root $root -Owner $owner -Policy $Policy
        $plan.control_root = $control
        return $plan
    } finally { if ($gate) { $gate.Dispose() } }
}

function Enter-RustCacheCapacityReservation {
    param([Parameter(Mandatory)][string]$CacheRoot, [Parameter(Mandatory)][string]$BuildDir, [Parameter(Mandatory)][string]$TargetDir, [string]$TempDir = [IO.Path]::GetTempPath(), [string]$ControlRoot, $Policy)
    $control = Resolve-RustCacheCapacityControlRoot -ControlRoot $ControlRoot; $root = Join-Path $control 'capacity-v1'; $owner = Get-RustCacheCapacityOwner
    [IO.Directory]::CreateDirectory($root) | Out-Null
    $gate = Enter-RustCacheCapacityGate -Root $root
    try {
        Assert-RustCacheCapacityControl -Root $root -Owner $owner -Initialize
        $plan = Get-RustCacheCapacityPlanCore -CacheRoot $CacheRoot -BuildDir $BuildDir -TargetDir $TargetDir -TempDir $TempDir -Root $root -Owner $owner -Policy $Policy
        if (-not $plan.admissible) { throw 'RUST_CACHE_CAPACITY_INSUFFICIENT: Free space cannot cover the safety floor, L1 quota margin, active reservations, and estimated build/target/TEMP growth. Reclaim reviewed cache or explicitly choose another available tier.' }
        $id = [Guid]::NewGuid().ToString('N'); $leasePath = Join-Path $root "leases\$id.json"
        [IO.Directory]::CreateDirectory((Join-Path $root 'leases')) | Out-Null
        $reservation = [pscustomobject]@{ schema = 'elon.rust_cache.capacity_lease.v1'; id = $id; scope = 'machine-user'; control_root = $control; reservation_root = $root; lease_path = $leasePath; machine_id_sha256 = $owner.machine_id_sha256; user_id_sha256 = $owner.user_id_sha256; pid = $owner.pid; process_started_ticks = $owner.process_started_ticks; created_utc = [DateTime]::UtcNow.ToString('o'); volumes = @($plan.volumes | Where-Object { $_.estimated_growth_bytes -gt 0 }) }
        Write-RustCacheCapacityJson -Path $leasePath -Value $reservation
        return $reservation
    } finally { if ($gate) { $gate.Dispose() } }
}

function Exit-RustCacheCapacityReservation {
    param([Parameter(Mandatory)]$Reservation)
    $owner = Get-RustCacheCapacityOwner
    if ($Reservation.schema -ne 'elon.rust_cache.capacity_lease.v1' -or [string]$Reservation.id -cnotmatch '^[0-9a-f]{32}$' -or
        $Reservation.machine_id_sha256 -cne $owner.machine_id_sha256 -or $Reservation.user_id_sha256 -cne $owner.user_id_sha256 -or
        $Reservation.pid -ne $owner.pid -or $Reservation.process_started_ticks -ne $owner.process_started_ticks) { throw 'RUST_CACHE_CAPACITY_RELEASE_INVALID: Only the owning live invocation can release this reservation.' }
    $control = Resolve-RustCacheCapacityControlRoot -ControlRoot $Reservation.control_root; $root = Join-Path $control 'capacity-v1'
    $expectedPath = Join-Path $root "leases\$($Reservation.id).json"
    if ($Reservation.lease_path -cne $expectedPath -or $Reservation.reservation_root -cne $root) { throw 'RUST_CACHE_CAPACITY_RELEASE_INVALID: Reservation path binding changed.' }
    $gate = Enter-RustCacheCapacityGate -Root $root -ReadOnly
    try {
        Assert-RustCacheCapacityControl -Root $root -Owner $owner
        [void](Assert-RustCacheCapacityPath -Path (Join-Path $root 'leases') -LocalOnly)
        $stored = Read-RustCacheCapacityJson -Path $expectedPath
        foreach ($field in @('schema', 'id', 'machine_id_sha256', 'user_id_sha256', 'pid', 'process_started_ticks', 'control_root', 'reservation_root', 'lease_path')) {
            if ([string]$stored.$field -cne [string]$Reservation.$field) { throw 'RUST_CACHE_CAPACITY_RELEASE_INVALID: Stored reservation identity changed.' }
        }
        [IO.File]::Delete($expectedPath)
    } finally { if ($gate) { $gate.Dispose() } }
}

Export-ModuleMember -Function Get-RustCacheCapacityPlan, Enter-RustCacheCapacityReservation, Exit-RustCacheCapacityReservation, Resolve-RustCacheCapacityControlRoot
