<#
.SYNOPSIS
    Elon task completion check.

.DESCRIPTION
    Use this before the final AI report. AndroidFeature verifies that local
    HEAD equals origin/main and that server /app/version.json matches the
    local Android version.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
#>
param(
    [ValidateSet("AndroidFeature", "DocsOnly", "Server")]
    [string]$Kind = "AndroidFeature",

    [switch]$SkipGitStatus
)

$ErrorActionPreference = "Stop"

$RepoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
$GradlePath = Join-Path $RepoRoot "android\app\build.gradle"
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

if ($Kind -eq "AndroidFeature") {
    if ($head -ne $originMain) {
        Stop-Check "Android task is not complete: HEAD is not pushed to origin/main. HEAD=$($head.Substring(0, 7)) origin/main=$($originMain.Substring(0, 7))"
    }

    if (-not (Test-Path $GradlePath)) {
        Stop-Check "Android build.gradle was not found: $GradlePath"
    }

    $content = Get-Content $GradlePath -Encoding UTF8
    $versionCodeLine = $content | Where-Object { $_ -match 'versionCode' } | Select-Object -First 1
    $versionNameLine = $content | Where-Object { $_ -match 'versionName' } | Select-Object -First 1
    if (-not $versionCodeLine -or -not $versionNameLine) {
        Stop-Check "Could not read versionCode/versionName from android/app/build.gradle."
    }

    $localCodeText = ($versionCodeLine -replace '.*versionCode\s+', '').Trim()
    $localNameText = ($versionNameLine -replace '.*versionName\s+', '').Trim()
    $localCode = [int]$localCodeText
    $localName = $localNameText.Trim([char]34)

    try {
        $remoteVersion = Invoke-RestMethod "$ServerUrl/app/version.json" -TimeoutSec 10
    } catch {
        Stop-Check "Could not read server /app/version.json: $_"
    }

    if ([int]$remoteVersion.versionCode -ne $localCode -or [string]$remoteVersion.versionName -ne $localName) {
        Stop-Check "Server APK version mismatch: local v$localName build $localCode, server v$($remoteVersion.versionName) build $($remoteVersion.versionCode)."
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
    Write-Host "  version:     v$localName (build $localCode)"
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

    Write-Host "Server completion check passed:" -ForegroundColor Green
    Write-Host "  HEAD:        $($head.Substring(0, 7))"
    Write-Host "  origin/main: $($originMain.Substring(0, 7))"
    Write-Host "  health:      $($health | ConvertTo-Json -Compress)"
    exit 0
}
