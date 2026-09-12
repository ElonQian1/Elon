#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory)][switch]$CreateFixture,
    [int]$ExpectedAdapterVersion=361,
    [string]$Adb='D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$r=New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$report=[ordered]@{schema='elon.collapsed_composer_ui.v1';passed=$false;stage='prepare';sent=0;restored=$false;awake_restored=$false;content_exported=$false}
$origin=$null; $changed=$false
$prompt='ELON_EXTENDED_TOOL_ACCEPTANCE_V1: Reply exactly COMPOSER_READY'
function Main { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState }
function Web { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state }
function Act([string]$Name,[hashtable]$Arguments=@{}) { Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Name -Arguments $Arguments }
function Ui([string]$Step) { Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass CanvasUiAcceptance -Step $Step -ResultPrefix CANVAS_UI_RESULT }
function Stage([string]$Name) { $report.stage=$Name; Write-Host "COLLAPSED_COMPOSER_STAGE=$Name" }
try {
    if(-not $CreateFixture){throw 'fixture_creation_required'}
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if(-not (Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready){throw 'device_locked'}
    $origin=Main; $before=Web
    if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_package_mismatch'}
    if($null -eq $origin.input.has_text -or $null -eq $origin.input.text_length){throw 'native_input_state_unavailable'}
    if($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        -not $before.authenticated -or $before.streaming -or $before.dictation_active -or
        $origin.input.has_text -or [int]$before.input.official_draft_length -gt 0){throw 'idle_native_chat_required'}
    Assert-ChatGptWebSmokeAdapterVersion -State $before -ExpectedAdapterVersion $ExpectedAdapterVersion
    Start-ChatGptWebSmokeAwakeLease -Runtime $r|Out-Null
    $changed=$true
    Act 'start_new_web_chat_conversation'|Out-Null
    Wait-ChatGptWebSmokeState -Runtime $r -MainState -TimeoutSec 30 -Description 'empty native composer' -Predicate {
        param($s) $s.social_chat.web_chat_composer_ready -eq $true -and $s.social_chat.message_count -eq 0
    }|Out-Null
    Stage 'native_draft'
    $report.native_draft=(Ui 'set_composer_fixture').draft_set
    Stage 'collapsed_send_visible'
    $collapsed=(Ui 'collapse_composer').composer
    $report.collapsed=$collapsed.collapsed_preview -and -not $collapsed.editor
    $report.send_visible=$collapsed.send
    $report.voice_visible=$collapsed.voice
    if(-not $report.collapsed -or -not $collapsed.send -or $collapsed.voice){throw 'collapsed_draft_send_missing'}
    Stage 'native_send'
    $since=[long](Main).social_chat.web_chat_last_send_command.observed_at_ms
    Ui 'send_fixture'|Out-Null
    $report.sent=1
    $until=[DateTimeOffset]::UtcNow.AddSeconds(90)
    do {
        if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_package_mismatch'}
        $main=Main; $web=Web; $receipt=$main.social_chat.web_chat_last_send_command
        if($receipt.observed_at_ms -gt $since -and $receipt.ok -ne $true){throw 'native_send_unconfirmed'}
        if($receipt.observed_at_ms -gt $since -and $receipt.ok -and
            ([string]$receipt.detail).StartsWith('official_runtime_v1:') -and -not $web.streaming -and -not $main.social_chat.web_chat_streaming){
            $anchor=@($web.conversation.messages|Where-Object role -eq user)|Select-Object -Last 1
            if($anchor.id){
                $context=Act 'chatgpt_get_context' @{message_offset=[int]$anchor.index;message_limit=10}
                $users=@($context.messages|Where-Object role -eq user)
                if($context.conversation_url -cne $web.conversation.url -or $users.Count -ne 1 -or $users[0].content -cne $prompt){throw 'fixture_anchor_mismatch'}
                $reply=@($context.messages|Where-Object{$_.role -eq 'assistant' -and $_.state -eq 'completed' -and $_.content -match '^COMPOSER(?:\\)?_READY[.!]?$'})
                if($reply.Count -eq 1){$report.runtime_send=$true; $report.reply_received=$true; break}
            }
        }
        Start-Sleep -Seconds 2
    }while([DateTimeOffset]::UtcNow -lt $until)
    if(-not $report.reply_received){throw 'native_reply_timeout'}
    Stage 'complete'
} catch {
    $message=[string]$_.Exception.Message
    $report.error=if($message -cmatch '^[a-z0-9_]+$'){$message}
        elseif($message -match '^Semantic UI acceptance failed: ([a-z_]+)$'){$Matches[1]}
        else{'collapsed_composer_acceptance_failed'}
} finally {
    try {
        if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_changed_skip_cleanup'}
        $current=Main
        if($changed -and $current.input.has_text){Ui 'clear_fixture_draft'|Out-Null}
        if($changed -and $origin){
            $report.restored=Restore-WebChatNativeConversation -Runtime $r -ProviderId chatgpt_web -ConversationPath $origin.social_chat.web_chat_conversation_path -TimeoutSec 45
        }
        $after=Main
        $report.restored=$report.restored -and $after.input.text_length -eq $origin.input.text_length -and
            $after.social_chat.web_chat_conversation_path -ceq $origin.social_chat.web_chat_conversation_path
    } catch {$report.cleanup_failed=$true}
    try {$report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r}catch{$report.awake_restored=$false}
    $report.passed=$report.stage -eq 'complete' -and $report.restored -and $report.awake_restored -and -not $report.cleanup_failed
    $report|ConvertTo-Json -Depth 5 -Compress
}
if(-not $report.passed){exit 1}
