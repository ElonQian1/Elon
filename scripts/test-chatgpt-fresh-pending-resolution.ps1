#requires -Version 7.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'resolve-chatgpt-fresh-pending.ps1') -DefinitionsOnly
$p = [pscustomobject]@{ schema='elon.fresh_text_pending.v1'; source='native_fixture'; new_conversation=$true;
    replay_allowed=$false; user_message_id='aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    prompt='ELON_FRESH_TEXT_ACCEPTANCE_V1 first 1789300000000. Reply exactly FRESH_FIRST_1789300000000.';
    created_at=[DateTimeOffset]::Now }
$today = ([DateTimeOffset]$p.created_at).ToLocalTime().ToString('yyyy-MM-dd')
function Row($digit, $date=$null, $project=$null) {
    $id = "$digit$digit$digit$digit$digit$digit$digit$digit-$digit$digit$digit$digit-$digit$digit$digit$digit-$digit$digit$digit$digit-$digit$digit$digit$digit$digit$digit$digit$digit$digit$digit$digit$digit"
    [pscustomobject]@{id=$id;path="/c/$id";activity_dates=@($date);project_id=$project}
}
function Assert($value, $label) { if (!$value) { throw "failed:$label" }; $script:passed++ }
$passed=0
Assert (Test-FreshPendingFixture $p) 'controlled_fixture'
$old=Row a; $recent=Row b $today; $other=Row c $today 'g-p-project'; $bad=Row d $today; $bad.path='/c/foreign'
$selected=@(Select-FreshPendingCandidates $p @($old,$recent,$recent,$other,$bad) 20)
Assert ($selected.Count -eq 2) 'unique_owned_normal_routes'
Assert ($selected[0].id -ceq $recent.id) 'activity_first'
Assert ($selected[1].id -ceq $old.id) 'undated_not_dropped'
Assert (@(Select-FreshPendingCandidates $p @($old,$recent) 1).Count -eq 1) 'bounded'
foreach($field in @('schema','source','new_conversation','replay_allowed','user_message_id','prompt')) {
    $badPending=$p|ConvertTo-Json|ConvertFrom-Json
    $badPending.$field='invalid'
    Assert (!(Test-FreshPendingFixture $badPending)) "reject_$field"
}
$web=[pscustomobject]@{authenticated=$true;streaming=$false;dictation_active=$false;
    private_voice_native_research=@{phase='idle'};input=@{text=''}}
$main=[pscustomobject]@{input=@{has_text=$false};social_chat=@{web_chat_streaming=$false}}
$trial=[pscustomobject]@{schema='elon.fresh_text_trial.v1';version=7;pending=$false;armed=$false}
Assert (Test-FreshPendingReadOnlyIdle $web $main $trial) 'idle'
foreach ($field in @('streaming','dictation_active')) {
    $web.$field=$true; Assert (!(Test-FreshPendingReadOnlyIdle $web $main $trial)) "busy_$field"; $web.$field=$false
}
$trial.pending=$true; Assert (!(Test-FreshPendingReadOnlyIdle $web $main $trial)) 'pending_writer'
$trial.pending=$false; $web.input.text='draft'; Assert (!(Test-FreshPendingReadOnlyIdle $web $main $trial)) 'user_draft'
Write-Output "FRESH_PENDING_RESOLUTION_TESTS=passed count=$passed"
