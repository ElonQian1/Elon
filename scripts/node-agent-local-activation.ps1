Set-StrictMode -Version Latest

if (-not (Get-Command Write-NodeAgentReleaseJsonAtomic -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot 'node-agent-release-outbox.ps1')
}

function Get-NodeAgentLocalActivationRoot {
    param([string]$ExplicitRoot = '')
    if (-not [string]::IsNullOrWhiteSpace($ExplicitRoot)) {
        return [System.IO.Path]::GetFullPath($ExplicitRoot)
    }
    $local = [System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData)
    if ([string]::IsNullOrWhiteSpace($local)) { throw 'LOCALAPPDATA is unavailable; cannot stage a local node release.' }
    return (Join-Path $local 'Elon\local-node-releases-v1')
}

function Test-NodeAgentInstalledExecutablePath {
    param(
        [Parameter(Mandatory = $true)][string]$ActualPath,
        [Parameter(Mandatory = $true)][string]$ExpectedPath
    )
    if ([string]::IsNullOrWhiteSpace($ActualPath) -or [string]::IsNullOrWhiteSpace($ExpectedPath)) {
        return $false
    }
    try {
        return ([System.IO.Path]::GetFullPath($ActualPath)).Equals(
            [System.IO.Path]::GetFullPath($ExpectedPath),
            [System.StringComparison]::OrdinalIgnoreCase
        )
    } catch {
        return $false
    }
}

function Get-NodeAgentInstalledAdminListener {
    param(
        [int[]]$Ports = (@(7799) + @(7800..7819)),
        [string]$ExpectedClientPath = ''
    )
    if ([string]::IsNullOrWhiteSpace($ExpectedClientPath)) {
        $local = [System.Environment]::GetFolderPath(
            [System.Environment+SpecialFolder]::LocalApplicationData
        )
        if ([string]::IsNullOrWhiteSpace($local)) { return $null }
        $ExpectedClientPath = Join-Path $local 'ElonNode\一龙开发平台.exe'
    }
    foreach ($port in $Ports) {
        $listeners = @(Get-NetTCPConnection -State Listen -LocalPort $port -ErrorAction SilentlyContinue)
        foreach ($listener in $listeners) {
            if ([string]$listener.LocalAddress -notin @('127.0.0.1', '::1')) { continue }
            try {
                $process = Get-Process -Id ([int]$listener.OwningProcess) -ErrorAction Stop
                if (-not (Test-NodeAgentInstalledExecutablePath `
                    -ActualPath ([string]$process.Path) -ExpectedPath $ExpectedClientPath)) {
                    continue
                }
                return [pscustomobject]@{
                    Port = [int]$port
                    ProcessId = [int]$listener.OwningProcess
                    ExecutablePath = [string]$process.Path
                }
            } catch {}
        }
    }
    return $null
}

function Read-NodeAgentLocalReleaseState {
    param([Parameter(Mandatory = $true)][string]$StatePath)
    $json = [System.IO.File]::ReadAllText($StatePath, [System.Text.Encoding]::UTF8)
    return ($json | ConvertFrom-Json)
}

function Set-NodeAgentLocalReleaseState {
    param([Parameter(Mandatory = $true)][string]$StatePath, [Parameter(Mandatory = $true)]$State)
    $State.updated_at_ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    Write-NodeAgentReleaseJsonAtomic -Path $StatePath -Value $State
}

function Register-NodeAgentVerifiedLocalRelease {
    param(
        [Parameter(Mandatory = $true)][string]$StateRoot,
        [Parameter(Mandatory = $true)][string]$GitSha,
        [Parameter(Mandatory = $true)][string]$Version,
        [Parameter(Mandatory = $true)][string]$ReleaseIdentity,
        [Parameter(Mandatory = $true)][string]$WindowsClientPackage,
        [Parameter(Mandatory = $true)][string]$WindowsClientSha256,
        [long]$VerifiedAtMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    )
    $root = Get-NodeAgentLocalActivationRoot -ExplicitRoot $StateRoot
    $sha = $GitSha.Trim().ToLowerInvariant()
    if ($sha -notmatch '^[0-9a-f]{40}$') { throw 'GitSha must be a full 40-character SHA.' }
    if ($ReleaseIdentity -ne "$Version+$sha") { throw 'Local release identity must exactly match version and full Git SHA.' }
    $expected = $WindowsClientSha256.Trim().ToLowerInvariant()
    if ($expected -notmatch '^[0-9a-f]{64}$') { throw 'Windows client SHA-256 must be a 64-character hexadecimal value.' }
    $actual = Get-NodeAgentFileSha256 -Path $WindowsClientPackage
    if ($actual -ne $expected) { throw "Windows client package SHA-256 mismatch: expected=$expected actual=$actual" }

    $releaseDir = Join-Path $root (Join-Path 'releases' $sha)
    New-Item -ItemType Directory -Path $releaseDir -Force | Out-Null
    $packagePath = Join-Path $releaseDir 'elon-node-agent-windows.zip'
    Copy-Item -LiteralPath $WindowsClientPackage -Destination $packagePath -Force
    if ((Get-NodeAgentFileSha256 -Path $packagePath) -ne $expected) {
        throw 'Local release staging copy failed SHA-256 verification.'
    }
    $statePath = Join-Path $releaseDir 'state.json'
    $existing = if (Test-Path -LiteralPath $statePath) { Read-NodeAgentLocalReleaseState -StatePath $statePath } else { $null }
    if ($existing -and [string]$existing.release_identity -ne $ReleaseIdentity) {
        throw 'A staged release directory contains a different immutable identity.'
    }
    $state = [pscustomobject][ordered]@{
        schema = 'elon.node_local_release.v1'
        git_sha = $sha
        version = $Version
        release_identity = $ReleaseIdentity
        package_path = $packagePath
        package_sha256 = $expected
        verified_at_ms = $VerifiedAtMs
        activation_state = 'restart_scheduled'
        local_terminal_state = 'pending'
        superseded_by = $null
        last_error = $null
        updated_at_ms = $VerifiedAtMs
    }
    Write-NodeAgentReleaseJsonAtomic -Path $statePath -Value $state

    $releaseRoot = Join-Path $root 'releases'
    foreach ($otherFile in Get-ChildItem -LiteralPath $releaseRoot -Filter 'state.json' -File -Recurse) {
        if ($otherFile.FullName -eq $statePath) { continue }
        try {
            $other = Read-NodeAgentLocalReleaseState -StatePath $otherFile.FullName
            if ([long]$other.verified_at_ms -le $VerifiedAtMs -and
                [string]$other.activation_state -in @('verified','restart_scheduled','waiting_for_terminal')) {
                $other.activation_state = 'superseded'
                $other.superseded_by = $ReleaseIdentity
                Set-NodeAgentLocalReleaseState -StatePath $otherFile.FullName -State $other
            }
        } catch {
            throw "Cannot safely supersede local release state $($otherFile.FullName): $($_.Exception.Message)"
        }
    }
    return [pscustomobject]@{ StatePath = $statePath; ReleaseIdentity = $ReleaseIdentity; PackagePath = $packagePath }
}

function Get-LatestNodeAgentVerifiedLocalRelease {
    param([Parameter(Mandatory = $true)][string]$StateRoot)
    $root = Join-Path (Get-NodeAgentLocalActivationRoot -ExplicitRoot $StateRoot) 'releases'
    if (-not (Test-Path -LiteralPath $root -PathType Container)) { return $null }
    $candidates = @()
    foreach ($file in Get-ChildItem -LiteralPath $root -Filter 'state.json' -File -Recurse) {
        try {
            $state = Read-NodeAgentLocalReleaseState -StatePath $file.FullName
            if ([string]$state.activation_state -notin @('verified','restart_scheduled','waiting_for_terminal')) { continue }
            if (-not (Test-Path -LiteralPath ([string]$state.package_path) -PathType Leaf)) { continue }
            if ((Get-NodeAgentFileSha256 -Path ([string]$state.package_path)) -ne [string]$state.package_sha256) { continue }
            $state | Add-Member -NotePropertyName state_path -NotePropertyValue $file.FullName -Force
            $candidates += $state
        } catch {
            Write-Warning "Skipping invalid local release $($file.FullName): $($_.Exception.Message)"
        }
    }
    if ($candidates.Count -eq 0) { return $null }
    return @($candidates | Sort-Object @{ Expression = { [long]$_.verified_at_ms }; Descending = $true }, @{ Expression = { [string]$_.git_sha }; Descending = $true } | Select-Object -First 1)[0]
}

function Test-NodeAgentActivationOwnerGate {
    param([Parameter(Mandatory = $true)]$Status)
    $count = 0
    if ($Status.PSObject.Properties.Name -contains 'active_cli_prompt_count') {
        $count = [int]$Status.active_cli_prompt_count
    }
    $ids = @()
    if ($Status.PSObject.Properties.Name -contains 'active_cli_prompt_task_ids') {
        $ids = @($Status.active_cli_prompt_task_ids)
    }
    $runtimeCount = if ($Status.PSObject.Properties.Name -contains 'active_task_runtime') { @($Status.active_task_runtime).Count } else { 0 }
    $activeControlCount = 0
    if ($Status.PSObject.Properties.Name -contains 'cli_session_bridge' -and $null -ne $Status.cli_session_bridge -and
        $Status.cli_session_bridge.PSObject.Properties.Name -contains 'state_summary' -and
        $null -ne $Status.cli_session_bridge.state_summary -and
        $Status.cli_session_bridge.state_summary.PSObject.Properties.Name -contains 'active_control_count') {
        $activeControlCount = [int]$Status.cli_session_bridge.state_summary.active_control_count
    }
    if ($count -gt 0 -or $ids.Count -gt 0 -or $runtimeCount -gt 0 -or $activeControlCount -gt 0) {
        return [pscustomobject]@{ Safe = $false; Reason = 'live_owner'; ActiveTaskIds = $ids }
    }
    return [pscustomobject]@{ Safe = $true; Reason = 'no_live_owner'; ActiveTaskIds = @() }
}

function New-NodeAgentActivationReceipt {
    param(
        [Parameter(Mandatory = $true)]$Release,
        [Parameter(Mandatory = $true)][string]$Outcome,
        [Parameter(Mandatory = $true)][long]$StartedAtMs,
        [Parameter(Mandatory = $true)][string]$RollbackState,
        [string]$FailurePhase = '',
        [string]$ErrorMessage = '',
        [string]$PriorReleaseIdentity = '',
        $PrepareResult = $null
    )
    $snapshotHash = $null
    $snapshotDirectory = $null
    if ($null -ne $PrepareResult) {
        if ($PrepareResult.PSObject.Properties.Name -contains 'SnapshotManifestSha256') {
            $snapshotHash = [string]$PrepareResult.SnapshotManifestSha256
        }
        if ($PrepareResult.PSObject.Properties.Name -contains 'SnapshotRoot') {
            $snapshotDirectory = Split-Path -Leaf ([string]$PrepareResult.SnapshotRoot)
        }
    }
    return [pscustomobject][ordered]@{
        schema = 'elon.node_local_activation_receipt.v1'
        target_release_identity = [string]$Release.release_identity
        prior_release_identity = if ([string]::IsNullOrWhiteSpace($PriorReleaseIdentity)) { $null } else { $PriorReleaseIdentity }
        outcome = $Outcome
        failure_phase = if ([string]::IsNullOrWhiteSpace($FailurePhase)) { $null } else { $FailurePhase }
        rollback_state = $RollbackState
        snapshot_manifest_sha256 = $snapshotHash
        snapshot_directory = $snapshotDirectory
        error = if ([string]::IsNullOrWhiteSpace($ErrorMessage)) { $null } else { $ErrorMessage }
        started_at_ms = $StartedAtMs
        finished_at_ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    }
}

function Save-NodeAgentActivationResult {
    param($Release, [string]$State, [string]$ErrorMessage = '', $Receipt = $null)
    if (-not ($Release.PSObject.Properties.Name -contains 'state_path')) { return }
    $persisted = Read-NodeAgentLocalReleaseState -StatePath ([string]$Release.state_path)
    $persisted.activation_state = $State
    if ($persisted.PSObject.Properties.Name -notcontains 'local_terminal_state') {
        $persisted | Add-Member -NotePropertyName local_terminal_state -NotePropertyValue 'pending'
    }
    if ($State -eq 'activated') {
        $persisted.local_terminal_state = 'complete'
    } elseif ($State -in @('rolled_back','failed')) {
        $persisted.local_terminal_state = 'failed'
    } elseif ($State -in @('verified','restart_scheduled','waiting_for_terminal')) {
        $persisted.local_terminal_state = 'pending'
    }
    $persisted.last_error = if ([string]::IsNullOrWhiteSpace($ErrorMessage)) { $null } else { $ErrorMessage }
    if ($null -ne $Receipt) {
        $receiptPath = Join-Path (Split-Path -Parent ([string]$Release.state_path)) 'activation-receipt.json'
        Write-NodeAgentReleaseJsonAtomic -Path $receiptPath -Value $Receipt
        $receiptHash = Get-NodeAgentFileSha256 -Path $receiptPath
        $persisted | Add-Member -NotePropertyName activation_receipt -NotePropertyValue $Receipt -Force
        $persisted | Add-Member -NotePropertyName activation_receipt_path -NotePropertyValue $receiptPath -Force
        $persisted | Add-Member -NotePropertyName activation_receipt_sha256 -NotePropertyValue $receiptHash -Force
    }
    Set-NodeAgentLocalReleaseState -StatePath ([string]$Release.state_path) -State $persisted
}

function Invoke-NodeAgentActivationTransaction {
    param(
        [Parameter(Mandatory = $true)]$Release,
        [Parameter(Mandatory = $true)][scriptblock]$OwnerGate,
        [scriptblock]$Prepare = $null,
        [Parameter(Mandatory = $true)][scriptblock]$Apply,
        [Parameter(Mandatory = $true)][scriptblock]$Health,
        [Parameter(Mandatory = $true)][scriptblock]$Rollback,
        [string]$PriorReleaseIdentity = ''
    )
    $startedAtMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $gate = & $OwnerGate
    if (-not $gate.Safe) {
        Save-NodeAgentActivationResult -Release $Release -State 'waiting_for_terminal'
        return [pscustomobject]@{ activation_state = 'waiting_for_terminal'; reason = [string]$gate.Reason }
    }
    $prepareResult = $null
    if ($null -ne $Prepare) {
        try {
            $prepareResult = & $Prepare $Release
        } catch {
            $preparationError = $_.Exception.Message
            $receipt = New-NodeAgentActivationReceipt -Release $Release -Outcome 'failed' `
                -StartedAtMs $startedAtMs -RollbackState 'not_required' -FailurePhase 'prepare' `
                -ErrorMessage $preparationError -PriorReleaseIdentity $PriorReleaseIdentity
            Save-NodeAgentActivationResult -Release $Release -State 'failed' `
                -ErrorMessage $preparationError -Receipt $receipt
            return [pscustomobject]@{
                activation_state = 'failed'
                phase = 'prepare'
                error = $preparationError
                receipt = $receipt
            }
        }
    }
    # Prepare (especially a verified rollback snapshot) can take long enough for the
    # desktop shell or a CLI owner to reopen. No installation mutation has happened
    # yet, so re-run the same gate and defer cleanly instead of treating the repair
    # entrypoint's safety refusal as an apply failure that needs rollback.
    $preApplyGate = & $OwnerGate
    if (-not $preApplyGate.Safe) {
        Save-NodeAgentActivationResult -Release $Release -State 'waiting_for_terminal'
        return [pscustomobject]@{
            activation_state = 'waiting_for_terminal'
            reason = [string]$preApplyGate.Reason
            phase = 'pre_apply'
        }
    }
    $applyResult = $null
    $failurePhase = 'apply'
    try {
        $applyResult = & $Apply $Release $prepareResult
        $failurePhase = 'health'
        if (-not (& $Health $Release)) { throw 'The newly activated node failed its exact-release health check.' }
        $receipt = New-NodeAgentActivationReceipt -Release $Release -Outcome 'activated' `
            -StartedAtMs $startedAtMs -RollbackState 'not_required' `
            -PriorReleaseIdentity $PriorReleaseIdentity -PrepareResult $prepareResult
        Save-NodeAgentActivationResult -Release $Release -State 'activated' -Receipt $receipt
        return [pscustomobject]@{
            activation_state = 'activated'
            release_identity = [string]$Release.release_identity
            receipt = $receipt
        }
    } catch {
        $activationError = $_.Exception.Message
        try {
            & $Rollback $Release $prepareResult $applyResult
            $receipt = New-NodeAgentActivationReceipt -Release $Release -Outcome 'rolled_back' `
                -StartedAtMs $startedAtMs -RollbackState 'succeeded' -FailurePhase $failurePhase `
                -ErrorMessage $activationError -PriorReleaseIdentity $PriorReleaseIdentity `
                -PrepareResult $prepareResult
            Save-NodeAgentActivationResult -Release $Release -State 'rolled_back' `
                -ErrorMessage $activationError -Receipt $receipt
            return [pscustomobject]@{
                activation_state = 'rolled_back'
                phase = $failurePhase
                error = $activationError
                receipt = $receipt
            }
        } catch {
            $rollbackError = $_.Exception.Message
            $combinedError = "$activationError; rollback: $rollbackError"
            $receipt = New-NodeAgentActivationReceipt -Release $Release -Outcome 'failed' `
                -StartedAtMs $startedAtMs -RollbackState 'failed' -FailurePhase $failurePhase `
                -ErrorMessage $combinedError -PriorReleaseIdentity $PriorReleaseIdentity `
                -PrepareResult $prepareResult
            Save-NodeAgentActivationResult -Release $Release -State 'failed' `
                -ErrorMessage $combinedError -Receipt $receipt
            return [pscustomobject]@{
                activation_state = 'failed'
                phase = 'rollback'
                error = $combinedError
                receipt = $receipt
            }
        }
    }
}

function Start-NodeAgentPostTerminalActivator {
    param(
        [Parameter(Mandatory = $true)][string]$StateRoot,
        [int]$PollSeconds = 3,
        [int]$WaitTimeoutSeconds = 21600
    )
    $root = Get-NodeAgentLocalActivationRoot -ExplicitRoot $StateRoot
    $runtimeDir = Join-Path $root 'runtime'
    New-Item -ItemType Directory -Path $runtimeDir -Force | Out-Null
    foreach ($name in @(
        'node-agent-release-outbox.ps1',
        'node-agent-local-activation.ps1',
        'node-agent-local-rollback.ps1',
        'node-agent-post-terminal-activator.ps1'
    )) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $runtimeDir $name) -Force
    }
    $script = Join-Path $runtimeDir 'node-agent-post-terminal-activator.ps1'
    $args = @('-NoProfile','-ExecutionPolicy','Bypass','-File',$script,'-StateRoot',$root,'-PollSeconds',[string]$PollSeconds,'-WaitTimeoutSeconds',[string]$WaitTimeoutSeconds)
    $process = Start-Process -FilePath 'powershell.exe' -ArgumentList $args -WindowStyle Hidden -PassThru
    return $process.Id
}
