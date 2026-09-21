#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [ValidateSet('inspect','open_group','open_article','refresh','close','chat','original','metrics')][string]$Step = 'inspect',
    [string]$Group = '',
    [string]$ArticleDescription = '',
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
if (-not $target) { throw 'Device is not registered to the main project.' }
if ($Step -eq 'metrics') {
    $response = & (Join-Path $PSScriptRoot 'invoke-apk-mcp.ps1') -DeviceSerial $DeviceSerial -Tool ui_state -NoBootstrap -RequestTimeoutSec 10
    # Explicitly select bounded performance data; never print the rest of the native UI state.
    $response.result.structuredContent.external_reader | ConvertTo-Json -Depth 12 -Compress
    return
}
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $target.hardwareSerial
$parameters = @{}
if ($Step -eq 'open_group') {
    if (-not $Group.Trim()) { throw 'Group is required.' }
    $parameters.group_b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($Group))
}
if ($Step -eq 'open_article') {
    if (-not $ArticleDescription.Trim()) { throw 'ArticleDescription is required.' }
    $parameters.article_b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($ArticleDescription))
}
Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass SocialLinkUiAcceptance -Step $Step -Parameters $parameters -ResultPrefix SOCIAL_LINK_UI_RESULT |
    ConvertTo-Json -Depth 6 -Compress
