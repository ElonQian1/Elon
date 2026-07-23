Set-StrictMode -Version Latest

function Wait-NodePublicDevHandshake {
    param(
        [string]$Token,
        [bool]$UseRemoteToken,
        [int]$TimeoutSec,
        [Parameter(Mandatory = $true)][string]$TargetReleaseIdentity
    )
    if ($SkipHandshakeWait -or $TimeoutSec -le 0) {
        Write-Host '  已跳过公开开发握手等待。' -ForegroundColor Yellow
        return
    }
    Write-Host "  等待在线公开开发节点重连并完成握手（最多 ${TimeoutSec}s）..." -ForegroundColor Yellow
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $lastReport = $null
    while ($true) {
        try {
            $status = Invoke-NodePublicDevHandshakeStatus -Token $Token -UseRemoteToken $UseRemoteToken
            if ($null -eq $status -or $null -eq $status.public_dev_handshake) {
                throw '服务器未返回公开开发握手诊断，无法确认目标构建已生效。'
            }
            $report = $status.public_dev_handshake
            $lastReport = $report
            $summary = $report.summary
            $onlineNodes = @($report.nodes | Where-Object { $_.public_dev_enabled -and $_.online })
            $targetReadyNodes = @($onlineNodes | Where-Object {
                Test-NodeAgentPublishHandshakeReady -Node $_ -TargetReleaseIdentity $TargetReleaseIdentity
            })
            Write-Host ("  目标构建握手：ready {0}/{1}，平台握手 ready {2}/{3}，offline {4}" -f `
                $targetReadyNodes.Count, $onlineNodes.Count, $summary.ready_public_dev, `
                $summary.public_dev_enabled, $summary.offline_public_dev) -ForegroundColor DarkGray
            $pending = @($onlineNodes | Where-Object {
                -not (Test-NodeAgentPublishHandshakeReady -Node $_ -TargetReleaseIdentity $TargetReleaseIdentity)
            })
            if ($pending.Count -eq 0) {
                Write-Host '  在线公开开发节点均已运行目标构建并完成握手。' -ForegroundColor Green
                Write-Output 'NODE_AGENT_TARGET_BUILD_STATUS=ready'
                Write-Output "NODE_AGENT_TARGET_BUILD_READY=$($targetReadyNodes.Count)"
                Write-Output 'NODE_AGENT_TARGET_BUILD_PENDING=0'
                return
            }
            $sample = @($pending | Select-Object -First 5 | ForEach-Object {
                $owner = if ($_.owner_nickname) { $_.owner_nickname } elseif ($_.owner_account) { $_.owner_account } else { $_.owner_user_id }
                $reported = if ($_.agent_version) { $_.agent_version } else { 'unknown-build' }
                "$($_.display_name)/$owner/$($_.public_dev_handshake_status)/$reported"
            })
            if ($sample.Count -gt 0) { Write-Host ('  待握手节点：' + ($sample -join '；')) -ForegroundColor DarkYellow }
        } catch {
            if ($RequireAllOnlineTargetBuild) { throw "公开开发握手诊断失败：$($_.Exception.Message)" }
            Write-Host "  公开开发握手诊断不可用；异步发布保留待核对状态：$($_.Exception.Message)" -ForegroundColor DarkYellow
            Write-Output 'NODE_AGENT_TARGET_BUILD_STATUS=unverified'
            return
        }
        if ((Get-Date) -ge $deadline) { break }
        Start-Sleep -Seconds 5
    }
    if ($null -ne $lastReport) {
        $pending = @($lastReport.nodes | Where-Object {
            $_.public_dev_enabled -and $_.online -and -not (
                Test-NodeAgentPublishHandshakeReady -Node $_ -TargetReleaseIdentity $TargetReleaseIdentity)
        })
        if ($pending.Count -gt 0) {
            $onlineCount = @($lastReport.nodes | Where-Object { $_.public_dev_enabled -and $_.online }).Count
            $readyCount = $onlineCount - $pending.Count
            Write-Output 'NODE_AGENT_TARGET_BUILD_STATUS=partial'
            Write-Output "NODE_AGENT_TARGET_BUILD_READY=$readyCount"
            Write-Output "NODE_AGENT_TARGET_BUILD_PENDING=$($pending.Count)"
            if ($RequireAllOnlineTargetBuild) { throw "公开开发握手等待超时，仍有 $($pending.Count) 个在线节点未达到目标构建。" }
            return
        }
    }
    if ($RequireAllOnlineTargetBuild) { throw '公开开发握手等待超时，未拿到完整目标报告。' }
    Write-Output 'NODE_AGENT_TARGET_BUILD_STATUS=unverified'
}
