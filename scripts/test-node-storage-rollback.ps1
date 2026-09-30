$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0
. (Join-Path $PSScriptRoot 'maintain-node-rollback.ps1')

$script:RollbackAssertions = 0
$script:RollbackUnicode = -join @([char]0x4E2D,[char]0x6587)
$script:RollbackClient = -join @([char]0x4E00,[char]0x9F99,[char]0x5F00,[char]0x53D1,[char]0x5E73,[char]0x53F0) + '.exe'
$script:RollbackUninstaller = -join @([char]0x5378,[char]0x8F7D,[char]0x4E00,[char]0x9F99,[char]0x5F00,[char]0x53D1,[char]0x5E73,[char]0x53F0) + '.exe'
$script:RollbackPrivateMarker = 'FIXTURE_PRIVATE_CONFIGURATION_MUST_STAY_LOCAL'
function Assert-RollbackTest([bool]$Condition, [string]$Message) {
    $script:RollbackAssertions++
    if (-not $Condition) { throw "ASSERT FAILED: $Message" }
}
function Assert-RollbackThrows([scriptblock]$Action, [string]$Pattern) {
    $thrown = $false
    try { & $Action | Out-Null } catch {
        $thrown = $true
        Assert-RollbackTest ($_.Exception.Message -match $Pattern) "unexpected failure: $($_.Exception.Message)"
    }
    Assert-RollbackTest $thrown "expected rejection: $Pattern"
}
function Write-RollbackTestJson([string]$Path, $Value) {
    [IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 15), (New-Object Text.UTF8Encoding($false)))
}
function Read-RollbackTestJson([string]$Path) {
    return (Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json)
}
function Set-RollbackTestOld([string]$Path, [int]$Age = 60) {
    foreach ($item in @(Get-ChildItem -LiteralPath $Path -Recurse -Force) + @(Get-Item -LiteralPath $Path)) {
        $item.LastWriteTimeUtc = [datetime]::UtcNow.AddDays(-$Age)
    }
}
function New-RollbackTestSnapshot {
    param([string]$Root, [int]$Id, [int]$Age = 60, [string]$PriorSha = '', [string]$TargetSha = '')
    if (-not $PriorSha) { $PriorSha = ($Id + 1000).ToString('x40') }
    if (-not $TargetSha) { $TargetSha = ($Id + 2000).ToString('x40') }
    $when = [DateTimeOffset]::UtcNow.AddDays(-$Age)
    $name = $when.UtcDateTime.ToString('yyyyMMddHHmmssfff') + '-' + $Id.ToString('x8')
    $path = Join-Path (Join-Path $Root 'rollback') $name
    $payload = Join-Path $path 'ElonNode'
    New-Item -ItemType Directory -Path (Join-Path $payload '_internal\pc-next-dist') -Force | Out-Null
    $content = [ordered]@{}
    $content[$script:RollbackClient] = 'MZ fake client fixture, never executed'
    $content[$script:RollbackUninstaller] = 'MZ fake uninstaller fixture, never executed'
    $content['_internal\elon-desktop.exe'] = 'MZ fake desktop fixture, never executed'
    $content['_internal\node-agent-version.json'] = '{"version":"0.3.69","gitSha":"' + $PriorSha + '"}'
    $content['_internal\node-agent.env'] = $script:RollbackPrivateMarker
    $content['_internal\node-agent.env.example'] = 'EXAMPLE_ONLY=1'
    $content['_internal\desktop-review-credential.ps1'] = '# fixture support script'
    $content['_internal\new-desktop-review-ticket.ps1'] = '# fixture support script'
    $content['_internal\README.txt'] = $script:RollbackUnicode
    $content['_internal\pc-next-dist\app.js'] = 'window.fixture = true;'
    $entries = @()
    foreach ($relative in $content.Keys) {
        $file = Join-Path $payload $relative
        [IO.File]::WriteAllText($file, [string]$content[$relative], (New-Object Text.UTF8Encoding($false)))
        $entries += [pscustomobject]@{ relative_path=$relative; length=[long](Get-Item -LiteralPath $file).Length; sha256=(Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() }
    }
    $manifest = [ordered]@{ schema='elon.node_local_rollback_snapshot.v1'; prior_release_identity="0.3.69+$PriorSha"; created_at_ms=$when.ToUnixTimeMilliseconds(); allowlist_version=1; files=$entries }
    $manifestPath = Join-Path $path 'manifest.json'
    Write-RollbackTestJson $manifestPath $manifest
    $manifestHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText((Join-Path $path 'manifest.sha256'), $manifestHash)
    $release = Join-Path (Join-Path $Root 'releases') $TargetSha
    New-Item -ItemType Directory -Path $release -Force | Out-Null
    $receipt = [ordered]@{
        schema='elon.node_local_activation_receipt.v1'; target_release_identity="0.3.69+$TargetSha"; prior_release_identity="0.3.69+$PriorSha"
        outcome='activated'; failure_phase=$null; rollback_state='not_required'; snapshot_manifest_sha256=$manifestHash
        snapshot_directory=$name; error=$null; started_at_ms=$when.ToUnixTimeMilliseconds(); finished_at_ms=$when.AddMinutes(1).ToUnixTimeMilliseconds()
    }
    $receiptPath = Join-Path $release 'activation-receipt.json'
    Write-RollbackTestJson $receiptPath $receipt
    $state = [ordered]@{
        schema='elon.node_local_release.v1'; git_sha=$TargetSha; version='0.3.69'; release_identity="0.3.69+$TargetSha"
        package_path=(Join-Path $release 'elon-node-agent-windows.zip'); package_sha256=('a' * 64)
        verified_at_ms=$when.ToUnixTimeMilliseconds(); activation_state='activated'; local_terminal_state='complete'; superseded_by=$null
        last_error=$null; updated_at_ms=$when.AddMinutes(1).ToUnixTimeMilliseconds(); activation_receipt=$receipt
        activation_receipt_path=$receiptPath; activation_receipt_sha256=(Get-FileHash -LiteralPath $receiptPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    $statePath = Join-Path $release 'state.json'
    Write-RollbackTestJson $statePath $state
    Set-RollbackTestOld $path $Age
    Set-RollbackTestOld $release $Age
    return [pscustomobject]@{ Path=$path; Root=$Root; Name=$name; ManifestPath=$manifestPath; ManifestHash=$manifestHash; StatePath=$statePath; ReceiptPath=$receiptPath; PriorIdentity="0.3.69+$PriorSha"; TargetIdentity="0.3.69+$TargetSha" }
}
function Set-RollbackTestState($Fixture, [string]$Activation, [string]$Terminal) {
    $state = Read-RollbackTestJson $Fixture.StatePath
    $state.activation_state=$Activation; $state.local_terminal_state=$Terminal
    Write-RollbackTestJson $Fixture.StatePath $state
    Set-RollbackTestOld (Split-Path -Parent $Fixture.StatePath)
}
function Update-RollbackTestEvidence($Fixture) {
    $manifest=Read-RollbackTestJson $Fixture.ManifestPath
    $hash=(Get-FileHash -LiteralPath $Fixture.ManifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText((Join-Path $Fixture.Path 'manifest.sha256'),$hash)
    $receipt=Read-RollbackTestJson $Fixture.ReceiptPath
    $receipt.snapshot_manifest_sha256=$hash; $receipt.prior_release_identity=$manifest.prior_release_identity
    Write-RollbackTestJson $Fixture.ReceiptPath $receipt
    $state=Read-RollbackTestJson $Fixture.StatePath
    $state.activation_receipt=$receipt
    $state.activation_receipt_sha256=(Get-FileHash -LiteralPath $Fixture.ReceiptPath -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-RollbackTestJson $Fixture.StatePath $state
    Set-RollbackTestOld $Fixture.Path
}

$fixtureRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ('..\.ai-tmp\rollback-test-' + $script:RollbackUnicode + '-' + [Guid]::NewGuid().ToString('N'))))
New-Item -ItemType Directory -Path $fixtureRoot -Force | Out-Null
$script:RollbackCurrentIdentity = '0.3.69+' + ('f' * 40)
$script:RollbackMockRoots = @()
$script:RollbackMockIdentity = [pscustomobject]@{
    Identity=$script:RollbackCurrentIdentity; Version='0.3.69'; GitSha=('f' * 40); ProcessId=1234
    ProcessStartUtc='2026-01-01T00:00:00.0000000Z'; ExecutablePath=(Join-Path $fixtureRoot 'installed.exe')
    MetadataPath=(Join-Path $fixtureRoot 'installed.json'); MetadataSha256=('b' * 64)
}
$originalRoots = ${function:Get-ElonRollbackRoots}
$originalIdentity = ${function:Get-ElonRollbackInstalledIdentity}
$originalIdle = ${function:Assert-ElonRollbackIdle}
$originalLocks = ${function:Enter-ElonRollbackMaintenanceLocks}
$originalArchive = ${function:Invoke-ElonRollbackSplitArchive}
$savedLocal = $env:LOCALAPPDATA
$junctions = @()
function New-RollbackTestScenario([string]$Name) {
    $root = Join-Path $fixtureRoot $Name
    $null = New-RollbackTestSnapshot $root 1 5
    $null = New-RollbackTestSnapshot $root 2 6
    $candidate = New-RollbackTestSnapshot $root 3 70
    return [pscustomobject]@{ Root=$root; Candidate=$candidate }
}
function New-RollbackTestPlan($Scenario, [string]$Name) {
    $script:RollbackMockRoots = @($Scenario.Root)
    $planPath = Join-Path $fixtureRoot ($Name + '.json')
    $plan = Invoke-ElonRollbackMaintenance -Mode Preview -PlanPath $planPath `
        -ArchiveRoot (Join-Path $fixtureRoot ($Name + '-shared')) -PrivateRoot (Join-Path $fixtureRoot ($Name + '-private'))
    return [pscustomobject]@{ Path=$planPath; Sha256=(Get-FileHash -LiteralPath $planPath -Algorithm SHA256).Hash; Plan=$plan }
}
try {
    $scenario = New-RollbackTestScenario 'policy'
    $duplicate = New-RollbackTestSnapshot $scenario.Root 4 80 -PriorSha ((1001).ToString('x40'))
    $currentTarget = New-RollbackTestSnapshot $scenario.Root 5 70 -TargetSha ('f' * 40)
    $currentPrior = New-RollbackTestSnapshot $scenario.Root 6 70 -PriorSha ('f' * 40)
    $young = New-RollbackTestSnapshot $scenario.Root 7 20
    $failed = New-RollbackTestSnapshot $scenario.Root 8 70
    Set-RollbackTestState $failed 'failed' 'failed'
    $pending = New-RollbackTestSnapshot $scenario.Root 9 70
    Set-RollbackTestState $pending 'superseded' 'pending'
    $unreferenced = New-RollbackTestSnapshot $scenario.Root 10 70
    $state = Read-RollbackTestJson $unreferenced.StatePath
    foreach ($key in @('activation_receipt','activation_receipt_path','activation_receipt_sha256')) { $state.PSObject.Properties.Remove($key) }
    Write-RollbackTestJson $unreferenced.StatePath $state
    Remove-Item -LiteralPath $unreferenced.ReceiptPath -Force
    $policy = Get-ElonRollbackPlan -Roots @($scenario.Root) -CurrentIdentity $script:RollbackCurrentIdentity
    Assert-RollbackTest ($policy.BlockedRoots.Count -eq 0) 'valid root must not be blocked'
    Assert-RollbackTest ($policy.Candidates.Count -eq 1 -and $policy.Candidates[0].Path -eq $scenario.Candidate.Path) 'only old successful snapshot is eligible'
    Assert-RollbackTest ($duplicate.Path -in @($policy.Protected.Path)) 'all snapshots of retained prior identity remain protected'
    foreach ($item in @($currentTarget,$currentPrior,$young,$failed,$pending,$unreferenced)) {
        Assert-RollbackTest ($item.Path -in @($policy.Protected.Path)) ('must retain ' + $item.Name)
    }
    Assert-RollbackThrows { Get-ElonRollbackPlan -Roots @($scenario.Root) -CurrentIdentity '0.3.69' } 'identity'
    Assert-RollbackThrows { Get-ElonRollbackPlan -Roots @($scenario.Root) -CurrentIdentity $script:RollbackCurrentIdentity -KeepNewest 1 } 'KeepNewest'

    foreach ($kind in @('missing-manifest','bad-receipt-hash','unknown-entry','reparse')) {
        $bad = New-RollbackTestScenario $kind
        switch ($kind) {
            'missing-manifest' { Remove-Item -LiteralPath $bad.Candidate.ManifestPath -Force }
            'bad-receipt-hash' { $state=Read-RollbackTestJson $bad.Candidate.StatePath; $state.activation_receipt_sha256=('0' * 64); Write-RollbackTestJson $bad.Candidate.StatePath $state }
            'unknown-entry' { New-Item -ItemType Directory -Path (Join-Path $bad.Root 'rollback\unknown') | Out-Null }
            'reparse' {
                $outside=Join-Path $fixtureRoot 'junction-outside'; New-Item -ItemType Directory -Path $outside -Force | Out-Null
                $link=Join-Path $bad.Candidate.Path 'ElonNode\_internal\pc-next-dist\link'
                New-Item -ItemType Junction -Path $link -Target $outside | Out-Null; $junctions += $link
            }
        }
        $badPolicy = Get-ElonRollbackPlan -Roots @($bad.Root) -CurrentIdentity $script:RollbackCurrentIdentity
        if ($kind -eq 'unknown-entry') {
            Assert-RollbackTest ($bad.Candidate.Path -in @($badPolicy.Candidates | ForEach-Object { $_.Path })) 'unknown snapshot must not hide independent valid candidate'
            Assert-RollbackTest ((Join-Path $bad.Root 'rollback\unknown') -in @($badPolicy.Protected | ForEach-Object { $_.Path })) 'unknown snapshot must be protected'
        } else {
            Assert-RollbackTest ($bad.Candidate.Path -notin @($badPolicy.Candidates | ForEach-Object { $_.Path })) ($kind + ' must never be selected')
            Assert-RollbackTest ($badPolicy.BlockedRoots.Count -gt 0 -or $bad.Candidate.Path -in @($badPolicy.Protected | ForEach-Object { $_.Path })) ($kind + ' must explicitly report protection')
        }
    }
    $legacy=New-RollbackTestScenario 'legacy-prior'
    $legacyEntry=New-RollbackTestSnapshot $legacy.Root 4 80
    $manifest=Read-RollbackTestJson $legacyEntry.ManifestPath; $manifest.prior_release_identity='0.3.69'
    Write-RollbackTestJson $legacyEntry.ManifestPath $manifest; Update-RollbackTestEvidence $legacyEntry
    $legacyPolicy=Get-ElonRollbackPlan -Roots @($legacy.Root) -CurrentIdentity $script:RollbackCurrentIdentity
    Assert-RollbackTest ($legacyPolicy.BlockedRoots.Count -eq 0 -and $legacyPolicy.Candidates.Count -eq 1) 'bound legacy version-only prior protects itself without blocking unrelated valid snapshot'
    Assert-RollbackTest ($legacyEntry.Path -in @($legacyPolicy.Protected | ForEach-Object { $_.Path })) 'version-only prior never qualifies for reclaim'

    $splitSource = New-RollbackTestScenario 'split-source'
    $sourceClient = Join-Path $splitSource.Candidate.Path ('ElonNode\' + $script:RollbackClient)
    Set-Content -LiteralPath $sourceClient -Stream 'PrivateConfig' -Value $script:RollbackPrivateMarker -NoNewline -Encoding UTF8
    Assert-RollbackThrows { Invoke-ElonRollbackSplitArchive -Path $splitSource.Candidate.Path -AllowedRoot (Join-Path $splitSource.Root 'rollback') -ArchiveRoot (Join-Path $fixtureRoot 'ads-shared') -PrivateRoot (Join-Path $fixtureRoot 'ads-private') } 'stream|ADS'
    Assert-RollbackTest (Test-Path -LiteralPath $sourceClient) 'alternate stream rejection preserves source'
    Assert-RollbackTest (@(Get-Item -LiteralPath $sourceClient -Stream 'PrivateConfig').Count -eq 1) 'unknown alternate stream is preserved with source'
    Remove-Item -LiteralPath $sourceClient -Stream 'PrivateConfig'
    $split = Invoke-ElonRollbackSplitArchive -Path $splitSource.Candidate.Path -AllowedRoot (Join-Path $splitSource.Root 'rollback') `
        -ArchiveRoot (Join-Path $fixtureRoot 'split-shared') -PrivateRoot (Join-Path $fixtureRoot 'split-private')
    Assert-RollbackTest (Test-Path -LiteralPath $splitSource.Candidate.Path) 'archive alone cannot delete source'
    $sharedFiles = @(Get-ChildItem -LiteralPath $split.ArchiveDirectory -File -Recurse -Force)
    Assert-RollbackTest ($sharedFiles.Count -eq 4) 'shared archive contains only three fixed program binaries and safe index'
    Assert-RollbackTest (@($sharedFiles.Name | Where-Object { $_ -notin @('client.exe','desktop.exe','uninstaller.exe','programs.json') }).Count -eq 0) 'no environment, scripts, manifests or other private payload on share'
    $sharedClient = Join-Path $split.ArchiveDirectory 'client.exe'
    Assert-RollbackTest (@(Get-Item -LiteralPath $sharedClient -Stream 'PrivateConfig' -ErrorAction SilentlyContinue).Count -eq 0) 'source private alternate stream never reaches shared binary'
    Set-Content -LiteralPath $sharedClient -Stream 'Injected' -Value 'UNTRUSTED_SHARED_ALTERNATE_STREAM' -NoNewline -Encoding UTF8
    foreach ($file in $sharedFiles) {
        Assert-RollbackTest (-not ([IO.File]::ReadAllText($file.FullName)).Contains($script:RollbackPrivateMarker)) 'fixture private marker cannot leave local storage'
    }
    $privateEnv = Join-Path $split.PrivateDirectory 'snapshot\ElonNode\_internal\node-agent.env'
    Assert-RollbackTest ([IO.File]::ReadAllText($privateEnv) -eq $script:RollbackPrivateMarker) 'private configuration remains local with exact bytes'
    Assert-ElonRollbackPrivateAcl $split.PrivateDirectory -ProtectedDirectory
    Assert-RollbackTest ($split.SourceBytes -eq ($split.SharedBytes + $split.PrivateBytes)) 'shared and retained byte counts reconcile'
    $restoreRoot = Join-Path $fixtureRoot 'restores'
    $restoredPath = Join-Path $restoreRoot $script:RollbackUnicode
    Assert-RollbackThrows { Restore-ElonRollbackSplitArchive -ReceiptPath $split.ReceiptPath -Path $restoredPath -AllowedRoot $restoreRoot } 'stream|ADS'
    Assert-RollbackTest (-not(Test-Path -LiteralPath $restoredPath)) 'unmanifested shared stream rejects restore before target creation'
    Remove-Item -LiteralPath $sharedClient -Stream 'Injected'
    $restored = Restore-ElonRollbackSplitArchive -ReceiptPath $split.ReceiptPath -Path $restoredPath -AllowedRoot $restoreRoot
    Assert-RollbackTest $restored.Restored 'split restore succeeds'
    Assert-RollbackTest (@(Get-Item -LiteralPath (Join-Path $restoredPath ('ElonNode\' + $script:RollbackClient)) -Stream 'Injected' -ErrorAction SilentlyContinue).Count -eq 0) 'shared alternate stream is not restored into program'
    $validated = Test-NodeAgentRollbackSnapshot -SnapshotRoot $restoredPath -ExpectedPriorReleaseIdentity $splitSource.Candidate.PriorIdentity
    Assert-RollbackTest ($validated.ManifestSha256 -eq $splitSource.Candidate.ManifestHash) 'restored tree passes original rollback manifest verification'
    $childScript=Join-Path $fixtureRoot 'restore-cross-shell.ps1'
    [IO.File]::WriteAllText($childScript, @'
param($ModuleRoot,$ReceiptPath,$RestorePath,$AllowedRoot)
$ErrorActionPreference='Stop'
. (Join-Path $ModuleRoot 'node-storage-rollback-archive.ps1')
$result=Restore-ElonRollbackSplitArchive -ReceiptPath $ReceiptPath -Path $RestorePath -AllowedRoot $AllowedRoot
if(-not $result.Restored){throw 'cross-shell restore failed'}
Test-NodeAgentRollbackSnapshot -SnapshotRoot $RestorePath | Out-Null
Write-Host 'CROSS_SHELL_RESTORE=PASS'
'@)
    $otherShell=if($PSVersionTable.PSVersion.Major -ge 7){'powershell.exe'}else{'pwsh.exe'}
    $savedModules=$env:PSModulePath
    try {
        if($otherShell -eq 'powershell.exe') {
            $env:PSModulePath=([IO.Path]::Combine($env:ProgramFiles,'WindowsPowerShell\Modules')+';'+[IO.Path]::Combine($env:WINDIR,'System32\WindowsPowerShell\v1.0\Modules'))
        }
        & $otherShell -NoProfile -File $childScript -ModuleRoot $PSScriptRoot -ReceiptPath $split.ReceiptPath -RestorePath (Join-Path $restoreRoot 'cross-shell') -AllowedRoot $restoreRoot
        Assert-RollbackTest ($LASTEXITCODE -eq 0) 'same archive restores and verifies in the other PowerShell version'
    } finally { $env:PSModulePath=$savedModules }
    Assert-RollbackThrows { Restore-ElonRollbackSplitArchive -ReceiptPath $split.ReceiptPath -Path $restoredPath -AllowedRoot $restoreRoot } 'existing target'
    Assert-RollbackThrows { Restore-ElonRollbackSplitArchive -ReceiptPath $split.ReceiptPath -Path (Join-Path $fixtureRoot 'outside-restore') -AllowedRoot $restoreRoot } 'escaped'
    $receiptBytes = [IO.File]::ReadAllBytes($split.ReceiptPath)
    foreach ($kind in @('machine','owner','escape')) {
        $receipt = Read-RollbackTestJson $split.ReceiptPath
        switch ($kind) {
            'machine' { $receipt.MachineKey = 'foreign-machine' }
            'owner' { $receipt.OwnerSid = 'S-1-5-21-1-2-3-99999' }
            'escape' { $receipt.Programs[0].RelativePath = '..\outside.exe' }
        }
        Write-RollbackTestJson $split.ReceiptPath $receipt
        Assert-RollbackThrows { Restore-ElonRollbackSplitArchive -ReceiptPath $split.ReceiptPath -Path (Join-Path $restoreRoot ('rejected-' + $kind)) -AllowedRoot $restoreRoot } 'machine/user identity|mapping is unsafe|escaped|unsafe'
        [IO.File]::WriteAllBytes($split.ReceiptPath, $receiptBytes)
    }
    $sharedClient = Join-Path $split.ArchiveDirectory 'client.exe'
    $clientBytes = [IO.File]::ReadAllBytes($sharedClient)
    [IO.File]::WriteAllText($sharedClient,'tampered')
    Assert-RollbackThrows { Restore-ElonRollbackSplitArchive -ReceiptPath $split.ReceiptPath -Path (Join-Path $restoreRoot 'bad-program') -AllowedRoot $restoreRoot } 'hash mismatch'
    [IO.File]::WriteAllBytes($sharedClient,$clientBytes)
    $envBytes = [IO.File]::ReadAllBytes($privateEnv)
    [IO.File]::AppendAllText($privateEnv,'tampered')
    Assert-RollbackThrows { Restore-ElonRollbackSplitArchive -ReceiptPath $split.ReceiptPath -Path (Join-Path $restoreRoot 'bad-private') -AllowedRoot $restoreRoot } 'private.*changed'
    [IO.File]::WriteAllBytes($privateEnv,$envBytes)
    Assert-RollbackThrows { Invoke-ElonRollbackSplitArchive -Path $splitSource.Candidate.Path -AllowedRoot (Join-Path $splitSource.Root 'rollback') -ArchiveRoot (Join-Path $fixtureRoot 'overlap-share') -PrivateRoot $splitSource.Candidate.Path } 'overlap'
    Assert-RollbackThrows { Join-ElonRollbackSafePath $restoreRoot '..\escaped' } 'unsafe|escaped'
    $unsafePrivate=Join-Path $fixtureRoot 'inherited-private'
    New-Item -ItemType Directory -Path $unsafePrivate | Out-Null
    $beforeAcl=(Get-Acl -LiteralPath $unsafePrivate).Sddl
    Assert-RollbackThrows { Invoke-ElonRollbackSplitArchive -Path $splitSource.Candidate.Path -AllowedRoot (Join-Path $splitSource.Root 'rollback') -ArchiveRoot (Join-Path $fixtureRoot 'unsafe-private-share') -PrivateRoot $unsafePrivate } 'unsafe|ACL'
    Assert-RollbackTest ((Get-Acl -LiteralPath $unsafePrivate).Sddl -eq $beforeAcl) 'untrusted existing private root ACL is rejected without being changed'

    $lockScenario=New-RollbackTestScenario 'held-lock'
    $env:LOCALAPPDATA=Join-Path $fixtureRoot 'lock-local-data'
    New-Item -ItemType Directory -Path (Join-Path $env:LOCALAPPDATA 'ElonNode\_internal') -Force | Out-Null
    $held=[IO.File]::Open((Join-Path $lockScenario.Root 'post-terminal-activator.lock'),[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
    try { Assert-RollbackThrows { & $originalLocks -Roots @($lockScenario.Root) } '.' } finally { $held.Dispose() }
    $freeHandles=@(& $originalLocks -Roots @($lockScenario.Root))
    try { Assert-RollbackTest ($freeHandles.Count -eq 4) 'all fixture activator and updater gates are held together' } finally { foreach($handle in $freeHandles){$handle.Dispose()} }
    $env:LOCALAPPDATA=$savedLocal

    # Only fixture-backed identity/roots/idle/locks are replaced; production has no bypass flag.
    ${function:Get-ElonRollbackRoots} = { return $script:RollbackMockRoots }
    ${function:Get-ElonRollbackInstalledIdentity} = { return $script:RollbackMockIdentity }
    ${function:Assert-ElonRollbackIdle} = { param($Roots,$ExpectedIdentity) }
    ${function:Enter-ElonRollbackMaintenanceLocks} = { param($Roots) return @() }
    $exact = New-RollbackTestScenario 'exact-plan'
    $review = New-RollbackTestPlan $exact 'exact-plan'
    Assert-RollbackTest ($review.Plan.Policy.Candidates.Count -eq 1) 'CLI preview returns one reviewed fixture candidate'
    Assert-RollbackThrows { Invoke-ElonRollbackMaintenance -Mode Apply -PlanPath $review.Path -ExpectedPlanSha256 ('0' * 64) } 'exact reviewed.*SHA256'
    Assert-RollbackTest (Test-Path -LiteralPath $exact.Candidate.Path) 'bad plan hash preserves source'
    $planBytes=[IO.File]::ReadAllBytes($review.Path)
    $foreignPlan=Read-RollbackTestJson $review.Path; $foreignPlan.PowerShellMajor=if($PSVersionTable.PSVersion.Major -eq 5){7}else{5}
    Write-RollbackTestJson $review.Path $foreignPlan
    $foreignHash=(Get-FileHash -LiteralPath $review.Path -Algorithm SHA256).Hash
    Assert-RollbackThrows { Invoke-ElonRollbackMaintenance -Mode Apply -PlanPath $review.Path -ExpectedPlanSha256 $foreignHash } 'same PowerShell major'
    [IO.File]::WriteAllBytes($review.Path,$planBytes)
    $script:RollbackMockIdentity.Identity = '0.3.69+' + ('e' * 40)
    Assert-RollbackThrows { Invoke-ElonRollbackMaintenance -Mode Apply -PlanPath $review.Path -ExpectedPlanSha256 $review.Sha256 } 'runtime changed: Identity|runtime.*identity'
    Assert-RollbackTest (Test-Path -LiteralPath $exact.Candidate.Path) 'identity drift preserves source'
    $script:RollbackMockIdentity.Identity = $script:RollbackCurrentIdentity
    [IO.File]::AppendAllText((Join-Path $exact.Candidate.Path ('ElonNode\' + $script:RollbackClient)), 'changed')
    Assert-RollbackThrows { Invoke-ElonRollbackMaintenance -Mode Apply -PlanPath $review.Path -ExpectedPlanSha256 $review.Sha256 } 'candidates|evidence|snapshot|payload'
    Assert-RollbackTest (Test-Path -LiteralPath $exact.Candidate.Path) 'source drift preserves source'
    $stateDrift=New-RollbackTestScenario 'state-drift'; $stateReview=New-RollbackTestPlan $stateDrift 'state-drift'
    Set-RollbackTestState $stateDrift.Candidate 'waiting_for_terminal' 'pending'
    Assert-RollbackThrows { Invoke-ElonRollbackMaintenance -Mode Apply -PlanPath $stateReview.Path -ExpectedPlanSha256 $stateReview.Sha256 } 'candidates|evidence'
    Assert-RollbackTest (Test-Path -LiteralPath $stateDrift.Candidate.Path) 'state drift preserves source'
    foreach($driftMode in @('state','new-reference','protected-manifest','root-extra-file','source-ads')) {
        $afterCopy=New-RollbackTestScenario ('after-copy-'+$driftMode); $afterReview=New-RollbackTestPlan $afterCopy ('after-copy-'+$driftMode)
        $script:RollbackActualArchive=$originalArchive; $script:RollbackAfterCopyFixture=$afterCopy.Candidate; $script:RollbackAfterCopyMode=$driftMode
        ${function:Invoke-ElonRollbackSplitArchive} = {
            param($Path,$AllowedRoot,$ArchiveRoot,$PrivateRoot)
            $receipt = & $script:RollbackActualArchive -Path $Path -AllowedRoot $AllowedRoot -ArchiveRoot $ArchiveRoot -PrivateRoot $PrivateRoot
            $entry=$script:RollbackAfterCopyFixture
            if($script:RollbackAfterCopyMode -eq 'state') { Set-RollbackTestState $entry 'waiting_for_terminal' 'pending' }
            elseif($script:RollbackAfterCopyMode -eq 'new-reference') {
                $newSha=(9999).ToString('x40'); $newRelease=Join-Path (Join-Path $entry.Root 'releases') $newSha
                New-Item -ItemType Directory -Path $newRelease | Out-Null
                $state=Read-RollbackTestJson $entry.StatePath; $event=Read-RollbackTestJson $entry.ReceiptPath
                $event.target_release_identity='0.3.69+'+$newSha
                $eventPath=Join-Path $newRelease 'activation-receipt.json'; Write-RollbackTestJson $eventPath $event
                $state.git_sha=$newSha; $state.release_identity=$event.target_release_identity; $state.activation_receipt=$event
                $state.activation_receipt_path=$eventPath; $state.activation_receipt_sha256=(Get-FileHash -LiteralPath $eventPath -Algorithm SHA256).Hash.ToLowerInvariant()
                Write-RollbackTestJson (Join-Path $newRelease 'state.json') $state
            } elseif($script:RollbackAfterCopyMode -eq 'protected-manifest') {
                $sibling=Get-ChildItem -LiteralPath (Join-Path $entry.Root 'rollback') -Directory | Where-Object { $_.FullName -ne $entry.Path } | Select-Object -First 1
                [IO.File]::AppendAllText((Join-Path $sibling.FullName 'manifest.json'), ' ')
            } elseif($script:RollbackAfterCopyMode -eq 'root-extra-file') {
                [IO.File]::WriteAllText((Join-Path $entry.Path 'unclassified.txt'),'must preserve this new file')
            } else {
                $client=Join-Path $entry.Path ('ElonNode\'+$script:RollbackClient)
                $beforeTime=(Get-Item -LiteralPath $client).LastWriteTimeUtc; $rootTime=(Get-Item -LiteralPath $entry.Path).LastWriteTimeUtc
                Set-Content -LiteralPath $client -Stream 'LatePrivateConfig' -Value $script:RollbackPrivateMarker -NoNewline -Encoding UTF8
                # Keep primary-stream metadata unchanged so the final ADS guard is exercised.
                [IO.File]::SetLastWriteTimeUtc($client,$beforeTime); [IO.Directory]::SetLastWriteTimeUtc($entry.Path,$rootTime)
            }
            return $receipt
        }
        Assert-RollbackThrows { Invoke-ElonRollbackMaintenance -Mode Apply -PlanPath $afterReview.Path -ExpectedPlanSha256 $afterReview.Sha256 } 'candidates|evidence|catalog|metadata changed|unclassified|alternate data stream'
        Assert-RollbackTest (Test-Path -LiteralPath $afterCopy.Candidate.Path) ($driftMode+' after verified archive preserves source')
        if($driftMode -eq 'source-ads') {
            $client=Join-Path $afterCopy.Candidate.Path ('ElonNode\'+$script:RollbackClient)
            Assert-RollbackTest (@(Get-Item -LiteralPath $client -Stream 'LatePrivateConfig').Count -eq 1) 'late alternate stream remains with preserved source'
        }
        ${function:Invoke-ElonRollbackSplitArchive}=$originalArchive
    }
    $success=New-RollbackTestScenario 'successful-reclaim'; $successReview=New-RollbackTestPlan $success 'successful-reclaim'
    $result=Invoke-ElonRollbackMaintenance -Mode Apply -PlanPath $successReview.Path -ExpectedPlanSha256 $successReview.Sha256
    Assert-RollbackTest ($result.DeletedCount -eq 1 -and $result.Results[0].Deleted) 'reviewed fixture is archived and reclaimed'
    Assert-RollbackTest (-not(Test-Path -LiteralPath $success.Candidate.Path)) 'only reviewed old snapshot is removed'
    Assert-RollbackTest (@(Get-ChildItem -LiteralPath (Join-Path $success.Root 'rollback') -Directory).Count -eq 2) 'two newest prior identities remain'
    Assert-RollbackTest (Test-Path -LiteralPath $result.Results[0].RestoreReceipt) 'durable local restore receipt exists'
    Assert-RollbackTest (@(Get-ChildItem -LiteralPath $result.OperationDirectory -Filter '*.reclaimed.json').Count -eq 1) 'durable reclaim result exists'
    $recovery = Restore-ElonRollbackSplitArchive -ReceiptPath $result.Results[0].RestoreReceipt -Path (Join-Path $restoreRoot 'after-reclaim') -AllowedRoot $restoreRoot
    Assert-RollbackTest $recovery.Restored 'deleted fixture remains recoverable from split archive'
} finally {
    ${function:Get-ElonRollbackRoots}=$originalRoots
    ${function:Get-ElonRollbackInstalledIdentity}=$originalIdentity
    ${function:Assert-ElonRollbackIdle}=$originalIdle
    ${function:Enter-ElonRollbackMaintenanceLocks}=$originalLocks
    ${function:Invoke-ElonRollbackSplitArchive}=$originalArchive
    $env:LOCALAPPDATA=$savedLocal
    foreach($link in $junctions) { if(Test-Path -LiteralPath $link) { [IO.Directory]::Delete($link) } }
    $expectedParent=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\.ai-tmp')).TrimEnd('\')+'\'
    $resolved=[IO.Path]::GetFullPath($fixtureRoot)
    if(-not $resolved.StartsWith($expectedParent,[StringComparison]::OrdinalIgnoreCase) -or (Split-Path -Leaf $resolved) -notlike 'rollback-test-*') { throw 'Fixture cleanup escaped its checked root.' }
    if(Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
Write-Host ('ROLLBACK_TEST_ASSERTIONS=' + $script:RollbackAssertions)
Write-Host 'ROLLBACK_TEST_RESULT=PASS'
