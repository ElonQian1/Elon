Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'apk-adb-autodeploy.ps1')
function Assert-That($ok,$message) { if (-not $ok) { throw $message } }
$script:calls=@()
$script:mode='usb'
function Invoke-ElonAdbCommand {
    param($AdbPath,$Arguments,$TimeoutSeconds)
    $script:calls += ($Arguments -join ' ')
    $text=''; $code=0; $serial=$Arguments[1]
    if ($Arguments[0] -eq 'devices') { $text="List of devices attached`nusb-a device`nemulator-5554 device" }
    elseif ($Arguments[0] -eq 'mdns') { $text='adb-hw-a-session _adb-tls-connect._tcp 192.168.1.10:37001' }
    elseif ($Arguments[0] -eq 'connect') { $text='connected' }
    elseif ($Arguments[2] -eq 'get-state') {
        if ($script:mode -eq 'offline') { $text='offline'; $code=1 }
        elseif ($script:mode -eq 'unauthorized') { $text='unauthorized'; $code=1 }
        elseif ($serial -eq 'usb-a' -and $script:mode -eq 'usb') { $text='device' }
        elseif ($serial -match ':' -and $script:mode -ne 'usb') { $text='device' }
        else { $text='not found'; $code=1 }
    }
    elseif ($Arguments[3] -eq 'getprop') { $text=if($script:mode -eq 'mismatch'){'someone-else'}else{'hw-a'} }
    else { throw "Unexpected command: $Arguments" }
    [pscustomobject]@{ExitCode=$code;Stdout=$text;Stderr='';Text=$text}
}
$target=[pscustomobject]@{hardwareSerial='hw-a';serial='192.168.1.1:5555';label='phone-a'}
$r=Find-ElonApkAdbTransport 'fake' $target
Assert-That ($r.Serial -eq 'usb-a') 'USB hardware proof must win over stale wireless endpoint'
Assert-That (-not ($script:calls -match '^connect')) 'USB selection must avoid unnecessary wireless connects'
Assert-That (-not ($script:calls -match '-s emulator')) 'Do not treat emulators as registered physical phones'
$script:mode='wifi'; $script:calls=@()
$r=Find-ElonApkAdbTransport 'fake' $target
Assert-That ($r.Status -eq 'online' -and $r.Serial -eq $target.serial) 'Wireless endpoint must be verified'
$script:mode='mismatch'; $r=Find-ElonApkAdbTransport 'fake' $target
Assert-That ($r.Status -eq 'identity_mismatch') 'A reused wireless IP must not select another phone'
$script:mode='offline'; $r=Find-ElonApkAdbTransport 'fake' $target
Assert-That ($r.Status -eq 'offline') 'Offline must remain distinct from install failure'
$script:mode='unauthorized'; $r=Find-ElonApkAdbTransport 'fake' $target
Assert-That ($r.Status -eq 'unauthorized') 'Unauthorized must require user action'

# Exercise the orchestration with one offline phone, one online phone, and duplicate transports.
$fixture=Join-Path ([IO.Path]::GetTempPath()) ('elon-adb-discovery-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixture | Out-Null
try {
    $apk=Join-Path $fixture 'app.apk'; Set-Content -LiteralPath $apk -Value 'fixture'
    $configPath=Join-Path $fixture 'config.json'; $receipt=Join-Path $fixture 'receipt.json'
    @{schemaVersion=1;targets=@($target,$target,[pscustomobject]@{label='phone-b';hardwareSerial='hw-b';serial='192.168.1.2:5555'})} |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $configPath
    function Resolve-ElonApkAdbPath { param($ConfiguredPath,$RequestedPath); 'fake' }
    function Find-ElonApkAdbTransport {
        param($AdbPath,$Target)
        [pscustomobject]@{Serial='usb-a';Status=$(if($Target.hardwareSerial -eq 'hw-a'){'online'}else{'offline'})}
    }
    $script:installed=0
    function Invoke-ElonTargetAdbDeployment {
        param($AdbPath,$Target,$ApkPath,$PackageName,$ExpectedVersionCode,$MaxAttempts,$RetryDelaySeconds,$LaunchAfterInstall)
        $script:installed++
        [pscustomobject]@{Label=$Target.label;Serial=$Target.serial;Status='updated'}
    }
    $results=@(Invoke-ElonApkAdbAutodeploy $apk 12 -ConfigPath $configPath -ReceiptPath $receipt)
    Assert-That ($results.Count -eq 2 -and $script:installed -eq 1) 'Same hardware must install once; offline phone must be recorded'
    $saved=Get-Content -LiteralPath $receipt -Raw | ConvertFrom-Json
    Assert-That ($saved.targets.Count -eq 2 -and $saved.apkSha256.Length -eq 64) 'Receipt must bind APK and all phones'
    function Invoke-ElonTargetAdbDeployment { throw 'install rejected' }
    $failed=$false
    try { Invoke-ElonApkAdbAutodeploy $apk 12 -ConfigPath $configPath -ReceiptPath $receipt | Out-Null } catch { $failed=$true }
    Assert-That $failed 'Online installation errors must fail the release postflight'
    $saved=Get-Content -LiteralPath $receipt -Raw | ConvertFrom-Json
    Assert-That ($saved.targets.Count -eq 2) 'Failure on one phone must not skip checking the other'
} finally {
    $resolved=[IO.Path]::GetFullPath($fixture)
    if ($resolved.StartsWith([IO.Path]::GetFullPath([IO.Path]::GetTempPath())) -and (Split-Path $resolved -Leaf) -like 'elon-adb-discovery-*') {
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
Write-Host 'APK_ADB_DISCOVERY_TESTS=passed cases=8'
