function ConvertTo-RustCacheExtendedPath {
    param([Parameter(Mandatory)][string]$Path)

    $full = [IO.Path]::GetFullPath($Path)
    if ($env:OS -ne 'Windows_NT' -or $full.StartsWith('\\?\')) { return $full }
    if ($full.StartsWith('\\')) { return '\\?\UNC\' + $full.Substring(2) }
    return '\\?\' + $full
}

function Get-RustCacheWindowsVolumeBytes {
    param([Parameter(Mandatory)][string]$Path)

    if (-not ('ElonRustCache.NativeDiskSpace' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace ElonRustCache {
    public static class NativeDiskSpace {
        [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool GetDiskFreeSpaceEx(string path,
            out ulong available, out ulong total, out ulong free);
    }
}
'@
    }
    [UInt64]$available = 0; [UInt64]$total = 0; [UInt64]$free = 0
    $ok = [ElonRustCache.NativeDiskSpace]::GetDiskFreeSpaceEx(
        $Path.TrimEnd('\', '/') + '\', [ref]$available, [ref]$total, [ref]$free)
    if (-not $ok) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        $detail = [ComponentModel.Win32Exception]::new($code).Message
        throw "RUST_CACHE_STORAGE_UNAVAILABLE: Cannot query cache volume '$Path' (Win32=$code): $detail. Reconnect the share or explicitly select an available local cache root."
    }
    return [pscustomobject]@{ total_bytes = [int64]$total; free_bytes = [int64]$available }
}

function Get-RustCacheStorageVolume {
    param([Parameter(Mandatory)][string]$CacheRoot)

    $rootPath = [IO.Path]::GetPathRoot([IO.Path]::GetFullPath($CacheRoot))
    if ($env:OS -eq 'Windows_NT') {
        $bytes = Get-RustCacheWindowsVolumeBytes -Path $rootPath
    } else {
        $drive = [IO.DriveInfo]::new($rootPath)
        $bytes = [pscustomobject]@{ total_bytes = $drive.TotalSize; free_bytes = $drive.AvailableFreeSpace }
    }
    return [pscustomobject]@{
        root = $rootPath
        total_bytes = [int64]$bytes.total_bytes
        free_bytes = [int64]$bytes.free_bytes
        free_percent = if ($bytes.total_bytes -gt 0) { [math]::Round(100.0 * $bytes.free_bytes / $bytes.total_bytes, 2) } else { 0 }
    }
}

function Test-RustCacheNetworkPath {
    param([Parameter(Mandatory)][string]$Path)

    $full = [IO.Path]::GetFullPath($Path)
    if ($full.StartsWith('\\?\UNC\', [StringComparison]::OrdinalIgnoreCase)) { return $true }
    if ($full.StartsWith('\\') -and -not $full.StartsWith('\\?\')) { return $true }
    if ($env:OS -ne 'Windows_NT') { return $false }
    $drive = [IO.DriveInfo]::new([IO.Path]::GetPathRoot($full))
    return $drive.DriveType -eq [IO.DriveType]::Network
}

function Get-RustCacheMachineIdentity {
    if ($script:MachineIdentity) { return $script:MachineIdentity }
    $identity = $null
    if ($env:OS -eq 'Windows_NT') {
        $key = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('SOFTWARE\Microsoft\Cryptography')
        try { if ($key) { $identity = [string]$key.GetValue('MachineGuid') } } finally { if ($key) { $key.Dispose() } }
    } elseif (Test-Path -LiteralPath '/etc/machine-id') {
        $identity = [IO.File]::ReadAllText('/etc/machine-id').Trim()
    } else {
        # Local platforms without machine-id retain their host-scoped lock behavior.
        $identity = [Environment]::MachineName
    }
    if ([string]::IsNullOrWhiteSpace($identity)) {
        throw 'RUST_CACHE_MACHINE_ID_UNAVAILABLE: Cannot establish machine identity; shared-cache ownership cannot be verified.'
    }
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $hash = $sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($identity.ToLowerInvariant())) } finally { $sha.Dispose() }
    $script:MachineIdentity = ([BitConverter]::ToString($hash)).Replace('-', '').ToLowerInvariant()
    return $script:MachineIdentity
}

function Assert-RustCacheNetworkRootOwner {
    param([Parameter(Mandatory)][string]$CacheRoot, [switch]$ClaimEmptyRoot)

    $ancestor = [IO.DirectoryInfo]::new([IO.Path]::GetFullPath($CacheRoot))
    while ($null -ne $ancestor) {
        $item = Get-Item -LiteralPath $ancestor.FullName -Force -ErrorAction SilentlyContinue
        if ($item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "RUST_CACHE_SHARED_REPARSE_ROOT: Refusing a shared cache through a reparse point: $($ancestor.FullName)"
        }
        $ancestor = $ancestor.Parent
    }
    if (-not (Test-RustCacheNetworkPath -Path $CacheRoot)) { return }
    $marker = Join-Path $CacheRoot '.rust-cache-owner.json'
    $machine = Get-RustCacheMachineIdentity
    if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) {
        if (-not $ClaimEmptyRoot) {
            throw "RUST_CACHE_SHARED_ROOT_UNCLAIMED: Shared cache requires its own machine owner marker: $CacheRoot. Initialize a new empty per-PC directory with rust-cache install."
        }
        $existing = @(Get-ChildItem -LiteralPath $CacheRoot -Force -ErrorAction Stop)
        if ($existing.Count -gt 0) {
            throw "RUST_CACHE_SHARED_ROOT_UNCLAIMED: Refusing to claim a populated shared cache: $CacheRoot. Use a new empty per-PC directory; migrate reviewed data after ownership is initialized."
        }
        $payload = [ordered]@{ schema = 'elon.rust_cache.root_owner.v1'; machine_id_sha256 = $machine; created_utc = [DateTime]::UtcNow.ToString('o') }
        $stream = $null
        try {
            $stream = [IO.File]::Open($marker, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
            $bytes = [Text.Encoding]::UTF8.GetBytes(($payload | ConvertTo-Json -Compress))
            $stream.Write($bytes, 0, $bytes.Length)
            $stream.Flush()
        } catch [IO.IOException] {
            if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) { throw }
            # Another initializer may have won. Its marker must validate below.
        } finally { if ($stream) { $stream.Dispose() } }
    }
    $markerItem = Get-Item -LiteralPath $marker -Force -ErrorAction Stop
    if (($markerItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "RUST_CACHE_SHARED_OWNER_INVALID: Shared cache owner marker must not be a reparse point: $marker"
    }
    try { $owner = Get-Content -LiteralPath $marker -Raw -Encoding UTF8 -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop } catch {
        throw "RUST_CACHE_SHARED_OWNER_INVALID: Cannot validate shared cache owner marker: $marker. Preserve the directory and inspect its owner."
    }
    if ($owner.schema -ne 'elon.rust_cache.root_owner.v1' -or [string]$owner.machine_id_sha256 -notmatch '^[0-9a-f]{64}$') {
        throw "RUST_CACHE_SHARED_OWNER_INVALID: Invalid shared cache owner marker: $marker"
    }
    if ([string]$owner.machine_id_sha256 -ne $machine) {
        throw "RUST_CACHE_SHARED_ROOT_FOREIGN: This shared cache belongs to another PC: $CacheRoot. Select a separate per-PC cache directory; never replace its owner marker."
    }
}

function Get-RustCacheLockOwnerLocality {
    param([AllowNull()]$Owner, [Parameter(Mandatory)][string]$LockPath)

    if ($Owner -and $Owner.PSObject.Properties['machine_id_sha256']) {
        if ([string]$Owner.machine_id_sha256 -eq (Get-RustCacheMachineIdentity)) { return 'local' }
        return 'foreign'
    }
    if (Test-RustCacheNetworkPath -Path $LockPath) { return 'unknown-network-owner' }
    return 'local'
}

Export-ModuleMember -Function ConvertTo-RustCacheExtendedPath, Get-RustCacheStorageVolume, Test-RustCacheNetworkPath, Get-RustCacheMachineIdentity, Assert-RustCacheNetworkRootOwner, Get-RustCacheLockOwnerLocality
