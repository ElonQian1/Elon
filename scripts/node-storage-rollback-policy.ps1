# Read-only rollback selection and held maintenance guards. Callers own archive/reclaim.
Set-StrictMode -Version 2.0
. (Join-Path $PSScriptRoot 'node-storage-archive.ps1')
. (Join-Path $PSScriptRoot 'node-storage-paths.ps1')
. (Join-Path $PSScriptRoot 'node-agent-local-activation.ps1')
. (Join-Path $PSScriptRoot 'node-agent-local-rollback.ps1')

function Get-ElonRollbackJson {
    param([string]$Path)
    Assert-ElonArchivePath $Path
    return Read-ElonStorageJson $Path
}

function Get-ElonRollbackCanonical {
    param($Value)
    if ($null -eq $Value) { return $null }
    if ($Value -is [System.Collections.IDictionary]) {
        $ordered = [ordered]@{}
        foreach ($key in @($Value.Keys | Sort-Object)) { $ordered[$key] = Get-ElonRollbackCanonical $Value[$key] }
        return $ordered
    }
    if ($Value -is [pscustomobject]) {
        $ordered = [ordered]@{}
        foreach ($property in @($Value.PSObject.Properties | Sort-Object Name)) {
            $ordered[$property.Name] = Get-ElonRollbackCanonical $property.Value
        }
        return $ordered
    }
    if ($Value -is [array]) { return ,@($Value | ForEach-Object { Get-ElonRollbackCanonical $_ }) }
    return $Value
}

function Test-ElonRollbackIdentity {
    param([string]$Identity)
    return $Identity -cmatch '^[^+\s]+\+[0-9a-f]{40}$'
}

function Get-ElonRollbackUtcTicks {
    param([Parameter(Mandatory = $true)]$Value)
    if ($Value -is [DateTimeOffset]) { return $Value.UtcDateTime.Ticks }
    if ($Value -is [DateTime]) { return $Value.ToUniversalTime().Ticks }
    return [DateTimeOffset]::Parse([string]$Value, [Globalization.CultureInfo]::InvariantCulture,
        [Globalization.DateTimeStyles]::AssumeUniversal).UtcDateTime.Ticks
}

function Get-ElonRollbackRoots {
    $roots = @()
    $legacy = Join-Path $env:LOCALAPPDATA 'Elon\local-node-releases-v1'
    if (Test-Path -LiteralPath $legacy -PathType Container) { $roots += $legacy }
    $configPath = Join-Path $env:APPDATA 'elon-node-agent\node.json'
    if (Test-Path -LiteralPath $configPath -PathType Leaf) {
        $config = Get-ElonRollbackJson $configPath
        if ($config.PSObject.Properties.Name -contains 'node_data_root' -and $config.node_data_root) {
            if (-not ($config.PSObject.Properties.Name -contains 'install_id') -or -not $config.install_id -or
                -not (Test-ElonOwnedNodeDataRoot -RootPath $config.node_data_root -ExpectedInstallId $config.install_id)) {
                throw 'Persisted node data root ownership is not verified.'
            }
            $managed = Join-Path $config.node_data_root 'release-state\local-node-releases-v1'
            if (Test-Path -LiteralPath $managed -PathType Container) { $roots += $managed }
        }
    }
    foreach ($root in @($roots | Sort-Object -Unique)) {
        Assert-ElonArchivePath $root
        Get-ElonArchiveFullPath $root
    }
}

function Get-ElonRollbackInstalledIdentity {
    $install = Join-Path $env:LOCALAPPDATA 'ElonNode'
    $executable = Join-Path $install '一龙开发平台.exe'
    $metadataPath = Join-Path $install '_internal\node-agent-version.json'
    Assert-ElonArchivePath $executable
    $metadata = Get-ElonRollbackJson $metadataPath
    $identity = '{0}+{1}' -f $metadata.version, $metadata.gitSha
    if (-not (Test-ElonRollbackIdentity $identity)) { throw 'Installed release identity is invalid.' }
    $beforeHash = (Get-FileHash -LiteralPath $metadataPath -Algorithm SHA256 -ErrorAction Stop).Hash
    $listener = Get-NodeAgentInstalledAdminListener -ExpectedClientPath $executable
    if ($null -eq $listener) { throw 'Installed runtime listener is unavailable.' }
    $started = (Get-Process -Id $listener.ProcessId -ErrorAction Stop).StartTime.ToUniversalTime().ToString('o')
    $request = [Net.HttpWebRequest]::Create("http://127.0.0.1:$($listener.Port)/api/status")
    $request.Proxy = $null; $request.Timeout = 2000; $request.ReadWriteTimeout = 2000
    $response = $request.GetResponse()
    try {
        $reader = New-Object IO.StreamReader($response.GetResponseStream(), [Text.Encoding]::UTF8)
        try { $status = $reader.ReadToEnd() | ConvertFrom-Json -ErrorAction Stop } finally { $reader.Dispose() }
    } finally { $response.Dispose() }
    $after = Get-NodeAgentInstalledAdminListener -Ports @([int]$listener.Port) -ExpectedClientPath $executable
    if ($null -eq $after -or $after.ProcessId -ne $listener.ProcessId -or
        (Get-Process -Id $after.ProcessId -ErrorAction Stop).StartTime.ToUniversalTime().ToString('o') -ne $started -or
        (Get-FileHash -LiteralPath $metadataPath -Algorithm SHA256 -ErrorAction Stop).Hash -ne $beforeHash -or
        [string]$status.release_identity -cne $identity -or [string]$status.version -cne [string]$metadata.version -or
        [string]$status.build_git_sha -cne [string]$metadata.gitSha) {
        throw 'Installed metadata and stable runtime identity do not agree.'
    }
    [pscustomobject]@{ Identity = $identity; Version = [string]$metadata.version; GitSha = [string]$metadata.gitSha
        ProcessId = [int]$listener.ProcessId; ProcessStartUtc = $started; ExecutablePath = $executable
        MetadataPath = $metadataPath; MetadataSha256 = $beforeHash }
}

function Assert-ElonRollbackIdle {
    param([string[]]$Roots, $ExpectedIdentity = $null)
    foreach ($process in Get-CimInstance Win32_Process -ErrorAction Stop) {
        $command = [string]$process.CommandLine
        if ($command -match '(?i)(node-agent-post-terminal-activator|node-agent-remote-release-worker|publish-node-agent)\.ps1|--(?:repair(?:-background)?|update(?:-background)?|uninstall)(?:["\s]|$)' -or
            $command -match '(?i)elon-node-agent-update-|update\.bat(?:["\s]|$)') {
            throw 'Node activation, repair or update process is present; rollback maintenance is deferred.'
        }
        foreach ($root in $Roots) {
            if ($process.ExecutablePath -and (Test-ElonArchiveWithin $process.ExecutablePath (Join-Path $root 'rollback'))) {
                throw 'A rollback snapshot is executing.'
            }
        }
    }
    if ($null -ne $ExpectedIdentity) {
        $current = Get-ElonRollbackInstalledIdentity
        $expected = if ($ExpectedIdentity -is [string]) { $ExpectedIdentity } else { [string]$ExpectedIdentity.Identity }
        if ($current.Identity -cne $expected) { throw 'Installed runtime changed since rollback planning.' }
        if ($ExpectedIdentity -isnot [string] -and ($current.ProcessId -ne $ExpectedIdentity.ProcessId -or
            (Get-ElonRollbackUtcTicks $current.ProcessStartUtc) -ne (Get-ElonRollbackUtcTicks $ExpectedIdentity.ProcessStartUtc) -or
            $current.MetadataSha256 -ne $ExpectedIdentity.MetadataSha256)) {
            throw 'Installed runtime process or metadata changed during maintenance.'
        }
    }
}

function Enter-ElonRollbackMaintenanceLocks {
    param([string[]]$Roots)
    $handles = New-Object 'Collections.Generic.List[IO.FileStream]'
    try {
        $paths = @($Roots | Sort-Object -Unique | ForEach-Object { Join-Path $_ 'post-terminal-activator.lock' })
        $internal = Join-Path $env:LOCALAPPDATA 'ElonNode\_internal'
        foreach ($name in @('update.spawn.lock', 'update.owner.lock', 'update.apply.lock')) { $paths += Join-Path $internal $name }
        foreach ($path in $paths) {
            Assert-ElonArchivePath $path
            if (-not (Test-Path -LiteralPath (Split-Path -Parent $path) -PathType Container)) { throw 'Maintenance lock parent is missing.' }
            $handles.Add([IO.File]::Open($path, [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None))
        }
        # These are the existing managed activator/updater gates. Direct legacy repair
        # entrypoints do not all participate; callers also revalidate runtime/processes.
        return $handles.ToArray()
    } catch {
        foreach ($handle in $handles) { $handle.Dispose() }
        throw
    }
}

function Get-ElonRollbackReleaseReferences {
    param([string]$Root)
    $references = @(); $catalog = @()
    $releaseRoot = Join-Path $Root 'releases'
    if (-not (Test-Path -LiteralPath $releaseRoot -PathType Container)) { throw 'Release state graph is missing.' }
    foreach ($directory in Get-ChildItem -LiteralPath $releaseRoot -Force -ErrorAction Stop | Sort-Object Name) {
        Assert-ElonArchivePath $directory.FullName
        if (-not $directory.PSIsContainer -or $directory.Name -cnotmatch '^[0-9a-f]{40}$') { throw 'Unknown release state entry.' }
        $statePath = Join-Path $directory.FullName 'state.json'
        $state = Get-ElonRollbackJson $statePath
        if ($state.schema -cne 'elon.node_local_release.v1' -or $state.git_sha -cne $directory.Name -or
            $state.release_identity -cne ('{0}+{1}' -f $state.version, $state.git_sha) -or
            -not (Test-ElonRollbackIdentity $state.release_identity)) { throw 'Release state identity is invalid.' }
        $stateHash = (Get-FileHash -LiteralPath $statePath -Algorithm SHA256 -ErrorAction Stop).Hash
        $receiptPath = Join-Path $directory.FullName 'activation-receipt.json'
        $hasInline = $state.PSObject.Properties.Name -contains 'activation_receipt' -and $null -ne $state.activation_receipt
        $hasReceiptFields = @('activation_receipt_path', 'activation_receipt_sha256') | Where-Object { $state.PSObject.Properties.Name -contains $_ }
        $catalog += [pscustomobject]@{ Path = $statePath; Sha256 = $stateHash }
        if (-not $hasInline -and -not (Test-Path -LiteralPath $receiptPath) -and @($hasReceiptFields).Count -eq 0) { continue }
        if (-not $hasInline -or @($hasReceiptFields).Count -ne 2 -or
            (Get-ElonArchiveFullPath $state.activation_receipt_path) -ne (Get-ElonArchiveFullPath $receiptPath)) { throw 'Activation receipt binding is incomplete.' }
        $receipt = Get-ElonRollbackJson $receiptPath
        $receiptHash = (Get-FileHash -LiteralPath $receiptPath -Algorithm SHA256 -ErrorAction Stop).Hash
        if ($receiptHash -ine [string]$state.activation_receipt_sha256 -or
            (Get-ElonArchiveDigest (Get-ElonRollbackCanonical $receipt)) -ne (Get-ElonArchiveDigest (Get-ElonRollbackCanonical $state.activation_receipt)) -or
            $receipt.schema -cne 'elon.node_local_activation_receipt.v1' -or $receipt.target_release_identity -cne $state.release_identity -or
            [long]$receipt.started_at_ms -le 0 -or [long]$receipt.finished_at_ms -lt [long]$receipt.started_at_ms) { throw 'Activation receipt graph is inconsistent.' }
        $catalog += [pscustomobject]@{ Path = $receiptPath; Sha256 = $receiptHash }
        if (-not $receipt.snapshot_directory -and -not $receipt.snapshot_manifest_sha256) { continue }
        $fullPriorIdentity = Test-ElonRollbackIdentity $receipt.prior_release_identity
        $legacyPriorVersion = [string]$receipt.prior_release_identity -cmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:[-.][A-Za-z0-9]+)*$'
        if ([string]$receipt.snapshot_directory -cnotmatch '^[0-9]{17}-[0-9a-f]{8}$' -or
            [string]$receipt.snapshot_manifest_sha256 -cnotmatch '^[0-9a-f]{64}$' -or
            (-not $fullPriorIdentity -and -not $legacyPriorVersion)) { throw 'Activation snapshot reference is invalid.' }
        $successful = $state.activation_state -ceq 'activated' -and $state.local_terminal_state -ceq 'complete' -and
            $receipt.outcome -ceq 'activated' -and $receipt.rollback_state -ceq 'not_required' -and -not $receipt.failure_phase -and -not $receipt.error -and $fullPriorIdentity
        $references += [pscustomobject]@{ SnapshotName = [string]$receipt.snapshot_directory
            ManifestSha256 = [string]$receipt.snapshot_manifest_sha256; PriorIdentity = [string]$receipt.prior_release_identity
            TargetIdentity = [string]$receipt.target_release_identity; StartedAtMs = [long]$receipt.started_at_ms
            FinishedAtMs = [long]$receipt.finished_at_ms; Successful = $successful
            StatePath = $statePath; StateSha256 = $stateHash; ReceiptPath = $receiptPath; ReceiptSha256 = $receiptHash }
    }
    [pscustomobject]@{ References = @($references); Catalog = @($catalog) }
}

function Get-ElonRollbackSnapshotMetadata {
    param([string]$Path, $Inventory)
    $manifestPath = Join-Path $Path 'manifest.json'
    $hashPath = Join-Path $Path 'manifest.sha256'
    $manifest = Get-ElonRollbackJson $manifestPath
    Assert-ElonArchivePath $hashPath
    $expected = [IO.File]::ReadAllText($hashPath, [Text.Encoding]::UTF8).Trim()
    $actual = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
    if ($expected -cnotmatch '^[0-9a-f]{64}$' -or $expected -cne $actual -or
        $manifest.schema -cne 'elon.node_local_rollback_snapshot.v1' -or [int]$manifest.allowlist_version -ne 1 -or
        -not (Test-ElonRollbackIdentity $manifest.prior_release_identity) -or [long]$manifest.created_at_ms -le 0 -or
        [long]$manifest.created_at_ms -gt [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()) { throw 'Snapshot manifest metadata is invalid.' }
    $declared = New-Object 'Collections.Generic.Dictionary[string,object]' ([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in @($manifest.files)) {
        $relative = ([string]$file.relative_path).Replace('/', '\')
        Assert-NodeAgentRollbackRelativePath $relative
        if ($relative -match ':' -or [string]$file.sha256 -cnotmatch '^[0-9a-f]{64}$' -or [long]$file.length -lt 0 -or
            $declared.ContainsKey($relative)) { throw 'Rollback manifest payload metadata is invalid.' }
        $declared.Add($relative, $file)
    }
    if ($declared.Count -eq 0) { throw 'Rollback manifest is empty.' }
    $seen = 0
    foreach ($file in @($Inventory.Files)) {
        $relative = [string]$file.RelativePath
        if ($relative -cin @('manifest.json', 'manifest.sha256')) { continue }
        if (-not $relative.StartsWith('ElonNode\', [StringComparison]::Ordinal)) { throw 'Unknown file outside rollback payload.' }
        $payload = $relative.Substring(9)
        if (-not $declared.ContainsKey($payload) -or [long]$declared[$payload].length -ne [long]$file.Length) { throw 'Snapshot payload membership or size changed.' }
        $seen++
    }
    if ($seen -ne $declared.Count) { throw 'Rollback snapshot is missing declared payload files.' }
    foreach ($directory in @($Inventory.Directories)) {
        if ([string]$directory.RelativePath -cne 'ElonNode' -and
            -not ([string]$directory.RelativePath).StartsWith('ElonNode\', [StringComparison]::Ordinal)) { throw 'Unknown directory outside rollback payload.' }
    }
    # Full payload hashes are verified by the archive executor, not every policy rescan.
    [pscustomobject]@{ Manifest = $manifest; ManifestSha256 = $actual }
}

function Get-ElonRollbackEvidenceCatalog {
    param([Parameter(Mandatory = $true)][string[]]$Roots)
    $catalog = @()
    foreach ($root in @($Roots | ForEach-Object { Get-ElonArchiveFullPath $_ } | Sort-Object -Unique)) {
        Assert-ElonArchivePath $root
        if (-not (Test-Path -LiteralPath $root -PathType Container)) { throw 'Rollback evidence root is missing.' }
        $graph = Get-ElonRollbackReleaseReferences $root
        foreach ($entry in @($graph.Catalog)) {
            $catalog += [pscustomobject]@{ Kind = 'release_evidence'; Path = $entry.Path; Sha256 = $entry.Sha256 }
        }
        $rollbackRoot = Join-Path $root 'rollback'
        Assert-ElonArchivePath $rollbackRoot
        if (-not (Test-Path -LiteralPath $rollbackRoot -PathType Container)) { continue }
        foreach ($entry in Get-ChildItem -LiteralPath $rollbackRoot -Force -ErrorAction Stop | Sort-Object Name) {
            $isReparse = ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0
            $evidence = [ordered]@{ Kind = 'snapshot_evidence'; Path = $entry.FullName; Root = $root
                EntryType = $(if ($entry.PSIsContainer) { 'directory' } else { 'file' }); IsReparsePoint = $isReparse
                ManifestStatus = 'not_inspected'; ManifestSha256 = $null
                ManifestHashStatus = 'not_inspected'; ManifestHashFileSha256 = $null }
            # Unknown entries are structural evidence only; never follow their children.
            if (-not $isReparse -and $entry.PSIsContainer -and $entry.Name -cmatch '^[0-9]{17}-[0-9a-f]{8}$') {
                foreach ($spec in @(@('manifest.json', 'ManifestStatus', 'ManifestSha256'),
                    @('manifest.sha256', 'ManifestHashStatus', 'ManifestHashFileSha256'))) {
                    $path = Join-Path $entry.FullName $spec[0]
                    try { $attributes = [IO.File]::GetAttributes($path) }
                    catch [IO.FileNotFoundException] { $evidence[$spec[1]] = 'missing'; continue }
                    catch [IO.DirectoryNotFoundException] { $evidence[$spec[1]] = 'missing'; continue }
                    if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { $evidence[$spec[1]] = 'reparse'; continue }
                    if (($attributes -band [IO.FileAttributes]::Directory) -ne 0) { $evidence[$spec[1]] = 'directory'; continue }
                    $evidence[$spec[1]] = 'file'
                    $evidence[$spec[2]] = (Get-FileHash -LiteralPath $path -Algorithm SHA256 -ErrorAction Stop).Hash
                }
            }
            $catalog += [pscustomobject]$evidence
        }
    }
    return @($catalog | Sort-Object Kind, Path)
}

function Get-ElonRollbackPlan {
    param([Parameter(Mandatory = $true)][string[]]$Roots, [Parameter(Mandatory = $true)][string]$CurrentIdentity,
        [ValidateRange(2, 100)][int]$KeepNewest = 2, [ValidateRange(30, 36500)][int]$MinAgeDays = 30)
    if (-not (Test-ElonRollbackIdentity $CurrentIdentity)) { throw 'Current release identity is invalid.' }
    $normalized = @($Roots | ForEach-Object { Get-ElonArchiveFullPath $_ } | Sort-Object -Unique)
    $snapshots = @(); $protected = @(); $blocked = @(); $catalog = @()
    foreach ($root in $normalized) {
        $rootSnapshots = @()
        try {
            Assert-ElonArchivePath $root
            $graph = Get-ElonRollbackReleaseReferences $root
            $catalog += @($graph.Catalog)
            $rollbackRoot = Join-Path $root 'rollback'
            if (-not (Test-Path -LiteralPath $rollbackRoot -PathType Container)) { continue }
            Assert-ElonArchivePath $rollbackRoot
            foreach ($directory in Get-ChildItem -LiteralPath $rollbackRoot -Force -ErrorAction Stop | Sort-Object Name) {
                try {
                    Assert-ElonArchivePath $directory.FullName
                    if (-not $directory.PSIsContainer -or $directory.Name -cnotmatch '^[0-9]{17}-[0-9a-f]{8}$') { throw 'Unknown or pending rollback entry.' }
                    $inventory = Get-ElonArchiveInventory -Root $directory.FullName
                    $verified = Get-ElonRollbackSnapshotMetadata -Path $directory.FullName -Inventory $inventory
                    $manifest = $verified.Manifest
                    $entry = [pscustomobject]@{ Path = $directory.FullName; Root = $root; AllowedRoot = $rollbackRoot
                        SnapshotName = $directory.Name; PriorIdentity = [string]$manifest.prior_release_identity
                        ManifestSha256 = [string]$verified.ManifestSha256; CreatedAtMs = [long]$manifest.created_at_ms
                        EstimatedBytes = [long]$inventory.Bytes; SourceDigest = [string]$inventory.SourceDigest
                        LatestWriteUtc = [string]$inventory.LatestWriteUtc; References = @($graph.References | Where-Object { $_.SnapshotName -ceq $directory.Name }) }
                    foreach ($reference in $entry.References) {
                        if ($reference.ManifestSha256 -cne $entry.ManifestSha256 -or $reference.PriorIdentity -cne $entry.PriorIdentity -or
                            $entry.CreatedAtMs -lt $reference.StartedAtMs -or $entry.CreatedAtMs -gt $reference.FinishedAtMs) { throw 'Snapshot and activation receipt disagree.' }
                    }
                    $rootSnapshots += $entry
                    $catalog += [pscustomobject]@{ Path = $entry.Path; ManifestSha256 = $entry.ManifestSha256; SourceDigest = $entry.SourceDigest }
                } catch {
                    # A bad snapshot cannot authorize another snapshot. Preserve it without
                    # discarding an independently verified release-state graph for this root.
                    $protected += [pscustomobject]@{ Path = $directory.FullName; Reason = ('invalid_snapshot: ' + $_.Exception.Message) }
                    $catalog += [pscustomobject]@{ Path = $directory.FullName; Protected = $true
                        Attributes = [string]$directory.Attributes; WriteTicks = $directory.LastWriteTimeUtc.Ticks }
                }
            }
            $snapshots += $rootSnapshots
        } catch {
            $blocked += [pscustomobject]@{ Root = $root; Reason = $_.Exception.Message }
        }
    }
    $recentIdentities = @($snapshots | Sort-Object CreatedAtMs -Descending | Select-Object -ExpandProperty PriorIdentity -Unique | Select-Object -First $KeepNewest)
    $cutoff = [DateTimeOffset]::UtcNow.AddDays(-$MinAgeDays).ToUnixTimeMilliseconds()
    $candidates = @()
    foreach ($snapshot in $snapshots) {
        $reason = $null
        if ($snapshot.PriorIdentity -ceq $CurrentIdentity -or @($snapshot.References | Where-Object { $_.TargetIdentity -ceq $CurrentIdentity }).Count -gt 0) { $reason = 'current_release_related' }
        elseif ($snapshot.PriorIdentity -in $recentIdentities) { $reason = 'recent_distinct_prior_identity' }
        elseif ($snapshot.CreatedAtMs -ge $cutoff -or [DateTime]::Parse($snapshot.LatestWriteUtc).ToUniversalTime() -ge [DateTime]::UtcNow.AddDays(-$MinAgeDays)) { $reason = 'inside_retention_window' }
        elseif (@($snapshot.References).Count -ne 1) { $reason = 'missing_or_ambiguous_activation_reference' }
        elseif (-not $snapshot.References[0].Successful) { $reason = 'activation_not_successfully_closed' }
        if ($reason) { $protected += [pscustomobject]@{ Path = $snapshot.Path; Reason = $reason }; continue }
        $reference = $snapshot.References[0]
        $candidates += [pscustomobject]@{ Path = $snapshot.Path; Root = $snapshot.Root; AllowedRoot = $snapshot.AllowedRoot
            SnapshotName = $snapshot.SnapshotName; PriorIdentity = $snapshot.PriorIdentity; TargetIdentity = $reference.TargetIdentity
            ManifestSha256 = $snapshot.ManifestSha256; StatePath = $reference.StatePath; StateSha256 = $reference.StateSha256
            ReceiptPath = $reference.ReceiptPath; ReceiptSha256 = $reference.ReceiptSha256; CreatedAtMs = $snapshot.CreatedAtMs
            EstimatedBytes = $snapshot.EstimatedBytes; SourceDigest = $snapshot.SourceDigest }
    }
    [pscustomobject]@{ Schema = 'elon.rollback_reclaim_plan.v1'; Roots = $normalized
        RootsFingerprint = Get-ElonArchiveDigest ([ordered]@{ Items = $normalized }); CurrentIdentity = $CurrentIdentity
        KeepNewest = $KeepNewest; MinAgeDays = $MinAgeDays
        SnapshotCatalogDigest = Get-ElonArchiveDigest ([ordered]@{ Items = @($catalog | Sort-Object Path) })
        Candidates = @($candidates | Sort-Object Path); Protected = @($protected); BlockedRoots = @($blocked) }
}
