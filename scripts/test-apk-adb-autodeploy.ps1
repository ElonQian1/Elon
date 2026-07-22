param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'apk-adb-autodeploy.ps1')

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$publishScriptPath = Join-Path $PSScriptRoot 'publish-apk.ps1'
$tokens = $null
$parseErrors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $publishScriptPath,
    [ref]$tokens,
    [ref]$parseErrors
)
Assert-True (@($parseErrors).Count -eq 0) 'publish-apk.ps1 must remain valid in Windows PowerShell 5.1'
$postflightScript = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'apk-publish-postflight.ps1') -Raw
Assert-True ($postflightScript.Contains('Invoke-ElonApkAdbAutodeploy -ApkPath $ApkPath')) `
    'The Windows APK postflight must invoke unattended ADB deployment after publishing'

function Write-FakeAdb {
    param([string]$Path, [string]$LogPath)
    $escapedLog = $LogPath.Replace('%', '%%')
    $content = @(
        '@echo off'
        ('echo %*>>"{0}"' -f $escapedLog)
        'if "%1"=="connect" echo connected to %2& exit /b 0'
        'if "%3"=="get-state" echo device& exit /b 0'
        'if "%3"=="install" echo Success& exit /b 0'
        'if "%3"=="shell" if "%4"=="getprop" echo hardware-123& exit /b 0'
        'if "%3"=="shell" if "%4"=="dumpsys" echo versionCode=901 minSdk=26 targetSdk=34& exit /b 0'
        'if "%3"=="shell" if "%4"=="am" exit /b 0'
        'if "%3"=="shell" if "%4"=="monkey" echo Events injected: 1& exit /b 0'
        'echo unexpected fake adb arguments: %*& exit /b 9'
    ) -join [Environment]::NewLine
    Set-Content -LiteralPath $Path -Value $content -Encoding Ascii
}

$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-apk-adb-autodeploy-" + [Guid]::NewGuid().ToString('N'))
try {
    New-Item -ItemType Directory -Force -Path $fixtureRoot | Out-Null
    $adb = Join-Path $fixtureRoot 'adb.cmd'
    $log = Join-Path $fixtureRoot 'adb.log'
    $apk = Join-Path $fixtureRoot 'release apk.apk'
    $config = Join-Path $fixtureRoot 'targets.json'
    Set-Content -LiteralPath $apk -Value 'fixture' -Encoding Ascii
    Write-FakeAdb -Path $adb -LogPath $log

    @{
        schemaVersion = 1
        enabled = $true
        adbPath = $adb
        packageName = 'com.elon.app'
        maxAttempts = 1
        retryDelaySeconds = 0
        launchAfterInstall = $true
        targets = @(@{
            label = 'whitelisted-phone'
            serial = '192.168.1.20:5555'
            hardwareSerial = 'hardware-123'
        })
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $config -Encoding UTF8

    $result = @(Invoke-ElonApkAdbAutodeploy -ApkPath $apk -ExpectedVersionCode 901 -ConfigPath $config)
    Assert-True ($result.Count -eq 1 -and $result[0].Status -eq 'updated') `
        'A whitelisted device must report an updated result'
    $calls = Get-Content -LiteralPath $log -Raw
    Assert-True ($calls.Contains('install -r')) 'Release deployment must preserve data with adb install -r'
    Assert-True ($calls.Contains('shell dumpsys package com.elon.app')) `
        'Release deployment must verify the installed package version'
    Assert-True ($calls.Contains('shell monkey -p com.elon.app')) `
        'Release deployment must relaunch the application without user input'

    $badConfig = Get-Content -LiteralPath $config -Raw | ConvertFrom-Json
    $badConfig.targets[0].hardwareSerial = 'different-phone'
    $badConfig | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $config -Encoding UTF8
    $mismatchRejected = $false
    try {
        Invoke-ElonApkAdbAutodeploy -ApkPath $apk -ExpectedVersionCode 901 -ConfigPath $config | Out-Null
    } catch {
        $mismatchRejected = $_.Exception.Message.Contains('Hardware serial mismatch')
    }
    Assert-True $mismatchRejected 'A reused IP must not bypass the hardware serial whitelist'

    Remove-Item -LiteralPath $config -Force
    $skipped = @(Invoke-ElonApkAdbAutodeploy -ApkPath $apk -ExpectedVersionCode 901 -ConfigPath $config)
    Assert-True ($skipped.Count -eq 0) 'A machine without an opt-in config must skip ADB deployment'
} finally {
    Remove-Item -LiteralPath $fixtureRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host 'APK_ADB_AUTODEPLOY_TESTS=passed' -ForegroundColor Green
