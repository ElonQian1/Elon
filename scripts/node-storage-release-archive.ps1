# Release-history policy. Deliberately excludes Rust caches and Git worktrees.
Set-StrictMode -Version 2.0
. (Join-Path $PSScriptRoot 'node-storage-archive.ps1')
. (Join-Path $PSScriptRoot 'node-storage-paths.ps1')

function Read-ElonReleaseArchiveJson {
    param([string]$Path)
    Assert-ElonArchivePath $Path
    return (Get-Content -LiteralPath $Path -Raw -Encoding UTF8 -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop)
}

function Assert-ElonReleaseArchiveIdle {
    param([string]$Path, [string]$Sha)
    foreach ($process in Get-CimInstance Win32_Process -ErrorAction Stop) {
        if ($process.ExecutablePath -and (Test-ElonArchiveWithin $process.ExecutablePath $Path)) {
            throw 'A process is running from this release candidate.'
        }
        if ($process.Name -match '^(powershell|pwsh|cmd|bash)(\.exe)?$' -and
            $process.CommandLine -match '(?i)(node-agent-post-terminal-activator|node-agent-remote-release-worker|publish-node-agent)\.ps1') {
            throw 'A node release writer is running; archive is deferred.'
        }
    }
    $installedPath = Join-Path $env:LOCALAPPDATA 'ElonNode\_internal\node-agent-version.json'
    if (Test-Path -LiteralPath $installedPath) {
        $installed = Read-ElonReleaseArchiveJson $installedPath
        if ($installed.PSObject.Properties.Name -contains 'gitSha' -and [string]$installed.gitSha -eq $Sha) {
            throw 'The installed release is protected.'
        }
    }
}

function Get-ElonReleaseArchiveStates {
    param([string]$Root)
    $releases = Join-Path $Root 'local-node-releases-v1\releases'
    if (-not (Test-Path -LiteralPath $releases)) { return @() }
    $states = @()
    foreach ($dir in Get-ChildItem -LiteralPath $releases -Directory -Force -ErrorAction Stop) {
        if ($dir.Name -notmatch '^[0-9a-f]{40}$') { continue }
        $file = Join-Path $dir.FullName 'state.json'
        if (-not (Test-Path -LiteralPath $file)) { throw "Release state is missing: $($dir.Name)" }
        $state = Read-ElonReleaseArchiveJson $file
        if ($state.schema -ne 'elon.node_local_release.v1' -or $state.git_sha -ne $dir.Name) { throw 'Invalid local release identity.' }
        $states += [pscustomobject]@{ Path = $file; Sha = $dir.Name; VerifiedAtMs = [long]$state.verified_at_ms; State = $state }
    }
    return @($states | Sort-Object VerifiedAtMs -Descending)
}

function Assert-ElonReleaseArchiveCandidate {
    param($Candidate, [string]$Root, [int]$MinAgeDays, [int]$ReleaseKeepNewest, [switch]$CheckSnapshot)
    $source = Get-ElonArchiveFullPath $Candidate.Path
    $sha = Split-Path -Leaf $source
    if ($sha -notmatch '^[0-9a-f]{40}$' -or $Candidate.RemovalMode -ne 'filesystem') { throw 'Only exact release SHA directories can be archived.' }
    $statePath = $null
    if ($Candidate.Kind -eq 'terminal_outbox_event') {
        $allowed = Join-Path $Root 'release-outbox-v1'
        $expected = Join-Path (Join-Path $allowed 'events') $sha
        if ($source -ne (Get-ElonArchiveFullPath $expected)) { throw 'Outbox candidate escaped the events directory.' }
        $statePath = Join-Path $source 'event.json'
        $state = Read-ElonReleaseArchiveJson $statePath
        if ($state.schema -ne 'elon.node_release_outbox.v1' -or $state.git_sha -ne $sha -or $state.sync_state -notin @('synced', 'failed')) {
            throw 'Outbox state is no longer a verified terminal event.'
        }
    } elseif ($Candidate.Kind -eq 'terminal_local_release') {
        $allowed = Join-Path $Root 'local-node-releases-v1'
        $releasePath = Join-Path (Join-Path $allowed 'releases') $sha
        $applyPath = Join-Path (Join-Path $allowed 'apply') $sha
        if ($source -ne (Get-ElonArchiveFullPath $releasePath) -and $source -ne (Get-ElonArchiveFullPath $applyPath)) { throw 'Local release candidate escaped its exact scope.' }
        $statePath = Join-Path $releasePath 'state.json'
        $state = Read-ElonReleaseArchiveJson $statePath
        if ($state.schema -ne 'elon.node_local_release.v1' -or $state.git_sha -ne $sha -or
            $state.activation_state -notin @('superseded', 'failed') -or $state.local_terminal_state -notin @('complete', 'failed')) {
            throw 'Local release is active, pending, or has an unsupported terminal state.'
        }
        $newest = @(Get-ElonReleaseArchiveStates $Root | Select-Object -First $ReleaseKeepNewest)
        if ($sha -in @($newest | ForEach-Object { $_.Sha })) { throw 'Recent local releases are protected.' }
    } else { throw 'This archive workflow accepts release history only.' }
    if ((Get-ElonArchiveFullPath $Candidate.AllowedRoot) -ne (Get-ElonArchiveFullPath $allowed)) { throw 'Candidate allowed root was changed.' }
    Assert-ElonReleaseArchiveIdle -Path $source -Sha $sha
    $inventory = Get-ElonArchiveInventory -Root $source
    if ([DateTime]::Parse($inventory.LatestWriteUtc).ToUniversalTime() -ge [DateTime]::UtcNow.AddDays(-$MinAgeDays)) { throw 'Candidate is inside the retention window.' }
    if ($CheckSnapshot) {
        if ($Candidate.StatePath -ne $statePath -or $Candidate.StateSha256 -ne (Get-FileHash -LiteralPath $statePath -Algorithm SHA256).Hash -or
            [long]$Candidate.EstimatedBytes -ne $inventory.Bytes -or $Candidate.SourceDigest -ne $inventory.SourceDigest) {
            throw 'Reviewed candidate changed; generate and review a fresh plan.'
        }
    }
    return [pscustomobject]@{
        Path = $source; AllowedRoot = (Get-ElonArchiveFullPath $allowed); Kind = $Candidate.Kind; RemovalMode = 'filesystem'
        StatePath = $statePath; StateSha256 = (Get-FileHash -LiteralPath $statePath -Algorithm SHA256).Hash
        EstimatedBytes = $inventory.Bytes; LatestWriteUtc = $inventory.LatestWriteUtc; SourceDigest = $inventory.SourceDigest
    }
}

function New-ElonReleaseArchivePlan {
    param([string]$Root, [string]$ArchiveRoot, [int]$MinAgeDays, [int]$ReleaseKeepNewest, [string[]]$CandidateKind, [string[]]$CandidatePath)
    $proposed = @()
    $events = Join-Path $Root 'release-outbox-v1\events'
    if (Test-Path -LiteralPath $events) {
        foreach ($dir in Get-ChildItem -LiteralPath $events -Directory -Force -ErrorAction Stop) {
            if ($dir.Name -notmatch '^[0-9a-f]{40}$' -or -not (Test-Path -LiteralPath (Join-Path $dir.FullName 'event.json'))) { continue }
            $event = Read-ElonReleaseArchiveJson (Join-Path $dir.FullName 'event.json')
            if ($event.sync_state -in @('synced', 'failed')) {
                $proposed += [pscustomobject]@{ Path = $dir.FullName; AllowedRoot = (Join-Path $Root 'release-outbox-v1'); Kind = 'terminal_outbox_event'; RemovalMode = 'filesystem' }
            }
        }
    }
    $states = @(Get-ElonReleaseArchiveStates $Root)
    foreach ($entry in @($states | Select-Object -Skip $ReleaseKeepNewest)) {
        if ($entry.State.activation_state -notin @('superseded', 'failed') -or $entry.State.local_terminal_state -notin @('complete', 'failed')) { continue }
        $allowed = Join-Path $Root 'local-node-releases-v1'
        foreach ($category in @('apply', 'releases')) {
            $path = Join-Path (Join-Path $allowed $category) $entry.Sha
            if (Test-Path -LiteralPath $path) {
                $proposed += [pscustomobject]@{ Path = $path; AllowedRoot = $allowed; Kind = 'terminal_local_release'; RemovalMode = 'filesystem' }
            }
        }
    }
    if ($CandidateKind.Count) { $proposed = @($proposed | Where-Object { $_.Kind -in $CandidateKind }) }
    $requested = @($CandidatePath | ForEach-Object { Get-ElonArchiveFullPath $_ })
    if ($requested.Count) { $proposed = @($proposed | Where-Object { $_.Path -in $requested }) }
    $candidates = @(); $skipped = @()
    foreach ($candidate in @($proposed | Sort-Object Path -Unique)) {
        try {
            Assert-ElonArchiveScope $candidate.Path $candidate.AllowedRoot $ArchiveRoot
            $candidates += Assert-ElonReleaseArchiveCandidate $candidate $Root $MinAgeDays $ReleaseKeepNewest
        } catch { $skipped += [pscustomobject]@{ Path = $candidate.Path; Reason = $_.Exception.Message } }
    }
    foreach ($path in $requested) {
        if ($path -notin @($candidates | ForEach-Object { $_.Path })) { throw "Requested candidate is not eligible: $path" }
    }
    return [pscustomobject]@{
        Schema = 'elon.release_history_archive_plan.v1'; CreatedUtc = [DateTime]::UtcNow.ToString('o'); MachineKey = Get-ElonArchiveMachineKey
        Root = $Root; ArchiveRoot = (Get-ElonArchiveFullPath $ArchiveRoot); MinAgeDays = $MinAgeDays; ReleaseKeepNewest = $ReleaseKeepNewest
        Candidates = $candidates; Skipped = $skipped; EstimatedBytes = [long](($candidates | Measure-Object EstimatedBytes -Sum).Sum)
    }
}

function Invoke-ElonReleaseHistoryArchive {
    [CmdletBinding(SupportsShouldProcess = $true)]
    param(
        [ValidateSet('Preview', 'ArchiveOnly', 'ArchiveAndReclaim')][string]$Mode = 'Preview',
        [ValidateSet('Legacy', 'Managed')][string]$ReleaseLocation = 'Legacy',
        [string]$ArchiveRoot, [string]$PlanPath, [string]$ExpectedPlanSha256,
        [int]$MinAgeDays = 30, [int]$ReleaseKeepNewest = 3,
        [string[]]$CandidateKind = @(), [string[]]$CandidatePath = @()
    )
    if ([string]::IsNullOrWhiteSpace($ArchiveRoot) -or [string]::IsNullOrWhiteSpace($PlanPath)) { throw 'ArchiveRoot and ArchivePlanPath are required.' }
    $releaseRoot = if ($ReleaseLocation -eq 'Managed') { Get-ElonManagedNodeReleaseStateRoot } else { Join-Path $env:LOCALAPPDATA 'Elon' }
    if ([string]::IsNullOrWhiteSpace($releaseRoot)) { throw 'Managed node data root is unavailable or ownership is invalid.' }
    $root = Get-ElonArchiveFullPath $releaseRoot
    if ($Mode -eq 'Preview') {
        $plan = New-ElonReleaseArchivePlan $root $ArchiveRoot $MinAgeDays $ReleaseKeepNewest $CandidateKind $CandidatePath
        Write-ElonArchiveJson -Path (Get-ElonArchiveFullPath $PlanPath) -Value $plan
        Write-Host ('ARCHIVE_PLAN_SHA256=' + (Get-FileHash -LiteralPath $PlanPath -Algorithm SHA256).Hash)
        return $plan
    }
    if ($ExpectedPlanSha256 -notmatch '^[0-9a-fA-F]{64}$' -or
        (Get-FileHash -LiteralPath $PlanPath -Algorithm SHA256).Hash -ne $ExpectedPlanSha256) { throw 'The exact reviewed plan SHA256 is required.' }
    $plan = Read-ElonReleaseArchiveJson $PlanPath
    if ($plan.Schema -ne 'elon.release_history_archive_plan.v1' -or $plan.MachineKey -ne (Get-ElonArchiveMachineKey) -or
        $plan.Root -ne $root -or $plan.ArchiveRoot -ne (Get-ElonArchiveFullPath $ArchiveRoot)) { throw 'Archive plan identity or destination mismatch.' }
    if ([DateTime]::Parse($plan.CreatedUtc).ToUniversalTime() -lt [DateTime]::UtcNow.AddHours(-24)) { throw 'Archive plan expired; create a fresh preview.' }
    if ($plan.MinAgeDays -lt 7 -or $plan.MinAgeDays -gt 3650 -or $plan.ReleaseKeepNewest -lt 1 -or $plan.ReleaseKeepNewest -gt 20) { throw 'Invalid plan retention policy.' }
    if ($CandidateKind.Count -or $CandidatePath.Count) { throw 'Execution consumes the complete exact plan; filter during preview.' }
    $candidates = @($plan.Candidates | Sort-Object Path)
    if (@($candidates.Path | Sort-Object -Unique).Count -ne $candidates.Count) { throw 'Duplicate archive candidates.' }
    # Validate the entire selection before archiving the first candidate.
    foreach ($candidate in $candidates) {
        Assert-ElonArchiveScope $candidate.Path $candidate.AllowedRoot $ArchiveRoot
        Assert-ElonReleaseArchiveCandidate $candidate $root $plan.MinAgeDays $plan.ReleaseKeepNewest -CheckSnapshot | Out-Null
    }
    $results = @()
    foreach ($candidate in $candidates) {
        if (-not $PSCmdlet.ShouldProcess($candidate.Path, $Mode)) { continue }
        $validate = {
            param($path)
            if ($path -ne $candidate.Path) { throw 'Unexpected source passed to policy.' }
            Assert-ElonReleaseArchiveCandidate $candidate $root $plan.MinAgeDays $plan.ReleaseKeepNewest -CheckSnapshot | Out-Null
        }
        $remove = {
            param($path)
            Assert-ElonReleaseArchiveCandidate $candidate $root $plan.MinAgeDays $plan.ReleaseKeepNewest -CheckSnapshot | Out-Null
            # This is the same bounded filesystem removal mode as managed disk inspection.
            Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction Stop
        }
        $results += Invoke-ElonVerifiedTreeArchive -Path $candidate.Path -AllowedRoot $candidate.AllowedRoot -ArchiveRoot $ArchiveRoot -Mode $Mode -ValidateSource $validate -RemoveSource $remove
    }
    return $results
}
