#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateSet('inspect','open_create','create_fixture','open_fixture','open_model','model_default','model_latest','send_first','send_second',
        'select_fixture','submit_selection','retry_selection','share_answer','share_target','share_choose_group','share_submit',
        'open_card','continue_private','confirm_private','return_group','cancel','back')][string]$Step = 'inspect',
    [string]$GroupName = '',
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb `
    -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$parameters = @{}
if ($GroupName) { $parameters.group_b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($GroupName)) }
Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass GroupAiUiAcceptance -Step $Step `
    -Parameters $parameters -ResultPrefix GROUP_AI_UI_RESULT | ConvertTo-Json -Depth 4 -Compress
