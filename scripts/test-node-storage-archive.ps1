$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0
. (Join-Path $PSScriptRoot 'node-storage-release-archive.ps1')

$script:Assertions = 0
# Code points avoid PS5 interpreting this script's BOM-less source as ANSI.
$script:ArchiveTestUnicode = -join @([char]0x4E2D, [char]0x6587)
function Assert-ArchiveTest {
    param([bool]$Condition, [string]$Message)
    $script:Assertions++
    if (-not $Condition) { throw "ASSERT FAILED: $Message" }
}
function Assert-ArchiveThrows {
    param([scriptblock]$Action, [string]$Pattern)
    $thrown = $false
    try { & $Action | Out-Null } catch {
        $thrown = $true
        Assert-ArchiveTest ($_.Exception.Message -match $Pattern) "unexpected failure: $($_.Exception.Message)"
    }
    Assert-ArchiveTest $thrown "expected rejection: $Pattern"
}
function New-ArchiveTestTree {
    param([string]$Path)
    New-Item -ItemType Directory -Path (Join-Path $Path 'nested\empty') -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $Path 'nested\data.txt'), 'preserve exact bytes and empty directories')
    [IO.File]::WriteAllText((Join-Path $Path ($script:ArchiveTestUnicode + '.txt')), $script:ArchiveTestUnicode)
    return $Path
}
function Set-ArchiveTestOld {
    param([string]$Path)
    foreach ($item in @(Get-ChildItem -LiteralPath $Path -Recurse -Force) + @(Get-Item -LiteralPath $Path)) {
        $item.LastWriteTimeUtc = [DateTime]::UtcNow.AddDays(-40)
    }
}

$fixture = Join-Path (Join-Path $PSScriptRoot '..\.ai-tmp') ('storage-archive-test-' + $script:ArchiveTestUnicode + '-' + [Guid]::NewGuid().ToString('N'))
$fixture = [IO.Path]::GetFullPath($fixture)
New-Item -ItemType Directory -Path $fixture -Force | Out-Null
$savedLocal = $env:LOCALAPPDATA
$savedNodeRoot = $env:ELON_NODE_DATA_ROOT
$copyFunction = ${function:Copy-ElonArchiveFile}
$robocopyFunction = ${function:Invoke-ElonArchiveRobocopy}
$idleFunction = ${function:Assert-ElonReleaseArchiveIdle}
try {
    $allowed = Join-Path $fixture 'owned'
    $archive = Join-Path $fixture 'archive'
    $source = New-ArchiveTestTree (Join-Path $allowed 'one')
    $remove = { param($path) Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction Stop }
    $validate = { param($path) if (-not (Test-Path -LiteralPath $path)) { throw 'missing test source' } }
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot (Join-Path $fixture 'wrong') -ArchiveRoot $archive } 'outside'
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $allowed -AllowedRoot $allowed -ArchiveRoot $archive } 'equals'
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot $source } 'overlapping'
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot (Join-Path $source 'archive') } 'overlapping'
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot $archive -Mode ArchiveAndReclaim } 'requires'
    $gitSource = New-ArchiveTestTree (Join-Path $allowed 'git-source')
    [IO.File]::WriteAllText((Join-Path $gitSource '.git'), 'gitdir: protected')
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $gitSource -AllowedRoot $allowed -ArchiveRoot $archive } 'Git repositories'

    $outside = New-ArchiveTestTree (Join-Path $fixture 'outside')
    $junction = Join-Path $source 'junction'
    New-Item -ItemType Junction -Path $junction -Target $outside -ErrorAction Stop | Out-Null
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot $archive } 'reparse'
    [IO.Directory]::Delete($junction)
    $archiveLink = Join-Path $fixture 'archive-link'
    New-Item -ItemType Junction -Path $archiveLink -Target $outside -ErrorAction Stop | Out-Null
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot $archiveLink } 'reparse'
    [IO.Directory]::Delete($archiveLink)

    # Model an unavailable share at the actual copy boundary; never reach removal.
    ${function:Copy-ElonArchiveFile} = { param($Source, $Destination) throw 'simulated network copy failure' }
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot $archive -Mode ArchiveAndReclaim -ValidateSource $validate -RemoveSource $remove } 'network copy failure'
    Assert-ArchiveTest (Test-Path -LiteralPath (Join-Path $source 'nested\data.txt')) 'copy failure must preserve source'
    ${function:Copy-ElonArchiveFile} = $copyFunction

    # Modify source between initial inventory and final comparison after a successful copy.
    $script:archiveValidationCalls = 0
    $drift = {
        param($path)
        $script:archiveValidationCalls++
        if ($script:archiveValidationCalls -eq 2) { [IO.File]::AppendAllText((Join-Path $path 'nested\data.txt'), 'changed') }
    }
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot $archive -Mode ArchiveAndReclaim -ValidateSource $drift -RemoveSource $remove } 'drifted'
    Assert-ArchiveTest (Test-Path -LiteralPath $source) 'source drift must preserve source'

    # A delayed NTFS parent-directory mtime update must not masquerade as byte drift.
    $directoryTimeOnly = {
        param($path)
        $directory = Get-Item -LiteralPath (Join-Path $path 'nested')
        $directory.LastWriteTimeUtc = $directory.LastWriteTimeUtc.AddMilliseconds(1)
    }
    $directoryReceipt = Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot $archive -Mode ArchiveOnly -ValidateSource $directoryTimeOnly
    Assert-ArchiveTest ((Test-Path -LiteralPath $directoryReceipt.ReceiptPath) -and (Test-Path -LiteralPath $source)) 'directory mtime settling is harmless when all content and membership match'
    $script:archiveValidationCalls = 0
    $newDirectoryDrift = {
        param($path)
        $script:archiveValidationCalls++
        if ($script:archiveValidationCalls -eq 2) { New-Item -ItemType Directory -Path (Join-Path $path 'added-empty-directory') | Out-Null }
    }
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot $archive -Mode ArchiveAndReclaim -ValidateSource $newDirectoryDrift -RemoveSource $remove } 'drifted'
    Assert-ArchiveTest (Test-Path -LiteralPath $source) 'new empty directory membership must still prevent reclaim'

    $expected = [IO.File]::ReadAllText((Join-Path $source 'nested\data.txt'))
    $result = Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot $archive -Mode ArchiveOnly
    Assert-ArchiveTest ((Test-Path -LiteralPath $source) -and -not $result.Reclaimed) 'ArchiveOnly must preserve source'
    Assert-ArchiveTest (Test-Path -LiteralPath $result.ReceiptPath) 'verified receipt must exist'
    $restored = Join-Path $allowed 'restored'
    Restore-ElonVerifiedTreeArchive -ArchiveDirectory $result.ArchiveDirectory -Path $restored -AllowedRoot $allowed | Out-Null
    Assert-ArchiveTest ([IO.File]::ReadAllText((Join-Path $restored 'nested\data.txt')) -eq $expected) 'restored bytes must match'
    Assert-ArchiveTest (Test-Path -LiteralPath (Join-Path $restored 'nested\empty')) 'restore preserves empty directories'
    Assert-ArchiveTest ([IO.File]::ReadAllText((Join-Path $restored ($script:ArchiveTestUnicode + '.txt'))) -eq $script:ArchiveTestUnicode) 'UTF-8 manifest and receipt round-trip non-ASCII paths, names, and bytes'
    Assert-ArchiveThrows { Restore-ElonVerifiedTreeArchive -ArchiveDirectory $result.ArchiveDirectory -Path $restored -AllowedRoot $allowed } 'overwrite'
    [IO.File]::AppendAllText((Join-Path $result.ArchiveDirectory 'payload\nested\data.txt'), 'corrupt')
    Assert-ArchiveThrows { Restore-ElonVerifiedTreeArchive -ArchiveDirectory $result.ArchiveDirectory -Path (Join-Path $allowed 'corrupt-restore') -AllowedRoot $allowed } 'corrupt'
    $reclaimed = Invoke-ElonVerifiedTreeArchive -Path $source -AllowedRoot $allowed -ArchiveRoot $archive -Mode ArchiveAndReclaim -ValidateSource $validate -RemoveSource $remove
    Assert-ArchiveTest ($reclaimed.Reclaimed -and -not (Test-Path -LiteralPath $source)) 'verified reclaim removes only source'
    Restore-ElonVerifiedTreeArchive -ArchiveDirectory $reclaimed.ArchiveDirectory -Path $source -AllowedRoot $allowed | Out-Null
    Assert-ArchiveTest ([IO.File]::ReadAllText((Join-Path $source 'nested\data.txt')) -eq $expected) 'reclaimed source remains recoverable'

    # The parallel engine must retain the same verification and preservation gates.
    $parallelSource = New-ArchiveTestTree (Join-Path $allowed 'parallel-source')
    $parallel = Invoke-ElonVerifiedTreeArchive -Path $parallelSource -AllowedRoot $allowed -ArchiveRoot $archive -CopyEngine Robocopy
    $parallelRestore = Join-Path $allowed 'parallel-restore'
    Restore-ElonVerifiedTreeArchive -ArchiveDirectory $parallel.ArchiveDirectory -Path $parallelRestore -AllowedRoot $allowed -CopyEngine Robocopy | Out-Null
    Assert-ArchiveTest ((Get-ElonArchiveInventory $parallelSource -HashFiles).ContentDigest -eq (Get-ElonArchiveInventory $parallelRestore -HashFiles).ContentDigest) 'Robocopy archive and restore preserve bytes and empty directories'
    ${function:Invoke-ElonArchiveRobocopy} = { param($Source, $Destination) return 8 }
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $parallelSource -AllowedRoot $allowed -ArchiveRoot $archive -CopyEngine Robocopy -Mode ArchiveAndReclaim -ValidateSource $validate -RemoveSource $remove } 'Robocopy copy failed'
    Assert-ArchiveTest (Test-Path -LiteralPath $parallelSource) 'Robocopy failure must preserve source'
    ${function:Invoke-ElonArchiveRobocopy} = {
        param($Source, $Destination)
        $code = & $robocopyFunction $Source $Destination
        [IO.File]::AppendAllText((Join-Path $Destination 'nested\data.txt'), 'corruption')
        return $code
    }
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $parallelSource -AllowedRoot $allowed -ArchiveRoot $archive -CopyEngine Robocopy -Mode ArchiveAndReclaim -ValidateSource $validate -RemoveSource $remove } 'verification failed'
    Assert-ArchiveTest (Test-Path -LiteralPath $parallelSource) 'Robocopy payload corruption must preserve source'
    ${function:Invoke-ElonArchiveRobocopy} = {
        param($Source, $Destination)
        $code = & $robocopyFunction $Source $Destination
        $script:archiveInjectedLink = Join-Path $Destination 'payload-junction'
        New-Item -ItemType Junction -Path $script:archiveInjectedLink -Target $outside -ErrorAction Stop | Out-Null
        return $code
    }
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $parallelSource -AllowedRoot $allowed -ArchiveRoot $archive -CopyEngine Robocopy -Mode ArchiveAndReclaim -ValidateSource $validate -RemoveSource $remove } 'reparse'
    Assert-ArchiveTest (Test-Path -LiteralPath $parallelSource) 'Robocopy post-copy reparse rejection must preserve source'
    [IO.Directory]::Delete($script:archiveInjectedLink)
    ${function:Invoke-ElonArchiveRobocopy} = $robocopyFunction
    $parallelJunction = Join-Path $parallelSource 'junction'
    New-Item -ItemType Junction -Path $parallelJunction -Target $outside -ErrorAction Stop | Out-Null
    Assert-ArchiveThrows { Invoke-ElonVerifiedTreeArchive -Path $parallelSource -AllowedRoot $allowed -ArchiveRoot $archive -CopyEngine Robocopy } 'reparse'
    [IO.Directory]::Delete($parallelJunction)
    Assert-ArchiveTest (Test-Path -LiteralPath $parallelSource) 'Robocopy source reparse rejection must preserve source'

    # Exercise the public entrypoint with isolated release metadata and a tempting Rust root.
    $env:LOCALAPPDATA = Join-Path $fixture 'local'
    $elon = Join-Path $env:LOCALAPPDATA 'Elon'
    $sha = 'a' * 40
    $event = New-ArchiveTestTree (Join-Path $elon "release-outbox-v1\events\$sha")
    Write-ElonArchiveJson -Path (Join-Path $event 'event.json') -Value @{ schema = 'elon.node_release_outbox.v1'; git_sha = $sha; sync_state = 'synced' }
    Set-ArchiveTestOld $event
    $rust = New-ArchiveTestTree (Join-Path $elon 'rust-cache-v2')
    Set-ArchiveTestOld $rust
    $planPath = Join-Path $fixture ($script:ArchiveTestUnicode + '-release-plan.json')
    $inspect = Join-Path $PSScriptRoot 'inspect-node-disk-usage.ps1'
    ${function:Assert-ElonReleaseArchiveIdle} = { param($Path, $Sha) }
    $plan = & $inspect -ReleaseHistoryOnly -ArchiveRoot $archive -ArchivePlanPath $planPath -MinAgeDays 7
    Assert-ArchiveTest ($plan.Candidates.Count -eq 1 -and $plan.Candidates[0].Path -eq $event) 'release-only preview excludes all Rust roots'
    $digest = (Get-FileHash -LiteralPath $planPath -Algorithm SHA256).Hash
    Assert-ArchiveThrows { & $inspect -ReleaseHistoryOnly -ArchiveMode ArchiveOnly -ArchiveRoot $archive -ArchivePlanPath $planPath -ExpectedPlanSha256 ('0' * 64) } 'reviewed plan'
    Assert-ArchiveThrows { & $inspect -ReleaseHistoryOnly -Apply -ArchiveRoot $archive -ArchivePlanPath $planPath } 'cannot combine'
    [IO.File]::AppendAllText((Join-Path $event 'nested\data.txt'), 'new data')
    Assert-ArchiveThrows { & $inspect -ReleaseHistoryOnly -ArchiveMode ArchiveAndReclaim -ArchiveRoot $archive -ArchivePlanPath $planPath -ExpectedPlanSha256 $digest } 'retention|changed'
    Assert-ArchiveTest (Test-Path -LiteralPath $event) 'changed planned source is preserved'
    Set-ArchiveTestOld $event
    $planPath2 = Join-Path $fixture 'release-plan-2.json'
    & $inspect -ReleaseHistoryOnly -ArchiveRoot $archive -ArchivePlanPath $planPath2 -MinAgeDays 7 | Out-Null
    $digest2 = (Get-FileHash -LiteralPath $planPath2 -Algorithm SHA256).Hash
    $completed = @(& $inspect -ReleaseHistoryOnly -ArchiveMode ArchiveAndReclaim -ArchiveRoot $archive -ArchivePlanPath $planPath2 -ExpectedPlanSha256 $digest2)
    Assert-ArchiveTest ($completed.Count -eq 1 -and $completed[0].Reclaimed) 'entrypoint archives and reclaims exact reviewed candidate'
    Assert-ArchiveTest (Test-Path -LiteralPath $rust) 'entrypoint leaves Rust managed root intact'

    # Release policy must reject pending/unknown history and protect recent versions.
    $protectedEvent = New-ArchiveTestTree (Join-Path $elon ('release-outbox-v1\events\' + ('b' * 40)))
    $protectedState = Join-Path $protectedEvent 'event.json'
    $candidate = [pscustomobject]@{ Path = $protectedEvent; AllowedRoot = (Join-Path $elon 'release-outbox-v1'); Kind = 'terminal_outbox_event'; RemovalMode = 'filesystem' }
    foreach ($sync in @('pending', 'superseded', 'unknown')) {
        [IO.File]::WriteAllText($protectedState, (@{ schema = 'elon.node_release_outbox.v1'; git_sha = ('b' * 40); sync_state = $sync } | ConvertTo-Json))
        Set-ArchiveTestOld $protectedEvent
        Assert-ArchiveThrows { Assert-ElonReleaseArchiveCandidate $candidate $elon 7 1 } 'terminal event'
    }
    $releaseSha = 'c' * 40
    $release = New-ArchiveTestTree (Join-Path $elon "local-node-releases-v1\releases\$releaseSha")
    $releaseState = Join-Path $release 'state.json'
    $releaseCandidate = [pscustomobject]@{ Path = $release; AllowedRoot = (Join-Path $elon 'local-node-releases-v1'); Kind = 'terminal_local_release'; RemovalMode = 'filesystem' }
    foreach ($pair in @(@('activated', 'complete'), @('superseded', 'pending'), @('failed', 'pending'))) {
        [IO.File]::WriteAllText($releaseState, (@{ schema = 'elon.node_local_release.v1'; git_sha = $releaseSha; activation_state = $pair[0]; local_terminal_state = $pair[1]; verified_at_ms = 100 } | ConvertTo-Json))
        Set-ArchiveTestOld $release
        Assert-ArchiveThrows { Assert-ElonReleaseArchiveCandidate $releaseCandidate $elon 7 1 } 'active, pending'
    }
    [IO.File]::WriteAllText($releaseState, (@{ schema = 'elon.node_local_release.v1'; git_sha = $releaseSha; activation_state = 'failed'; local_terminal_state = 'failed'; verified_at_ms = 100 } | ConvertTo-Json))
    Set-ArchiveTestOld $release
    Assert-ArchiveThrows { Assert-ElonReleaseArchiveCandidate $releaseCandidate $elon 7 1 } 'Recent local releases'
    ${function:Assert-ElonReleaseArchiveIdle} = $idleFunction
    $installedFile = Join-Path $env:LOCALAPPDATA 'ElonNode\_internal\node-agent-version.json'
    New-Item -ItemType Directory -Path (Split-Path $installedFile -Parent) -Force | Out-Null
    [IO.File]::WriteAllText($installedFile, (@{ gitSha = $releaseSha } | ConvertTo-Json))
    Assert-ArchiveThrows { Assert-ElonReleaseArchiveIdle -Path $release -Sha $releaseSha } 'installed release'

    $env:ELON_NODE_DATA_ROOT = Join-Path $fixture 'managed-node'
    New-Item -ItemType Directory -Path $env:ELON_NODE_DATA_ROOT -Force | Out-Null
    Write-ElonArchiveJson -Path (Join-Path $env:ELON_NODE_DATA_ROOT '.elon-node-data-root.json') -Value @{ install_id = 'archive-test-owner' }
    $managedRoot = Join-Path $env:ELON_NODE_DATA_ROOT 'release-state'
    $managedEvent = New-ArchiveTestTree (Join-Path $managedRoot ('release-outbox-v1\events\' + ('d' * 40)))
    Write-ElonArchiveJson -Path (Join-Path $managedEvent 'event.json') -Value @{ schema = 'elon.node_release_outbox.v1'; git_sha = ('d' * 40); sync_state = 'synced' }
    Set-ArchiveTestOld $managedEvent
    $managedPlanPath = Join-Path $fixture 'managed-plan.json'
    $managedPlan = & $inspect -ReleaseHistoryOnly -ReleaseLocation Managed -ArchiveRoot $archive -ArchivePlanPath $managedPlanPath -MinAgeDays 7
    Assert-ArchiveTest ($managedPlan.Root -eq $managedRoot -and $managedPlan.Candidates.Count -eq 1) 'managed preview resolves only the owned release-state root'
    $managedDigest = (Get-FileHash -LiteralPath $managedPlanPath -Algorithm SHA256).Hash
    Assert-ArchiveThrows { & $inspect -ReleaseHistoryOnly -ArchiveMode ArchiveOnly -ArchiveRoot $archive -ArchivePlanPath $managedPlanPath -ExpectedPlanSha256 $managedDigest } 'identity or destination'
    $managedCopy = @(& $inspect -ReleaseHistoryOnly -ReleaseLocation Managed -ArchiveMode ArchiveOnly -ArchiveRoot $archive -ArchivePlanPath $managedPlanPath -ExpectedPlanSha256 $managedDigest)
    Assert-ArchiveTest ($managedCopy.Count -eq 1 -and -not $managedCopy[0].Reclaimed -and (Test-Path -LiteralPath $managedEvent)) 'managed archive uses its own approved plan and preserves source'
    Write-Host "PASS: verified storage archive ($script:Assertions assertions)"
} finally {
    $env:LOCALAPPDATA = $savedLocal
    $env:ELON_NODE_DATA_ROOT = $savedNodeRoot
    ${function:Copy-ElonArchiveFile} = $copyFunction
    ${function:Invoke-ElonArchiveRobocopy} = $robocopyFunction
    ${function:Assert-ElonReleaseArchiveIdle} = $idleFunction
    if ((Get-ElonArchiveFullPath $fixture).StartsWith((Get-ElonArchiveFullPath (Join-Path $PSScriptRoot '..\.ai-tmp')) + '\', [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction Stop
    }
}
