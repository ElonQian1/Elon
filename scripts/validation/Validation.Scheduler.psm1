function Get-ValidationProcessIdentity {
    param([int]$ProcessId = $PID)
    try {
        $process = Get-Process -Id $ProcessId -ErrorAction Stop
        return $process.StartTime.ToUniversalTime().Ticks.ToString()
    } catch { return $null }
}

function Get-ValidationOwner {
    param([Parameter(Mandatory)][string]$LockPath)
    $ownerPath = Join-Path $LockPath "owner.json"
    if (-not (Test-Path -LiteralPath $ownerPath)) { return $null }
    try { return Get-Content -Raw -LiteralPath $ownerPath | ConvertFrom-Json } catch { return $null }
}

function Test-ValidationOwnerAlive {
    param($Owner)
    if ($null -eq $Owner -or $null -eq $Owner.pid -or [string]::IsNullOrWhiteSpace($Owner.process_start_id)) { return $false }
    return (Get-ValidationProcessIdentity -ProcessId ([int]$Owner.pid)) -eq [string]$Owner.process_start_id
}

function Test-ValidationLeaseMatches {
    param($Expected, $Actual)
    return $Expected -and $Actual -and [string]$Expected.lease_id -eq [string]$Actual.lease_id -and
        [int]$Expected.pid -eq [int]$Actual.pid -and [string]$Expected.process_start_id -eq [string]$Actual.process_start_id
}

function Remove-ValidationLockIfOwned {
    param([Parameter(Mandatory)][string]$LockPath, [Parameter(Mandatory)]$ExpectedOwner)
    $actual = Get-ValidationOwner -LockPath $LockPath
    if (-not (Test-ValidationLeaseMatches $ExpectedOwner $actual)) { return $false }
    $retired = "$LockPath.retired-$($ExpectedOwner.lease_id)"
    try { Move-Item -LiteralPath $LockPath -Destination $retired -ErrorAction Stop } catch { return $false }
    $moved = Get-ValidationOwner -LockPath $retired
    if (Test-ValidationLeaseMatches $ExpectedOwner $moved) { Remove-Item -LiteralPath $retired -Recurse -Force -ErrorAction SilentlyContinue; return $true }
    return $false
}

function Remove-OwnerlessValidationLockAfterGrace {
    param([Parameter(Mandatory)][string]$LockPath,[int]$OwnerPublishGraceMilliseconds=1000)
    if (-not (Test-Path -LiteralPath $LockPath -PathType Container) -or (Get-ValidationOwner $LockPath)) { return $false }
    try { $age=([DateTime]::UtcNow-(Get-Item -LiteralPath $LockPath -ErrorAction Stop).LastWriteTimeUtc).TotalMilliseconds } catch { return $false }
    if ($age -lt $OwnerPublishGraceMilliseconds) { return $false }
    $retired="$LockPath.ownerless-$([Guid]::NewGuid().ToString('N'))"
    try { [IO.Directory]::Move($LockPath,$retired) } catch { return $false }
    if (Get-ValidationOwner $retired) {
        try { [IO.Directory]::Move($retired,$LockPath) } catch {}
        return $false
    }
    Remove-Item -LiteralPath $retired -Recurse -Force -ErrorAction SilentlyContinue
    return $true
}

function Enter-ValidationLock {
    param([Parameter(Mandatory)][string]$LockPath,[Parameter(Mandatory)][string]$Kind,[int]$TimeoutSeconds=3600,[int]$PollMilliseconds=100,[switch]$PersistWaiter,[int]$OwnerPublishGraceMilliseconds=1000)
    New-Item -ItemType Directory -Force -Path (Split-Path $LockPath -Parent) | Out-Null
    $started=[DateTime]::UtcNow; $deadline=$started.AddSeconds($TimeoutSeconds); $queued=$false
    $waiterId=[Guid]::NewGuid().ToString('N'); $waiterPath="$LockPath.waiters\$waiterId.json"
    try {
        while ($true) {
            $leaseId=[Guid]::NewGuid().ToString('N')
            $candidate="$LockPath.candidate-$leaseId"
            try {
                $owner=[ordered]@{lease_id=$leaseId;pid=$PID;process_start_id=(Get-ValidationProcessIdentity);kind=$Kind;started_utc=[DateTime]::UtcNow.ToString('o');command_line=[Environment]::CommandLine}
                New-Item -ItemType Directory -Path $candidate -ErrorAction Stop | Out-Null
                $owner | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $candidate 'owner.json') -Encoding UTF8
                [IO.Directory]::Move($candidate,$LockPath)
                return [pscustomobject]@{path=$LockPath;owner=$owner;queued=$queued;wait_ms=[int]([DateTime]::UtcNow-$started).TotalMilliseconds}
            } catch {
                Remove-Item -LiteralPath $candidate -Recurse -Force -ErrorAction SilentlyContinue
                $owner=Get-ValidationOwner $LockPath
                if ($owner -and -not (Test-ValidationOwnerAlive $owner)) { [void](Remove-ValidationLockIfOwned -LockPath $LockPath -ExpectedOwner $owner); continue }
                if (-not $owner -and (Remove-OwnerlessValidationLockAfterGrace -LockPath $LockPath -OwnerPublishGraceMilliseconds $OwnerPublishGraceMilliseconds)) { continue }
                if (-not $queued) {
                    $queued=$true; Write-Host "VALIDATION_QUEUE=waiting kind=$Kind owner_pid=$($owner.pid) owner_lease=$($owner.lease_id)"
                    if ($PersistWaiter) {
                        New-Item -ItemType Directory -Force -Path (Split-Path $waiterPath -Parent) | Out-Null
                        [ordered]@{waiter_id=$waiterId;pid=$PID;process_start_id=(Get-ValidationProcessIdentity);kind=$Kind;queued_utc=[DateTime]::UtcNow.ToString('o')} | ConvertTo-Json | Set-Content -LiteralPath $waiterPath -Encoding UTF8
                        Get-ChildItem (Split-Path $waiterPath -Parent) -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTimeUtc -Descending | Select-Object -Skip 64 | Remove-Item -Force -ErrorAction SilentlyContinue
                    }
                }
                if ([DateTime]::UtcNow -ge $deadline) { throw "Timed out waiting for validation resource: $LockPath" }
                Start-Sleep -Milliseconds $PollMilliseconds
            }
        }
    } finally {
        if ($PersistWaiter -and (Test-Path -LiteralPath $waiterPath)) {
            try { $waiter=Get-Content -Raw $waiterPath|ConvertFrom-Json; $waiter|Add-Member -NotePropertyName finished_utc -NotePropertyValue ([DateTime]::UtcNow.ToString('o')) -Force; $waiter|ConvertTo-Json|Set-Content -LiteralPath $waiterPath -Encoding UTF8 } catch {}
        }
    }
}

function Exit-ValidationLock { param($Lease); if ($Lease) { [void](Remove-ValidationLockIfOwned -LockPath $Lease.path -ExpectedOwner $Lease.owner) } }

function Get-ValidationResourceClass {
    param([Parameter(Mandatory)][string[]]$CargoArgs)
    $policy=Get-Content -Raw (Join-Path $PSScriptRoot 'policy.json') | ConvertFrom-Json
    $verb=if($CargoArgs.Count){$CargoArgs[0].ToLowerInvariant()}else{'unknown'}
    if ($verb -in @($policy.heavy_verbs) -or @($policy.heavy_flags | Where-Object { $CargoArgs -contains $_ }).Count) { return 'heavy' }; return 'light'
}

function Enter-ValidationResource {
    param([Parameter(Mandatory)][string]$StateRoot,[Parameter(Mandatory)][string]$Class,[int]$LightSlots=2,[int]$TimeoutSeconds=3600)
    $resourceRoot=Join-Path $StateRoot 'resources'; $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    if($Class -eq 'heavy') {
        $gate=Enter-ValidationLock (Join-Path $resourceRoot 'heavy.lock') 'heavy' $TimeoutSeconds
        $slots=@()
        try { for($i=0;$i -lt $LightSlots;$i++){ $remaining=[Math]::Max(0,[int]($deadline-[DateTime]::UtcNow).TotalSeconds); $slots+=Enter-ValidationLock (Join-Path $resourceRoot "light-$i.lock") "heavy-reserved-$i" $remaining }; return [pscustomobject]@{class='heavy';leases=@($gate)+$slots;wait_ms=[int](@($gate)+$slots|Measure-Object wait_ms -Sum).Sum} } catch { @($slots)+@($gate)|ForEach-Object{Exit-ValidationLock $_}; throw }
    }
    while([DateTime]::UtcNow -lt $deadline) {
        if(Test-Path (Join-Path $resourceRoot 'heavy.lock')) { Start-Sleep -Milliseconds 100; continue }
        for($i=0;$i -lt $LightSlots;$i++) { try { $slot=Enter-ValidationLock (Join-Path $resourceRoot "light-$i.lock") "light-$i" 0; if(Test-Path (Join-Path $resourceRoot 'heavy.lock')){Exit-ValidationLock $slot;break}; return [pscustomobject]@{class='light';leases=@($slot);wait_ms=$slot.wait_ms} } catch {} }
        Start-Sleep -Milliseconds 100
    }; throw 'Timed out waiting for a lightweight validation slot.'
}

function Exit-ValidationResource { param($Lease); if($Lease){ @($Lease.leases)|ForEach-Object{Exit-ValidationLock $_} } }
Export-ModuleMember -Function Get-ValidationProcessIdentity,Get-ValidationOwner,Test-ValidationOwnerAlive,Test-ValidationLeaseMatches,Remove-OwnerlessValidationLockAfterGrace,Enter-ValidationLock,Exit-ValidationLock,Get-ValidationResourceClass,Enter-ValidationResource,Exit-ValidationResource
