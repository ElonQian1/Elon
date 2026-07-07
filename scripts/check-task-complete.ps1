<#
.SYNOPSIS
    Elon task completion check.

.DESCRIPTION
    Use this before the final AI report. CodePushed/CodeSync verifies that
    local HEAD is already contained in origin/main, even if newer commits have
    landed. AndroidFeature verifies that local
    HEAD equals origin/main and that server /app/version.json points at the
    pushed source commit. APK version numbers are assigned by the server.
    Server verifies /health and that /api/server/version.gitSha points at
    the pushed source commit. PcFrontend performs the same server release
    provenance check and also verifies that /pc serves the built frontend
    shell, because /pc user-visible changes are delivered through
    pc-next-dist during server publish. Server version numbers are assigned by
    the release claim API and are not compared with server/Cargo.toml.
    NodeAgent verifies that /api/node-agent/version and the Windows download
    endpoints point at the pushed source commit.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodeSync

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind NodeAgent

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind PcFrontend
#>
param(
    [ValidateSet("CodePushed", "CodeSync", "AndroidFeature", "NodeAgent", "DocsOnly", "Server", "PcFrontend")]
    [string]$Kind = "CodePushed",

    [switch]$SkipGitStatus
)

$ErrorActionPreference = "Stop"

$RepoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
$ServerUrl = "http://43.139.149.158:8080"

. (Join-Path $PSScriptRoot "direct-network.ps1")

Set-ElonProjectDirectNetwork
Set-Location $RepoRoot
Set-ElonProjectDirectGitSsh

function Stop-Check {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-GitFetchFailureHint {
    param([string]$Output)

    $text = if ($Output) { $Output } else { "" }
    if ($text -match '(Could not resolve host|Name or service not known|Temporary failure in name resolution)') {
        return "网络/DNS 无法解析 GitHub，请检查网络、DNS 或代理后重试。"
    }
    if ($text -match '(Failed to connect|Connection timed out|Connection reset|Connection refused|Operation timed out|HTTP/2 stream|early EOF|The remote end hung up unexpectedly)') {
        return "网络连接到 GitHub 不稳定或超时，通常是临时抖动；脚本已短重试但仍失败。"
    }
    if ($text -match '(Permission denied|Authentication failed|Repository not found|Could not read from remote repository|Host key verification failed|publickey)') {
        return "Git 远端认证或仓库权限异常，请检查 SSH key、GitHub 权限和 origin 地址。"
    }
    return "Git fetch 失败，原因未能自动分类；请查看原始输出。"
}

function Invoke-GitFetchWithRetry {
    param(
        [string[]]$GitArgs = @("fetch", "origin", "main"),
        [int]$Attempts = 3,
        [int]$DelaySeconds = 2
    )

    $lastOutput = ""
    for ($i = 1; $i -le $Attempts; $i++) {
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = & git -c http.proxy= -c https.proxy= @GitArgs 2>&1
        } finally {
            $ErrorActionPreference = $oldPreference
        }
        $lastOutput = ($output -join "`n").Trim()
        if ($LASTEXITCODE -eq 0) {
            if ($i -gt 1) {
                Write-Host "GIT_FETCH_RETRY=success_after_$i"
            }
            return
        }

        $hint = Get-GitFetchFailureHint -Output $lastOutput
        Write-Host "GIT_FETCH_RETRY=attempt_$i/$Attempts failed: $hint" -ForegroundColor Yellow
        if ($i -lt $Attempts) {
            Start-Sleep -Seconds $DelaySeconds
        }
    }

    $finalHint = Get-GitFetchFailureHint -Output $lastOutput
    Stop-Check "无法确认远端 main 状态：git $($GitArgs -join ' ') 连续失败 $Attempts 次。$finalHint 原始输出：$lastOutput"
}

function Get-GitWorktreeEntries {
    $entries = @()
    $current = @{}
    foreach ($line in (& git worktree list --porcelain)) {
        if ($line -eq "") {
            if ($current.Count -gt 0) {
                $entries += [pscustomobject]$current
                $current = @{}
            }
            continue
        }
        $kv = $line -split " ", 2
        switch ($kv[0]) {
            "worktree" { $current["Path"] = $kv[1] }
            "HEAD"     { $current["Head"] = $kv[1] }
            "branch"   { $current["Branch"] = ($kv[1] -replace "^refs/heads/","") }
            "bare"     { $current["Bare"] = $true }
            "detached" { $current["Detached"] = $true }
        }
    }
    if ($current.Count -gt 0) {
        $entries += [pscustomobject]$current
    }
    return $entries
}

function Get-CleanupStatusFromGitOutput {
    param([string]$Output)

    $lower = $Output.ToLowerInvariant()
    if (
        $lower.Contains("non-fast-forward") -or
        $lower.Contains("ff-only") -or
        $lower.Contains("fast-forward") -or
        $lower.Contains("diverg") -or
        $lower.Contains("fetch first")
    ) {
        return "local_main_diverged"
    }
    return "cleanup_failed"
}

function Sync-MainWorktreeIfClean {
    & git rev-parse --verify origin/main *> $null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  MAIN_BASELINE_SYNC=skipped_no_origin_main"
        return
    }

    & git worktree prune *> $null
    $mainWorktree = Get-GitWorktreeEntries |
        Where-Object { $_.Branch -eq "main" -and $_.Path } |
        Select-Object -First 1
    if (-not $mainWorktree) {
        Write-Host "  MAIN_BASELINE_SYNC=skipped_no_main_worktree"
        return
    }

    $status = (& git -C $mainWorktree.Path status --porcelain=v1 --untracked-files=normal)
    if (-not [string]::IsNullOrWhiteSpace(($status -join "`n"))) {
        Write-Host "  MAIN_BASELINE_SYNC=blocked_dirty:$($mainWorktree.Path)"
        Write-Host "  CLEANUP_STATUS=cleanup_failed"
        Write-Host "  CLEANUP_STATUS_DETAIL=main_baseline_dirty_not_task_failure"
        return
    }

    $before = (& git -C $mainWorktree.Path rev-parse --short HEAD).Trim()
    $mergeOutput = & git -C $mainWorktree.Path merge --ff-only origin/main 2>&1
    if ($LASTEXITCODE -ne 0) {
        $cleanupStatus = Get-CleanupStatusFromGitOutput -Output ($mergeOutput -join ' ')
        Write-Host "  MAIN_BASELINE_SYNC=failed:${cleanupStatus}:$($mergeOutput -join ' ')" -ForegroundColor Yellow
        Write-Host "  CLEANUP_STATUS=$cleanupStatus"
        Write-Host "  CLEANUP_STATUS_DETAIL=main_baseline_sync_warning_not_task_failure"
        return
    }
    $after = (& git -C $mainWorktree.Path rev-parse --short HEAD).Trim()
    if ($before -eq $after) {
        Write-Host "  MAIN_BASELINE_SYNC=already_current:$after"
    } else {
        Write-Host "  MAIN_BASELINE_SYNC=fast_forwarded:$before->$after"
    }
}

function Test-DownloadUrl {
    param(
        [Parameter(Mandatory = $true)][string]$Url,
        [Parameter(Mandatory = $true)][string]$Label
    )

    try {
        $headParams = @{
            Uri = $Url
            Method = "Head"
            TimeoutSec = 10
            UseBasicParsing = $true
        }
        $headParams = Add-ElonProjectDirectRequestParameters -Params $headParams -CommandName "Invoke-WebRequest"
        $response = Invoke-WebRequest @headParams
    } catch {
        try {
            $getParams = @{
                Uri = $Url
                Method = "Get"
                Headers = @{ Range = "bytes=0-0" }
                TimeoutSec = 10
                UseBasicParsing = $true
            }
            $getParams = Add-ElonProjectDirectRequestParameters -Params $getParams -CommandName "Invoke-WebRequest"
            $response = Invoke-WebRequest @getParams
        } catch {
            Stop-Check "$Label download URL is unavailable: $_"
        }
    }

    if ($response.StatusCode -lt 200 -or $response.StatusCode -ge 400) {
        Stop-Check "$Label download URL returned unexpected status: $($response.StatusCode)"
    }

    return $response.StatusCode
}

Set-Location $RepoRoot

if (-not $SkipGitStatus) {
    $status = git status --short
    if ($status) {
        Write-Host "Working tree still has uncommitted changes:" -ForegroundColor Yellow
        $status | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
        Stop-Check "Task is not complete: working tree is not clean."
    }
}

Invoke-GitFetchWithRetry -GitArgs @("fetch", "origin", "main")
Sync-MainWorktreeIfClean

$head = (git rev-parse HEAD).Trim()
$originMain = (git rev-parse origin/main).Trim()

if ($Kind -eq "CodePushed" -or $Kind -eq "CodeSync") {
    git merge-base --is-ancestor $head $originMain | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Stop-Check "Code push is not complete: local HEAD is not contained in origin/main. HEAD=$($head.Substring(0, 7)) origin/main=$($originMain.Substring(0, 7))"
    }

    Write-Host "$Kind completion check passed:" -ForegroundColor Green
    Write-Host "  HEAD:        $($head.Substring(0, 7))"
    Write-Host "  origin/main: $($originMain.Substring(0, 7))"
    Write-Host "  CODE_SYNC_STATUS=synced"
    Write-Host "  NODE_AGENT_RELEASE_STATUS=not_attempted"
    Write-Host "  APK_RELEASE_STATUS=not_attempted"
    Write-Host "  SERVER_RELEASE_STATUS=not_attempted"
    if ($head -eq $originMain) {
        Write-Host "  status:      local HEAD is the current origin/main tip"
    } else {
        Write-Host "  status:      local HEAD is already contained in origin/main"
    }
    exit 0
}

if ($Kind -eq "AndroidFeature") {
    if ($head -ne $originMain) {
        Stop-Check "Android task is not complete: HEAD is not pushed to origin/main. HEAD=$($head.Substring(0, 7)) origin/main=$($originMain.Substring(0, 7))"
    }

    try {
        $remoteVersionParams = @{
            Uri = "$ServerUrl/app/version.json"
            TimeoutSec = 10
        }
        $remoteVersionParams = Add-ElonProjectDirectRequestParameters -Params $remoteVersionParams -CommandName "Invoke-RestMethod"
        $remoteVersion = Invoke-RestMethod @remoteVersionParams
    } catch {
        Stop-Check "Could not read server /app/version.json: $_"
    }

    $remoteGitSha = [string]$remoteVersion.gitSha
    if ([string]::IsNullOrWhiteSpace($remoteGitSha)) {
        Stop-Check "Server /app/version.json does not include gitSha; APK deploy provenance is unknown."
    }

    if (-not ($head.StartsWith($remoteGitSha) -or $remoteGitSha.StartsWith($head))) {
        Stop-Check "Server APK gitSha mismatch: local HEAD=$($head.Substring(0, 7)), server gitSha=$remoteGitSha."
    }

    try {
        $apkHeadParams = @{
            Uri = "$ServerUrl/app/ElonSpeed-latest.apk"
            Method = "Head"
            TimeoutSec = 10
            UseBasicParsing = $true
        }
        $apkHeadParams = Add-ElonProjectDirectRequestParameters -Params $apkHeadParams -CommandName "Invoke-WebRequest"
        $apkHead = Invoke-WebRequest @apkHeadParams
    } catch {
        try {
            $apkGetParams = @{
                Uri = "$ServerUrl/app/ElonSpeed-latest.apk"
                Method = "Get"
                Headers = @{ Range = "bytes=0-0" }
                TimeoutSec = 10
                UseBasicParsing = $true
            }
            $apkGetParams = Add-ElonProjectDirectRequestParameters -Params $apkGetParams -CommandName "Invoke-WebRequest"
            $apkHead = Invoke-WebRequest @apkGetParams
        } catch {
            Stop-Check "APK download URL is unavailable: $_"
        }
    }

    if ($apkHead.StatusCode -lt 200 -or $apkHead.StatusCode -ge 400) {
        Stop-Check "APK download URL returned unexpected status: $($apkHead.StatusCode)"
    }

    Write-Host "Android APK completion check passed:" -ForegroundColor Green
    Write-Host "  HEAD:        $($head.Substring(0, 7))"
    Write-Host "  origin/main: $($originMain.Substring(0, 7))"
    Write-Host "  CODE_SYNC_STATUS=synced"
    Write-Host "  NODE_AGENT_RELEASE_STATUS=not_attempted"
    Write-Host "  APK_RELEASE_STATUS=published"
    Write-Host "  SERVER_RELEASE_STATUS=not_attempted"
    Write-Host "  version:     v$($remoteVersion.versionName) (build $($remoteVersion.versionCode))"
    Write-Host "  APK gitSha:  $remoteGitSha"
    Write-Host "  download:    $ServerUrl/app/ElonSpeed-latest.apk"
    exit 0
}

if ($Kind -eq "NodeAgent") {
    git merge-base --is-ancestor $head $originMain | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Stop-Check "Node agent task is not complete: local HEAD is not contained in origin/main. HEAD=$($head.Substring(0, 7)) origin/main=$($originMain.Substring(0, 7))"
    }

    try {
        $nodeVersionParams = @{
            Uri = "$ServerUrl/api/node-agent/version"
            TimeoutSec = 10
        }
        $nodeVersionParams = Add-ElonProjectDirectRequestParameters -Params $nodeVersionParams -CommandName "Invoke-RestMethod"
        $nodeVersion = Invoke-RestMethod @nodeVersionParams
    } catch {
        Stop-Check "Could not read server /api/node-agent/version: $_"
    }

    $nodeGitSha = [string]$nodeVersion.gitSha
    if ([string]::IsNullOrWhiteSpace($nodeGitSha)) {
        Stop-Check "Server /api/node-agent/version does not include gitSha; node-agent deploy provenance is unknown."
    }

    if (-not ($head.StartsWith($nodeGitSha) -or $nodeGitSha.StartsWith($head))) {
        Stop-Check "Server node-agent gitSha mismatch: local HEAD=$($head.Substring(0, 7)), server gitSha=$nodeGitSha."
    }

    $windowsClientUrl = [string]$nodeVersion.windowsClientDownloadUrl
    if ([string]::IsNullOrWhiteSpace($windowsClientUrl)) {
        $windowsClientUrl = "$ServerUrl/api/node-agent/download/windows-client"
    }
    $windowsExeUrl = [string]$nodeVersion.downloadUrl
    if ([string]::IsNullOrWhiteSpace($windowsExeUrl)) {
        $windowsExeUrl = "$ServerUrl/api/node-agent/download/windows"
    }

    $clientStatus = Test-DownloadUrl -Url $windowsClientUrl -Label "Windows client package"
    $exeStatus = Test-DownloadUrl -Url $windowsExeUrl -Label "Windows node exe"

    Write-Host "Node agent completion check passed:" -ForegroundColor Green
    Write-Host "  HEAD:        $($head.Substring(0, 7))"
    Write-Host "  origin/main: $($originMain.Substring(0, 7))"
    Write-Host "  CODE_SYNC_STATUS=synced"
    Write-Host "  NODE_AGENT_RELEASE_STATUS=published"
    Write-Host "  APK_RELEASE_STATUS=not_attempted"
    Write-Host "  SERVER_RELEASE_STATUS=not_attempted"
    Write-Host "  version:     v$($nodeVersion.version)"
    Write-Host "  gitSha:      $nodeGitSha"
    Write-Host "  client zip:  $windowsClientUrl (HTTP $clientStatus)"
    Write-Host "  exe:         $windowsExeUrl (HTTP $exeStatus)"
    exit 0
}

if ($Kind -eq "DocsOnly") {
    Write-Host "DocsOnly completion check passed:" -ForegroundColor Green
    Write-Host "  HEAD:        $($head.Substring(0, 7))"
    Write-Host "  origin/main: $($originMain.Substring(0, 7))"
    Write-Host "  CODE_SYNC_STATUS=synced"
    Write-Host "  NODE_AGENT_RELEASE_STATUS=not_attempted"
    Write-Host "  APK_RELEASE_STATUS=not_attempted"
    Write-Host "  SERVER_RELEASE_STATUS=not_attempted"
    exit 0
}

if ($Kind -eq "Server" -or $Kind -eq "PcFrontend") {
    if ($head -ne $originMain) {
        Stop-Check "$Kind task is not complete: HEAD is not pushed to origin/main."
    }

    try {
        $healthParams = @{
            Uri = "$ServerUrl/health"
            TimeoutSec = 10
        }
        $healthParams = Add-ElonProjectDirectRequestParameters -Params $healthParams -CommandName "Invoke-RestMethod"
        $health = Invoke-RestMethod @healthParams
    } catch {
        Stop-Check "Server health check failed: $_"
    }

    try {
        $serverVersionParams = @{
            Uri = "$ServerUrl/api/server/version"
            TimeoutSec = 10
        }
        $serverVersionParams = Add-ElonProjectDirectRequestParameters -Params $serverVersionParams -CommandName "Invoke-RestMethod"
        $serverVersion = Invoke-RestMethod @serverVersionParams
    } catch {
        Stop-Check "Server version check failed: $_"
    }

    $serverSha = [string]$serverVersion.gitSha
    if ([string]::IsNullOrWhiteSpace($serverSha)) {
        Stop-Check "Server version check did not include gitSha; deployed provenance is unknown."
    }
    if ($serverSha -ne $head) {
        Stop-Check "Server gitSha mismatch: local $($head.Substring(0, 7)), server $($serverSha.Substring(0, [Math]::Min(7, $serverSha.Length)))."
    }

    $pcStatus = $null
    if ($Kind -eq "PcFrontend") {
        try {
            $pcParams = @{
                Uri = "$ServerUrl/pc"
                Method = "Get"
                TimeoutSec = 10
                UseBasicParsing = $true
            }
            $pcParams = Add-ElonProjectDirectRequestParameters -Params $pcParams -CommandName "Invoke-WebRequest"
            $pcResponse = Invoke-WebRequest @pcParams
        } catch {
            Stop-Check "PC frontend check failed: could not load /pc: $_"
        }

        if ($pcResponse.StatusCode -lt 200 -or $pcResponse.StatusCode -ge 400) {
            Stop-Check "PC frontend /pc returned unexpected status: $($pcResponse.StatusCode)"
        }
        $pcContent = [string]$pcResponse.Content
        if ($pcContent -notmatch '<div id="root"') {
            Stop-Check "PC frontend /pc did not return the React shell; pc-next-dist may not have been published."
        }
        $pcStatus = $pcResponse.StatusCode
    }

    if ($Kind -eq "PcFrontend") {
        Write-Host "PC frontend completion check passed:" -ForegroundColor Green
    } else {
        Write-Host "Server completion check passed:" -ForegroundColor Green
    }
    Write-Host "  HEAD:        $($head.Substring(0, 7))"
    Write-Host "  origin/main: $($originMain.Substring(0, 7))"
    Write-Host "  CODE_SYNC_STATUS=synced"
    Write-Host "  NODE_AGENT_RELEASE_STATUS=not_attempted"
    Write-Host "  APK_RELEASE_STATUS=not_attempted"
    Write-Host "  SERVER_RELEASE_STATUS=published"
    if ($Kind -eq "PcFrontend") {
        Write-Host "  PC_FRONTEND_RELEASE_STATUS=published"
        Write-Host "  /pc:         HTTP $pcStatus"
    }
    Write-Host "  health:      $($health | ConvertTo-Json -Compress)"
    Write-Host "  version:     v$($serverVersion.versionName) ($($serverVersion.gitSha))"
    exit 0
}
