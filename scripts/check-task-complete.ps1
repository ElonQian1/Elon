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
    the pushed source commit. Server version numbers are assigned by the
    release claim API and are not compared with server/Cargo.toml.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodeSync

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
#>
param(
    [ValidateSet("CodePushed", "CodeSync", "AndroidFeature", "DocsOnly", "Server")]
    [string]$Kind = "CodePushed",

    [switch]$SkipGitStatus
)

$ErrorActionPreference = "Stop"

$RepoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
$ServerUrl = "http://43.139.149.158:8080"

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
            $output = & git @GitArgs 2>&1
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
        $remoteVersion = Invoke-RestMethod "$ServerUrl/app/version.json" -TimeoutSec 10
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
        $apkHead = Invoke-WebRequest -Uri "$ServerUrl/app/ElonSpeed-latest.apk" -Method Head -TimeoutSec 10 -UseBasicParsing
    } catch {
        try {
            $apkHead = Invoke-WebRequest -Uri "$ServerUrl/app/ElonSpeed-latest.apk" -Method Get -Headers @{ Range = "bytes=0-0" } -TimeoutSec 10 -UseBasicParsing
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
    Write-Host "  APK_RELEASE_STATUS=published"
    Write-Host "  SERVER_RELEASE_STATUS=not_attempted"
    Write-Host "  version:     v$($remoteVersion.versionName) (build $($remoteVersion.versionCode))"
    Write-Host "  APK gitSha:  $remoteGitSha"
    Write-Host "  download:    $ServerUrl/app/ElonSpeed-latest.apk"
    exit 0
}

if ($Kind -eq "DocsOnly") {
    Write-Host "DocsOnly completion check passed:" -ForegroundColor Green
    Write-Host "  HEAD:        $($head.Substring(0, 7))"
    Write-Host "  origin/main: $($originMain.Substring(0, 7))"
    Write-Host "  CODE_SYNC_STATUS=synced"
    Write-Host "  APK_RELEASE_STATUS=not_attempted"
    Write-Host "  SERVER_RELEASE_STATUS=not_attempted"
    exit 0
}

if ($Kind -eq "Server") {
    if ($head -ne $originMain) {
        Stop-Check "Server task is not complete: HEAD is not pushed to origin/main."
    }

    try {
        $health = Invoke-RestMethod "$ServerUrl/health" -TimeoutSec 10
    } catch {
        Stop-Check "Server health check failed: $_"
    }

    try {
        $serverVersion = Invoke-RestMethod "$ServerUrl/api/server/version" -TimeoutSec 10
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

    Write-Host "Server completion check passed:" -ForegroundColor Green
    Write-Host "  HEAD:        $($head.Substring(0, 7))"
    Write-Host "  origin/main: $($originMain.Substring(0, 7))"
    Write-Host "  CODE_SYNC_STATUS=synced"
    Write-Host "  APK_RELEASE_STATUS=not_attempted"
    Write-Host "  SERVER_RELEASE_STATUS=published"
    Write-Host "  health:      $($health | ConvertTo-Json -Compress)"
    Write-Host "  version:     v$($serverVersion.versionName) ($($serverVersion.gitSha))"
    exit 0
}
