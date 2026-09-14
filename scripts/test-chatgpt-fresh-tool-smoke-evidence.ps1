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
function Clear-Fixture {
    $before=@{page_generation=1;conversation=@{url='https://chatgpt.com/c/11111111-1111-4111-8111-111111111111'};
        command_requests=@(@{request_id='old'})}
    $after=@{page_generation=1;conversation=$before.conversation.Clone();streaming=$false;input=@{text=''};
        command_requests=@(@{request_id='clear';expected_web_action='select_composer_tool';status='succeeded';
            result=@{ok=$true}})}
    return @{Before=$before;After=$after;ToolId='web_search';Items=@(
        @{semantic='web_search';selected=$false},@{semantic='image_generation';selected=$false})}
}
$f=Clear-Fixture
Check (Test-ChatGptFreshToolCleared @f) 'clear_confirmed'
$f.ToolId='image_generation'
Check (Test-ChatGptFreshToolCleared @f) 'image_clear_confirmed'
foreach($case in @('old_receipt','wrong_action','queued','failed','not_ok','string_ok','no_receipt',
    'duplicate','selected','other_selected','missing_option','duplicate_option','unknown_selected',
    'generation','route','streaming','unknown_streaming','draft')) {
    $f=Clear-Fixture
    switch($case){
        'old_receipt' {$f.After.command_requests[0].request_id='old'}
        'wrong_action' {$f.After.command_requests[0].expected_web_action='list_composer_tools'}
        'queued' {$f.After.command_requests[0].status='queued'}
        'failed' {$f.After.command_requests[0].status='failed'}
        'not_ok' {$f.After.command_requests[0].result.ok=$false}
        'string_ok' {$f.After.command_requests[0].result.ok='true'}
        'no_receipt' {$f.After.command_requests=@()}
        'duplicate' {$f.After.command_requests+=$f.After.command_requests[0].Clone()}
        'selected' {$f.Items[0].selected=$true}
        'other_selected' {$f.Items[1].selected=$true}
        'missing_option' {$f.Items=@($f.Items[1])}
        'duplicate_option' {$f.Items+=$f.Items[0].Clone()}
        'unknown_selected' {$f.Items[0].Remove('selected')}
        'generation' {$f.After.page_generation=2}
        'route' {$f.After.conversation.url='https://chatgpt.com/'}
        'streaming' {$f.After.streaming=$true}
        'unknown_streaming' {$f.After.Remove('streaming')}
        'draft' {$f.After.input.text='changed'}
    }
    Check (!(Test-ChatGptFreshToolCleared @f)) "clear_$case"
}
Set-StrictMode -Version Latest
$f=Clear-Fixture
$f.After=[pscustomobject]@{}
Check (!(Test-ChatGptFreshToolCleared @f)) 'clear_missing_evidence_strict'
Write-Output "FRESH_TOOL_EVIDENCE_TESTS=passed count=$count"
