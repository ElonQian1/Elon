#requires -Version 7.0
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'chatgpt-fresh-attachment-smoke-evidence.ps1')
$count=0
function Check([bool]$Value,[string]$Name) {if(!$Value){throw "fresh_attachment_evidence_failed:$Name"};$script:count++}
function Fixture {
    $path='/c/11111111-1111-4111-8111-111111111111'
    $prompt='synthetic file prompt'
    $facts='ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready; ELON_PRIVATE_PDF_FIXTURE_V1=ready; three blue squares and one red circle'
    $user=@{id='user';role='user';content=$prompt}
    $answer=@{id='answer';role='assistant';state='completed';content=$facts}
    return @{Baseline=@{conversation=@{url="https://chatgpt.com$path";messages=@()}};
        Web=@{streaming=$false;conversation=@{url="https://chatgpt.com$path";messages=@($user,$answer)}};
        Main=@{social_chat=@{web_chat_conversation_path=$path;web_chat_streaming=$false}};
        Native=@{control_ok=$true;provider_id='chatgpt_web';conversation_path=$path;streaming=$false;messages=@(
            @{role='user';source_message_id='user';content=$prompt;content_truncated=$false},
            @{role='friend';source_message_id='answer';content=$facts;content_truncated=$false})};Prompt=$prompt}
}
$f=Fixture
Check (Test-ChatGptFreshAttachmentNativeEvidence @f) 'matched_new_turn'
foreach($text in @('3 solid blue squares and 1 solid red circle', 'three blue squares and one red circle')) {
    Check (Test-ChatGptFreshMediaFacts ('ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready; ELON_PRIVATE_PDF_FIXTURE_V1=ready; '+$text)) 'exact_count_variant'
}
foreach($text in @('2 solid blue squares and 1 solid red circle', '3 solid blue squares and 2 solid red circles',
    '3 files, blue squares and 1 red circle', '3 blue squares and red circle')) {
    Check (!(Test-ChatGptFreshMediaFacts ('ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready; ELON_PRIVATE_PDF_FIXTURE_V1=ready; '+$text))) 'wrong_or_unbound_count'
}
foreach($case in @('old_reply','old_native_reply','missing_pdf','missing_image','native_missing_fact','streaming',
    'unknown_streaming','native_streaming','native_truncated','user_truncated','wrong_source','later_user','duplicate_user','other_provider','other_route')) {
    $f=Fixture
    switch($case) {
        'old_reply' {$f.Baseline.conversation.messages=@($f.Web.conversation.messages[1])}
        'old_native_reply' {$f.Native.messages[1].source_message_id='old'}
        'missing_pdf' {$f.Web.conversation.messages[1].content=$f.Web.conversation.messages[1].content.Replace('ELON_PRIVATE_PDF_FIXTURE_V1=ready','')}
        'missing_image' {$f.Web.conversation.messages[1].content=$f.Web.conversation.messages[1].content.Replace('three blue squares','blue shapes')}
        'native_missing_fact' {$f.Native.messages[1].content='file unavailable'}
        'streaming' {$f.Web.streaming=$true}
        'unknown_streaming' {$f.Web.Remove('streaming')}
        'native_streaming' {$f.Native.streaming=$true}
        'native_truncated' {$f.Native.messages[1].content_truncated=$true}
        'user_truncated' {$f.Native.messages[0].content_truncated=$true}
        'wrong_source' {$f.Native.messages[0].source_message_id='unrelated'}
        'later_user' {$f.Native.messages+=@{role='user';source_message_id='later';content='other'}}
        'duplicate_user' {$f.Web.conversation.messages+=@{role='user';id='extra';content=$f.Prompt}}
        'other_provider' {$f.Native.provider_id='google_web'}
        'other_route' {$f.Native.conversation_path='/c/22222222-2222-4222-8222-222222222222'}
    }
    Check (!(Test-ChatGptFreshAttachmentNativeEvidence @f)) $case
}
Write-Output "FRESH_ATTACHMENT_EVIDENCE_TESTS_PASSED=$count"
