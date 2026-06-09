<#
.SYNOPSIS
    Elon task completion check.

.DESCRIPTION
    Use this before the final AI report. CodePushed/CodeSync verifies that
    local HEAD is already contained in origin/main, even if newer commits have
    landed. AndroidFeature verifies that local
    HEAD equals origin/main and that server /app/version.json points at the
    pushed source commit. APK version numbers are assigned by the server.
    Server verifies /health and /api/server/version.

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
$ServerCargoPath = Join-Path $RepoRoot "server\Cargo.toml"
$ServerUrl = "http://43.139.149.158:8080"

function Stop-Check {
    param([string]$Message)
    Write-Error $Message
    exit 1
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

git fetch origin main | Out-Null
if ($LASTEXITCODE -ne 0) {
    Stop-Check "Could not fetch origin/main; remote state is unknown."
}

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
    Write-Host "  version:     v$($remoteVersion.versionName) (build $($remoteVersion.versionCode))"
    Write-Host "  APK gitSha:  $remoteGitSha"
    Write-Host "  download:    $ServerUrl/app/ElonSpeed-latest.apk"
    exit 0
}

if ($Kind -eq "DocsOnly") {
    Write-Host "DocsOnly completion check passed:" -ForegroundColor Green
    Write-Host "  HEAD:        $($head.Substring(0, 7))"
    Write-Host "  origin/main: $($originMain.Substring(0, 7))"
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

    if (-not (Test-Path $ServerCargoPath)) {
        Stop-Check "server/Cargo.toml was not found: $ServerCargoPath"
    }
    $serverCargo = Get-Content $ServerCargoPath -Encoding UTF8 -Raw
    $localServerVersion = [regex]::Match($serverCargo, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
    if (-not $localServerVersion) {
        Stop-Check "Could not read server package version from server/Cargo.toml."
    }

    try {
        $serverVersion = Invoke-RestMethod "$ServerUrl/api/server/version" -TimeoutSec 10
    } catch {
        Stop-Check "Server version check failed: $_"
    }

    if ([string]$serverVersion.versionName -ne $localServerVersion) {
        Stop-Check "Server version mismatch: local v$localServerVersion, server v$($serverVersion.versionName)."
    }

    Write-Host "Server completion check passed:" -ForegroundColor Green
    Write-Host "  HEAD:        $($head.Substring(0, 7))"
    Write-Host "  origin/main: $($originMain.Substring(0, 7))"
    Write-Host "  health:      $($health | ConvertTo-Json -Compress)"
    Write-Host "  version:     v$($serverVersion.versionName) ($($serverVersion.gitSha))"
    exit 0
}
