param(
    [Parameter(Mandatory = $true)][string]$OutboxRoot,
    [int]$AttemptTimeoutSeconds = 600,
    [int]$IdleTimeoutSeconds = 21600,
    [int]$MaxAttempts = 8
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'node-agent-release-outbox.ps1')

function Write-OutboxWorkerLog {
    param([string]$Message)
    $logDir = Join-Path $OutboxRoot 'logs'
    New-Item -ItemType Directory -Path $logDir -Force | Out-Null
    $safe = $Message -replace '(?i)(authorization|bearer|token|password)\s*[:=]\s*\S+', '$1=[redacted]'
    $line = "{0} {1}`r`n" -f [DateTime]::UtcNow.ToString('o'), $safe
    [System.IO.File]::AppendAllText((Join-Path $logDir 'remote-release-worker.log'), $line, (New-Object System.Text.UTF8Encoding($false)))
}

function Resolve-EventSourceWorktree {
    param($Event)
    $sourceRoot = Join-Path $OutboxRoot (Join-Path 'sources' ([string]$Event.git_sha))
    $manifest = Join-Path $sourceRoot 'scripts\publish-node-agent.ps1'
    if (Test-Path -LiteralPath $manifest -PathType Leaf) {
        $actual = (& git -C $sourceRoot rev-parse HEAD 2>$null | Select-Object -First 1)
        if ([string]$actual -eq [string]$Event.git_sha) { return $sourceRoot }
        throw 'Recovered outbox source worktree has the wrong immutable Git SHA.'
    }
    $parent = Split-Path -Parent $sourceRoot
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    $gitDir = [string]$Event.git_common_dir
    if (-not (Test-Path -LiteralPath $gitDir -PathType Container)) { throw 'Durable Git common directory is unavailable.' }
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & git --git-dir=$gitDir worktree add --detach $sourceRoot ([string]$Event.git_sha) 2>&1 | Out-Null
        $gitExit = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($gitExit -ne 0) { throw 'Could not reconstruct the immutable remote release source worktree.' }
    return $sourceRoot
}

function Invoke-RemoteReleaseAttemptProcess {
    param([string]$EventPath, $Event)
    $source = Resolve-EventSourceWorktree -Event $Event
    $script = Join-Path $source 'scripts\publish-node-agent.ps1'
    $arguments = @('-NoProfile','-ExecutionPolicy','Bypass','-File',$script,'-SynchronousRemote','-RemoteOutboxEventPath',$EventPath)
    if ($Event.PSObject.Properties.Name -contains 'include_linux' -and [bool]$Event.include_linux) {
        $arguments += '-IncludeLinux'
    }
    $process = Start-Process -FilePath 'powershell.exe' -ArgumentList $arguments -WorkingDirectory $source `
        -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit($AttemptTimeoutSeconds * 1000)) {
        try { $process.Kill() } catch {}
        return [pscustomobject]@{ Outcome = 'retry'; ErrorCode = 'bounded_attempt_timeout' }
    }
    if ($process.ExitCode -eq 0) { return [pscustomobject]@{ Outcome = 'success'; ErrorCode = '' } }
    return [pscustomobject]@{ Outcome = 'retry'; ErrorCode = "remote_process_exit_$($process.ExitCode)" }
}

$lockPath = Join-Path $OutboxRoot 'remote-release-worker.lock'
New-Item -ItemType Directory -Path $OutboxRoot -Force | Out-Null
$lock = $null
try {
    try {
        $lock = New-Object System.IO.FileStream($lockPath, [System.IO.FileMode]::OpenOrCreate, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
    } catch {
        exit 0
    }
    $idleDeadline = [DateTime]::UtcNow.AddSeconds($IdleTimeoutSeconds)
    while ([DateTime]::UtcNow -lt $idleDeadline) {
        $due = @(Get-DueNodeAgentRemoteReleaseEvents -OutboxRoot $OutboxRoot)
        if ($due.Count -eq 0) {
            $pending = @(Get-ChildItem -LiteralPath (Join-Path $OutboxRoot 'events') -Filter event.json -File -Recurse -ErrorAction SilentlyContinue | ForEach-Object {
                try { Read-NodeAgentRemoteReleaseEvent -EventPath $_.FullName } catch { $null }
            } | Where-Object { $null -ne $_ -and [string]$_.sync_state -in @('pending','retrying') })
            if ($pending.Count -eq 0) { exit 0 }
            Start-Sleep -Seconds 2
            continue
        }
        foreach ($item in $due) {
            try {
                $result = Invoke-RemoteReleaseAttemptProcess -EventPath $item.Path -Event $item.Event
                Complete-NodeAgentRemoteReleaseAttempt -EventPath $item.Path -Outcome $result.Outcome `
                    -ErrorCode $result.ErrorCode -MaxAttempts $MaxAttempts | Out-Null
                Write-OutboxWorkerLog "$($item.Event.event_id) outcome=$($result.Outcome) code=$($result.ErrorCode)"
            } catch {
                Complete-NodeAgentRemoteReleaseAttempt -EventPath $item.Path -Outcome retry `
                    -ErrorCode 'worker_exception' -MaxAttempts $MaxAttempts | Out-Null
                Write-OutboxWorkerLog "$($item.Event.event_id) worker_exception=$($_.Exception.Message)"
            }
        }
    }
    Write-OutboxWorkerLog 'idle timeout elapsed; pending state remains durable for the next startup sweep'
} finally {
    if ($null -ne $lock) { $lock.Dispose() }
}
