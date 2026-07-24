Set-StrictMode -Version Latest

function Get-NodeAgentReleaseOutboxRoot {
    param([string]$ExplicitRoot = '')
    if (-not [string]::IsNullOrWhiteSpace($ExplicitRoot)) {
        return [System.IO.Path]::GetFullPath($ExplicitRoot)
    }
    $local = [System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData)
    if ([string]::IsNullOrWhiteSpace($local)) {
        throw 'LOCALAPPDATA is unavailable; cannot create the durable node release outbox.'
    }
    return (Join-Path $local 'Elon\release-outbox-v1')
}

function Write-NodeAgentReleaseJsonAtomic {
    param([Parameter(Mandatory = $true)][string]$Path, [Parameter(Mandatory = $true)]$Value)
    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    $temp = Join-Path $parent ((Split-Path -Leaf $Path) + '.' + [Guid]::NewGuid().ToString('N') + '.tmp')
    try {
        $json = $Value | ConvertTo-Json -Depth 12
        [System.IO.File]::WriteAllText($temp, $json, (New-Object System.Text.UTF8Encoding($false)))
        if (Test-Path -LiteralPath $Path) {
            $backup = $Path + '.previous'
            try {
                [System.IO.File]::Replace($temp, $Path, $backup, $true)
                Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue
                return
            } catch {
                Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue
            }
        }
        Move-Item -LiteralPath $temp -Destination $Path -Force
    } finally {
        Remove-Item -LiteralPath $temp -Force -ErrorAction SilentlyContinue
    }
}

function Assert-NodeAgentOutboxTextSafe {
    param([string]$Name, [string]$Value)
    if ([string]::IsNullOrWhiteSpace($Value)) { return }
    if ($Value -match '(?i)(bearer\s+[a-z0-9._-]+|admin[_-]?token\s*[:=]|authorization\s*:|password\s*[:=])') {
        throw "$Name appears to contain a credential; refusing to persist it in the release outbox."
    }
}

function Get-NodeAgentFileSha256 {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [ValidateRange(1, 40)][int]$Attempts = 12,
        [ValidateRange(0, 5000)][int]$RetryDelayMilliseconds = 250
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Release artifact does not exist: $Path" }
    $lastError = ''
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        try {
            $stream = [System.IO.File]::OpenRead($Path)
            try {
                $sha256 = [System.Security.Cryptography.SHA256]::Create()
                try {
                    return ([System.BitConverter]::ToString($sha256.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
                } finally {
                    $sha256.Dispose()
                }
            } finally {
                $stream.Dispose()
            }
        } catch {
            $lastError = $_.Exception.Message
            if ($attempt -lt $Attempts -and $RetryDelayMilliseconds -gt 0) {
                Start-Sleep -Milliseconds $RetryDelayMilliseconds
            }
        }
    }
    throw "Release artifact could not be hashed after $Attempts bounded attempts: $Path; $lastError"
}

function Read-NodeAgentRemoteReleaseEvent {
    param([Parameter(Mandatory = $true)][string]$EventPath)
    if (-not (Test-Path -LiteralPath $EventPath -PathType Leaf)) { throw "Outbox event does not exist: $EventPath" }
    $json = [System.IO.File]::ReadAllText($EventPath, [System.Text.Encoding]::UTF8)
    return ($json | ConvertFrom-Json)
}

function Add-NodeAgentRemoteReleaseEvent {
    param(
        [Parameter(Mandatory = $true)][string]$OutboxRoot,
        [Parameter(Mandatory = $true)][string]$GitSha,
        [Parameter(Mandatory = $true)][string]$Version,
        [Parameter(Mandatory = $true)][string]$ReleaseIdentity,
        [string]$Changelog = '',
        [Parameter(Mandatory = $true)][string]$WindowsExe,
        [Parameter(Mandatory = $true)][string]$WindowsClientPackage,
        [string]$RipgrepPackage = '',
        [Parameter(Mandatory = $true)][string]$GitCommonDir,
        [switch]$IncludeLinux
    )
    $sha = $GitSha.Trim().ToLowerInvariant()
    if ($sha -notmatch '^[0-9a-f]{40}$') { throw 'GitSha must be a full 40-character SHA.' }
    if ($ReleaseIdentity -ne "$Version+$sha") { throw 'ReleaseIdentity must exactly match Version + full GitSha.' }
    Assert-NodeAgentOutboxTextSafe -Name 'Changelog' -Value $Changelog

    $root = Get-NodeAgentReleaseOutboxRoot -ExplicitRoot $OutboxRoot
    $eventDir = Join-Path $root (Join-Path 'events' $sha)
    $eventPath = Join-Path $eventDir 'event.json'
    $winSha = Get-NodeAgentFileSha256 -Path $WindowsExe
    $clientSha = Get-NodeAgentFileSha256 -Path $WindowsClientPackage
    $ripgrepSha = if ([string]::IsNullOrWhiteSpace($RipgrepPackage)) { '' } else { Get-NodeAgentFileSha256 -Path $RipgrepPackage }

    if (Test-Path -LiteralPath $eventPath -PathType Leaf) {
        $existing = Read-NodeAgentRemoteReleaseEvent -EventPath $eventPath
        $existingIncludeLinux = $existing.PSObject.Properties.Name -contains 'include_linux' -and
            [bool]$existing.include_linux
        if ([string]$existing.git_sha -ne $sha -or
            [string]$existing.release_identity -ne $ReleaseIdentity -or
            [string]$existing.artifacts.windows_exe_sha256 -ne $winSha -or
            [string]$existing.artifacts.windows_client_sha256 -ne $clientSha -or
            [bool]$existingIncludeLinux -ne [bool]$IncludeLinux) {
            throw 'An outbox event already exists for this SHA with different immutable identity or artifacts.'
        }
        return [pscustomobject]@{ EventPath = $eventPath; Created = $false; Event = $existing }
    }

    $artifactDir = Join-Path $eventDir 'artifacts'
    New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null
    $winCopy = Join-Path $artifactDir 'elon-pc-node.exe'
    $clientCopy = Join-Path $artifactDir 'elon-node-agent-windows.zip'
    Copy-Item -LiteralPath $WindowsExe -Destination $winCopy -Force
    Copy-Item -LiteralPath $WindowsClientPackage -Destination $clientCopy -Force
    $ripgrepCopy = ''
    if (-not [string]::IsNullOrWhiteSpace($RipgrepPackage)) {
        $ripgrepCopy = Join-Path $artifactDir 'ripgrep-windows.zip'
        Copy-Item -LiteralPath $RipgrepPackage -Destination $ripgrepCopy -Force
    }
    if ((Get-NodeAgentFileSha256 -Path $winCopy) -ne $winSha -or
        (Get-NodeAgentFileSha256 -Path $clientCopy) -ne $clientSha) {
        throw 'Durable outbox artifact copy failed SHA-256 verification.'
    }

    $now = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $event = [ordered]@{
        schema = 'elon.node_release_outbox.v1'
        event_id = "node-agent-$sha"
        git_sha = $sha
        version = $Version
        release_identity = $ReleaseIdentity
        changelog = $Changelog
        include_linux = [bool]$IncludeLinux
        local_result_independent = $true
        sync_state = 'pending'
        attempt_count = 0
        created_at_ms = $now
        updated_at_ms = $now
        next_attempt_at_ms = $now
        last_error_code = $null
        git_common_dir = [System.IO.Path]::GetFullPath($GitCommonDir)
        artifacts = [ordered]@{
            windows_exe = $winCopy
            windows_exe_sha256 = $winSha
            windows_client = $clientCopy
            windows_client_sha256 = $clientSha
            ripgrep = $ripgrepCopy
            ripgrep_sha256 = $ripgrepSha
        }
    }
    Write-NodeAgentReleaseJsonAtomic -Path $eventPath -Value $event
    return [pscustomobject]@{ EventPath = $eventPath; Created = $true; Event = (Read-NodeAgentRemoteReleaseEvent -EventPath $eventPath) }
}

function Complete-NodeAgentRemoteReleaseAttempt {
    param(
        [Parameter(Mandatory = $true)][string]$EventPath,
        [Parameter(Mandatory = $true)][ValidateSet('success','retry','failed')][string]$Outcome,
        [string]$ErrorCode = '',
        [long]$NowMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(),
        [int]$MaxAttempts = 8
    )
    if ($MaxAttempts -lt 1) { throw 'MaxAttempts must be at least one.' }
    $event = Read-NodeAgentRemoteReleaseEvent -EventPath $EventPath
    if ([string]$event.sync_state -eq 'synced' -and $Outcome -eq 'success') { return $event }
    $attempts = [int]$event.attempt_count + 1
    $event.attempt_count = $attempts
    $event.updated_at_ms = $NowMs
    if ($Outcome -eq 'success') {
        $event.sync_state = 'synced'
        $event.next_attempt_at_ms = $null
        $event.last_error_code = $null
        $event | Add-Member -NotePropertyName synced_at_ms -NotePropertyValue $NowMs -Force
    } elseif ($Outcome -eq 'failed' -or $attempts -ge $MaxAttempts) {
        $event.sync_state = 'failed'
        $event.next_attempt_at_ms = $null
        $event.last_error_code = if ([string]::IsNullOrWhiteSpace($ErrorCode)) { 'remote_release_failed' } else { $ErrorCode }
    } else {
        $event.sync_state = 'retrying'
        $delaySeconds = [Math]::Min(900, [Math]::Pow(2, [Math]::Min(9, $attempts - 1)))
        $event.next_attempt_at_ms = $NowMs + [long]($delaySeconds * 1000)
        $event.last_error_code = if ([string]::IsNullOrWhiteSpace($ErrorCode)) { 'retryable_remote_failure' } else { $ErrorCode }
    }
    # Remote state cannot own or reverse the independently persisted local activation result.
    if (-not [bool]$event.local_result_independent) {
        throw 'Outbox local-result independence marker was corrupted; refusing to update remote state.'
    }
    Write-NodeAgentReleaseJsonAtomic -Path $EventPath -Value $event
    return (Read-NodeAgentRemoteReleaseEvent -EventPath $EventPath)
}

function Get-DueNodeAgentRemoteReleaseEvents {
    param([Parameter(Mandatory = $true)][string]$OutboxRoot, [long]$NowMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())
    $eventsRoot = Join-Path (Get-NodeAgentReleaseOutboxRoot -ExplicitRoot $OutboxRoot) 'events'
    if (-not (Test-Path -LiteralPath $eventsRoot -PathType Container)) { return @() }
    $due = @()
    foreach ($file in Get-ChildItem -LiteralPath $eventsRoot -Filter 'event.json' -File -Recurse) {
        try {
            $event = Read-NodeAgentRemoteReleaseEvent -EventPath $file.FullName
            if ([string]$event.sync_state -notin @('pending','retrying')) { continue }
            if ($null -ne $event.next_attempt_at_ms -and [long]$event.next_attempt_at_ms -gt $NowMs) { continue }
            $due += [pscustomobject]@{ Path = $file.FullName; Event = $event }
        } catch {
            Write-Warning "Skipping corrupt node release outbox event $($file.FullName): $($_.Exception.Message)"
        }
    }
    return @($due | Sort-Object { [long]$_.Event.created_at_ms }, { [string]$_.Event.event_id })
}

function Start-NodeAgentRemoteReleaseWorker {
    param(
        [Parameter(Mandatory = $true)][string]$OutboxRoot,
        [int]$AttemptTimeoutSeconds = 3600,
        [int]$IdleTimeoutSeconds = 21600
    )
    $root = Get-NodeAgentReleaseOutboxRoot -ExplicitRoot $OutboxRoot
    $runtimeDir = Join-Path $root 'runtime'
    New-Item -ItemType Directory -Path $runtimeDir -Force | Out-Null
    foreach ($name in @('node-agent-release-outbox.ps1','node-agent-remote-release-worker.ps1')) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $runtimeDir $name) -Force
    }
    $worker = Join-Path $runtimeDir 'node-agent-remote-release-worker.ps1'
    $command = 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "{0}" -OutboxRoot "{1}" -AttemptTimeoutSeconds {2} -IdleTimeoutSeconds {3}' -f `
        $worker, $root, $AttemptTimeoutSeconds, $IdleTimeoutSeconds
    try {
        $runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
        New-Item -Path $runKey -Force | Out-Null
        Set-ItemProperty -Path $runKey -Name 'ElonNodeReleaseOutboxV1' -Value $command -Type String
    } catch {
        Write-Warning "Could not register the user-level outbox restart sweep: $($_.Exception.Message)"
    }
    $args = @('-NoProfile','-ExecutionPolicy','Bypass','-File',$worker,'-OutboxRoot',$root,
        '-AttemptTimeoutSeconds',[string]$AttemptTimeoutSeconds,'-IdleTimeoutSeconds',[string]$IdleTimeoutSeconds)
    $process = Start-Process -FilePath 'powershell.exe' -ArgumentList $args -WindowStyle Hidden -PassThru
    try { $process.PriorityClass = 'BelowNormal' } catch {}
    return $process.Id
}
