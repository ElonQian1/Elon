#requires -Version 7.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-fresh-project-smoke.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-text-smoke-evidence.ps1')
$project = 'g-p-' + ('a' * 32)
$path = "/g/$project-fixture/c/00000000-1111-2222-3333-444444444444"
if (!(Test-ChatGptFreshProjectPath $path $project)) { throw 'valid_project_path_rejected' }
foreach ($bad in @($path + '?x=1', $path + '/', $path.Replace($project, 'g-p-' + ('b' * 32)), '/c/00000000-1111-2222-3333-444444444444')) {
    if (Test-ChatGptFreshProjectPath $bad $project) { throw 'foreign_project_path_accepted' }
}
function Users {
    @(@{role='user';content='Read all three attached test files. Reply in English: quote the exact first line from each document, then describe the shapes in the image, including their counts and colors. If an attachment is unavailable, say so instead of guessing.'},
      @{role='assistant';content='synthetic response'},
      @{role='user';content='ELON_FRESH_TEXT_ACCEPTANCE_V1 first 1780000000000. Reply exactly FRESH_FIRST_1780000000000.'})
}
$users = Users
if (!(Test-ChatGptFreshProjectFixtureUsers $users)) { throw 'owned_fixture_rejected' }
$earlierCitation = @{role='user';content='Read the attached text file. Quote its exact first line and cite the uploaded file using a file citation in your answer. Do not create, copy, or modify any file.'}
if (!(Test-ChatGptFreshProjectFixtureUsers @(@($earlierCitation) + $users))) { throw 'prior_owned_citation_rejected' }
$invalidUsers = @(
    {param($u) $u[0].content='personal conversation'},
    {param($u) $u[2].content=$u[2].content.Replace('FIRST_', 'FOLLOWUP_')},
    {param($u) $u[2].content=$u[2].content.Replace('FIRST_1780000000000', 'FIRST_1780000000001')},
    {param($u) $u[2].content=@('not','text')},
    {param($u) $u[2].content=$u[0].content}
)
foreach ($change in $invalidUsers) {
    $u = Users; & $change $u
    if (Test-ChatGptFreshProjectFixtureUsers $u) { throw 'unowned_fixture_accepted' }
}
if (Test-ChatGptFreshProjectFixtureUsers @($users + $users[2])) { throw 'duplicate_marker_accepted' }
if (Test-ChatGptFreshProjectFixtureUsers @()) { throw 'empty_fixture_accepted' }
function Files {
    @{receipt=@{status='succeeded';result=@{ok=$true}};
      index=@{stale=$false;conversation_path=$path;files=@(@{role='user';name='elon-chatgpt-attachment-fixture-v1.txt'})}}
}
$f = Files
if (!(Test-ChatGptFreshProjectFiles $f.receipt $f.index $path)) { throw 'owned_file_rejected' }
$invalidFiles = @(
    {param($f) $f.receipt.result.ok=$false}, {param($f) $f.receipt.result.ok='true'},
    {param($f) $f.receipt.status='pending'}, {param($f) $f.index.stale=$true},
    {param($f) $f.index.stale='false'}, {param($f) $f.index.conversation_path='/c/other'},
    {param($f) $f.index.files[0].role='assistant'}, {param($f) $f.index.files[0].name='other.txt'}
)
foreach ($change in $invalidFiles) {
    $f = Files; & $change $f
    if (Test-ChatGptFreshProjectFiles $f.receipt $f.index $path) { throw 'unproven_project_file_accepted' }
}
$before = @{conversation=@{url="https://chatgpt.com$path";messages=@(@{role='user';id='old';content='prior'})}}
$after = @{conversation=@{url="https://chatgpt.com$path";messages=@($before.conversation.messages + @{role='user';id='new';content='fixture'})}}
$main = @{social_chat=@{web_chat_conversation_path=$path}}
if (!(Test-ChatGptFreshSendContinuity $before $after $main -Prompt fixture -ProjectId $project)) { throw 'project_continuity_rejected' }
if (Test-ChatGptFreshSendContinuity $before $after $main -Prompt fixture -ProjectId $project -NewConversation) { throw 'project_new_scope_accepted' }
if (Test-ChatGptFreshSendContinuity $before $after $main -Prompt fixture -ProjectId ('g-p-' + ('b' * 32))) { throw 'different_project_continuity_accepted' }
$after.conversation.messages += @{role='user';id='duplicate';content='fixture'}
if (Test-ChatGptFreshSendContinuity $before $after $main -Prompt fixture -ProjectId $project) { throw 'project_duplicate_send_accepted' }
$old = @{observed_at_ms=1000L}
$probe = @{action='probe_conversation_project';ok=$true;observed_at_ms=1001L}
if (!(Test-ChatGptFreshProjectMembershipReceipt $old $probe)) { throw 'fresh_device_time_rejected' }
if (!(Test-ChatGptFreshProjectMembershipReceipt $null $probe)) { throw 'first_device_receipt_rejected' }
foreach ($value in @(1000L, 999L, '1002', $null)) {
    $probe.observed_at_ms=$value
    if (Test-ChatGptFreshProjectMembershipReceipt $old $probe) { throw 'stale_or_untyped_receipt_accepted' }
}
$probe.observed_at_ms=1001L; $probe.ok='true'
if (Test-ChatGptFreshProjectMembershipReceipt $old $probe) { throw 'untyped_membership_accepted' }
function Fallback {
    $u = Users
    $u[2].id = '11111111-1111-4111-8111-111111111111'
    @{pending=@{schema='elon.fresh_text_pending.v1';source='native_fixture';new_conversation=$false;
        replay_allowed=$false;project_id=$project;expected_path=$path;prompt=$u[2].content};
      web=@{authenticated=$true;streaming=$false;conversation=@{url="https://chatgpt.com$path";
        messages=@($u + @{role='assistant';state='completed';content='FRESH_FIRST_1780000000000'})}};
      main=@{active_surface='social_ai';social_chat=@{web_chat_provider_id='chatgpt_web';web_chat_conversation_path=$path;
        messages=@(@{role='friend';content='FRESH_FIRST_1780000000000'})}}}
}
$f=Fallback
if (!(Test-ChatGptFreshProjectFallbackReadback $f.pending $f.web $f.main)) { throw 'known_fallback_readback_rejected' }
$changes=@({param($f)$f.pending.replay_allowed=$true}, {param($f)$f.pending.new_conversation=$true},
    {param($f)$f.web.streaming=$true}, {param($f)$f.web.conversation.messages[3].state='streaming'},
    {param($f)$f.web.conversation.messages+=@{role='user';id='duplicate';content=$f.pending.prompt}},
    {param($f)$f.main.social_chat.web_chat_conversation_path='/c/other'}, {param($f)$f.pending.user_message_id='different'})
foreach($change in $changes){$f=Fallback;& $change $f
    if(Test-ChatGptFreshProjectFallbackReadback $f.pending $f.web $f.main){throw 'unconfirmed_fallback_readback_accepted'}}
Write-Output 'FRESH_PROJECT_SMOKE=passed path_cases=5 ownership_cases=9 file_cases=9 continuity_cases=4 membership_cases=7'
Write-Output 'FRESH_PROJECT_READBACK=passed cases=8 no_replay=true'
