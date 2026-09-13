#requires -Version 7.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-fresh-text-smoke-evidence.ps1')
foreach ($name in @('smoke-chatgpt-fresh-text-dispatch.ps1', 'chatgpt-fresh-text-smoke-evidence.ps1')) {
    $tokens = $null; $errors = $null
    [void][Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $name), [ref]$tokens, [ref]$errors)
    if ($errors.Count) { throw "parse_failed:$name" }
}
function Fixture {
    @{
        before = @{schema='elon.fresh_text_trial.v1';version=7;armed=$true;pending=$false;attempts=0}
        after = @{schema='elon.fresh_text_trial.v1';version=7;armed=$false;pending=$false;attempts=1;
            operation='send';dispatched=$true;accepted=$true;reconciled=$true;stream_events=3;history='reconciled'}
        receipt = @{expected_web_action='send_prompt';status='succeeded';result=@{ok=$true;detail='private_text_v1:accepted'}}
    }
}
$f = Fixture
if (!(Test-ChatGptFreshSendEvidence -Before $f.before -After $f.after -Receipt $f.receipt)) { throw 'valid_send_rejected' }
$f.before.armed = $false
if (!(Test-ChatGptFreshSendEvidence -Before $f.before -After $f.after -Receipt $f.receipt -UseDefault)) { throw 'default_send_rejected' }
$cases = @(
    {param($f) $f.after.attempts=0}, {param($f) $f.after.attempts=2},
    {param($f) $f.after.attempts='1'}, {param($f) $f.after.accepted='true'},
    {param($f) $f.after.pending=$true}, {param($f) $f.after.reconciled=$false},
    {param($f) $f.after.stream_events=0}, {param($f) $f.after.history='verified'},
    {param($f) $f.after.operation='regenerate'}, {param($f) $f.after.version=6},
    {param($f) $f.before.pending=$true}, {param($f) $f.before.armed=$false},
    {param($f) $f.receipt.result.detail='official_runtime_v1:observed'},
    {param($f) $f.receipt.result.ok='true'}, {param($f) $f.receipt.status='running'}
)
foreach ($change in $cases) {
    $f = Fixture; & $change $f
    if (Test-ChatGptFreshSendEvidence -Before $f.before -After $f.after -Receipt $f.receipt) { throw 'unproven_send_accepted' }
}
$path = '/c/00000000-1111-2222-3333-444444444444'
$before = @{conversation=@{url='https://chatgpt.com/';messages=@()}}
$after = @{conversation=@{url="https://chatgpt.com$path";messages=@(@{role='user';id='user-1';content='fixture'})}}
$main = @{active_surface='social_ai';social_chat=@{web_chat_provider_id='chatgpt_web';
    web_chat_conversation_path=$path;web_chat_streaming=$false};input=@{has_text=$false}}
$web = @{surface='chatgpt_web';streaming=$false;input=@{text=''};conversation=@{url="https://chatgpt.com$path"}}
function Continuity([bool]$New = $true) {
    Test-ChatGptFreshSendContinuity -Before $before -After $after -Main $main -Prompt 'fixture' -NewConversation:$New
}
if (!(Continuity)) { throw 'new_identity_rejected' }
$before.conversation.url = "https://chatgpt.com$path"
if (Continuity) { throw 'existing_route_cannot_prove_new' }
$before.conversation.messages = @(@{role='user';id='old-1';content='prior'})
$after.conversation.messages += @{role='user';id='old-1';content='prior'}
if (!(Continuity $false)) { throw 'followup_identity_rejected' }
$after.conversation.messages[1].id = 'other'
if (Continuity $false) { throw 'prior_user_identity_changed' }
$after.conversation.messages[1].id = 'old-1'
$after.conversation.messages += @{role='user';id='duplicate';content='fixture'}
if (Continuity $false) { throw 'duplicate_send_accepted' }
$after.conversation.messages = @($after.conversation.messages | Select-Object -First 2)
$main.social_chat.web_chat_conversation_path = '/c/aaaaaaaa-1111-2222-3333-444444444444'
if (Continuity $false) { throw 'native_and_provider_routes_disagree' }
$main.social_chat.web_chat_conversation_path = $path
$f = Fixture
function RestoreSafe([bool]$Awaiting = $false) {
    Test-ChatGptFreshTextRestoreSafe -AwaitingResult $Awaiting -Trial $f.after -Main $main -Web $web -ExpectedPath $path
}
if (!(RestoreSafe)) { throw 'settled_cleanup_rejected' }
if (RestoreSafe $true) { throw 'unknown_click_cannot_navigate' }
$f.after.pending = 'false'
if (RestoreSafe) { throw 'untyped_pending_cannot_navigate' }
$f.after.pending = $true
if (RestoreSafe) { throw 'pending_write_cannot_navigate' }
$f.after.pending = $false; $f.after.armed = $true
if (RestoreSafe) { throw 'armed_trial_cannot_navigate' }
$f.after.armed = $false; $web.input.text = 'user draft'
if (RestoreSafe) { throw 'user_draft_cannot_be_replaced' }
$web.input.text = ''; $main.social_chat.web_chat_streaming = $true
if (RestoreSafe) { throw 'streaming_cleanup_rejected' }
$main.social_chat.web_chat_streaming = $false; $main.active_surface='conversation_home'
if (RestoreSafe) { throw 'another_native_surface_cannot_be_replaced' }
$main.active_surface='social_ai'; $web.conversation.url='https://chatgpt.com/images'
if (RestoreSafe) { throw 'changed_provider_route_cannot_be_replaced' }
$source = Get-Content (Join-Path $PSScriptRoot 'smoke-chatgpt-fresh-text-dispatch.ps1') -Raw
$guard = $source.IndexOf('$script:awaitingResult = $true')
$click = $source.IndexOf('-Step send_fresh_text_fixture')
$evidence = $source.IndexOf('$freshConfirmed =')
$settled = $source.IndexOf('$script:awaitingResult = $false')
if ($guard -lt 0 -or $click -lt $guard -or $evidence -lt $click -or $settled -lt $evidence -or
    !$source.Contains('if (!$NewConversation)') -or
    !$source.Contains("Native-Send 'seed' `$false `$true") -or
    !$source.Contains('($NewConversation -and $kind -eq ''first'')') -or
    !$source.Contains('Test-ChatGptFreshTextRestoreSafe -AwaitingResult $awaitingResult') -or
    !$source.Contains('pending_fresh_fixture_requires_readonly_resolution') -or
    !$source.Contains('replay_allowed=$false') -or
    $source.Contains('Write-Output "FRESH_TEXT_PROGRESS') -or
    $source.Contains('-Action set_input_text')) { throw 'native_send_safety_wiring_changed' }
Write-Output "FRESH_TEXT_SEND_EVIDENCE=passed send_negative_cases=$($cases.Count) continuity_cases=6 cleanup_cases=9"
