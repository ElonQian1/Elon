#requires -Version 7.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-fresh-background-smoke.ps1')
$before = @{schema='elon.fresh_text_trial.v1';pending=$false;armed=$false;attempts=2}
$current = @{schema='elon.fresh_text_trial.v1';pending=$true;armed=$false;attempts=3
    accepted=$true;dispatched=$true;phase='streaming';stream_events=2}
if (!(Test-ChatGptFreshBackgroundAdmission $before $current)) { throw 'active_fixture_rejected' }
foreach ($change in @(@{schema='wrong'},@{pending=$false},@{armed=$true},@{attempts=4},
    @{accepted=$false},@{dispatched=$false},@{phase='completed'},@{stream_events=0},
    @{accepted='true'},@{attempts='3'},@{stream_events=$null})) {
    $invalid = $current.Clone()
    foreach ($key in $change.Keys) { $invalid[$key] = $change[$key] }
    if (Test-ChatGptFreshBackgroundAdmission $before $invalid) { throw 'unowned_or_inactive_fixture_accepted' }
}
$before.pending=$true
if (Test-ChatGptFreshBackgroundAdmission $before $current) { throw 'preexisting_writer_accepted' }
Write-Output 'FRESH_BACKGROUND_ADMISSION_TESTS=13 passed'
