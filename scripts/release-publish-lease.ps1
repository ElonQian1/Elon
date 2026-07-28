Set-StrictMode -Version Latest
$script:ReleasePublishLeaseScriptPath = $PSCommandPath

function Invoke-ElonReleaseLeaseRequest {
    param(
        [Parameter(Mandatory)][string]$Uri,
        [string]$Method = 'GET',
        [object]$Body = $null,
        [int]$TimeoutSec = 20
    )
    $bodyFile = [System.IO.Path]::GetTempFileName()
    $responseFile = [System.IO.Path]::GetTempFileName()
    try {
        $args = @('--noproxy', '*', '-sS', '--max-time', $TimeoutSec, '-o', $responseFile,
            '-w', '%{http_code}', '-X', $Method)
        if ($null -ne $Body) {
            [System.IO.File]::WriteAllText(
                $bodyFile,
                ($Body | ConvertTo-Json -Depth 8 -Compress),
                [System.Text.UTF8Encoding]::new($false)
            )
            $args += @('-H', 'Content-Type: application/json; charset=utf-8', '--data-binary', "@$bodyFile")
        }
        $args += $Uri
        $statusText = (& curl.exe @args 2>&1) -join ''
        if ($LASTEXITCODE -ne 0) { throw "release API curl failed: $Uri (exit=$LASTEXITCODE)" }
        $status = 0
        if (-not [int]::TryParse($statusText.Trim(), [ref]$status)) { throw "invalid release API status: $statusText" }
        $responseText = Get-Content -LiteralPath $responseFile -Raw -Encoding UTF8
        if ($status -lt 200 -or $status -ge 300) { throw "release API HTTP ${status}: $responseText" }
        if ([string]::IsNullOrWhiteSpace($responseText)) { return $null }
        return $responseText | ConvertFrom-Json
    } finally {
        Remove-Item -LiteralPath $bodyFile, $responseFile -Force -ErrorAction SilentlyContinue
    }
}

function Test-ElonNodeAgentLeaseBootstrapFallback {
    param([Parameter(Mandatory)][string]$Message)

    return $Message.Contains('release API HTTP 400:') `
        -and $Message.Contains('"error":"bad-kind"') `
        -and $Message.Contains('node_agent')
}

function Get-ElonReleaseBatchId {
    param([Parameter(Mandatory)][string]$Sha)
    $configured = [string]$env:ELON_RELEASE_BATCH_ID
    if (-not [string]::IsNullOrWhiteSpace($configured)) { return $configured.Trim() }
    return "release-$($Sha.Trim())"
}

function Wait-ElonGlobalPublishLease {
    param(
        [Parameter(Mandatory)][object]$Claim,
        [Parameter(Mandatory)][string]$Kind,
        [Parameter(Mandatory)][string]$ReleaseApiBase,
        [int]$LeaseSecs = 180
    )
    if ($Claim.action -eq 'coalesced') { return $Claim }
    while ($Claim.action -eq 'wait') {
        Write-Host "   Global publish lease waiting (FIFO $($Claim.queuePosition)); heartbeat active..." -ForegroundColor Yellow
        Start-Sleep -Seconds 5
        $escapedToken = [Uri]::EscapeDataString([string]$Claim.token)
        $status = Invoke-ElonReleaseLeaseRequest -Uri "$ReleaseApiBase/status?token=$escapedToken&compact=true" -TimeoutSec 60
        $tokenStatus = $status.tokenStatus
        if (-not $tokenStatus) { throw 'release/status did not return tokenStatus' }
        switch ([string]$tokenStatus.action) {
            'build' { return $tokenStatus }
            'coalesced' { return $tokenStatus }
            'finished' {
                if ($tokenStatus.success) { return $tokenStatus }
                throw "queued publish failed: $($tokenStatus.errorMessage)"
            }
            'wait' {
                $Claim.queuePosition = [int]$tokenStatus.queuePosition
                try {
                    Invoke-ElonReleaseLeaseRequest -Uri "$ReleaseApiBase/heartbeat" -Method POST -Body @{
                        kind = $Kind; token = [string]$Claim.token; leaseSecs = $LeaseSecs
                        sha = [string]$Claim.sha; batchId = [string]$Claim.batchId
                        stage = [string]$Claim.stage; stageStatus = 'queued'
                    } | Out-Null
                } catch {
                    if (-not $_.Exception.Message.Contains('running release stage cannot return to queued')) {
                        throw
                    }
                    # The previous owner may finish between status and heartbeat. In that race the
                    # server promotes this token to owner/running, so re-read instead of regressing
                    # the durable stage to queued or failing a valid publication.
                    $status = Invoke-ElonReleaseLeaseRequest -Uri "$ReleaseApiBase/status?token=$escapedToken&compact=true" -TimeoutSec 60
                    $tokenStatus = $status.tokenStatus
                    if (-not $tokenStatus) { throw 'release/status did not return tokenStatus after promotion race' }
                    if ([string]$tokenStatus.action -eq 'build') { return $tokenStatus }
                    if ([string]$tokenStatus.action -eq 'coalesced') { return $tokenStatus }
                    if ([string]$tokenStatus.action -eq 'finished' -and $tokenStatus.success) {
                        return $tokenStatus
                    }
                    throw
                }
            }
            default { throw "publish lease became invalid: $($tokenStatus | ConvertTo-Json -Compress)" }
        }
    }
    return $Claim
}

function Enter-ElonGlobalPublishLease {
    param(
        [Parameter(Mandatory)][object]$Claim,
        [Parameter(Mandatory)][string]$Kind,
        [Parameter(Mandatory)][string]$ReleaseApiBase,
        [int]$LeaseSecs = 180
    )
    $result = Wait-ElonGlobalPublishLease @PSBoundParameters
    if ($result.action -eq 'coalesced' -or ($result.action -eq 'finished' -and $result.success)) {
        Write-Host '   Same SHA already published; coalesced without rebuilding.' -ForegroundColor Green
        return $null
    }
    if ($result.action -ne 'build') {
        throw "global publish lease was not granted: $($result | ConvertTo-Json -Compress)"
    }
    return $result
}

function Enter-ElonNodeAgentPublishLease {
    param(
        [Parameter(Mandatory)][string]$ReleaseApiBase,
        [Parameter(Mandatory)][string]$Sha,
        [Parameter(Mandatory)][string]$VersionName,
        [Parameter(Mandatory)][string]$BuilderId
    )
    try {
        $claim = Invoke-ElonReleaseLeaseRequest -Uri "$ReleaseApiBase/claim" -Method POST -Body @{
            kind = 'node_agent'; sha = $Sha; builderId = $BuilderId
            builderLabel = "publish-node-agent.ps1 @ $BuilderId"
            currentVersionName = $VersionName; leaseSecs = 180
            batchId = Get-ElonReleaseBatchId -Sha $Sha; stage = 'windows_node'
        }
    } catch {
        if (-not (Test-ElonNodeAgentLeaseBootstrapFallback -Message $_.Exception.Message)) { throw }
        Write-Warning 'The release API predates the node_agent lane; bootstrap with the local lock and pre-upload SHA gate. The global lease will activate after the server upgrade.'
        return [pscustomobject]@{
            action = 'build'
            token = ''
            legacyFallback = $true
        }
    }
    $result = Wait-ElonGlobalPublishLease -Claim $claim -Kind 'node_agent' `
        -ReleaseApiBase $ReleaseApiBase -LeaseSecs 180
    if ($result.action -eq 'coalesced' -or ($result.action -eq 'finished' -and $result.success)) {
        Write-Host '   Same node SHA already published; entering artifact-verified broadcast replay.' -ForegroundColor Green
        $result | Add-Member -NotePropertyName replayOnly -NotePropertyValue $true -Force
        return $result
    }
    if ($result.action -ne 'build') {
        throw "global node publish lease was not granted: $($result | ConvertTo-Json -Compress)"
    }
    return $result
}

function Update-ElonReleaseStage {
    param(
        [Parameter(Mandatory)][string]$ReleaseApiBase,
        [Parameter(Mandatory)][string]$Kind,
        [Parameter(Mandatory)][string]$Token,
        [Parameter(Mandatory)][string]$Sha,
        [Parameter(Mandatory)][string]$BatchId,
        [Parameter(Mandatory)][string]$Stage,
        [string]$Phase = '',
        [ValidateSet('queued','running','succeeded','failed')][string]$Status = 'running',
        [int]$LeaseSecs = 180
    )
    if ([string]::IsNullOrWhiteSpace($Token)) { return }
    $stageStatus = if ([string]::IsNullOrWhiteSpace($Phase)) { $Status } else { 'running' }
    $body = @{
        kind = $Kind; token = $Token; leaseSecs = $LeaseSecs
        sha = $Sha; batchId = $BatchId; stage = $Stage; stageStatus = $stageStatus
    }
    if (-not [string]::IsNullOrWhiteSpace($Phase)) {
        $body.phase = $Phase
        $body.phaseStatus = $Status
    }
    Invoke-ElonReleaseLeaseRequest -Uri "$ReleaseApiBase/heartbeat" -Method POST -Body $body | Out-Null
}

function Start-ElonReleaseHeartbeat {
    param(
        [Parameter(Mandatory)][string]$ReleaseApiBase,
        [Parameter(Mandatory)][string]$Kind,
        [Parameter(Mandatory)][string]$Token,
        [Parameter(Mandatory)][string]$Sha,
        [Parameter(Mandatory)][string]$BatchId,
        [Parameter(Mandatory)][string]$Stage,
        [int]$IntervalSecs = 30,
        [int]$LeaseSecs = 180
    )
    if ([string]::IsNullOrWhiteSpace($Token)) { return $null }
    # The caller observes the first heartbeat synchronously. A broken API or
    # rejected identity must stop the long operation before it begins.
    Update-ElonReleaseStage -ReleaseApiBase $ReleaseApiBase -Kind $Kind -Token $Token `
        -Sha $Sha -BatchId $BatchId -Stage $Stage -Status 'running' -LeaseSecs $LeaseSecs
    $helperPath = $script:ReleasePublishLeaseScriptPath
    if ([string]::IsNullOrWhiteSpace($helperPath)) {
        throw "无法解析 release heartbeat helper 路径"
    }
    return Start-Job -ScriptBlock {
        param($Path, $Api, $LeaseKind, $LeaseToken, $ReleaseSha, $Batch, $LeaseStage, $Interval, $Lease)
        . $Path
        while ($true) {
            Start-Sleep -Seconds $Interval
            Update-ElonReleaseStage -ReleaseApiBase $Api -Kind $LeaseKind -Token $LeaseToken `
                -Sha $ReleaseSha -BatchId $Batch -Stage $LeaseStage -Status 'running' -LeaseSecs $Lease
        }
    } -ArgumentList $helperPath, $ReleaseApiBase, $Kind, $Token, $Sha, $BatchId, $Stage, $IntervalSecs, $LeaseSecs
}

function New-ElonReleaseStageContext {
    param(
        [Parameter(Mandatory)][string]$ReleaseApiBase,
        [Parameter(Mandatory)][string]$Kind,
        [Parameter(Mandatory)][string]$Token,
        [Parameter(Mandatory)][string]$Sha,
        [Parameter(Mandatory)][string]$BatchId,
        [Parameter(Mandatory)][string]$Stage
    )
    [pscustomobject]@{ ReleaseApiBase = $ReleaseApiBase; Kind = $Kind; Token = $Token; Sha = $Sha; BatchId = $BatchId; Stage = $Stage }
}

function Set-ElonReleasePhase {
    param(
        [Parameter(Mandatory)][object]$Context,
        [Parameter(Mandatory)][string]$Phase,
        [ValidateSet('queued','running','succeeded','failed')][string]$Status = 'running'
    )
    Update-ElonReleaseStage -ReleaseApiBase $Context.ReleaseApiBase -Kind $Context.Kind `
        -Token $Context.Token -Sha $Context.Sha -BatchId $Context.BatchId `
        -Stage $Context.Stage -Phase $Phase -Status $Status
}

function Start-ElonReleaseContextHeartbeat {
    param([Parameter(Mandatory)][object]$Context)
    Start-ElonReleaseHeartbeat -ReleaseApiBase $Context.ReleaseApiBase -Kind $Context.Kind `
        -Token $Context.Token -Sha $Context.Sha -BatchId $Context.BatchId -Stage $Context.Stage
}

function Stop-ElonReleaseHeartbeat {
    param([object]$HeartbeatJob)
    if ($null -eq $HeartbeatJob) { return }
    $failure = $null
    if ($HeartbeatJob.State -eq 'Failed') {
        $failure = $HeartbeatJob.ChildJobs[0].JobStateInfo.Reason
    }
    try {
        Receive-Job -Job $HeartbeatJob -Keep -ErrorAction Stop | Out-Null
    } catch {
        $failure = $_.Exception
    }
    if ($HeartbeatJob.State -eq 'Running') { Stop-Job -Job $HeartbeatJob -ErrorAction Stop }
    Remove-Job -Job $HeartbeatJob -Force -ErrorAction Stop
    if ($null -ne $failure) { throw "release heartbeat failed closed: $failure" }
}

function Complete-ElonReleaseLease {
    param(
        [Parameter(Mandatory)][string]$ReleaseApiBase,
        [Parameter(Mandatory)][string]$Kind,
        [Parameter(Mandatory)][string]$Token,
        [Parameter(Mandatory)][bool]$Success,
        [Parameter(Mandatory)][string]$Sha,
        [Parameter(Mandatory)][string]$BatchId,
        [Parameter(Mandatory)][string]$Stage,
        [string]$VersionName = '',
        [int]$VersionCode = 0,
        [string]$ErrorMessage = ''
    )
    $body = @{
        kind = $Kind; token = $Token; success = $Success
        sha = $Sha; batchId = $BatchId; stage = $Stage
    }
    if ($Success) {
        if ($VersionName) { $body.versionName = $VersionName }
        if ($VersionCode -gt 0) { $body.versionCode = $VersionCode }
    } elseif ($ErrorMessage) {
        $body.errorMessage = $ErrorMessage
    }
    Invoke-ElonReleaseLeaseRequest -Uri "$ReleaseApiBase/finish" -Method POST -Body $body | Out-Null
}

function Complete-ElonReleaseContext {
    param(
        [Parameter(Mandatory)][object]$Context,
        [Parameter(Mandatory)][bool]$Success,
        [string]$VersionName = '',
        [int]$VersionCode = 0,
        [string]$ErrorMessage = ''
    )
    Complete-ElonReleaseLease -ReleaseApiBase $Context.ReleaseApiBase -Kind $Context.Kind `
        -Token $Context.Token -Success $Success -Sha $Context.Sha -BatchId $Context.BatchId `
        -Stage $Context.Stage -VersionName $VersionName -VersionCode $VersionCode -ErrorMessage $ErrorMessage
}
