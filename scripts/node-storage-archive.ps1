# Verified cold archives only. Cache ownership and deletion policy belong to callers.
Set-StrictMode -Version 2.0

function Get-ElonArchiveFullPath {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not [IO.Path]::IsPathRooted($Path) -or $Path -match '^[A-Za-z]:($|[^\\/])') {
        throw "Archive paths must be absolute: $Path"
    }
    $full = [IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
    if ($full -match '(?<!^[A-Za-z]):' -or $full -match '[*?]') {
        throw "Unsupported archive path: $Path"
    }
    return $full
}

function Test-ElonArchiveWithin {
    param([string]$Path, [string]$Root)
    $pathKey = Get-ElonArchiveFullPath $Path
    $rootKey = Get-ElonArchiveFullPath $Root
    return $pathKey.Equals($rootKey, [StringComparison]::OrdinalIgnoreCase) -or
        $pathKey.StartsWith($rootKey + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)
}

function Assert-ElonArchivePath {
    param([string]$Path)
    $current = Get-ElonArchiveFullPath $Path
    while ($current) {
        try {
            # One filesystem request per ancestor; avoid two provider/SMB round trips.
            $attributes = [IO.File]::GetAttributes($current)
            if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Archive path contains a reparse point: $current"
            }
        } catch [IO.FileNotFoundException] { }
        catch [IO.DirectoryNotFoundException] { }
        # Access denied, disconnected shares and other I/O errors remain terminating.
        $parent = [IO.Directory]::GetParent($current)
        if ($null -eq $parent -or $parent.FullName -eq $current) { break }
        $current = $parent.FullName
    }
}

function Assert-ElonArchiveScope {
    param([string]$Path, [string]$AllowedRoot, [string]$ArchiveRoot)
    $source = Get-ElonArchiveFullPath $Path
    $allowed = Get-ElonArchiveFullPath $AllowedRoot
    $archive = Get-ElonArchiveFullPath $ArchiveRoot
    if ($allowed -eq ([IO.Path]::GetPathRoot($allowed)).TrimEnd('\', '/')) {
        throw 'A volume root cannot authorize an archive source.'
    }
    if ($source -eq $allowed -or -not (Test-ElonArchiveWithin $source $allowed)) {
        throw 'Archive source is outside its allowed root or equals that root.'
    }
    if ((Test-ElonArchiveWithin $archive $source) -or (Test-ElonArchiveWithin $source $archive)) {
        throw 'Archive and source must not be equal, nested, or overlapping.'
    }
    Assert-ElonArchivePath $source
    Assert-ElonArchivePath $archive
    if (-not (Test-Path -LiteralPath $source -PathType Container)) { throw 'Archive source is not a directory.' }
}

function Get-ElonArchiveDigest {
    param([Parameter(Mandatory = $true)]$Value)
    $json = ConvertTo-Json -InputObject $Value -Depth 12 -Compress
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($json)))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function Get-ElonArchiveInventory {
    param([string]$Root, [switch]$HashFiles)
    $ErrorActionPreference = 'Stop'
    $rootPath = Get-ElonArchiveFullPath $Root
    Assert-ElonArchivePath $rootPath
    $pending = New-Object 'Collections.Generic.Stack[string]'
    $files = New-Object 'Collections.Generic.List[object]'
    $directories = New-Object 'Collections.Generic.List[object]'
    $pending.Push($rootPath)
    [long]$bytes = 0
    $latest = (Get-Item -LiteralPath $rootPath -Force).LastWriteTimeUtc
    while ($pending.Count) {
        $dir = $pending.Pop()
        Assert-ElonArchivePath $dir
        foreach ($item in Get-ChildItem -LiteralPath $dir -Force -ErrorAction Stop) {
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Reparse point in archive source: $($item.FullName)" }
            if ($item.Name -eq '.git') { throw 'Git repositories and worktrees require their managed archive workflow.' }
            if ($item.LastWriteTimeUtc -gt $latest) { $latest = $item.LastWriteTimeUtc }
            $relative = $item.FullName.Substring($rootPath.Length + 1)
            if ($item.PSIsContainer) {
                $directories.Add([pscustomobject]@{ RelativePath = $relative; WriteTicks = $item.LastWriteTimeUtc.Ticks })
                $pending.Push($item.FullName)
            } else {
                $hash = if ($HashFiles) { (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant() } else { $null }
                $files.Add([pscustomobject]@{ RelativePath = $relative; Length = [long]$item.Length; Sha256 = $hash; WriteTicks = $item.LastWriteTimeUtc.Ticks })
                $bytes += $item.Length
            }
        }
    }
    $orderedFiles = @($files | Sort-Object RelativePath)
    $orderedDirs = @($directories | Sort-Object RelativePath)
    $content = [ordered]@{
        Directories = @($orderedDirs | ForEach-Object { $_.RelativePath })
        Files = @($orderedFiles | Select-Object RelativePath, Length, Sha256)
    }
    [pscustomobject]@{
        Files = $orderedFiles; Directories = $orderedDirs; Bytes = $bytes
        LatestWriteUtc = $latest.ToString('o'); ContentDigest = Get-ElonArchiveDigest $content
        SourceDigest = Get-ElonArchiveDigest ([ordered]@{ Files = $orderedFiles; Directories = $orderedDirs; RootTicks = (Get-Item -LiteralPath $rootPath).LastWriteTimeUtc.Ticks })
        # NTFS may publish a parent directory's mtime after a newly closed child file.
        # Membership and every file's bytes/hash/mtime remain strict drift evidence;
        # directory times still participate in retention and the reviewed plan above.
        StableSourceDigest = Get-ElonArchiveDigest ([ordered]@{ Files = $orderedFiles; Directories = @($orderedDirs | ForEach-Object { $_.RelativePath }) })
    }
}

function Write-ElonArchiveJson {
    param([string]$Path, $Value)
    Assert-ElonArchivePath $Path
    $json = ConvertTo-Json -InputObject $Value -Depth 12
    $stream = New-Object IO.FileStream($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try {
        $bytes = (New-Object Text.UTF8Encoding($false)).GetBytes($json)
        $stream.Write($bytes, 0, $bytes.Length)
        $stream.Flush($true)
    } finally { $stream.Dispose() }
}

function Copy-ElonArchiveFile {
    param([string]$Source, [string]$Destination)
    Assert-ElonArchivePath $Source
    Assert-ElonArchivePath $Destination
    # No automatic retry: an unavailable share fails closed and preserves the source.
    [IO.File]::Copy($Source, $Destination, $false)
}

function Copy-ElonArchiveInventory {
    param([string]$Source, [string]$Destination, $Inventory)
    $progress = [Diagnostics.Stopwatch]::StartNew()
    $copiedFiles = 0; $createdDirectories = 0; [long]$copiedBytes = 0
    Assert-ElonArchivePath $Destination
    New-Item -ItemType Directory -Path $Destination -ErrorAction Stop | Out-Null
    foreach ($dir in $Inventory.Directories) {
        $target = Get-ElonArchiveFullPath (Join-Path $Destination $dir.RelativePath)
        if (-not (Test-ElonArchiveWithin $target $Destination)) { throw 'Directory escaped archive payload.' }
        Assert-ElonArchivePath $target
        New-Item -ItemType Directory -Path $target -Force -ErrorAction Stop | Out-Null
        $createdDirectories++
        if ($progress.Elapsed.TotalSeconds -ge 10) {
            Write-Host "ARCHIVE_COPY_PROGRESS DIRECTORIES=$createdDirectories/$($Inventory.Directories.Count) FILES=0/$($Inventory.Files.Count) BYTES=0"
            $progress.Restart()
        }
    }
    foreach ($file in $Inventory.Files) {
        $from = Get-ElonArchiveFullPath (Join-Path $Source $file.RelativePath)
        $to = Get-ElonArchiveFullPath (Join-Path $Destination $file.RelativePath)
        if (-not (Test-ElonArchiveWithin $from $Source) -or -not (Test-ElonArchiveWithin $to $Destination)) { throw 'File escaped archive payload.' }
        Copy-ElonArchiveFile -Source $from -Destination $to
        [IO.File]::SetLastWriteTimeUtc($to, (New-Object DateTime([long]$file.WriteTicks, [DateTimeKind]::Utc)))
        $copiedFiles++; $copiedBytes += $file.Length
        if ($progress.Elapsed.TotalSeconds -ge 10) {
            Write-Host "ARCHIVE_COPY_PROGRESS FILES=$copiedFiles/$($Inventory.Files.Count) BYTES=$copiedBytes/$($Inventory.Bytes)"
            $progress.Restart()
        }
    }
}

function Get-ElonArchiveMachineKey {
    $identity = [Environment]::MachineName + '|' + [Environment]::UserName
    if ($env:OS -eq 'Windows_NT') {
        $identity = [string](Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Cryptography' -Name MachineGuid).MachineGuid + '|' +
            [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    }
    return (Get-ElonArchiveDigest $identity).Substring(0, 24)
}

function Invoke-ElonVerifiedTreeArchive {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$AllowedRoot,
        [Parameter(Mandatory = $true)][string]$ArchiveRoot,
        [ValidateSet('ArchiveOnly', 'ArchiveAndReclaim')][string]$Mode = 'ArchiveOnly',
        [scriptblock]$ValidateSource,
        [scriptblock]$RemoveSource
    )
    $ErrorActionPreference = 'Stop'
    Assert-ElonArchiveScope $Path $AllowedRoot $ArchiveRoot
    if ($Mode -eq 'ArchiveAndReclaim' -and ($null -eq $ValidateSource -or $null -eq $RemoveSource)) {
        throw 'Reclaim requires the owning workflow validation and removal callbacks.'
    }
    $source = Get-ElonArchiveFullPath $Path
    $lockPath = Join-Path $AllowedRoot ('.archive-' + (Get-ElonArchiveDigest $source).Substring(0, 24) + '.lock')
    Assert-ElonArchivePath $lockPath
    $lock = New-Object IO.FileStream($lockPath, [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
    try {
        if ($ValidateSource) { & $ValidateSource $source }
        Write-Host "ARCHIVE_PHASE=inventory SOURCE=$source"
        $before = Get-ElonArchiveInventory -Root $source -HashFiles
        $directory = Join-Path (Join-Path $ArchiveRoot (Get-ElonArchiveMachineKey)) ([DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ') + '-' + [Guid]::NewGuid().ToString('N'))
        Assert-ElonArchivePath $directory
        New-Item -ItemType Directory -Path $directory -Force -ErrorAction Stop | Out-Null
        $payload = Join-Path $directory 'payload'
        Write-Host "ARCHIVE_PHASE=copy BYTES=$($before.Bytes) SOURCE=$source"
        Copy-ElonArchiveInventory -Source $source -Destination $payload -Inventory $before
        Write-Host "ARCHIVE_PHASE=verify BYTES=$($before.Bytes) SOURCE=$source"
        $copied = Get-ElonArchiveInventory -Root $payload -HashFiles
        if ($before.ContentDigest -ne $copied.ContentDigest) { throw 'Archive payload verification failed; source retained.' }
        if ($ValidateSource) { & $ValidateSource $source }
        $after = Get-ElonArchiveInventory -Root $source -HashFiles
        if ($before.StableSourceDigest -ne $after.StableSourceDigest) { throw 'Source content, membership, or file timestamps drifted during archive; source retained.' }
        $manifest = [ordered]@{
            Schema = 'elon.verified_tree_archive.v1'; Source = $source; AllowedRoot = (Get-ElonArchiveFullPath $AllowedRoot)
            MachineKey = Get-ElonArchiveMachineKey; CreatedUtc = [DateTime]::UtcNow.ToString('o'); Inventory = $before
        }
        $manifestPath = Join-Path $directory 'manifest.json'
        Write-ElonArchiveJson -Path $manifestPath -Value $manifest
        $manifestHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash
        $receiptPath = Join-Path $directory 'verified.json'
        Write-ElonArchiveJson -Path $receiptPath -Value ([ordered]@{
            Schema = 'elon.verified_tree_archive_receipt.v1'; ManifestSha256 = $manifestHash
            Source = $source; ContentDigest = $before.ContentDigest; Bytes = $before.Bytes; VerifiedUtc = [DateTime]::UtcNow.ToString('o')
        })
        # Read the durable receipt back before the owning workflow can remove anything.
        $receipt = Get-Content -LiteralPath $receiptPath -Raw -ErrorAction Stop | ConvertFrom-Json
        if ($receipt.ManifestSha256 -ne (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash -or
            $receipt.Source -ne $source -or $receipt.ContentDigest -ne $before.ContentDigest -or $receipt.Bytes -ne $before.Bytes) {
            throw 'Archive receipt read-back mismatch.'
        }
        $removed = $false
        if ($Mode -eq 'ArchiveAndReclaim') {
            Write-Host "ARCHIVE_PHASE=reclaim BYTES=$($before.Bytes) SOURCE=$source"
            Assert-ElonArchiveScope $source $AllowedRoot $ArchiveRoot
            & $ValidateSource $source
            $final = Get-ElonArchiveInventory -Root $source -HashFiles
            if ($before.StableSourceDigest -ne $final.StableSourceDigest) { throw 'Source content, membership, or file timestamps drifted before reclaim; source retained.' }
            & $RemoveSource $source
            $removed = -not (Test-Path -LiteralPath $source)
            if (-not $removed) { throw 'Owning workflow did not fully reclaim the source.' }
            Write-ElonArchiveJson -Path (Join-Path $directory 'reclaimed.json') -Value @{ Source = $source; ManifestSha256 = $manifestHash; ReclaimedUtc = [DateTime]::UtcNow.ToString('o') }
        }
        Write-Host "ARCHIVE_PHASE=complete BYTES=$($before.Bytes) RECLAIMED=$removed RECEIPT=$receiptPath"
        return [pscustomobject]@{ Source = $source; ArchiveDirectory = $directory; ReceiptPath = $receiptPath; Bytes = $before.Bytes; Reclaimed = $removed }
    } finally { $lock.Dispose() }
}

function Restore-ElonVerifiedTreeArchive {
    param([string]$ArchiveDirectory, [string]$Path, [string]$AllowedRoot)
    $ErrorActionPreference = 'Stop'
    Assert-ElonArchivePath $ArchiveDirectory
    $manifestPath = Join-Path $ArchiveDirectory 'manifest.json'
    $receipt = Get-Content -LiteralPath (Join-Path $ArchiveDirectory 'verified.json') -Raw -ErrorAction Stop | ConvertFrom-Json
    if ($receipt.ManifestSha256 -ne (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash) { throw 'Archive manifest integrity mismatch.' }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.Schema -ne 'elon.verified_tree_archive.v1' -or $receipt.Schema -ne 'elon.verified_tree_archive_receipt.v1') { throw 'Unknown archive schema.' }
    $target = Get-ElonArchiveFullPath $Path
    if ($target -eq (Get-ElonArchiveFullPath $AllowedRoot) -or -not (Test-ElonArchiveWithin $target $AllowedRoot)) { throw 'Restore target escaped allowed root.' }
    if ((Test-ElonArchiveWithin $target $ArchiveDirectory) -or (Test-ElonArchiveWithin $ArchiveDirectory $target)) { throw 'Restore target overlaps archive.' }
    Assert-ElonArchivePath $target
    if (Test-Path -LiteralPath $target) { throw 'Restore refuses to overwrite an existing target.' }
    $payload = Join-Path $ArchiveDirectory 'payload'
    $inventory = Get-ElonArchiveInventory -Root $payload -HashFiles
    if ($inventory.ContentDigest -ne $receipt.ContentDigest -or $inventory.ContentDigest -ne $manifest.Inventory.ContentDigest) { throw 'Archive payload is corrupt.' }
    Copy-ElonArchiveInventory -Source $payload -Destination $target -Inventory $inventory
    if ((Get-ElonArchiveInventory -Root $target -HashFiles).ContentDigest -ne $inventory.ContentDigest) { throw 'Restored tree verification failed.' }
    return [pscustomobject]@{ Path = $target; Bytes = $inventory.Bytes; Restored = $true }
}
