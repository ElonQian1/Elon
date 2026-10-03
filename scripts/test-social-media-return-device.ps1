#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$Group,
    [Parameter(Mandatory)][string]$CardDescription,
    [Parameter(Mandatory)][ValidateSet('bilibili','xiaohongshu','douyin')][string]$Platform,
    [ValidateSet('older','newer')][string]$Direction = 'older',
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'apk-adb-autodeploy.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$config = Get-ElonProjectApkAdbConfig
$identity = Invoke-ElonAdbCommand -AdbPath $Adb -Arguments @('-s',$DeviceSerial,'shell','getprop','ro.serialno') -TimeoutSeconds 5
if ($identity.ExitCode -ne 0) { throw 'Device identity probe failed.' }
$target = @($config.targets | Where-Object { $_.hardwareSerial -eq $identity.Stdout.Trim() }) | Select-Object -First 1
if (!$target) { throw 'Device is not registered to the main project.' }
$package = @{bilibili='tv.danmaku.bili';xiaohongshu='com.xingin.xhs';douyin='com.ss.android.ugc.aweme'}[$Platform]
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $target.hardwareSerial
function Foreground {
    $data = Invoke-ElonAdbCommand -AdbPath $Adb -Arguments @('-s',$DeviceSerial,'shell','dumpsys','activity','activities') -TimeoutSeconds 10
    if ($data.ExitCode -ne 0) { throw 'Task probe failed.' }
    $match = [regex]::Match($data.Stdout, 'topResumedActivity=ActivityRecord\{[^\r\n]*?\s([A-Za-z0-9._]+)/(\S+)\s+t(\d+)')
    if (!$match.Success) { throw 'No foreground activity.' }
    [pscustomobject]@{package=$match.Groups[1].Value;activity=$match.Groups[2].Value;task=[int]$match.Groups[3].Value}
}
$before = Foreground
if ($before.package -ne 'com.elon.app') { throw 'Open the target group in Yilong before running this test.' }
$parameters = @{ group_b64=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($Group));
    card_b64=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($CardDescription)); expected_package=$package; direction=$Direction }
$opened = Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass SocialMediaReturnAcceptance -Step open -Parameters $parameters -ResultPrefix SOCIAL_MEDIA_RETURN_RESULT -TimeoutSeconds 60
Start-Sleep -Seconds 8
$stages = @([pscustomobject]@{stage='before';foreground=$before}, [pscustomobject]@{stage='opened';foreground=(Foreground)})
for ($n = 1; $n -le 2; $n++) {
    # Never press Back after leaving the selected external App (e.g. on a login/system dialog).
    if ((Foreground).package -ne $package) { break }
    $back = Invoke-ElonAdbCommand -AdbPath $Adb -Arguments @('-s',$DeviceSerial,'shell','input','keyevent','4') -TimeoutSeconds 5
    if ($back.ExitCode -ne 0) { throw 'System Back failed.' }
    Start-Sleep -Seconds 3
    $stages += [pscustomobject]@{stage="back_$n";foreground=(Foreground)}
}
$after = Foreground
[pscustomobject]@{schema='elon.media_return_acceptance.v1';platform=$Platform;card_clicked=$opened.card_clicked;
    back_kind='android_system_back';back_interval_ms=3000;stages=$stages;
    returned_to_same_task=($after.package -eq $before.package -and $after.task -eq $before.task)} | ConvertTo-Json -Depth 5 -Compress
