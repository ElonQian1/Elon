function Get-ValidationOwner {
    param([Parameter(Mandatory)][string]$LockPath)
    $ownerPath = Join-Path $LockPath "owner.json"
    if (-not (Test-Path -LiteralPath $ownerPath)) { return $null }
    try { return Get-Content -Raw -LiteralPath $ownerPath | ConvertFrom-Json } catch { return $null }
}

function Test-ValidationOwnerAlive {
    param($Owner)
    if ($null -eq $Owner -or $null -eq $Owner.pid) { return $false }
    return $null -ne (Get-Process -Id ([int]$Owner.pid) -ErrorAction SilentlyContinue)
}

function Enter-ValidationLock {
    param(
        [Parameter(Mandatory)][string]$LockPath,
        [Parameter(Mandatory)][string]$Kind,
        [int]$TimeoutSeconds = 3600,
        [int]$PollMilliseconds = 250
    )
    New-Item -ItemType Directory -Force -Path (Split-Path $LockPath -Parent) | Out-Null
    $started = [DateTime]::UtcNow
    $deadline = $started.AddSeconds($TimeoutSeconds)
    $queued = $false
    while ($true) {
        try {
            New-Item -ItemType Directory -Path $LockPath -ErrorAction Stop | Out-Null
            $owner = [ordered]@{ pid=$PID; kind=$Kind; started_utc=[DateTime]::UtcNow.ToString("o"); command_line=[Environment]::CommandLine }
            $owner | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $LockPath "owner.json") -Encoding UTF8
            return [pscustomobject]@{ path=$LockPath; owner=$owner; queued=$queued; wait_ms=[int]([DateTime]::UtcNow-$started).TotalMilliseconds }
        } catch {
            $owner = Get-ValidationOwner -LockPath $LockPath
            if (-not (Test-ValidationOwnerAlive -Owner $owner)) {
                Remove-Item -LiteralPath $LockPath -Recurse -Force -ErrorAction SilentlyContinue
                continue
            }
            if (-not $queued) {
                Write-Host "VALIDATION_QUEUE=waiting kind=$Kind owner_pid=$($owner.pid)"
                $queued = $true
            }
            if ([DateTime]::UtcNow -ge $deadline) { throw "Timed out waiting for validation resource: $LockPath" }
            Start-Sleep -Milliseconds $PollMilliseconds
        }
    }
}

function Exit-ValidationLock {
    param($Lease)
    if ($null -eq $Lease) { return }
    $owner = Get-ValidationOwner -LockPath $Lease.path
    if ($owner -and [int]$owner.pid -eq $PID) { Remove-Item -LiteralPath $Lease.path -Recurse -Force -ErrorAction SilentlyContinue }
}

function Get-ValidationResourceClass {
    param([Parameter(Mandatory)][string[]]$CargoArgs)
    $verb = if ($CargoArgs.Count) { $CargoArgs[0].ToLowerInvariant() } else { "unknown" }
    $heavy = $verb -in @("test", "build", "bench", "install", "rustc") -or $CargoArgs -contains "--release" -or $CargoArgs -contains "--all-targets"
    if ($heavy) { return "heavy" }
    return "light"
}

function Enter-ValidationResource {
    param([Parameter(Mandatory)][string]$StateRoot, [Parameter(Mandatory)][string]$Class, [int]$LightSlots=2, [int]$TimeoutSeconds=3600)
    if ($Class -eq "heavy") {
        return Enter-ValidationLock -LockPath (Join-Path $StateRoot "resources\heavy.lock") -Kind "heavy" -TimeoutSeconds $TimeoutSeconds
    }
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        for ($i=0; $i -lt $LightSlots; $i++) {
            try { return Enter-ValidationLock -LockPath (Join-Path $StateRoot "resources\light-$i.lock") -Kind "light-$i" -TimeoutSeconds 0 } catch { }
        }
        Start-Sleep -Milliseconds 250
    }
    throw "Timed out waiting for a lightweight validation slot."
}

Export-ModuleMember -Function Get-ValidationOwner, Test-ValidationOwnerAlive, Enter-ValidationLock, Exit-ValidationLock, Get-ValidationResourceClass, Enter-ValidationResource
