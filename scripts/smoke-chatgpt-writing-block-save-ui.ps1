#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [string]$ExistingFixturePath=$env:ELON_WRITING_FIXTURE_PATH,
    [string]$Adb='D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$r=New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$report=[ordered]@{schema='elon.writing_save_ui.v1';passed=$false;stage='prepare';sent=0;cloud_write_attempts=0;restored=$false;awake_restored=$false;private_content_exported=$false}
$opened=$false; $editedHash=''; $origin=$null; $changed=$false
$prompt='ELON_EXTENDED_TOOL_ACCEPTANCE_V1: For a native block editor test, draft a short email in your editable writing block. Put the exact marker ELON_TEXT_BLOCK_ACCEPTANCE_V1 in the email body. Then add one separate Python fenced code block containing only print("ELON_TEXT_BLOCK_ACCEPTANCE_V1"). Do not use Canvas or execute anything.'
function Main { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState }
function Web { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state }
function Act([string]$Name,[hashtable]$Arguments=@{}) { Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Name -Arguments $Arguments }
function Ui([string]$Step,[hashtable]$Parameters=@{}) {
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass TextBlockUiAcceptance -Step $Step -Parameters $Parameters -ResultPrefix TEXT_BLOCK_UI_RESULT
}
function Stage([string]$Name) { $report.stage=$Name; Write-Host "WRITING_SAVE_STAGE=$Name" }
try {
    if($ExistingFixturePath -cnotmatch '^/c/[a-f0-9-]{36}$'){throw 'owned_fixture_required'}
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if(-not (Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready){throw 'device_locked'}
    $origin=Main; $before=Web
    if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r) -or $origin.active_surface -ne 'social_ai' -or
        $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or $origin.input.has_text -ne $false -or
        -not $before.authenticated -or $before.streaming -or $before.dictation_active -or
        [int]$before.input.official_draft_length -ne 0){throw 'idle_native_chat_required'}
    Assert-ChatGptWebSmokeAdapterVersion -State $before -ExpectedAdapterVersion (Resolve-ChatGptWebSmokeExpectedAdapterVersion)
    Start-ChatGptWebSmokeAwakeLease -Runtime $r|Out-Null; $changed=$true
    Act open_web_chat_conversation @{conversation_path=$ExistingFixturePath}|Out-Null
    $web=Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 35 -Description 'owned writing block' -Predicate {
        param($s) @($s.conversation.messages|ForEach-Object {$_.parts}|Where-Object type -eq writing_block).Count -eq 1 -and -not $s.streaming
    }
    $users=@($web.conversation.messages|Where-Object role -eq user)
    if($users.Count -ne 1 -or $users[0].content -cne $prompt -or
        ([uri]$web.conversation.url).AbsolutePath -cne $ExistingFixturePath){throw 'fixture_ownership_mismatch'}
    $generation=$web.page_generation
    Start-Sleep -Seconds 3
    $web=Web
    if($web.page_generation -ne $generation){throw 'conversation_reloaded'}
    $report.hot_navigation_stable=$true
    $parts=@()
    foreach($m in $web.conversation.messages|Where-Object { $_.role -eq 'assistant' -and $_.state -eq 'completed' }) {
        for($i=0;$i -lt @($m.parts).Count;$i++){if($m.parts[$i].type -eq 'writing_block'){$parts+=@{message=$m.id;index=$i}}}
    }
    if($parts.Count -ne 1){throw 'writing_selection_ambiguous'}
    $item=$parts[0]
    Act chatgpt_reveal_message @{message_id=$item.message;part_index=$item.index;target='message'}|Out-Null
    $key=([string]$item.message -creplace '[^A-Za-z0-9_.:-]','_')
    $selector="web-chat-message-part:chatgpt_web:${key}:$($item.index):writing_block"
    $oldRequests=@((Web).command_requests|ForEach-Object request_id)
    Stage native_open
    $opened=$true; $original=(Ui open @{selector=$selector}).body
    Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 30 -Description 'writing save preparation' -Predicate {
        param($s) @($s.command_requests|Where-Object { $_.request_id -notin $oldRequests -and $_.expected_web_action -eq 'writing_block' -and $_.result.detail -eq 'writing_ready' }).Count -gt 0
    }.GetNewClosure()|Out-Null
    $edited=(Ui edit @{expected_hash=$original.sha256}).body; $editedHash=$edited.sha256
    $stem='elon-block-acceptance-'+[Guid]::NewGuid().ToString('N').Substring(0,16)
    $export=Ui export @{expected_hash=$editedHash;stem=$stem;extension='md'}
    $names=Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','find','/sdcard/Download','-maxdepth','1','-name',"elon-*-$stem.md") -Label 'locate owned writing export'
    $files=@($names.Trim() -split "`r?`n"|Where-Object {$_})
    if($files.Count -ne 1 -or $files[0] -cnotmatch "^/sdcard/Download/elon-[a-f0-9-]+-$stem\.md`$"){throw 'owned_export_missing'}
    $sum=Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','sha256sum',$files[0]) -Label 'verify owned writing export'
    if(($sum -split '\s+')[0] -cne $export.sha256){throw 'export_bytes_mismatch'}
    $report.export_bytes_match=$true; $report.export_extension='md'
    Stage native_save
    $report.cloud_write_attempts=1
    $saved=Ui save @{expected_hash=$editedHash}
    $report.cloud_save_confirmed=$saved.saved
    Ui close|Out-Null; $opened=$false
    Stage reopen_saved
    Start-Sleep -Seconds 3
    Act chatgpt_reveal_message @{message_id=$item.message;part_index=$item.index;target='message'}|Out-Null
    $opened=$true; $reopened=(Ui open @{selector=$selector}).body
    if($reopened.sha256 -cne $editedHash){throw 'saved_native_content_mismatch'}
    $report.saved_native_content_matches=$true
    Ui close|Out-Null; $opened=$false; $editedHash=''
    if((Web).page_generation -ne $generation){throw 'save_reloaded_conversation'}
    $report.save_without_reload=$true
    Stage complete
} catch {
    $message=[string]$_.Exception.Message
    $report.error=if($message -cmatch '^[a-z0-9_]+$'){$message}
        elseif($message -match '^Semantic UI acceptance failed: ([a-z_]+)$'){$Matches[1]}
        else{'writing_save_acceptance_failed'}
    $report.error_line=$_.InvocationInfo.ScriptLineNumber
} finally {
    try {
        if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_changed_skip_cleanup'}
        if($opened){
            if($editedHash -and $report.export_bytes_match){Ui discard_owned_local @{expected_hash=$editedHash}|Out-Null}
            else{Ui close|Out-Null}
        }
        if($changed -and $origin){$report.restored=Restore-WebChatNativeConversation -Runtime $r -ProviderId chatgpt_web -ConversationPath $origin.social_chat.web_chat_conversation_path -TimeoutSec 35}
        $report.restored=$report.restored -and (Main).input.text_length -eq $origin.input.text_length
        if($report.restored){Act show_conversation_home|Out-Null}
    }catch{$report.cleanup_failed=$true}
    try{$report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r}catch{$report.awake_restored=$false}
    $report.passed=$report.stage -eq 'complete' -and $report.restored -and $report.awake_restored -and -not $report.cleanup_failed
    $report|ConvertTo-Json -Depth 4 -Compress
}
if(-not $report.passed){exit 1}
