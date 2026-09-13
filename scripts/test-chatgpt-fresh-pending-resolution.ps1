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
$observed=$p|ConvertTo-Json|ConvertFrom-Json
$observed | Add-Member observed_path $old.path
$direct=@(Select-FreshPendingCandidates $observed @() 20)
Assert ($direct.Count -eq 1 -and $direct[0].path -ceq $old.path) 'observed_route_without_directory'
$ordered=@(Select-FreshPendingCandidates $observed @($recent,$old,$old) 20)
Assert ($ordered.Count -eq 2 -and $ordered[0].path -ceq $old.path) 'observed_before_dates_deduplicated'
foreach ($change in @(
    {param($p) $p.observed_path='https://chatgpt.com'+$p.observed_path},
    {param($p) $p.observed_path+='?different=1'},
    {param($p) $p.observed_path='/g/project/c/other'},
    {param($p) $p.observed_path='/c/foreign'},
    {param($p) $p.observed_path=@($p.observed_path)},
    {param($p) $p.observed_path=$true})) {
    $invalid=$observed|ConvertTo-Json|ConvertFrom-Json
    & $change $invalid
    Assert (!(Test-FreshPendingFixture $invalid)) 'reject_invalid_observed_route'
}
$directoryCalls=0
function Invoke-ChatGptWebSmokeAction {
    param($Runtime, $Action, $Arguments)
    if ($Action -cne 'chatgpt_get_conversations' -or $Arguments.offset -ne 0) { throw 'unexpected_directory_request' }
    $script:directoryCalls++
    @{control_ok=$true;offset=0;has_more=$false;conversations=@($recent,$old)}
}
$queue=@{candidates=@(Select-FreshPendingCandidates $observed @() 20);index=0;directory_loaded=$false}
$report=@{directory_reads=0}
$candidate=Get-FreshPendingNextCandidate -Queue $queue -Pending $observed -Report $report -Limit 20
Assert ($candidate.path -ceq $old.path -and $directoryCalls -eq 0) 'direct_before_any_directory_request'
$candidate=Get-FreshPendingNextCandidate -Queue $queue -Pending $observed -Report $report -Limit 20
Assert ($candidate.path -ceq $recent.path -and $directoryCalls -eq 1) 'catalogue_only_after_direct_miss'
Assert ($null -eq (Get-FreshPendingNextCandidate -Queue $queue -Pending $observed -Report $report -Limit 20)) 'catalogue_ends'
Assert ($directoryCalls -eq 1 -and $report.directory_reads -eq 1) 'catalogue_not_reloaded'
$queue=@{candidates=@(Select-FreshPendingCandidates $observed @() 1);index=0;directory_loaded=$false}
Get-FreshPendingNextCandidate -Queue $queue -Pending $observed -Report $report -Limit 1 | Out-Null
Assert ($null -eq (Get-FreshPendingNextCandidate -Queue $queue -Pending $observed -Report $report -Limit 1)) 'direct_counts_toward_limit'
Assert ($directoryCalls -eq 1) 'limit_prevents_directory_read'
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
$source=Get-Content (Join-Path $PSScriptRoot 'resolve-chatgpt-fresh-pending.ps1') -Raw
$wait=$source.IndexOf("-MainState -Description 'native pending lookup route'")
$inspect=$source.IndexOf('$report.inspected++')
Assert ($wait -gt 0 -and $inspect -gt $wait) 'native_route_before_inspection'
Assert ($source.Contains("catch { `$report.restoration_error = 'restoration_unconfirmed'")) 'restoration_failure_preserves_report'
Write-Output "FRESH_PENDING_RESOLUTION_TESTS=passed count=$passed"
