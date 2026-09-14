#requires -Version 7.0
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'chatgpt-fresh-tool-smoke-evidence.ps1')
$count=0
function Check([bool]$Value,[string]$Name) {
    if (!$Value) { throw "fresh_tool_evidence_failed:$Name" }
    $script:count++
}
function Fixture {
    $path='/c/11111111-1111-4111-8111-111111111111'
    $user=@{id='user';role='user';content='synthetic tool prompt'}
    $answer=@{id='answer';role='assistant';state='completed';parts=@(@{type='citation'})}
    $before=@{conversation=@{url="https://chatgpt.com$path";messages=@()}}
    $web=@{streaming=$false;conversation=@{url="https://chatgpt.com$path";messages=@($user,$answer)}}
    $main=@{social_chat=@{web_chat_conversation_path=$path;web_chat_streaming=$false}}
    $native=@{control_ok=$true;provider_id='chatgpt_web';conversation_path=$path;streaming=$false;messages=@(
        @{role='user';source_message_id='user';content='synthetic tool prompt';content_truncated=$false},
        @{role='friend';source_message_id='answer';parts_truncated=$false;parts=@(@{type='citation'})})}
    return @{Baseline=$before;Web=$web;Main=$main;Native=$native;Prompt='synthetic tool prompt';PartType='citation'}
}
$f=Fixture
Check (Get-ChatGptFreshToolNativeEvidence @f).ready 'matched_citation'
$f.PartType='image';$f.Web.conversation.messages[1].parts[0].type='image';$f.Native.messages[1].parts[0].type='image'
Check (Get-ChatGptFreshToolNativeEvidence @f).ready 'matched_image'
foreach ($case in @('other_route','streaming','native_streaming','unknown_streaming','web_output',
    'native_output','native_source','truncated_parts','truncated_prompt','unrelated_user','native_later_user',
    'web_later_user','native_owner','incomplete_answer','old_output','old_native_reply')) {
    $f=Fixture
    switch ($case) {
        'other_route' {$f.Web.conversation.url='https://chatgpt.com/c/22222222-2222-4222-8222-222222222222'}
        'streaming' {$f.Web.streaming=$true}
        'native_streaming' {$f.Native.streaming=$true}
        'unknown_streaming' {$f.Web.Remove('streaming')}
        'web_output' {$f.Web.conversation.messages[1].parts=@()}
        'native_output' {$f.Native.messages[1].parts=@()}
        'native_source' {$f.Native.messages[1].source_message_id='other'}
        'truncated_parts' {$f.Native.messages[1].parts_truncated=$true}
        'truncated_prompt' {$f.Native.messages[0].content_truncated=$true}
        'unrelated_user' {$f.Native.messages[0].source_message_id='other'}
        'native_later_user' {$f.Native.messages+=@{role='user';source_message_id='later';content='other'}}
        'web_later_user' {$f.Web.conversation.messages+=@{role='user';id='later';content='other'}}
        'native_owner' {$f.Native.provider_id='google_web'}
        'incomplete_answer' {$f.Web.conversation.messages[1].state='streaming'}
        'old_output' {$f.Web.conversation.messages=@($f.Web.conversation.messages[1],$f.Web.conversation.messages[0])}
        'old_native_reply' {
            $old=@{id='old';role='assistant';state='completed';parts=@(@{type='citation'})}
            $f.Web.conversation.messages=@($old)+$f.Web.conversation.messages
            $f.Native.messages[1].source_message_id='old'
        }
    }
    Check (!(Get-ChatGptFreshToolNativeEvidence @f).ready) $case
}
Write-Output "FRESH_TOOL_EVIDENCE_TESTS=passed count=$count"
