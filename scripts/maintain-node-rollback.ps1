param(
    [ValidateSet('Preview', 'Apply')][string]$Mode = 'Preview',
    [string]$PlanPath, [string]$ExpectedPlanSha256,
    [string]$ArchiveRoot, [string]$PrivateRoot,
    [ValidateRange(2, 20)][int]$KeepNewest = 2,
    [ValidateRange(30, 3650)][int]$MinAgeDays = 30
)

# A reviewed, local maintenance operation; never runs automatically on activation.
Set-StrictMode -Version 2.0
. (Join-Path $PSScriptRoot 'node-storage-rollback-policy.ps1')
. (Join-Path $PSScriptRoot 'node-storage-rollback-archive.ps1')

function Assert-ElonRollbackRuntimeUnchanged {
    param($Expected, [string[]]$Roots)
    Assert-ElonRollbackIdle -Roots $Roots -ExpectedIdentity $Expected
    $actual = Get-ElonRollbackInstalledIdentity
    foreach ($key in @('Identity', 'ProcessId', 'ExecutablePath', 'MetadataSha256')) {
        if ([string]$actual.$key -ne [string]$Expected.$key) { throw "Installed runtime changed: $key" }
    }
    if ((Get-ElonRollbackUtcTicks $actual.ProcessStartUtc) -ne (Get-ElonRollbackUtcTicks $Expected.ProcessStartUtc)) {
        throw 'Installed runtime changed: ProcessStartUtc'
    }
}

function Assert-ElonRollbackSelectionUnchanged {
    param($Reviewed, [string[]]$Removed = @())
    $fresh = Get-ElonRollbackPlan -Roots $Reviewed.Policy.Roots -CurrentIdentity $Reviewed.Installed.Identity `
        -KeepNewest $Reviewed.Policy.KeepNewest -MinAgeDays $Reviewed.Policy.MinAgeDays
    if ($fresh.RootsFingerprint -ne $Reviewed.Policy.RootsFingerprint) { throw 'Rollback roots changed.' }
    $expected = @($Reviewed.Policy.Candidates | Where-Object { $_.Path -notin $Removed } | Sort-Object Path)
    $actual = @($fresh.Candidates | Sort-Object Path)
    if ((Get-ElonArchiveDigest ([ordered]@{Items=$expected})) -ne (Get-ElonArchiveDigest ([ordered]@{Items=$actual}))) {
        throw 'Rollback candidates or their release evidence changed; create a fresh plan.'
    }
    return $fresh
}

function Assert-ElonRollbackEvidenceUnchanged {
    param($Reviewed, $Candidate, [string[]]$Removed = @())
    $roots = @(Get-ElonRollbackRoots | Sort-Object)
    if ((Get-ElonArchiveDigest ([ordered]@{Items=$roots})) -ne
        (Get-ElonArchiveDigest ([ordered]@{Items=@($Reviewed.Policy.Roots | Sort-Object)}))) { throw 'Owned rollback roots changed.' }
    $expected = @($Reviewed.EvidenceCatalog | Where-Object { $_.Kind -ne 'snapshot_evidence' -or $_.Path -notin $Removed } | Sort-Object Kind,Path)
    $actual = @(Get-ElonRollbackEvidenceCatalog -Roots $roots | Sort-Object Kind,Path)
    if ((Get-ElonArchiveDigest ([ordered]@{Items=$expected})) -ne (Get-ElonArchiveDigest ([ordered]@{Items=$actual}))) {
        throw 'Rollback release evidence or snapshot catalog changed; source preserved.'
    }
    $inventory = Get-ElonArchiveInventory -Root $Candidate.Path
    if ($inventory.SourceDigest -ne $Candidate.SourceDigest -or [long]$inventory.Bytes -ne [long]$Candidate.EstimatedBytes) {
        throw 'Reviewed rollback snapshot metadata changed; source preserved.'
    }
}

function Invoke-ElonRollbackMaintenance {
    [CmdletBinding()]
    param(
        [ValidateSet('Preview', 'Apply')][string]$Mode = 'Preview',
        [Parameter(Mandatory = $true)][string]$PlanPath,
        [string]$ExpectedPlanSha256, [string]$ArchiveRoot, [string]$PrivateRoot,
        [ValidateRange(2, 20)][int]$KeepNewest = 2,
        [ValidateRange(30, 3650)][int]$MinAgeDays = 30
    )
    $ErrorActionPreference = 'Stop'
    $planFile = Get-ElonArchiveFullPath $PlanPath
    Assert-ElonArchivePath $planFile
    $roots = @(Get-ElonRollbackRoots)
    if (-not $roots.Count) { throw 'No owned local rollback roots are available.' }
    if ($Mode -eq 'Preview') {
        if (-not $ArchiveRoot -or -not $PrivateRoot) { throw 'ArchiveRoot and PrivateRoot are required.' }
        $identity = Get-ElonRollbackInstalledIdentity
        Assert-ElonRollbackIdle -Roots $roots -ExpectedIdentity $identity
        $policy = Get-ElonRollbackPlan -Roots $roots -CurrentIdentity $identity.Identity -KeepNewest $KeepNewest -MinAgeDays $MinAgeDays
        $plan = [pscustomobject]@{
            Schema = 'elon.rollback_maintenance_plan.v1'; CreatedUtc = [DateTime]::UtcNow.ToString('o')
            MachineKey = Get-ElonArchiveMachineKey; Installed = $identity; Policy = $policy
            PowerShellMajor = $PSVersionTable.PSVersion.Major
            EvidenceCatalog = @(Get-ElonRollbackEvidenceCatalog -Roots $roots)
            ArchiveRoot = Get-ElonArchiveFullPath $ArchiveRoot; PrivateRoot = Get-ElonArchiveFullPath $PrivateRoot
        }
        Write-ElonArchiveJson -Path $planFile -Value $plan
        Write-Host ('ROLLBACK_PLAN_SHA256=' + (Get-FileHash -LiteralPath $planFile -Algorithm SHA256).Hash)
        return $plan
    }

    $planBytes = [IO.File]::ReadAllBytes($planFile)
    $hasher = [Security.Cryptography.SHA256]::Create()
    try { $planHash = ([BitConverter]::ToString($hasher.ComputeHash($planBytes))).Replace('-', '') }
    finally { $hasher.Dispose() }
    if ($ExpectedPlanSha256 -notmatch '^[0-9a-fA-F]{64}$' -or $planHash -ne $ExpectedPlanSha256) {
        throw 'The exact reviewed rollback plan SHA256 is required.'
    }
    $plan = [Text.Encoding]::UTF8.GetString($planBytes) | ConvertFrom-Json
    if ($plan.Schema -ne 'elon.rollback_maintenance_plan.v1' -or $plan.MachineKey -ne (Get-ElonArchiveMachineKey)) { throw 'Rollback plan identity mismatch.' }
    if ($plan.PowerShellMajor -ne $PSVersionTable.PSVersion.Major) { throw 'Execute the plan with the same PowerShell major version used to create it.' }
    $created = [DateTime]::Parse($plan.CreatedUtc).ToUniversalTime()
    if ($created -lt [DateTime]::UtcNow.AddHours(-24) -or $created -gt [DateTime]::UtcNow.AddMinutes(1)) { throw 'Rollback plan expired or has an invalid creation time.' }
    if ($plan.Policy.KeepNewest -lt 2 -or $plan.Policy.KeepNewest -gt 20 -or $plan.Policy.MinAgeDays -lt 30 -or $plan.Policy.MinAgeDays -gt 3650) { throw 'Invalid retention policy.' }
    if ((Get-ElonArchiveDigest ([ordered]@{Items=@($roots | Sort-Object)})) -ne (Get-ElonArchiveDigest ([ordered]@{Items=@($plan.Policy.Roots | Sort-Object)}))) { throw 'Owned rollback root set changed.' }
    if (($ArchiveRoot -and (Get-ElonArchiveFullPath $ArchiveRoot) -ne $plan.ArchiveRoot) -or
        ($PrivateRoot -and (Get-ElonArchiveFullPath $PrivateRoot) -ne $plan.PrivateRoot)) { throw 'Archive destination differs from the reviewed plan.' }
    $locks = @(); $removed = @(); $results = @()
    $operation = Join-Path (Split-Path $planFile -Parent) ('rollback-operation-' + [Guid]::NewGuid().ToString('N'))
    try {
        $locks = @(Enter-ElonRollbackMaintenanceLocks -Roots $roots)
        Assert-ElonRollbackRuntimeUnchanged -Expected $plan.Installed -Roots $roots
        Assert-ElonRollbackSelectionUnchanged $plan | Out-Null
        New-Item -ItemType Directory -Path $operation -ErrorAction Stop | Out-Null
        foreach ($candidate in @($plan.Policy.Candidates | Sort-Object Path)) {
            Assert-ElonRollbackRuntimeUnchanged -Expected $plan.Installed -Roots $roots
            Assert-ElonRollbackEvidenceUnchanged $plan $candidate $removed
            Write-Host "ROLLBACK_PHASE=archive SOURCE=$($candidate.Path)"
            $archive = Invoke-ElonRollbackSplitArchive -Path $candidate.Path -AllowedRoot $candidate.AllowedRoot `
                -ArchiveRoot $plan.ArchiveRoot -PrivateRoot $plan.PrivateRoot
            if ($archive.Source -ne $candidate.Path -or $archive.OriginalManifestSha256 -ne $candidate.ManifestSha256 -or
                [long]$archive.SourceBytes -ne [long]$candidate.EstimatedBytes) { throw 'Verified split archive differs from the approved candidate.' }
            Assert-ElonRollbackRuntimeUnchanged -Expected $plan.Installed -Roots $roots
            Assert-ElonRollbackEvidenceUnchanged $plan $candidate $removed
            # Reverify the original snapshot after copying, immediately before bounded removal.
            $finalSource = Assert-ElonRollbackSnapshotTree $candidate.Path
            if ($finalSource.Verified.PriorReleaseIdentity -ne $candidate.PriorIdentity -or
                $finalSource.Verified.ManifestSha256 -ne $archive.OriginalManifestSha256 -or
                (Get-ElonRollbackContentDigest $finalSource.Inventory) -ne $archive.OriginalContentDigest -or
                [long]$finalSource.Inventory.Bytes -ne [long]$archive.SourceBytes) { throw 'Source snapshot changed before removal.' }
            Assert-ElonArchiveScope -Path $candidate.Path -AllowedRoot $candidate.AllowedRoot -ArchiveRoot $plan.ArchiveRoot
            if ((Split-Path -Leaf $candidate.Path) -ne $candidate.SnapshotName -or
                (Split-Path -Parent $candidate.Path) -ne $candidate.AllowedRoot) { throw 'Snapshot deletion escaped its exact rollback directory.' }
            $proof = Join-Path $operation ($candidate.SnapshotName + '-' + [Guid]::NewGuid().ToString('N') + '.verified.json')
            Write-ElonArchiveJson -Path $proof -Value $archive
            Remove-Item -LiteralPath $candidate.Path -Recurse -Force -ErrorAction Stop
            if (Test-Path -LiteralPath $candidate.Path) { throw 'Snapshot removal did not complete.' }
            $removed += $candidate.Path
            $result = [pscustomobject]@{
                Source = $candidate.Path; Deleted = $true; SourceBytes = [long]$archive.SourceBytes
                SharedBytes = [long]$archive.SharedBytes; RetainedPrivateBytes = [long]$archive.PrivateBytes
                ArchiveDirectory = $archive.ArchiveDirectory; PrivateDirectory = $archive.PrivateDirectory
                RestoreReceipt = $archive.ReceiptPath; FinishedUtc = [DateTime]::UtcNow.ToString('o')
            }
            Write-ElonArchiveJson -Path ($proof -replace '\.verified\.json$', '.reclaimed.json') -Value $result
            $results += $result
            Write-Host "ROLLBACK_PHASE=reclaimed COUNT=$($results.Count) SOURCE=$($candidate.Path)"
        }
        $summary = [pscustomobject]@{Schema='elon.rollback_maintenance_result.v1';Mode='apply';OperationDirectory=$operation;PlanPath=$planFile;PlanSha256=$ExpectedPlanSha256;DeletedCount=$results.Count;Results=$results}
        Write-ElonArchiveJson -Path (Join-Path $operation 'result.json') -Value $summary
        return $summary
    } finally {
        foreach ($handle in $locks) { if ($null -ne $handle) { $handle.Dispose() } }
    }
}

if ($MyInvocation.InvocationName -ne '.') { Invoke-ElonRollbackMaintenance @PSBoundParameters }
