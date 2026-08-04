param(
    [Parameter(Mandatory = $true)][string]$StateRoot,
    [int]$PollSeconds = 3,
    [int]$WaitTimeoutSeconds = 21600
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'node-agent-release-outbox.ps1')
. (Join-Path $PSScriptRoot 'node-agent-local-activation.ps1')
. (Join-Path $PSScriptRoot 'node-agent-local-rollback.ps1')

function Write-ActivatorLog {
    param([string]$Message)
    $logDir = Join-Path $StateRoot 'logs'
    New-Item -ItemType Directory -Path $logDir -Force | Out-Null
    $line = "{0} {1}`r`n" -f [DateTime]::UtcNow.ToString('o'), $Message
    [System.IO.File]::AppendAllText((Join-Path $logDir 'post-terminal-activator.log'), $line, (New-Object System.Text.UTF8Encoding($false)))
}

function Get-InstalledNodeStatus {
    $listener = Get-NodeAgentInstalledAdminListener
    if ($null -eq $listener) { return $null }
    try {
        $request = [System.Net.HttpWebRequest]::Create("http://127.0.0.1:$($listener.Port)/api/status")
        $request.Proxy = $null
        $request.Timeout = 1200
        $request.ReadWriteTimeout = 1200
        $response = $request.GetResponse()
        try {
            $reader = New-Object System.IO.StreamReader($response.GetResponseStream(), [System.Text.Encoding]::UTF8)
            $value = $reader.ReadToEnd() | ConvertFrom-Json
            if ($value.PSObject.Properties.Name -notcontains 'release_identity') { return $null }
            $confirmed = Get-NodeAgentInstalledAdminListener -Ports @([int]$listener.Port)
            if ($null -eq $confirmed -or [int]$confirmed.ProcessId -ne [int]$listener.ProcessId) {
                return $null
            }
            return [pscustomobject]@{
                Port = [int]$listener.Port
                ProcessId = [int]$listener.ProcessId
                ExecutablePath = [string]$listener.ExecutablePath
                Status = $value
            }
        } finally {
            $response.Dispose()
        }
    } catch {
        return $null
    }
}

function Test-LoopbackNodeActivationOwnerGate {
    $node = Get-InstalledNodeStatus
    if ($null -eq $node) {
        return [pscustomobject]@{
            Safe = $false
            Reason = 'status_unavailable'
            ActiveTaskIds = @()
        }
    }
    $gate = Test-NodeAgentActivationOwnerGate -Status $node.Status
    if (-not $gate.Safe) { return $gate }
    $desktopPath = Join-Path ([System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData)) 'ElonNode\_internal\elon-desktop.exe'
    $desktopRunning = @(Get-Process -Name 'elon-desktop' -ErrorAction SilentlyContinue | Where-Object {
        try {
            -not [string]::IsNullOrWhiteSpace($_.Path) -and
                ([System.IO.Path]::GetFullPath($_.Path)).Equals([System.IO.Path]::GetFullPath($desktopPath), [System.StringComparison]::OrdinalIgnoreCase)
        } catch {
            $false
        }
    })
    if ($desktopRunning.Count -gt 0) {
        return [pscustomobject]@{
            Safe = $false
            Reason = 'desktop_shell_running'
            ActiveTaskIds = @()
        }
    }
    return $gate
}

function Expand-VerifiedReleasePackage {
    param($Release)
    $actual = Get-NodeAgentFileSha256 -Path ([string]$Release.package_path)
    if ($actual -ne [string]$Release.package_sha256) { throw 'Staged package SHA-256 changed after verification.' }
    $applyRoot = Join-Path $StateRoot (Join-Path 'apply' ([string]$Release.git_sha))
    if (Test-Path -LiteralPath $applyRoot) { Remove-Item -LiteralPath $applyRoot -Recurse -Force }
    New-Item -ItemType Directory -Path $applyRoot -Force | Out-Null
    Expand-Archive -LiteralPath ([string]$Release.package_path) -DestinationPath $applyRoot -Force
    $metadataPath = Join-Path $applyRoot '_internal\node-agent-version.json'
    if (-not (Test-Path -LiteralPath $metadataPath -PathType Leaf)) { throw 'Staged package is missing node-agent-version.json.' }
    $metadata = [System.IO.File]::ReadAllText($metadataPath, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
    $actualIdentity = "{0}+{1}" -f ([string]$metadata.version), ([string]$metadata.gitSha)
    if ($actualIdentity -ne [string]$Release.release_identity) {
        throw "Staged package identity mismatch: expected=$($Release.release_identity) actual=$actualIdentity"
    }
    $client = Join-Path $applyRoot '一龙开发平台.exe'
    if (-not (Test-Path -LiteralPath $client -PathType Leaf)) { throw 'Staged package is missing the repair entrypoint.' }
    return [pscustomobject]@{ Root = $applyRoot; Client = $client }
}

function Invoke-HiddenRepair {
    param([Parameter(Mandatory = $true)][string]$ClientPath, [int]$TimeoutSeconds = 180)
    $process = Start-Process -FilePath $ClientPath -ArgumentList '--repair-background' `
        -WorkingDirectory (Split-Path -Parent $ClientPath) -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        # Do not kill it: the installer may be in an atomic replace window. The health
        # check below will fail closed and rollback only after the process exits.
        throw "Repair entrypoint exceeded its bounded ${TimeoutSeconds}s wait."
    }
    if ($process.ExitCode -ne 0) { throw "Repair entrypoint failed with exit code $($process.ExitCode)." }
}

function Wait-ExactNodeHealth {
    param([string]$ReleaseIdentity, [int]$TimeoutSeconds = 90)
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        $node = Get-InstalledNodeStatus
        if ($node -and [string]$node.Status.release_identity -eq $ReleaseIdentity) { return $true }
        Start-Sleep -Milliseconds 500
    } while ([DateTime]::UtcNow -lt $deadline)
    return $false
}

$lockPath = Join-Path $StateRoot 'post-terminal-activator.lock'
New-Item -ItemType Directory -Path $StateRoot -Force | Out-Null
$lock = $null
try {
    try {
        $lock = New-Object System.IO.FileStream($lockPath, [System.IO.FileMode]::OpenOrCreate, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
    } catch {
        Write-ActivatorLog 'another post-terminal activator owns the durable lock; exiting'
        exit 0
    }
    $deadline = [DateTime]::UtcNow.AddSeconds($WaitTimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        $release = Get-LatestNodeAgentVerifiedLocalRelease -StateRoot $StateRoot
        if ($null -eq $release) {
            Write-ActivatorLog 'no verified scheduled release remains; exiting'
            exit 0
        }
        $node = Get-InstalledNodeStatus
        if ($null -eq $node) {
            Write-ActivatorLog 'node status unavailable; activation remains fail-closed'
            Start-Sleep -Seconds $PollSeconds
            continue
        }
        if ([string]$node.Status.release_identity -eq [string]$release.release_identity) {
            Save-NodeAgentActivationResult -Release $release -State 'activated'
            Write-ActivatorLog "target already active: $($release.release_identity)"
            exit 0
        }
        $gate = Test-NodeAgentActivationOwnerGate -Status $node.Status
        if (-not $gate.Safe) {
            Save-NodeAgentActivationResult -Release $release -State 'waiting_for_terminal'
            Write-ActivatorLog ("waiting for live owners: " + (@($gate.ActiveTaskIds) -join ','))
            Start-Sleep -Seconds $PollSeconds
            continue
        }

        # Re-read after the gate so a newer verified target supersedes this one before apply.
        $latest = Get-LatestNodeAgentVerifiedLocalRelease -StateRoot $StateRoot
        if ($null -eq $latest -or [string]$latest.release_identity -ne [string]$release.release_identity) { continue }
        $priorIdentity = [string]$node.Status.release_identity
        $installRoot = Join-Path ([System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData)) 'ElonNode'
        $backupName = "{0}-{1}" -f (
            (Get-Date).ToUniversalTime().ToString('yyyyMMddHHmmssfff'),
            [Guid]::NewGuid().ToString('N').Substring(0, 8)
        )
        $backupRoot = Join-Path $StateRoot (Join-Path 'rollback' $backupName)
        $result = Invoke-NodeAgentActivationTransaction -Release $latest `
            -OwnerGate { Test-LoopbackNodeActivationOwnerGate } `
            -Prepare {
                param($target)
                $expanded = Expand-VerifiedReleasePackage -Release $target
                $snapshot = New-NodeAgentRollbackSnapshot -InstallRoot $installRoot `
                    -SnapshotRoot $backupRoot -PriorReleaseIdentity $priorIdentity
                return [pscustomobject]@{
                    ExpandedClient = $expanded.Client
                    SnapshotRoot = $snapshot.SnapshotRoot
                    SnapshotManifestSha256 = $snapshot.ManifestSha256
                    PriorIdentity = $priorIdentity
                }
            } `
            -Apply {
                param($target, $prepared)
                if ($null -eq $prepared) { throw 'Verified rollback preparation context is unavailable.' }
                Invoke-HiddenRepair -ClientPath ([string]$prepared.ExpandedClient)
                return [pscustomobject]@{ RepairCompleted = $true }
            } `
            -Health { param($target) Wait-ExactNodeHealth -ReleaseIdentity ([string]$target.release_identity) } `
            -Rollback {
                param($target, $prepared, $applyResult)
                if ($null -eq $prepared -or
                    -not ($prepared.PSObject.Properties.Name -contains 'SnapshotRoot')) {
                    throw 'No verified rollback client tree is available.'
                }
                $verified = Test-NodeAgentRollbackSnapshot -SnapshotRoot ([string]$prepared.SnapshotRoot) `
                    -ExpectedPriorReleaseIdentity ([string]$prepared.PriorIdentity)
                Invoke-HiddenRepair -ClientPath ([string]$verified.ClientPath)
                if (-not (Wait-ExactNodeHealth -ReleaseIdentity ([string]$prepared.PriorIdentity))) {
                    throw 'Rollback runtime failed its exact-release health check.'
                }
            } `
            -PriorReleaseIdentity $priorIdentity
        Write-ActivatorLog ("activation result: " + ($result | ConvertTo-Json -Compress))
        if ([string]$result.activation_state -in @('activated','rolled_back','failed')) { exit 0 }
    }
    Write-ActivatorLog 'wait timeout elapsed; release remains scheduled without touching a live owner'
    exit 2
} catch {
    Write-ActivatorLog ("fatal activator error: " + $_.Exception.Message)
    exit 1
} finally {
    if ($null -ne $lock) { $lock.Dispose() }
}
