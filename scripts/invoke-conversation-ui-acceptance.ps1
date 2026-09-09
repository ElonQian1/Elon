#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateSet('inspect','model','model_advanced','model_level','select_model','tools','image','search','clear_image','clear_search','header','temporary','back',
        'conversation_actions','share','account_shares','share_list','close_shares','next_shares','previous_shares')]
    [string]$Step = 'inspect',
    [string]$Selector = '',
    [ValidateRange(0, 5)][int]$Level = 0,
    [string]$SdkRoot = 'D:/Android/sdk',
    [string]$JavaHome = 'C:/Program Files/Microsoft/jdk-21.0.11.10-hotspot'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb (Join-Path $SdkRoot 'platform-tools/adb.exe') `
    -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$legacyModelSelector = '^(web-chat-model-(?:option|parent|preset):|chatgpt-option:model:)[A-Za-z0-9_.:-]{1,140}$'
$productionModelSelector = '^chatgpt-composer-option:model:[A-Za-z0-9_.-]{1,96}:[^\r\n]{1,120}$'
if ($Step -eq 'select_model' -and $Selector -cnotmatch $legacyModelSelector -and $Selector -cnotmatch $productionModelSelector) {
    throw 'Only a visible native model option selector may be selected.'
}
$parameters = @{}
if ($Step -eq 'model_level') { $parameters.level = $Level }
if ($Selector) { $parameters.selector_b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($Selector)) }
Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass ConversationUiAcceptance -Step $Step `
    -Parameters $parameters -ResultPrefix CONVERSATION_UI_RESULT -SdkRoot $SdkRoot -JavaHome $JavaHome |
    ConvertTo-Json -Compress
