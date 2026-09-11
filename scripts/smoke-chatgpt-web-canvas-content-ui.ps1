#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [int]$ExpectedAdapterVersion=359,
    [string]$Adb='D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$r=New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$report=[ordered]@{schema='elon.canvas_content_ui.v1';passed=$false;stage='prepare';sent_messages=0;writes=0
    content_exported=$false;restored=$false;awake_restored=$false}
$origin=$null; $open=$false; $url=''
function Web { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state }
function Act([string]$Action) { Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Action }
function Menu([string]$Step) {
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass ConversationUiAcceptance -Step $Step -ResultPrefix CONVERSATION_UI_RESULT
}
function Link([string]$Step) {
    $p=@{resource='canvas'}
    if($url){$p.url_b64=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($url))}
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass SharedLinkUiAcceptance -Step $Step -Parameters $p -ResultPrefix SHARED_LINK_UI_RESULT
}
function Canvas([string]$Step) {
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass CanvasUiAcceptance -Step $Step -ResultPrefix CANVAS_UI_RESULT
}
function Wait-Share([object[]]$Before) {
    $state=Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 30 -Description 'canvas native share receipt' -Predicate {
        param($s) @($s.command_requests|Where-Object{ $_.request_id -notin $Before -and
            $_.expected_web_action -eq 'share_conversation' -and $_.status -in @('succeeded','failed','timed_out') }).Count -gt 0
    }.GetNewClosure()
    $receipt=@($state.command_requests|Where-Object{ $_.request_id -notin $Before -and $_.expected_web_action -eq 'share_conversation' })|Select-Object -Last 1
    if($receipt.status -ne 'succeeded'){
        $code=[string]$receipt.result.detail
        if($code -cmatch '^share_[a-z0-9_]+$'){throw $code}
        throw 'canvas_read_unconfirmed'
    }
    return @{receipt=$receipt;state=$state}
}
try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if(-not (Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready){throw 'device_locked'}
    $origin=Get-ChatGptWebNativeChatState -Runtime $r
    if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_package_mismatch'}
    if($null -eq $origin.input.has_text -or $null -eq $origin.input.text_length){throw 'native_input_state_unavailable'}
    $before=Web
    if($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        $origin.input.has_text -or -not $before.authenticated -or $before.streaming -or $before.dictation_active -or
        [int]$before.input.official_draft_length -gt 0){throw 'idle_native_chat_required'}
    Assert-ChatGptWebSmokeAdapterVersion -State $before -ExpectedAdapterVersion $ExpectedAdapterVersion
    Start-ChatGptWebSmokeAwakeLease -Runtime $r | Out-Null
    $report.stage='native_shared_list'
    $ids=@($before.command_requests|ForEach-Object request_id)
    Act 'open_chat_side_menu' | Out-Null
    $open=$true
    Menu 'conversation_actions' | Out-Null
    Menu 'share' | Out-Null
    Link 'open_canvas_list' | Out-Null
    $loaded=Wait-Share $ids
    $page=$loaded.receipt.result.detail|ConvertFrom-Json
    if($page.schema -cne 'elon.canvas_shares.v1' -or @($page.items).Count -eq 0 -or $page.offset -ne 0){throw 'shared_canvas_sample_missing'}
    $id=[string]$page.items[0].id
    if($id -cnotmatch '^[A-Za-z0-9_-]{1,128}$'){throw 'shared_canvas_id_unconfirmed'}
    $url='https://chatgpt.com/canvas/shared/'+$id
    Link 'select_first' | Out-Null
    $report.stage='native_shared_content'
    $ids=@((Web).command_requests|ForEach-Object request_id)
    Canvas 'view_shared' | Out-Null
    $read=Wait-Share $ids
    if($read.receipt.result.detail -cne 'share_canvas_ready'){throw 'shared_canvas_content_unconfirmed'}
    $visible=Canvas 'inspect_shared'
    if(-not $visible.body.native_body -or $visible.body.length -le 0 -or
        $visible.body.length -ne $read.state.canvas_content.content_length){throw 'native_canvas_content_mismatch'}
    $report.native_body=$true
    $report.body_length=$visible.body.length
    $report.document_type=$read.state.canvas_content.document_type
    $report.document_version=$read.state.canvas_content.document_version
    Canvas 'return_link' | Out-Null
    $report.returned_link=(Link 'inspect').selected_matches
    Link 'back_to_list' | Out-Null
    Link 'close_list' | Out-Null
    Act 'close_chat_side_menu' | Out-Null
    $open=$false
    $report.stage='complete'
} catch {
    $message=[string]$_.Exception.Message
    $report.error=if($message -cmatch '^[a-z0-9_]+$'){$message}
        elseif($message -match '^Semantic UI acceptance failed: ([a-z_]+)$'){$Matches[1]}
        else{'canvas_content_acceptance_failed'}
} finally {
    try {
        if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_changed_skip_cleanup'}
        if($open){
            if((Canvas 'inspect').shared_content){Canvas 'return_link'|Out-Null}
            $visible=Link 'inspect'
            if($visible.selected_dialog){Link 'back_to_list'|Out-Null}
            if((Link 'inspect').canvas_list){Link 'close_list'|Out-Null}else{Menu 'back'|Out-Null}
            Act 'close_chat_side_menu'|Out-Null
        }
        $after=Get-ChatGptWebNativeChatState -Runtime $r
        $report.restored=$origin -and $after.social_chat.web_chat_conversation_path -ceq $origin.social_chat.web_chat_conversation_path -and
            $after.input.text_length -eq $origin.input.text_length -and $after.chat_side_menu_open -eq $origin.chat_side_menu_open
    } catch {$report.cleanup_failed=$true}
    try {$report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r} catch {$report.awake_restored=$false}
    $report.passed=$report.stage -eq 'complete' -and $report.returned_link -and $report.restored -and $report.awake_restored -and -not $report.cleanup_failed
    $report|ConvertTo-Json -Depth 4 -Compress
}
if(-not $report.passed){exit 1}
