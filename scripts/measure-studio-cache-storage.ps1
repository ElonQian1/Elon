[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$CacheRoot,
    [ValidateRange(8,256)][int]$Megabytes = 64,
    [ValidateRange(8,512)][int]$SmallFiles = 128,
    [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
Import-Module "$PSScriptRoot\rust-cache\RustCache.NetworkStorage.psm1" -DisableNameChecking
$root = [IO.Path]::GetFullPath($CacheRoot)
if (-not (Test-Path -LiteralPath $root -PathType Container)) { throw 'Choose an existing owned cache root.' }
Assert-RustCacheNetworkRootOwner -CacheRoot $root
if (-not (Test-Path -LiteralPath (Join-Path $root '.rust-cache-owner.json') -PathType Leaf)) {
    throw 'A verified owner marker is required for the storage probe.'
}
$volume = Get-RustCacheStorageVolume -CacheRoot $root
if ($volume.free_bytes -lt ($Megabytes * 1MB + 1GB)) { throw 'Insufficient spare space for the bounded probe.' }
$id = [Guid]::NewGuid().ToString('N')
$probe = Join-Path $root ('.studio-probe-' + $id)
$expected = $root.TrimEnd('\','/') + [IO.Path]::DirectorySeparatorChar
if (-not $probe.StartsWith($expected,[StringComparison]::OrdinalIgnoreCase)) { throw 'Probe path escaped its root.' }
New-Item -ItemType Directory -Path $probe -ErrorAction Stop | Out-Null
$marker = Join-Path $probe '.probe-owner'
$created = New-Object 'Collections.Generic.List[string]'
$markerStream=[IO.File]::Open($marker,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try { $markerBytes=[Text.Encoding]::UTF8.GetBytes($id); $markerStream.Write($markerBytes,0,$markerBytes.Length) } finally { $markerStream.Dispose() }
$created.Add('.probe-owner')
$rng = [Security.Cryptography.RandomNumberGenerator]::Create()
$buffer = New-Object byte[] 1MB
$rng.GetBytes($buffer)
$rng.Dispose()
try {
    $large = Join-Path $probe 'large.bin'
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $stream = [IO.File]::Open($large,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
    $created.Add('large.bin')
    try { for($i=0;$i -lt $Megabytes;$i++){ $stream.Write($buffer,0,$buffer.Length) }; $stream.Flush($true) }
    finally { $stream.Dispose() }
    $writeSeconds = $watch.Elapsed.TotalSeconds
    $watch.Restart()
    $stream = [IO.File]::OpenRead($large)
    $readBytes = [long]0
    try { while(($n=$stream.Read($buffer,0,$buffer.Length)) -gt 0){ $readBytes += $n } }
    finally { $stream.Dispose() }
    $readSeconds = $watch.Elapsed.TotalSeconds
    if ($readBytes -ne $Megabytes * 1MB) { throw 'Probe read length mismatch.' }
    $small = New-Object byte[] 4096
    $latencies = @()
    for($i=0;$i -lt $SmallFiles;$i++) {
        $watch.Restart()
        $smallStream=[IO.File]::Open((Join-Path $probe ("small-$i.bin")),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
        $created.Add("small-$i.bin")
        try { $smallStream.Write($small,0,$small.Length) } finally { $smallStream.Dispose() }
        $latencies += $watch.Elapsed.TotalMilliseconds
    }
    $watch.Restart()
    for($i=0;$i -lt $SmallFiles;$i++) {
        $bytes = [IO.File]::ReadAllBytes((Join-Path $probe ("small-$i.bin")))
        if ($bytes.Length -ne 4096) { throw 'Small file length mismatch.' }
    }
    $smallReadSeconds = $watch.Elapsed.TotalSeconds
    $sorted = @($latencies | Sort-Object)
    $result = [pscustomobject]@{
        schema='elon.studio_cache.storage_probe.v1'; measured_utc=[DateTime]::UtcNow.ToString('o')
        network_path=(Test-RustCacheNetworkPath $root); sample_mib=$Megabytes; small_file_count=$SmallFiles
        sequential_write_mib_s=[math]::Round($Megabytes/$writeSeconds,2)
        warm_sequential_read_mib_s=[math]::Round($Megabytes/$readSeconds,2)
        small_write_p50_ms=[math]::Round($sorted[[int][math]::Floor(($SmallFiles-1)*0.50)],2)
        small_write_p95_ms=[math]::Round($sorted[[int][math]::Floor(($SmallFiles-1)*0.95)],2)
        warm_small_read_files_s=[math]::Round($SmallFiles/$smallReadSeconds,2)
        interpretation='Single-client buffered sample, read-after-write may hit OS/SMB memory caches. Not sustained HDD speed, cold-build speed, or fleet peak throughput.'
    }
    if ($OutputPath) { $result | ConvertTo-Json | Set-Content -LiteralPath $OutputPath -Encoding UTF8 }
    $result | ConvertTo-Json
} finally {
    $resolved = [IO.Path]::GetFullPath($probe)
    if ($resolved.StartsWith($expected,[StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path -Leaf $resolved) -eq ('.studio-probe-' + $id) -and
        (Test-Path -LiteralPath $marker -PathType Leaf) -and [IO.File]::ReadAllText($marker) -ceq $id) {
        Assert-RustCacheNetworkRootOwner -CacheRoot $root
        $probeItem = Get-Item -LiteralPath $resolved -Force
        if (-not $probeItem.PSIsContainer -or ($probeItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Probe directory changed; retained for inspection.' }
        $items = @(Get-ChildItem -LiteralPath $resolved -Force)
        $unknown = @($items | Where-Object { $_.PSIsContainer -or ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -or -not $created.Contains($_.Name) })
        if ($unknown.Count) { throw 'Probe contains unexpected entries; all files retained for inspection.' }
        foreach($leaf in $created) { [IO.File]::Delete((Join-Path $resolved $leaf)) }
        # A concurrently added file is retained: nonrecursive directory removal fails.
        [IO.Directory]::Delete($resolved,$false)
    } else { throw 'Probe ownership changed; retained for inspection.' }
}
