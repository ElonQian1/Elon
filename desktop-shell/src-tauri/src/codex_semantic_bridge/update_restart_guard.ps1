param(
    [Parameter(Mandatory = $true)][int]$DesktopPid,
    [Parameter(Mandatory = $true)][string]$ClientPath,
    [Parameter(Mandatory = $true)][string]$ExpectedReleaseIdentity,
    [Parameter(Mandatory = $true)][string]$ActionId
)

$ErrorActionPreference = 'Continue'
$installDir = Split-Path -Parent $ClientPath
$internalDir = Join-Path $installDir '_internal'
$localAppData = [Environment]::GetFolderPath('LocalApplicationData')
$stateRoot = Join-Path $localAppData 'Elon\desktop-update-restart-v1'
$logPath = Join-Path $stateRoot 'orchestrator.jsonl'
$applyLockPath = Join-Path $internalDir 'update.apply.lock'
$deadline = [DateTime]::UtcNow.AddHours(6)

function Write-GuardEvent {
    param([string]$Kind, [string]$Outcome, [string]$Detail = '')
    try {
        New-Item -ItemType Directory -Force -Path $stateRoot | Out-Null
        $safeDetail = if ($Detail.Length -gt 320) { $Detail.Substring(0, 320) } else { $Detail }
        $entry = [ordered]@{
            schema = 'elon.desktop_update_restart_guard.v1'
            at_ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
            action_id = $ActionId
            kind = $Kind
            outcome = $Outcome
            target_release_identity = $ExpectedReleaseIdentity
            detail = $safeDetail
        }
        Add-Content -LiteralPath $logPath -Encoding UTF8 -Value ($entry | ConvertTo-Json -Compress)
        if ((Get-Item -LiteralPath $logPath -ErrorAction SilentlyContinue).Length -gt 262144) {
            $tail = @(Get-Content -LiteralPath $logPath -Tail 128 -ErrorAction SilentlyContinue)
            Set-Content -LiteralPath $logPath -Encoding UTF8 -Value $tail
        }
    } catch {}
}

function Get-AdminPort {
    $port = 7799
    $envPath = Join-Path $internalDir 'node-agent.env'
    if (Test-Path -LiteralPath $envPath) {
        foreach ($line in Get-Content -LiteralPath $envPath -ErrorAction SilentlyContinue) {
            if ($line -match '^NODE_AGENT_ADMIN_PORT\s*=\s*(\d+)\s*$') {
                $candidate = [int]$Matches[1]
                if ($candidate -ge 1024 -and $candidate -le 65535) { $port = $candidate }
            }
        }
    }
    return $port
}

function Get-NodeStatus {
    param([int]$Port)
    try {
        $request = [System.Net.HttpWebRequest]::Create("http://127.0.0.1:$Port/api/status")
        $request.Proxy = $null
        $request.Timeout = 1200
        $request.ReadWriteTimeout = 1200
        $response = $request.GetResponse()
        try {
            $reader = New-Object System.IO.StreamReader($response.GetResponseStream(), [Text.Encoding]::UTF8)
            return ($reader.ReadToEnd() | ConvertFrom-Json)
        } finally {
            $response.Dispose()
        }
    } catch {
        return $null
    }
}

function Get-TargetLocalReleaseState {
    $releaseRoot = Join-Path $localAppData 'Elon\local-node-releases-v1\releases'
    if (-not (Test-Path -LiteralPath $releaseRoot)) { return '' }
    $states = Get-ChildItem -LiteralPath $releaseRoot -Filter 'state.json' -File -Recurse -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTimeUtc -Descending
    foreach ($stateFile in $states) {
        try {
            $state = Get-Content -LiteralPath $stateFile.FullName -Raw | ConvertFrom-Json
            if ([string]$state.release_identity -eq $ExpectedReleaseIdentity) {
                return [string]$state.activation_state
            }
        } catch {}
    }
    return ''
}

function Test-ApplyIdle {
    $stream = $null
    try {
        New-Item -ItemType Directory -Force -Path $internalDir | Out-Null
        $stream = New-Object System.IO.FileStream(
            $applyLockPath,
            [System.IO.FileMode]::OpenOrCreate,
            [System.IO.FileAccess]::ReadWrite,
            [System.IO.FileShare]::None
        )
        return $true
    } catch {
        return $false
    } finally {
        if ($null -ne $stream) { $stream.Dispose() }
    }
}

function Assert-InstalledIdentity {
    if ($ExpectedReleaseIdentity -notmatch '^[A-Za-z0-9._-]{1,48}\+[0-9A-Fa-f]{40,64}$') {
        throw 'invalid expected release identity'
    }
    $expectedInstallDir = [System.IO.Path]::GetFullPath((Join-Path $localAppData 'ElonNode'))
    $actualInstallDir = [System.IO.Path]::GetFullPath($installDir)
    if (-not $actualInstallDir.Equals($expectedInstallDir, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'client path is not the formal current-user installation'
    }
    if (-not (Test-Path -LiteralPath $ClientPath -PathType Leaf)) {
        throw 'installed client entrypoint is missing'
    }
}

function Wait-DesktopExit {
    $desktop = Get-CimInstance Win32_Process -Filter "ProcessId=$DesktopPid" -ErrorAction SilentlyContinue
    if ($null -ne $desktop) {
        $expectedDesktop = [System.IO.Path]::GetFullPath((Join-Path $internalDir 'elon-desktop.exe'))
        $actualDesktop = if ($desktop.ExecutablePath) { [System.IO.Path]::GetFullPath([string]$desktop.ExecutablePath) } else { '' }
        if (-not $actualDesktop.Equals($expectedDesktop, [StringComparison]::OrdinalIgnoreCase)) {
            throw 'desktop pid does not belong to the formal installed shell'
        }
        Wait-Process -Id $DesktopPid -Timeout 60 -ErrorAction Stop
    }
}

$port = Get-AdminPort
$updateProcess = $null
$outcome = 'not_started'
try {
    Assert-InstalledIdentity
    Write-GuardEvent 'guard.started' 'waiting_for_desktop_exit'
    Wait-DesktopExit
    Write-GuardEvent 'desktop.exited' 'ok'

    $targetState = Get-TargetLocalReleaseState
    if ($targetState -in @('verified', 'restart_scheduled', 'waiting_for_terminal')) {
        Write-GuardEvent 'update.trigger' 'reused_local_release' $targetState
    } else {
        $env:ELON_EXPECTED_UPDATE_RELEASE_IDENTITY = $ExpectedReleaseIdentity
        try {
            $updateProcess = Start-Process -FilePath $ClientPath -ArgumentList '--update' `
                -WorkingDirectory $installDir -WindowStyle Hidden -PassThru
        } finally {
            Remove-Item Env:\ELON_EXPECTED_UPDATE_RELEASE_IDENTITY -ErrorAction SilentlyContinue
        }
        if ($null -eq $updateProcess) { throw 'update launcher did not return a process handle' }
        Write-GuardEvent 'update.trigger' 'spawned' ("pid=" + $updateProcess.Id)
    }

    $oldHealthyAfterUpdateExit = 0
    while ([DateTime]::UtcNow -lt $deadline) {
        $status = Get-NodeStatus -Port $port
        $identity = if ($null -ne $status) { [string]$status.release_identity } else { '' }
        $targetState = Get-TargetLocalReleaseState
        $applyIdle = Test-ApplyIdle
        if ($identity -eq $ExpectedReleaseIdentity -and $applyIdle) {
            $outcome = 'activated'
            break
        }
        if ($targetState -in @('failed', 'rolled_back', 'superseded')) {
            $outcome = 'local_release_' + $targetState
            break
        }
        if ($null -ne $updateProcess) {
            $updateProcess.Refresh()
            if ($updateProcess.HasExited -and $applyIdle -and -not [string]::IsNullOrWhiteSpace($identity)) {
                $oldHealthyAfterUpdateExit += 1
                if ($oldHealthyAfterUpdateExit -ge 30) {
                    $outcome = 'target_not_activated'
                    break
                }
            } else {
                $oldHealthyAfterUpdateExit = 0
            }
        }
        Start-Sleep -Milliseconds 1000
    }
    if ($outcome -eq 'not_started') { $outcome = 'timeout' }
} catch {
    $outcome = 'guard_failed'
    Write-GuardEvent 'guard.error' $outcome ($_.Exception.Message)
} finally {
    $reopenDeadline = [DateTime]::UtcNow.AddSeconds(90)
    while ([DateTime]::UtcNow -lt $reopenDeadline -and -not (Test-ApplyIdle)) {
        Start-Sleep -Milliseconds 500
    }
    try {
        Start-Process -FilePath $ClientPath -WorkingDirectory $installDir -WindowStyle Hidden | Out-Null
        Write-GuardEvent 'desktop.reopen' $outcome
    } catch {
        Write-GuardEvent 'desktop.reopen' 'failed' ($_.Exception.Message)
    }
    Remove-Item -LiteralPath $PSCommandPath -Force -ErrorAction SilentlyContinue
}
