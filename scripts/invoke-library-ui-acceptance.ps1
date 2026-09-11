#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateSet('inspect','inspect_entry','features','library','browse','query','clear_query','refresh','more',
        'file','attach','remove_staged','rename','set_fixture_name','confirm_rename','upload_fixture_copy','select_upload_copy',
        'trash','confirm_fixture_trash','close_mutation','close_detail','back',
        'download','wait_download','close_download','query_fixture',
        'gallery','gallery_select','gallery_wait','gallery_inspect','gallery_next','gallery_previous',
        'gallery_preview','gallery_close_preview','gallery_close','gallery_download','gallery_download_verified')]
    [string]$Step = 'inspect',
    [string]$Handle = '',
    [string]$FixtureName = '',
    [string]$SdkRoot = 'D:/Android/sdk',
    [string]$JavaHome = 'C:/Program Files/Microsoft/jdk-21.0.11.10-hotspot'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb (Join-Path $SdkRoot 'platform-tools/adb.exe') `
    -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
if ($Step -eq 'file' -and $Handle -notmatch '^[a-zA-Z0-9_-]{1,160}$') { throw 'Invalid library handle.' }
if ($Step -eq 'set_fixture_name' -and $FixtureName -notmatch '^elon[-_][a-z0-9_.-]{1,150}$') {
    throw 'Only a bounded ELON fixture name is accepted.'
}
if ($Step -eq 'confirm_fixture_trash' -and $FixtureName -cnotmatch '^ELON-library-disposable-[a-f0-9]{12}\.txt$') {
    throw 'Only the dedicated disposable fixture may be soft-deleted.'
}
if ($Step -eq 'query_fixture' -and $FixtureName -cnotmatch '^elon-chatgpt-(?:attachment|media)-fixture-v1\.(?:txt|png|pdf)$') {
    throw 'Only a fixed acceptance attachment may be queried.'
}
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$parameters = @{}
if ($Handle) { $parameters.handle = $Handle }
if ($Step -in @('set_fixture_name','confirm_fixture_trash','query_fixture')) { $parameters.fixtureName = $FixtureName }
Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass LibraryUiAcceptance -Step $Step `
    -Parameters $parameters -ResultPrefix LIBRARY_UI_RESULT -SdkRoot $SdkRoot -JavaHome $JavaHome | ConvertTo-Json -Compress
