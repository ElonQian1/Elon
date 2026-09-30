# Split rollback storage: only fixed program binaries may leave this machine.
Set-StrictMode -Version 2.0
. (Join-Path $PSScriptRoot 'node-storage-archive.ps1')
. (Join-Path $PSScriptRoot 'node-agent-local-rollback.ps1')

function Get-ElonRollbackContentDigest {
    param($Inventory)
    $stream = New-Object IO.MemoryStream
    $writer = New-Object IO.BinaryWriter($stream, (New-Object Text.UTF8Encoding($false, $true)), $true)
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $writer.Write('elon.rollback.inventory.v1')
        $directories = [string[]]@($Inventory.Directories | ForEach-Object { [string]$_.RelativePath })
        [Array]::Sort($directories, [StringComparer]::Ordinal)
        foreach ($path in $directories) { $writer.Write('D'); $writer.Write($path.Replace('/', '\')) }
        $files = New-Object 'Collections.Generic.Dictionary[string,object]' ([StringComparer]::Ordinal)
        foreach ($file in $Inventory.Files) { $files.Add([string]$file.RelativePath, $file) }
        $paths = [string[]]@($files.Keys)
        [Array]::Sort($paths, [StringComparer]::Ordinal)
        foreach ($path in $paths) {
            $writer.Write('F'); $writer.Write($path.Replace('/', '\'))
            $writer.Write([long]$files[$path].Length); $writer.Write(([string]$files[$path].Sha256).ToLowerInvariant())
        }
        $writer.Flush()
        return ([BitConverter]::ToString($sha.ComputeHash($stream.ToArray()))).Replace('-', '').ToLowerInvariant()
    } finally { $sha.Dispose(); $writer.Dispose(); $stream.Dispose() }
}

function Get-ElonRollbackSharedSlot {
    param([string]$RelativePath)
    $client = -join @([char]0x4e00,[char]0x9f99,[char]0x5f00,[char]0x53d1,[char]0x5e73,[char]0x53f0)
    $uninstall = -join @([char]0x5378,[char]0x8f7d)
    $paths = @(('ElonNode\' + $client + '.exe'), ('ElonNode\' + $uninstall + $client + '.exe'), 'ElonNode\_internal\elon-desktop.exe')
    $slots = @('client.exe', 'uninstaller.exe', 'desktop.exe')
    for ($index = 0; $index -lt $paths.Count; $index++) {
        if ($RelativePath.Replace('/', '\') -eq $paths[$index]) { return $slots[$index] }
    }
    return $null
}

function Join-ElonRollbackSafePath {
    param([string]$Root, [string]$RelativePath)
    if ([string]::IsNullOrWhiteSpace($RelativePath) -or [IO.Path]::IsPathRooted($RelativePath) -or
        $RelativePath -match '[:*?]' -or $RelativePath -match '(^|[\\/])\.{1,2}([\\/]|$)') { throw 'Unsafe rollback relative path.' }
    $path = Get-ElonArchiveFullPath (Join-Path $Root $RelativePath)
    if ($path -eq (Get-ElonArchiveFullPath $Root) -or -not (Test-ElonArchiveWithin $path $Root)) { throw 'Rollback path escaped its root.' }
    Assert-ElonArchivePath $path
    return $path
}

function Assert-ElonRollbackLocalPath {
    param([string]$Path)
    $full = Get-ElonArchiveFullPath $Path
    if ($env:OS -ne 'Windows_NT' -or $full -notmatch '^[A-Za-z]:\\' -or
        $full -eq ([IO.Path]::GetPathRoot($full)).TrimEnd('\', '/') -or
        ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($full))).DriveType -ne [IO.DriveType]::Fixed) {
        throw 'Rollback private storage and restore targets require a local fixed drive.'
    }
    if (-not ('ElonRollbackStorage.DosDevice' -as [type])) {
        Add-Type -TypeDefinition @'
using System.Runtime.InteropServices;
using System.Text;
namespace ElonRollbackStorage {
    public static class DosDevice {
        [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
        public static extern uint QueryDosDevice(string name, StringBuilder target, int size);
    }
}
'@
    }
    $device = New-Object Text.StringBuilder(512)
    if ([ElonRollbackStorage.DosDevice]::QueryDosDevice($full.Substring(0,2), $device, $device.Capacity) -eq 0 -or
        $device.ToString().StartsWith('\??\', [StringComparison]::Ordinal)) { throw 'Mapped or unverifiable rollback storage drives are not allowed.' }
    Assert-ElonArchivePath $full
    $install = Join-Path $env:LOCALAPPDATA 'ElonNode'
    if ((Test-ElonArchiveWithin $full $install) -or (Test-ElonArchiveWithin $install $full)) { throw 'Rollback storage must not overlap the installed runtime.' }
    return $full
}

function Get-ElonRollbackOwnerSid {
    return [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
}

function Assert-ElonRollbackPrivateAcl {
    param([string]$Path, [switch]$ProtectedDirectory)
    Assert-ElonArchivePath $Path
    $acl = Get-Acl -LiteralPath $Path -ErrorAction Stop
    $allowed = @((Get-ElonRollbackOwnerSid), 'S-1-5-18', 'S-1-5-32-544') | Select-Object -Unique
    if ($ProtectedDirectory -and (-not $acl.AreAccessRulesProtected -or
        $acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne (Get-ElonRollbackOwnerSid))) { throw 'Private rollback directory owner or inheritance is unsafe.' }
    $seen = @()
    foreach ($rule in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
        $sid = $rule.IdentityReference.Value
        if ($sid -notin $allowed -or $rule.AccessControlType -ne [Security.AccessControl.AccessControlType]::Allow -or
            $rule.FileSystemRights -ne [Security.AccessControl.FileSystemRights]::FullControl) { throw 'Private rollback ACL grants unexpected access.' }
        if ($ProtectedDirectory -and $rule.InheritanceFlags -ne ([Security.AccessControl.InheritanceFlags]::ContainerInherit -bor [Security.AccessControl.InheritanceFlags]::ObjectInherit)) {
            throw 'Private rollback directory ACL does not protect descendants.'
        }
        $seen += $sid
    }
    if (@($allowed | Where-Object { $_ -notin $seen }).Count) { throw 'Private rollback ACL is incomplete.' }
}

function New-ElonRollbackPrivateDirectory {
    param([string]$Path, [switch]$AllowExisting)
    $full = Assert-ElonRollbackLocalPath $Path
    if (Test-Path -LiteralPath $full) {
        if (-not $AllowExisting -or -not (Test-Path -LiteralPath $full -PathType Container)) { throw 'Private rollback target already exists.' }
        Assert-ElonRollbackPrivateAcl $full -ProtectedDirectory
        return
    }
    New-Item -ItemType Directory -Path $full -ErrorAction Stop | Out-Null
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $acl.SetAccessRuleProtection($true, $false)
    $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier((Get-ElonRollbackOwnerSid))))
    foreach ($sid in (@((Get-ElonRollbackOwnerSid), 'S-1-5-18', 'S-1-5-32-544') | Select-Object -Unique)) {
        $identity = New-Object Security.Principal.SecurityIdentifier($sid)
        $rule = New-Object Security.AccessControl.FileSystemAccessRule($identity, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
        $acl.AddAccessRule($rule)
    }
    Set-Acl -LiteralPath $full -AclObject $acl -ErrorAction Stop
    Assert-ElonRollbackPrivateAcl $full -ProtectedDirectory
}

function Assert-ElonRollbackSnapshotTree {
    param([string]$Path)
    Assert-ElonArchivePath $Path
    $items = @(Get-ChildItem -LiteralPath $Path -Force -ErrorAction Stop)
    if ($items.Count -ne 3 -or @($items | Where-Object { $_.Name -notin @('ElonNode','manifest.json','manifest.sha256') }).Count) { throw 'Rollback snapshot root has unclassified entries.' }
    $inventory = Get-ElonArchiveInventory -Root $Path -HashFiles
    foreach ($file in $inventory.Files) {
        Assert-ElonRollbackPrimaryStreams (Join-ElonRollbackSafePath $Path $file.RelativePath)
    }
    $verified = Test-NodeAgentRollbackSnapshot -SnapshotRoot $Path
    $manifest = Get-Content -LiteralPath $verified.ManifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($manifest.allowlist_version -ne 1) { throw 'Unsupported rollback allowlist version.' }
    $shared = @($inventory.Files | Where-Object { Get-ElonRollbackSharedSlot $_.RelativePath })
    $slots = @($shared | ForEach-Object { Get-ElonRollbackSharedSlot $_.RelativePath })
    if ('client.exe' -notin $slots -or 'desktop.exe' -notin $slots) { throw 'Rollback snapshot lacks required program binaries.' }
    return [pscustomobject]@{ Inventory = $inventory; Verified = $verified; Shared = $shared }
}

function Assert-ElonRollbackPrimaryStreams {
    param([string]$Path)
    Assert-ElonArchivePath $Path
    foreach ($dataStream in @(Get-Item -LiteralPath $Path -Stream '*' -ErrorAction Stop)) {
        if ($dataStream.Stream -notin @(':$DATA', '::$DATA')) { throw 'Rollback file contains an unmanifested alternate data stream; preserve source.' }
    }
}

function Copy-ElonRollbackVerifiedFile {
    param([string]$Source, [string]$Destination, $Expected, [switch]$PrimaryStreamOnly)
    Assert-ElonArchivePath $Source
    Assert-ElonArchivePath $Destination
    New-Item -ItemType Directory -Path (Split-Path -Parent $Destination) -Force -ErrorAction Stop | Out-Null
    if ($PrimaryStreamOnly) {
        # The original manifest authenticates the primary stream only. Never export
        # an EXE's unmanifested alternate streams to shared storage or restore them.
        $inputStream = [IO.File]::Open($Source, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
        $outputStream = $null
        try {
            $outputStream = [IO.File]::Open($Destination, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
            $inputStream.CopyTo($outputStream)
            $outputStream.Flush($true)
        } finally { if ($outputStream) { $outputStream.Dispose() }; $inputStream.Dispose() }
    } else { [IO.File]::Copy($Source, $Destination, $false) }
    if ([long](Get-Item -LiteralPath $Destination -Force).Length -ne [long]$Expected.Length -or
        (Get-FileHash -LiteralPath $Destination -Algorithm SHA256).Hash -ne $Expected.Sha256) { throw 'Rollback copy verification failed; source preserved.' }
    [IO.File]::SetLastWriteTimeUtc($Destination, (New-Object DateTime([long]$Expected.WriteTicks, [DateTimeKind]::Utc)))
}

function Assert-ElonRollbackSplitStorage {
    param($Receipt)
    if ($Receipt.Schema -ne 'elon.rollback_split_archive_receipt.v1' -or $Receipt.DigestVersion -ne 'elon.rollback.inventory.v1' -or
        $Receipt.MachineKey -ne (Get-ElonArchiveMachineKey) -or $Receipt.OwnerSid -ne (Get-ElonRollbackOwnerSid) -or
        $Receipt.Id -notmatch '^[0-9a-f]{32}$') { throw 'Rollback receipt schema or machine/user identity mismatch.' }
    $privateRoot = Assert-ElonRollbackLocalPath $Receipt.PrivateRoot
    $private = Join-ElonRollbackSafePath $privateRoot $Receipt.Id
    $archive = Join-ElonRollbackSafePath (Join-ElonRollbackSafePath $Receipt.ArchiveRoot $Receipt.MachineKey) $Receipt.Id
    if ($Receipt.PrivateDirectory -ne $private -or $Receipt.ArchiveDirectory -ne $archive -or
        $Receipt.ReceiptPath -ne (Join-Path $private 'receipt.json')) { throw 'Rollback receipt storage paths drifted.' }
    Assert-ElonRollbackPrivateAcl $privateRoot -ProtectedDirectory
    Assert-ElonRollbackPrivateAcl $private -ProtectedDirectory
    $null = Get-ElonArchiveInventory -Root $private -HashFiles
    foreach ($item in Get-ChildItem -LiteralPath $private -Force -Recurse -ErrorAction Stop) { Assert-ElonRollbackPrivateAcl $item.FullName }
    $snapshot = Join-Path $private 'snapshot'
    $manifestPath = Join-Path $snapshot 'manifest.json'
    if ((Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash -ne $Receipt.OriginalManifestSha256) { throw 'Private original manifest changed.' }
    $inventory = Get-ElonArchiveInventory -Root $snapshot -HashFiles
    foreach ($file in $inventory.Files) { Assert-ElonRollbackPrimaryStreams (Join-ElonRollbackSafePath $snapshot $file.RelativePath) }
    if ((Get-ElonRollbackContentDigest $inventory) -ne $Receipt.PrivateContentDigest -or [long]$inventory.Bytes -ne [long]$Receipt.PrivateBytes) { throw 'Private rollback payload changed.' }
    $shared = @(Get-ChildItem -LiteralPath $archive -Force -ErrorAction Stop)
    $expectedNames = @('programs.json') + @($Receipt.Programs | ForEach-Object { $_.Slot })
    if ($shared.Count -ne $expectedNames.Count -or @($shared | Where-Object { $_.PSIsContainer -or $_.Name -notin $expectedNames }).Count) { throw 'Shared rollback archive has unclassified entries.' }
    foreach ($file in $shared) { Assert-ElonRollbackPrimaryStreams $file.FullName }
    $indexPath = Join-Path $archive 'programs.json'
    Assert-ElonArchivePath $indexPath
    if ((Get-FileHash -LiteralPath $indexPath -Algorithm SHA256).Hash -ne $Receipt.ProgramIndexSha256) { throw 'Shared program index changed.' }
    $seen = @()
    foreach ($program in @($Receipt.Programs)) {
        $slot = Get-ElonRollbackSharedSlot $program.RelativePath
        if (-not $slot -or $slot -ne $program.Slot -or $slot -in $seen) { throw 'Shared rollback program mapping is unsafe.' }
        $seen += $slot
        $file = Join-ElonRollbackSafePath $archive $slot
        if ([long](Get-Item -LiteralPath $file -Force).Length -ne [long]$program.Length -or
            (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ne $program.Sha256) { throw 'Shared rollback program hash mismatch.' }
    }
    if ('client.exe' -notin $seen -or 'desktop.exe' -notin $seen -or
        [long]$Receipt.SharedBytes -ne [long](($Receipt.Programs | Measure-Object Length -Sum).Sum) -or
        [long]$Receipt.SourceBytes -ne ([long]$Receipt.SharedBytes + [long]$Receipt.PrivateBytes)) { throw 'Rollback receipt byte counts or required programs are invalid.' }
    return $snapshot
}

function Invoke-ElonRollbackSplitArchive {
    [CmdletBinding()]
    param([Parameter(Mandatory = $true)][string]$Path, [Parameter(Mandatory = $true)][string]$AllowedRoot,
        [Parameter(Mandatory = $true)][string]$ArchiveRoot, [Parameter(Mandatory = $true)][string]$PrivateRoot)
    $ErrorActionPreference = 'Stop'
    Assert-ElonArchiveScope $Path $AllowedRoot $ArchiveRoot
    $source = Get-ElonArchiveFullPath $Path
    $archiveRootPath = Get-ElonArchiveFullPath $ArchiveRoot
    $privateRootPath = Assert-ElonRollbackLocalPath $PrivateRoot
    foreach ($other in @($source, $archiveRootPath)) {
        if ((Test-ElonArchiveWithin $privateRootPath $other) -or (Test-ElonArchiveWithin $other $privateRootPath)) { throw 'Rollback private, shared and source scopes overlap.' }
    }
    $before = Assert-ElonRollbackSnapshotTree $source
    New-ElonRollbackPrivateDirectory $privateRootPath -AllowExisting
    $id = [Guid]::NewGuid().ToString('N')
    $private = Join-ElonRollbackSafePath $privateRootPath $id
    New-ElonRollbackPrivateDirectory $private
    $snapshot = Join-Path $private 'snapshot'
    New-Item -ItemType Directory -Path $snapshot -ErrorAction Stop | Out-Null
    foreach ($dir in $before.Inventory.Directories) {
        New-Item -ItemType Directory -Path (Join-ElonRollbackSafePath $snapshot $dir.RelativePath) -ErrorAction Stop | Out-Null
    }
    $machineKey = Get-ElonArchiveMachineKey
    $archive = Join-ElonRollbackSafePath (Join-ElonRollbackSafePath $archiveRootPath $machineKey) $id
    if (Test-Path -LiteralPath $archive) { throw 'Shared archive already exists.' }
    New-Item -ItemType Directory -Path $archive -ErrorAction Stop | Out-Null
    $programs = @(); [long]$sharedBytes = 0
    foreach ($file in $before.Inventory.Files) {
        $slot = Get-ElonRollbackSharedSlot $file.RelativePath
        $destination = if ($slot) { Join-ElonRollbackSafePath $archive $slot } else { Join-ElonRollbackSafePath $snapshot $file.RelativePath }
        Copy-ElonRollbackVerifiedFile -Source (Join-ElonRollbackSafePath $source $file.RelativePath) -Destination $destination -Expected $file -PrimaryStreamOnly:([bool]$slot)
        if ($slot) {
            $programs += [pscustomobject]@{ Slot = $slot; RelativePath = $file.RelativePath; Length = $file.Length; Sha256 = $file.Sha256; WriteTicks = $file.WriteTicks }
            $sharedBytes += $file.Length
        }
    }
    $privateInventory = Get-ElonArchiveInventory -Root $snapshot -HashFiles
    $expectedPrivate = [ordered]@{
        Directories = @($before.Inventory.Directories)
        Files = @($before.Inventory.Files | Where-Object { -not (Get-ElonRollbackSharedSlot $_.RelativePath) } | Select-Object RelativePath,Length,Sha256)
    }
    if ((Get-ElonRollbackContentDigest $privateInventory) -ne (Get-ElonRollbackContentDigest $expectedPrivate)) { throw 'Private split inventory mismatch; source preserved.' }
    $after = Assert-ElonRollbackSnapshotTree $source
    if ($after.Verified.ManifestSha256 -ne $before.Verified.ManifestSha256 -or
        $after.Inventory.StableSourceDigest -ne $before.Inventory.StableSourceDigest) { throw 'Rollback source drifted; source preserved.' }
    $indexPath = Join-Path $archive 'programs.json'
    Write-ElonArchiveJson -Path $indexPath -Value @{ Schema = 'elon.rollback_shared_programs.v1'; Id = $id; Programs = @($programs | Select-Object Slot,Length,Sha256) }
    $receiptPath = Join-Path $private 'receipt.json'
    $receipt = [pscustomobject]@{
        Schema = 'elon.rollback_split_archive_receipt.v1'; Id = $id; Source = $source
        MachineKey = $machineKey; OwnerSid = Get-ElonRollbackOwnerSid
        OriginalManifestSha256 = $before.Verified.ManifestSha256; OriginalContentDigest = Get-ElonRollbackContentDigest $before.Inventory
        DigestVersion = 'elon.rollback.inventory.v1'
        ArchiveRoot = $archiveRootPath; ArchiveDirectory = $archive; PrivateRoot = $privateRootPath; PrivateDirectory = $private
        SharedBytes = $sharedBytes; PrivateBytes = [long]$privateInventory.Bytes; SourceBytes = [long]$before.Inventory.Bytes
        PrivateContentDigest = Get-ElonRollbackContentDigest $privateInventory; Programs = $programs
        ProgramIndexSha256 = (Get-FileHash -LiteralPath $indexPath -Algorithm SHA256).Hash; ReceiptPath = $receiptPath
    }
    Write-ElonArchiveJson -Path $receiptPath -Value $receipt
    $readback = Get-Content -LiteralPath $receiptPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ((Get-ElonArchiveDigest $readback) -ne (Get-ElonArchiveDigest $receipt)) { throw 'Rollback durable receipt read-back mismatch; source preserved.' }
    $null = Assert-ElonRollbackSplitStorage $readback
    return $receipt
}

function Restore-ElonRollbackSplitArchive {
    [CmdletBinding()]
    param([Parameter(Mandatory = $true)][string]$ReceiptPath, [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$AllowedRoot)
    $ErrorActionPreference = 'Stop'
    $receiptFull = Assert-ElonRollbackLocalPath $ReceiptPath
    Assert-ElonRollbackPrivateAcl $receiptFull
    $receipt = Get-Content -LiteralPath $receiptFull -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($receipt.ReceiptPath -ne $receiptFull) { throw 'Rollback receipt path does not match its binding.' }
    $target = Assert-ElonRollbackLocalPath $Path
    $allowed = Assert-ElonRollbackLocalPath $AllowedRoot
    if ($target -eq $allowed -or -not (Test-ElonArchiveWithin $target $allowed)) { throw 'Restore target escaped allowed root.' }
    foreach ($storage in @($receipt.PrivateRoot, $receipt.ArchiveRoot)) {
        if ((Test-ElonArchiveWithin $target $storage) -or (Test-ElonArchiveWithin $storage $target)) { throw 'Restore target overlaps split storage.' }
    }
    if (Test-Path -LiteralPath $target) { throw 'Rollback restore refuses an existing target.' }
    $snapshot = Assert-ElonRollbackSplitStorage $receipt
    $inventory = Get-ElonArchiveInventory -Root $snapshot -HashFiles
    New-ElonRollbackPrivateDirectory $target
    foreach ($dir in $inventory.Directories) {
        New-Item -ItemType Directory -Path (Join-ElonRollbackSafePath $target $dir.RelativePath) -ErrorAction Stop | Out-Null
    }
    foreach ($file in $inventory.Files) {
        Copy-ElonRollbackVerifiedFile -Source (Join-ElonRollbackSafePath $snapshot $file.RelativePath) -Destination (Join-ElonRollbackSafePath $target $file.RelativePath) -Expected $file
    }
    foreach ($program in $receipt.Programs) {
        Copy-ElonRollbackVerifiedFile -Source (Join-ElonRollbackSafePath $receipt.ArchiveDirectory $program.Slot) -Destination (Join-ElonRollbackSafePath $target $program.RelativePath) -Expected $program -PrimaryStreamOnly
    }
    $restored = Assert-ElonRollbackSnapshotTree $target
    if ($restored.Verified.ManifestSha256 -ne $receipt.OriginalManifestSha256 -or
        (Get-ElonRollbackContentDigest $restored.Inventory) -ne $receipt.OriginalContentDigest -or [long]$restored.Inventory.Bytes -ne [long]$receipt.SourceBytes) { throw 'Restored rollback snapshot verification failed.' }
    foreach ($item in Get-ChildItem -LiteralPath $target -Recurse -Force) { Assert-ElonRollbackPrivateAcl $item.FullName }
    return [pscustomobject]@{ Path = $target; Restored = $true; SourceBytes = [long]$receipt.SourceBytes; OriginalManifestSha256 = $restored.Verified.ManifestSha256 }
}
