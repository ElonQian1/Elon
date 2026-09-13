#requires -Version 5.1
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-fresh-trial-smoke.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-reply-state.ps1')
foreach ($file in @('chatgpt-fresh-trial-smoke.ps1','smoke-chatgpt-fresh-text-dispatch.ps1','smoke-chatgpt-web-regenerate.ps1')) {
    $tokens = $null; $errors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $file), [ref]$tokens, [ref]$errors)
    if (@($errors).Count) { throw "Parse failed: $file" }
}
function Fixture {
    [pscustomobject]@{
        before = [pscustomobject]@{schema='elon.fresh_text_trial.v1';version=6;armed=$true;pending=$false;attempts=2}
        after = [pscustomobject]@{schema='elon.fresh_text_trial.v1';version=6;operation='regenerate';attempts=3;
            dispatched=$true;accepted=$true;reconciled=$true;pending=$false;parent_role='user';stream_events=4}
        receipt = [pscustomobject]@{expected_web_action='regenerate_response';status='succeeded';
            result=[pscustomobject]@{ok=$true;detail='private_text_v1:regenerate_accepted'}}
    }
}
$good = Fixture
if (!(Test-ChatGptFreshRetryEvidence -Before $good.before -After $good.after -Receipt $good.receipt)) { throw 'Valid fresh evidence rejected.' }
foreach ($version in @(6, 7)) {
    $f = Fixture; $f.after.version = $version
    if (!(Test-ChatGptFreshRetryEvidence -Before $f.before -After $f.after -Receipt $f.receipt)) { throw 'Reviewed receipt version rejected.' }
}
$cases = @(
    {param($f) $f.after.operation='send'}, {param($f) $f.after.attempts=2}, {param($f) $f.after.attempts=4},
    {param($f) $f.after.attempts='3'}, {param($f) $f.after.accepted='true'},
    {param($f) $f.after.dispatched=$false}, {param($f) $f.after.accepted=$false},
    {param($f) $f.after.reconciled=$false}, {param($f) $f.after.pending=$true},
    {param($f) $f.after.parent_role='assistant'}, {param($f) $f.after.stream_events=0},
    {param($f) $f.after.version=5}, {param($f) $f.after.version=8}, {param($f) $f.after.version='7'},
    {param($f) $f.before.armed=$false},
    {param($f) $f.before.pending=$true}, {param($f) $f.receipt.status='failed'},
    {param($f) $f.receipt.result.detail='official_runtime_v1:regenerate_observed'}
)
foreach ($change in $cases) {
    $f=Fixture; & $change $f
    if (Test-ChatGptFreshRetryEvidence -Before $f.before -After $f.after -Receipt $f.receipt) { throw 'Unproven fresh route passed.' }
}
if (!(Test-ChatGptRegeneratedReplyIdentity -Receipt $good.receipt -IdentityChanged $false -ContentChanged $true -FreshHttpConfirmed $true)) {
    throw 'Fresh proof must permit a reused native row with changed text.'
}
if (Test-ChatGptRegeneratedReplyIdentity -Receipt $good.receipt -IdentityChanged $false -ContentChanged $true -FreshHttpConfirmed $false) {
    throw 'Legacy relay receipt alone cannot prove a new variant.'
}
if (Test-ChatGptRegeneratedReplyIdentity -Receipt $good.receipt -IdentityChanged $false -ContentChanged $false -FreshHttpConfirmed $true) {
    throw 'Unchanged native UI cannot pass.'
}
$baseline=[pscustomobject]@{surface='chatgpt_web';bridge_state='ready';adapter_current=$true;adapter_version=1;
    page_generation=1;streaming=$false;input=[pscustomobject]@{text='draft';text_length=5};
    conversation=[pscustomobject]@{url='https://chatgpt.com/c/fixture';attachments=@();messages=@(
        [pscustomobject]@{id='user-a';role='user';state='completed';content='fixture'},
        [pscustomobject]@{id='answer-a';role='assistant';state='completed';content='response'})}}
$current=$baseline | ConvertTo-Json -Depth 8 | ConvertFrom-Json
if ((Get-ChatGptRegenerateDocumentContinuity -Baseline $baseline -Current $current -ExpectedDraft 'draft') -cne 'ready') {
    throw 'Expected unsent draft must be admitted.'
}
$current.input.text='other'
if ((Get-ChatGptRegenerateDocumentContinuity -Baseline $baseline -Current $current -ExpectedDraft 'draft') -cne 'draft_present') {
    throw 'Same-length draft replacement must reject.'
}
$source=Get-Content (Join-Path $PSScriptRoot 'smoke-chatgpt-web-regenerate.ps1') -Raw
foreach($fragment in @('Test-ChatGptFreshRetryEvidence -Before $freshBefore', '-Step regenerate -Selector',
    '-ExpectedDraft $retryDraft', 'Fresh retry changed the unsent draft.',
    '-TrialRequested $freshTrialRequested', '(!$FreshHttp -or $freshCleanupConfirmed)',
    'if (!$FreshHttp)', 'fresh_http_confirmed')) {
    if (!$source.Contains($fragment)) { throw "Fresh native wiring missing: $fragment" }
}
$preflight = $source.IndexOf('$preflight = Invoke-ChatGptFreshTrial')
$newConversation = $source.IndexOf('-Action "chatgpt_new_conversation"', $preflight)
$seedGuard = $source.IndexOf('$seedAwaitingReply = $true')
$seedSend = $source.IndexOf('-Action "send_input"', $seedGuard)
$seedSettled = $source.IndexOf('$seedAwaitingReply = $false', $seedGuard)
$completeReply = $source.IndexOf('Initial ChatGPT regenerate probe did not produce a completed assistant message.')
if ($preflight -lt 0 -or $newConversation -lt $preflight -or $seedGuard -lt $newConversation -or
    $seedSend -lt $seedGuard -or $seedSettled -lt $completeReply -or
    !$source.Contains('-and !$seedAwaitingReply -and (!$FreshHttp -or $freshCleanupConfirmed)') -or
    !$source.Contains('if ($seedAwaitingReply) {')) { throw 'Uncertain seed guard wiring missing.' }
$cleanupState = $baseline
$endState = [pscustomobject]@{pending=$false;armed=$false}
$cleanupWrites = 0
$failEnd = $false
$failClear = $false
$endCalls = 0
function Invoke-ChatGptFreshTrial {
    param($Runtime, $Mode)
    $script:endCalls++
    if ($failEnd) { throw 'synthetic end failure' }
    $endState
}
function Invoke-ChatGptWebSmokeMcp { param($Runtime, $Tool) $cleanupState }
function Invoke-ChatGptWebSmokeAction {
    param($Runtime, $Action, $Arguments)
    $script:cleanupWrites++
    if ($Action -cne 'set_input_text' -or $Arguments.text -cne '') { throw 'Unexpected cleanup action.' }
}
function Wait-ChatGptWebSmokeState {
    param($Runtime, $TimeoutSec, [switch]$RequireChatGptForeground, $Description, $Predicate)
    if ($failClear) { throw 'synthetic clear timeout' }
    $clean = $baseline | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $clean.input.text=''; $clean.input.text_length=0
    if (!(& $Predicate $clean)) { throw 'Cleanup must confirm empty draft on the same document.' }
}
function Cleanup {
    param([bool]$Requested=$true)
    Close-ChatGptFreshRetryTrial -Runtime @{} -Baseline $baseline -TrialRequested $Requested -Draft 'draft' -WarningAction SilentlyContinue
}
if (!(Cleanup) -or $cleanupWrites -ne 1) { throw 'Settled trial cleanup failed.' }
$cleanupWrites=0; $endState.pending=$true
if ((Cleanup) -or $cleanupWrites) { throw 'Pending write must block cleanup.' }
$endState.pending='false'
if ((Cleanup) -or $cleanupWrites) { throw 'Unknown pending type must block cleanup.' }
$endState.pending=$false; $failEnd=$true
if ((Cleanup) -or $cleanupWrites) { throw 'Unknown trial result must block cleanup.' }
$failEnd=$false
$cleanupState=$baseline | ConvertTo-Json -Depth 8 | ConvertFrom-Json
$cleanupState.page_generation=2
if ((Cleanup) -or $cleanupWrites) { throw 'Changed page must retain draft.' }
$cleanupState.page_generation=1; $cleanupState.conversation.url='https://chatgpt.com/c/other'
if ((Cleanup) -or $cleanupWrites) { throw 'Changed conversation must retain draft.' }
$cleanupState.conversation.url=$baseline.conversation.url; $cleanupState.input.text='other'
if ((Cleanup) -or $cleanupWrites) { throw 'User-edited draft must remain untouched.' }
$cleanupState=$baseline; $failClear=$true
if (Cleanup) { throw 'Unconfirmed clear must block navigation.' }
$failClear=$false; $cleanupWrites=0; $endCalls=0
if (!(Cleanup $false) -or $cleanupWrites -ne 1 -or $endCalls) { throw 'Pre-arm failure must clean only the fixture.' }
Write-Output "CHATGPT_FRESH_TRIAL_CONTRACT=passed negative_cases=$($cases.Count) native_identity_cases=3 draft_cases=2 cleanup_cases=9"
