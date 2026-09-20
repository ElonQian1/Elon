#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateSet('inspect','open_create','create_fixture','open_fixture','send_first','send_second',
        'select_fixture','submit_selection','share_answer','cancel','back')][string]$Step = 'inspect'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb 'D:/Android/sdk/platform-tools/adb.exe' `
    -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass GroupAiUiAcceptance -Step $Step `
    -ResultPrefix GROUP_AI_UI_RESULT | ConvertTo-Json -Depth 4 -Compress
