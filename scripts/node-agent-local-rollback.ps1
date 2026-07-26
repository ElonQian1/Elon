Set-StrictMode -Version Latest

if (-not (Get-Command Write-NodeAgentReleaseJsonAtomic -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot 'node-agent-release-outbox.ps1')
}

$script:NodeAgentRollbackRootFiles = @(
    '一龙开发平台.exe',
    '卸载一龙开发平台.exe'
)
$script:NodeAgentRollbackInternalFiles = @(
    'desktop-review-credential.ps1',
    'elon-desktop.exe',
    'new-desktop-review-ticket.ps1',
    'node-agent-version.json',
    'node-agent.env',
    'node-agent.env.example',
    'README.txt'
)
$script:NodeAgentRollbackIgnoredRootItems = @(
    'ai-finish-contracts-v1',
    'supervisor-node-url.bootstrap-backup',
    'supervisor-node-url.cold-probe-backup',
    'supervisor-node-url.txt',
    'terminal-finalization-receipts-v1',
    'tools'
)
$script:NodeAgentRollbackIgnoredInternalItems = @(
    'elon-node-agent-windows.zip.new',
    'logs',
    'node-agent-version.json.new',
    'update-background-state.json',
    'update.apply.lock',
    'update.owner.lock',
    'update.spawn.lock',
    'watchdog.instance.lock'
)

function Get-NodeAgentRollbackRelativePath {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Path
    )
    $rootPath = [System.IO.Path]::GetFullPath($Root).TrimEnd('\', '/')
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $prefix = $rootPath + [System.IO.Path]::DirectorySeparatorChar
    if (-not $fullPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Rollback inventory path escaped its root: $fullPath"
    }
    return $fullPath.Substring($prefix.Length).Replace('/', '\')
}

function Assert-NodeAgentRollbackRelativePath {
    param([Parameter(Mandatory = $true)][string]$RelativePath)
    if ([string]::IsNullOrWhiteSpace($RelativePath) -or
        [System.IO.Path]::IsPathRooted($RelativePath) -or
        $RelativePath -match '(^|[\\/])\.\.([\\/]|$)') {
        throw "Rollback manifest contains an unsafe relative path: $RelativePath"
    }
    $normalized = $RelativePath.Replace('/', '\')
    if ($normalized -in $script:NodeAgentRollbackRootFiles) { return }
    if ($normalized.StartsWith('_internal\pc-next-dist\', [System.StringComparison]::OrdinalIgnoreCase)) { return }
    if ($normalized.StartsWith('_internal\', [System.StringComparison]::OrdinalIgnoreCase)) {
        $leaf = $normalized.Substring('_internal\'.Length)
        if ($leaf -in $script:NodeAgentRollbackInternalFiles) { return }
    }
    throw "Rollback manifest path is outside the stable client allowlist: $RelativePath"
}

function Assert-NodeAgentRollbackSourceItem {
    param([Parameter(Mandatory = $true)][System.IO.FileSystemInfo]$Item)
    if (($Item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Rollback source contains a reparse point: $($Item.FullName)"
    }
}

function Get-NodeAgentRollbackInventory {
    param([Parameter(Mandatory = $true)][string]$InstallRoot)
    $root = [System.IO.Path]::GetFullPath($InstallRoot)
    if (-not (Test-Path -LiteralPath $root -PathType Container)) {
        throw "Installed node client root does not exist: $root"
    }
    $required = @(
        (Join-Path $root '一龙开发平台.exe'),
        (Join-Path $root '_internal\elon-desktop.exe'),
        (Join-Path $root '_internal\node-agent-version.json')
    )
    foreach ($path in $required) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Rollback source is missing a required stable client file: $path"
        }
    }

    $files = New-Object System.Collections.Generic.List[System.IO.FileInfo]
    foreach ($item in Get-ChildItem -LiteralPath $root -Force) {
        Assert-NodeAgentRollbackSourceItem -Item $item
        if ($item.Name -in $script:NodeAgentRollbackIgnoredRootItems) { continue }
        if ($item.Name -eq '_internal' -and $item.PSIsContainer) { continue }
        if (-not $item.PSIsContainer -and $item.Name -in $script:NodeAgentRollbackRootFiles) {
            $files.Add([System.IO.FileInfo]$item)
            continue
        }
        throw "Installed node root contains an unclassified item; rollback must fail closed: $($item.Name)"
    }

    $internalRoot = Join-Path $root '_internal'
    foreach ($item in Get-ChildItem -LiteralPath $internalRoot -Force) {
        Assert-NodeAgentRollbackSourceItem -Item $item
        if ($item.Name -in $script:NodeAgentRollbackIgnoredInternalItems) { continue }
        if (-not $item.PSIsContainer -and $item.Name -in $script:NodeAgentRollbackInternalFiles) {
            $files.Add([System.IO.FileInfo]$item)
            continue
        }
        if ($item.PSIsContainer -and $item.Name -eq 'pc-next-dist') {
            foreach ($nested in Get-ChildItem -LiteralPath $item.FullName -Force -Recurse) {
                Assert-NodeAgentRollbackSourceItem -Item $nested
                if (-not $nested.PSIsContainer) {
                    $files.Add([System.IO.FileInfo]$nested)
                }
            }
            continue
        }
        throw "Installed _internal tree contains an unclassified item; rollback must fail closed: $($item.Name)"
    }
    return @($files | Sort-Object FullName)
}

function Test-NodeAgentRollbackSnapshot {
    param(
        [Parameter(Mandatory = $true)][string]$SnapshotRoot,
        [string]$ExpectedPriorReleaseIdentity = ''
    )
    $root = [System.IO.Path]::GetFullPath($SnapshotRoot)
    $manifestPath = Join-Path $root 'manifest.json'
    $hashPath = Join-Path $root 'manifest.sha256'
    $clientRoot = Join-Path $root 'ElonNode'
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf) -or
        -not (Test-Path -LiteralPath $hashPath -PathType Leaf) -or
        -not (Test-Path -LiteralPath $clientRoot -PathType Container)) {
        throw 'Rollback snapshot is incomplete.'
    }
    $expectedManifestHash = [System.IO.File]::ReadAllText($hashPath, [System.Text.Encoding]::UTF8).Trim().ToLowerInvariant()
    if ($expectedManifestHash -notmatch '^[0-9a-f]{64}$') {
        throw 'Rollback snapshot manifest hash is malformed.'
    }
    $actualManifestHash = Get-NodeAgentFileSha256 -Path $manifestPath
    if ($actualManifestHash -ne $expectedManifestHash) {
        throw "Rollback snapshot manifest hash mismatch: expected=$expectedManifestHash actual=$actualManifestHash"
    }
    $manifest = [System.IO.File]::ReadAllText($manifestPath, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
    if ([string]$manifest.schema -ne 'elon.node_local_rollback_snapshot.v1') {
        throw 'Rollback snapshot schema is unsupported.'
    }
    if (-not [string]::IsNullOrWhiteSpace($ExpectedPriorReleaseIdentity) -and
        [string]$manifest.prior_release_identity -ne $ExpectedPriorReleaseIdentity) {
        throw 'Rollback snapshot prior release identity does not match the activation source.'
    }

    $declared = New-Object 'System.Collections.Generic.Dictionary[string,object]' ([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in @($manifest.files)) {
        $relative = ([string]$entry.relative_path).Replace('/', '\')
        Assert-NodeAgentRollbackRelativePath -RelativePath $relative
        if ($declared.ContainsKey($relative)) { throw "Rollback snapshot contains a duplicate path: $relative" }
        if ([string]$entry.sha256 -notmatch '^[0-9a-f]{64}$' -or [long]$entry.length -lt 0) {
            throw "Rollback snapshot contains invalid file metadata: $relative"
        }
        $path = Join-Path $clientRoot $relative
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Rollback snapshot file is missing: $relative"
        }
        $file = Get-Item -LiteralPath $path -Force
        Assert-NodeAgentRollbackSourceItem -Item $file
        if ([long]$file.Length -ne [long]$entry.length) {
            throw "Rollback snapshot file length mismatch: $relative"
        }
        $actualHash = Get-NodeAgentFileSha256 -Path $path
        if ($actualHash -ne ([string]$entry.sha256).ToLowerInvariant()) {
            throw "Rollback snapshot file hash mismatch: $relative"
        }
        $declared.Add($relative, $entry)
    }
    if ($declared.Count -eq 0) { throw 'Rollback snapshot contains no stable client files.' }

    $actualFiles = @(Get-ChildItem -LiteralPath $clientRoot -File -Force -Recurse)
    if ($actualFiles.Count -ne $declared.Count) {
        throw 'Rollback snapshot contains undeclared or missing client files.'
    }
    foreach ($file in $actualFiles) {
        Assert-NodeAgentRollbackSourceItem -Item $file
        $relative = Get-NodeAgentRollbackRelativePath -Root $clientRoot -Path $file.FullName
        if (-not $declared.ContainsKey($relative)) {
            throw "Rollback snapshot contains an undeclared file: $relative"
        }
    }
    $clientPath = Join-Path $clientRoot '一龙开发平台.exe'
    return [pscustomobject]@{
        SnapshotRoot = $root
        ClientRoot = $clientRoot
        ClientPath = $clientPath
        ManifestPath = $manifestPath
        ManifestSha256 = $actualManifestHash
        PriorReleaseIdentity = [string]$manifest.prior_release_identity
        FileCount = $declared.Count
    }
}

function New-NodeAgentRollbackSnapshot {
    param(
        [Parameter(Mandatory = $true)][string]$InstallRoot,
        [Parameter(Mandatory = $true)][string]$SnapshotRoot,
        [Parameter(Mandatory = $true)][string]$PriorReleaseIdentity
    )
    if ([string]::IsNullOrWhiteSpace($PriorReleaseIdentity)) {
        throw 'Prior release identity is required for a rollback snapshot.'
    }
    $sourceRoot = [System.IO.Path]::GetFullPath($InstallRoot).TrimEnd('\', '/')
    $targetRoot = [System.IO.Path]::GetFullPath($SnapshotRoot).TrimEnd('\', '/')
    $sourcePrefix = $sourceRoot + [System.IO.Path]::DirectorySeparatorChar
    $targetPrefix = $targetRoot + [System.IO.Path]::DirectorySeparatorChar
    if ($targetRoot.Equals($sourceRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
        $targetRoot.StartsWith($sourcePrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
        $sourceRoot.StartsWith($targetPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Rollback snapshot root must be isolated from the installed client root.'
    }
    if (Test-Path -LiteralPath $targetRoot) {
        throw "Rollback snapshot target already exists: $targetRoot"
    }

    $inventory = @(Get-NodeAgentRollbackInventory -InstallRoot $sourceRoot)
    $parent = Split-Path -Parent $targetRoot
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    $pendingRoot = Join-Path $parent ('.' + (Split-Path -Leaf $targetRoot) + '.pending-' + [Guid]::NewGuid().ToString('N'))
    $moved = $false
    try {
        $pendingClientRoot = Join-Path $pendingRoot 'ElonNode'
        New-Item -ItemType Directory -Path $pendingClientRoot -Force | Out-Null
        $entries = New-Object System.Collections.Generic.List[object]
        foreach ($source in $inventory) {
            $relative = Get-NodeAgentRollbackRelativePath -Root $sourceRoot -Path $source.FullName
            Assert-NodeAgentRollbackRelativePath -RelativePath $relative
            $destination = Join-Path $pendingClientRoot $relative
            New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force | Out-Null
            [System.IO.File]::Copy($source.FullName, $destination, $false)
            $sourceHash = Get-NodeAgentFileSha256 -Path $source.FullName
            $destinationHash = Get-NodeAgentFileSha256 -Path $destination
            if ($sourceHash -ne $destinationHash) {
                throw "Rollback snapshot copy failed verification: $relative"
            }
            $entries.Add([pscustomobject][ordered]@{
                relative_path = $relative
                length = [long](Get-Item -LiteralPath $destination).Length
                sha256 = $destinationHash
            })
        }
        $manifest = [pscustomobject][ordered]@{
            schema = 'elon.node_local_rollback_snapshot.v1'
            prior_release_identity = $PriorReleaseIdentity
            created_at_ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
            allowlist_version = 1
            files = $entries.ToArray()
        }
        $pendingManifestPath = Join-Path $pendingRoot 'manifest.json'
        Write-NodeAgentReleaseJsonAtomic -Path $pendingManifestPath -Value $manifest
        $manifestHash = Get-NodeAgentFileSha256 -Path $pendingManifestPath
        [System.IO.File]::WriteAllText(
            (Join-Path $pendingRoot 'manifest.sha256'),
            $manifestHash,
            (New-Object System.Text.UTF8Encoding($false))
        )
        Move-Item -LiteralPath $pendingRoot -Destination $targetRoot
        $moved = $true
        return (Test-NodeAgentRollbackSnapshot -SnapshotRoot $targetRoot `
            -ExpectedPriorReleaseIdentity $PriorReleaseIdentity)
    } catch {
        if ($moved -and (Test-Path -LiteralPath $targetRoot)) {
            Remove-Item -LiteralPath $targetRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
        throw
    } finally {
        if (Test-Path -LiteralPath $pendingRoot) {
            Remove-Item -LiteralPath $pendingRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
