. (Join-Path $PSScriptRoot 'ai-task-terminal-finalization-receipt.ps1')

function Invoke-AiTerminalGitCapture {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string[]]$GitArgs
    )
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = @(& git -C $RepoPath @GitArgs 2>&1)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    [pscustomobject]@{
        ExitCode = $exitCode
        Text = (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
    }
}

function Invoke-AiTerminalGitRequired {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string[]]$GitArgs
    )
    $result = Invoke-AiTerminalGitCapture -RepoPath $RepoPath -GitArgs $GitArgs
    if ($result.ExitCode -ne 0) {
        throw "git $($GitArgs -join ' ') failed in $RepoPath`: $($result.Text)"
    }
    $result.Text
}

function Invoke-AiTerminalFetchWithRetry {
    param([Parameter(Mandatory = $true)][string]$RepoPath)
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        $result = Invoke-AiTerminalGitCapture -RepoPath $RepoPath -GitArgs @('fetch', 'origin')
        if ($result.ExitCode -eq 0) { return }
        if ($attempt -eq 3) { throw "git fetch origin failed: $($result.Text)" }
        Start-Sleep -Milliseconds (250 * $attempt)
    }
}

function Get-AiTerminalLeaseMarkerSnapshot {
    param([Parameter(Mandatory = $true)][string]$LockedPath)
    if (-not (Test-Path -LiteralPath $LockedPath -PathType Leaf)) { return $null }
    $before = [System.IO.FileInfo]::new($LockedPath)
    $bytes = [System.IO.File]::ReadAllBytes($LockedPath)
    $after = [System.IO.FileInfo]::new($LockedPath)
    if (-not $after.Exists -or $before.Length -ne $after.Length -or
        $before.CreationTimeUtc.Ticks -ne $after.CreationTimeUtc.Ticks -or
        $before.LastWriteTimeUtc.Ticks -ne $after.LastWriteTimeUtc.Ticks) {
        throw 'Platform lease marker changed while it was being observed.'
    }
    $text = "path=$(Normalize-AiTerminalPath $LockedPath)`nlength=$($after.Length)`ncreation=$($after.CreationTimeUtc.Ticks)`nwrite=$($after.LastWriteTimeUtc.Ticks)`nsha256=$(Get-AiTaskSha256 $bytes)"
    $encoding = [System.Text.UTF8Encoding]::new($false)
    Get-AiTaskSha256 ($encoding.GetBytes($text))
}

function Get-AiTerminalLeaseObservation {
    param([Parameter(Mandatory = $true)][string]$RepoPath)
    $gitDir = Get-AiTaskGitValue $RepoPath @('rev-parse', '--path-format=absolute', '--git-dir')
    $lockedPath = Join-Path $gitDir 'locked'
    for ($attempt = 0; $attempt -lt 3; $attempt++) {
        $before = Get-AiTerminalLeaseMarkerSnapshot -LockedPath $lockedPath
        $reason = Get-AiTaskWorktreeLeaseReason -RepoPath $RepoPath
        $after = Get-AiTerminalLeaseMarkerSnapshot -LockedPath $lockedPath
        if ([string]$before -eq [string]$after) {
            if ([string]::IsNullOrWhiteSpace($reason) -and $null -ne $after) {
                throw 'Platform lease marker exists without a readable lease reason.'
            }
            if (-not [string]::IsNullOrWhiteSpace($reason) -and $null -eq $after) {
                throw 'Platform lease reason exists without a durable marker.'
            }
            return [pscustomobject]@{ Reason = $reason; MarkerFingerprint = $after }
        }
    }
    throw 'Platform lease marker did not remain stable long enough to observe.'
}

function Get-AiTerminalLeaseKind {
    param(
        [Parameter(Mandatory = $true)]$Observation,
        [Parameter(Mandatory = $true)][string]$ExpectedLease
    )
    if ([string]::IsNullOrWhiteSpace([string]$Observation.Reason)) { return 'none' }
    if ([string]$Observation.Reason -eq $ExpectedLease) { return 'exact' }
    'foreign'
}

function Assert-AiTerminalExactLease {
    param(
        [Parameter(Mandatory = $true)]$Observation,
        [Parameter(Mandatory = $true)][string]$ExpectedLease,
        [string]$ExpectedMarker = ''
    )
    if ((Get-AiTerminalLeaseKind -Observation $Observation -ExpectedLease $ExpectedLease) -ne 'exact') {
        throw "Expected exact platform root lease, observed: $([string]$Observation.Reason)"
    }
    if ([string]$Observation.MarkerFingerprint -notmatch '^[0-9a-f]{64}$') {
        throw 'Platform root lease marker fingerprint is invalid.'
    }
    if (-not [string]::IsNullOrWhiteSpace($ExpectedMarker) -and
        [string]$Observation.MarkerFingerprint -ne $ExpectedMarker) {
        throw 'Platform root lease marker changed or was reacquired during finalization.'
    }
}

function Invoke-AiTerminalTestLeaseMutation {
    param([string]$Mutation, [string]$BasePath, [string]$TaskRoot, [string]$ExpectedLease)
    if ([string]::IsNullOrWhiteSpace($Mutation) -or $Mutation -eq 'None') { return }
    $current = Get-AiTaskWorktreeLeaseReason -RepoPath $TaskRoot
    if (-not [string]::IsNullOrWhiteSpace($current)) {
        $null = Invoke-AiTerminalGitRequired -RepoPath $BasePath -GitArgs @('worktree', 'unlock', $TaskRoot)
    }
    if ($Mutation -eq 'Missing') { return }
    $reason = if ($Mutation -eq 'Foreign') { 'elon-supervision:foreign-root' } else { $ExpectedLease }
    $null = Invoke-AiTerminalGitRequired -RepoPath $BasePath -GitArgs @('worktree', 'lock', '--reason', $reason, $TaskRoot)
}

function Assert-AiTerminalStateTransition {
    param(
        [AllowNull()]$Receipt,
        [Parameter(Mandatory = $true)]$Initial,
        [Parameter(Mandatory = $true)]$Fresh,
        [Parameter(Mandatory = $true)][string]$ExpectedLease
    )
    $initialKind = Get-AiTerminalLeaseKind -Observation $Initial -ExpectedLease $ExpectedLease
    $freshKind = Get-AiTerminalLeaseKind -Observation $Fresh -ExpectedLease $ExpectedLease
    if ($null -eq $Receipt) {
        if ($initialKind -ne 'exact' -or $freshKind -ne 'exact') {
            throw "Receipt-null finalization requires exact/exact lease observations; observed $initialKind/$freshKind."
        }
        Assert-AiTerminalExactLease -Observation $Fresh -ExpectedLease $ExpectedLease `
            -ExpectedMarker ([string]$Initial.MarkerFingerprint)
        return 'prepare_and_unlock'
    }
    if ([string]$Receipt.state -eq 'prepared') {
        if ($initialKind -eq 'none' -and $freshKind -eq 'none') { return 'resume_after_unlock' }
        if ($initialKind -eq 'exact' -and $freshKind -eq 'exact') {
            Assert-AiTerminalExactLease -Observation $Initial -ExpectedLease $ExpectedLease `
                -ExpectedMarker ([string]$Receipt.leaseMarkerFingerprint)
            Assert-AiTerminalExactLease -Observation $Fresh -ExpectedLease $ExpectedLease `
                -ExpectedMarker ([string]$Receipt.leaseMarkerFingerprint)
            return 'unlock_prepared'
        }
        throw "Prepared finalization rejects changed lease observations: $initialKind/$freshKind."
    }
    if ($initialKind -ne 'none' -or $freshKind -ne 'none') {
        throw "Completed finalization rejects reacquired or changing lease observations: $initialKind/$freshKind."
    }
    'replay_completed'
}

function Invoke-AiTaskPlatformFinalization {
    param(
        [Parameter(Mandatory = $true)][string]$TaskRoot,
        [Parameter(Mandatory = $true)][string]$BasePath,
        [Parameter(Mandatory = $true)][string]$TaskContract,
        [Parameter(Mandatory = $true)]$ValidatedContract,
        [ValidateSet('None','Missing','Foreign','Reacquire')][string]$TestLeaseMutationAfterIdentity = 'None',
        [switch]$TestFailAfterUnlock
    )
    $rootTaskId = [string]$ValidatedContract.supervisionRootTaskId
    $expectedLease = "elon-supervision:$rootTaskId"
    $receiptPath = Get-AiTerminalFinalizationReceiptPath -TaskContract $TaskContract `
        -RootTaskId $rootTaskId
    $receipt = Read-AiTerminalFinalizationReceipt -Path $receiptPath
    $initialLease = Get-AiTerminalLeaseObservation -RepoPath $TaskRoot
    $identity = Get-AiTerminalFinalizationIdentity -TaskRoot $TaskRoot -BasePath $BasePath `
        -TaskContract $TaskContract -ValidatedContract $ValidatedContract
    if ($null -ne $receipt) {
        Assert-AiTerminalFinalizationReceipt -Receipt $receipt -Identity $identity `
            -TaskContract $TaskContract -RootTaskId $rootTaskId
    }
    Invoke-AiTerminalTestLeaseMutation -Mutation $TestLeaseMutationAfterIdentity `
        -BasePath $BasePath -TaskRoot $TaskRoot -ExpectedLease $expectedLease
    $freshLease = Get-AiTerminalLeaseObservation -RepoPath $TaskRoot
    $transition = Assert-AiTerminalStateTransition -Receipt $receipt -Initial $initialLease `
        -Fresh $freshLease -ExpectedLease $expectedLease

    if ($transition -eq 'prepare_and_unlock') {
        $receipt = New-AiPreparedTerminalFinalizationReceipt -Identity $identity `
            -LeaseMarkerFingerprint ([string]$freshLease.MarkerFingerprint)
        Write-AiTerminalFinalizationReceipt -Path $receiptPath -Receipt $receipt
        $receipt = Read-AiTerminalFinalizationReceipt -Path $receiptPath
        Assert-AiTerminalFinalizationReceipt -Receipt $receipt -Identity $identity `
            -TaskContract $TaskContract -RootTaskId $rootTaskId
        $transition = 'unlock_prepared'
    }

    if ($transition -eq 'unlock_prepared') {
        Write-Host "TERMINAL_FINALIZATION_STATUS=prepared:$([string]$receipt.finalizationId)"
        # This observation is intentionally adjacent to unlock. It must not
        # reuse the earlier read taken before fetch/status/receipt validation.
        $preUnlockLease = Get-AiTerminalLeaseObservation -RepoPath $TaskRoot
        Assert-AiTerminalExactLease -Observation $preUnlockLease -ExpectedLease $expectedLease `
            -ExpectedMarker ([string]$receipt.leaseMarkerFingerprint)
        $unlock = Invoke-AiTerminalGitCapture -RepoPath $BasePath -GitArgs @('worktree', 'unlock', $TaskRoot)
        if ($unlock.ExitCode -ne 0) {
            throw "Unable to release exact platform task worktree lease: $($unlock.Text)"
        }
        $postUnlockLease = Get-AiTerminalLeaseObservation -RepoPath $TaskRoot
        if ((Get-AiTerminalLeaseKind -Observation $postUnlockLease -ExpectedLease $expectedLease) -ne 'none') {
            throw "Terminal finalization lease remained/reappeared after unlock: $($postUnlockLease.Reason)"
        }
        if ($TestFailAfterUnlock) {
            throw 'Injected interruption after platform unlock and missing-lease verification.'
        }
    } elseif ($transition -eq 'resume_after_unlock') {
        Write-Host "TERMINAL_FINALIZATION_STATUS=prepared:$([string]$receipt.finalizationId)"
    }

    $completedIdentity = Get-AiTerminalFinalizationIdentity -TaskRoot $TaskRoot -BasePath $BasePath `
        -TaskContract $TaskContract -ValidatedContract $ValidatedContract
    Assert-AiTerminalFinalizationReceipt -Receipt $receipt -Identity $completedIdentity `
        -TaskContract $TaskContract -RootTaskId $rootTaskId
    $completionLease = Get-AiTerminalLeaseObservation -RepoPath $TaskRoot
    if ((Get-AiTerminalLeaseKind -Observation $completionLease -ExpectedLease $expectedLease) -ne 'none') {
        throw "Terminal finalization cannot complete while a lease exists: $($completionLease.Reason)"
    }
    if ([string]$receipt.state -eq 'prepared') {
        $receipt.state = 'completed'
        $receipt.completedAtUtc = [DateTime]::UtcNow.ToString('o')
        Write-AiTerminalFinalizationReceipt -Path $receiptPath -Receipt $receipt
        $receipt = Read-AiTerminalFinalizationReceipt -Path $receiptPath
    }
    Assert-AiTerminalFinalizationReceipt -Receipt $receipt -Identity $completedIdentity `
        -TaskContract $TaskContract -RootTaskId $rootTaskId
    if ([string]$receipt.state -ne 'completed') {
        throw 'Terminal finalization receipt did not durably complete.'
    }
    Write-Host "TERMINAL_FINALIZATION_STATUS=completed:$([string]$receipt.finalizationId)"
    [string]$receipt.finalizationId
}
