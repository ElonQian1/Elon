#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateSet('inspect','open_document','browse_picker','download_fixture','search_fixture','select_fixture','send','cancel_picker')]
    [string]$Step = 'inspect',
    [switch]$UserConfirmedFixtureSend,
    [string]$SdkRoot = 'D:/Android/sdk',
    [string]$JavaHome = 'C:/Program Files/Microsoft/jdk-21.0.11.10-hotspot'
)
$ErrorActionPreference = 'Stop'
if ($Step -eq 'send' -and -not $UserConfirmedFixtureSend) { throw 'Explicit fixture-send consent is required.' }
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb (Join-Path $SdkRoot 'platform-tools/adb.exe') `
    -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass AttachmentPickerUiAcceptance -Step $Step `
    -ResultPrefix ATTACHMENT_PICKER_UI_RESULT -SdkRoot $SdkRoot -JavaHome $JavaHome | ConvertTo-Json -Compress
