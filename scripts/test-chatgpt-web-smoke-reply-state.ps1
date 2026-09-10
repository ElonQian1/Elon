$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-reply-state.ps1')
$marker = 'ELON-CHATGPT-REGENERATE-1789000000'
$prompt = "Reply with a fresh 12-character lowercase hexadecimal token, one space, then exactly: $marker"
$passed = 0
function Check([bool]$Condition, [string]$Name) {
    if (!$Condition) { throw "Failed: $Name" }
    $script:passed++
}
function Sample {
    [pscustomobject]@{
        surface='chatgpt_web'; adapter_current=$true; streaming=$false; bridge_state='ready'
        conversation=[pscustomobject]@{messages=@(
            [pscustomobject]@{id='private-user-id';role='user';content=$prompt;state='completed'},
            [pscustomobject]@{id='private-assistant-id';role='assistant';content="abcdef123456 $marker";state='completed'}
        )}
    }
}
function Observe($State) { Get-ChatGptRegenerateReplyState -State $State -Prompt $prompt -Marker $marker }
$s=Sample
$identity=Get-ChatGptExistingRegenerateProbe -State $s
Check ($identity.prompt -ceq $prompt -and $identity.marker -ceq $marker) 'existing_synthetic_identity'
Check (Observe $s).ready 'complete_reply'
$s.streaming=$true
Check ((Observe $s).reason -eq 'reply_streaming_or_unknown') 'active_reply'
$s.streaming=$null
Check ((Observe $s).reason -eq 'reply_streaming_or_unknown' -and !(Observe $s).streaming_known) 'unknown_stream'
$s.streaming=0
Check ((Observe $s).reason -eq 'reply_streaming_or_unknown') 'invalid_stream_type'
$s=Sample
$s.conversation.messages[0].content+="`n"
$rejected=$false
try {Get-ChatGptExistingRegenerateProbe $s | Out-Null} catch {$rejected=$true}
Check $rejected 'synthetic_prompt_exact_match'
$s=Sample
$s.conversation.messages=$s.conversation.messages[0..0]
Check ((Observe $s).reason -eq 'assistant_missing') 'no_assistant'
$s=Sample
$s.conversation.messages[1].state='streaming'
Check ((Observe $s).reason -eq 'assistant_incomplete') 'global_idle_does_not_complete_message'
$s=Sample
$s.conversation.messages[1].content='different reply'
Check ((Observe $s).reason -eq 'reply_marker_missing') 'wrong_reply'
$s=Sample
$s.conversation.messages[0].content='private unrelated user content'
Check ((Observe $s).reason -eq 'user_turn_mismatch') 'wrong_user'
$rejected=$false
try {Get-ChatGptExistingRegenerateProbe $s | Out-Null} catch {$rejected=$true}
Check $rejected 'reject_non_test_conversation'
$s=Sample
$s.conversation.messages+= $s.conversation.messages[0]
Check ((Observe $s).reason -eq 'user_turn_mismatch') 'additional_user_turn'
$rejected=$false
try {Get-ChatGptExistingRegenerateProbe $s | Out-Null} catch {$rejected=$true}
Check $rejected 'reject_extra_turn_for_reuse'
$s=Sample
$s.adapter_current=$false
Check ((Observe $s).reason -eq 'adapter_unavailable') 'stale_adapter'
$s=Sample
$s.surface='conversation_home'
Check ((Observe $s).reason -eq 'native_surface_unavailable') 'unbound_surface'
Check ((Observe $null).reason -eq 'native_surface_unavailable') 'null_snapshot'
$s=Sample
$s.conversation.messages[1].state='private arbitrary status'
$s.bridge_state='private arbitrary bridge'
$serialized=Observe $s | ConvertTo-Json -Compress
Check ($serialized -notmatch 'private|ELON-CHATGPT|abcdef123456|hexadecimal') 'allowlisted_diagnostics'
Check ((Observe $s).assistant_chars -eq $s.conversation.messages[1].content.Length) 'actual_content_length'
$script:foreground=$false
function Test-WebChatNativeChatSurfaceForeground($Runtime) { $script:foreground }
$rejected=$false
try {Assert-ChatGptRegenerateForeground -Runtime @{} } catch {
    $rejected=$_.Exception.Message -eq 'Regenerate acceptance interrupted: native_chat_not_foreground.'
}
Check $rejected 'foreground_interruption_is_explicit'
$script:foreground=$true
Assert-ChatGptRegenerateForeground -Runtime @{}
Check $true 'foreground_admitted'

# Execute the real smoke orchestration against in-memory MCP responses. No ADB,
# files, network, recording or external UI harness is invoked by these cases.
$ast=[System.Management.Automation.Language.Parser]::ParseFile(
    (Join-Path $PSScriptRoot 'smoke-chatgpt-web-regenerate.ps1'), [ref]$null, [ref]$null)
$definitions=@($ast.EndBlock.Statements | Where-Object {
    $_ -is [System.Management.Automation.Language.FunctionDefinitionAst]
} | ForEach-Object {$_.Extent.Text}) -join "`n"
$main=@($ast.EndBlock.Statements | Where-Object {
    $_ -is [System.Management.Automation.Language.TryStatementAst]
}) | Select-Object -Last 1
foreach ($case in @('reuse', 'background', 'non_probe')) {
    $outcome=& {
        param($Case, $Definitions, $Main)
        . ([scriptblock]::Create($Definitions))
        $state=Sample
        $state | Add-Member -NotePropertyMembers @{
            input=[pscustomobject]@{text_length=0};authenticated=$true;adapter_version=312
        }
        $state.conversation | Add-Member -NotePropertyMembers @{
            url='https://chatgpt.com/c/synthetic-probe';attachments=@();message_count=2
        }
        if ($Case -eq 'non_probe') {$state.conversation.messages[0].content='non test conversation'}
        $fixture=@{state=$state;foreground=($Case -ne 'background');actions=@();cleanup=0;recorded=0}
        function Test-WebChatNativeChatSurfaceForeground($Runtime) {$fixture.foreground}
        function Test-ChatGptWebSmokeActivityForeground($Runtime) {$fixture.foreground}
        function Invoke-ChatGptWebSmokeMcp($Runtime,$Tool,[switch]$MainState) {
            if ($MainState) {return [pscustomobject]@{
                active_surface='social_ai';social_chat=[pscustomobject]@{web_chat_provider_id='chatgpt_web'}
            }}
            $fixture.state
        }
        function Invoke-ChatGptWebSmokeAction($Runtime,$Action,$Arguments) {
            $fixture.actions+= $Action
            if ($Action -eq 'chatgpt_regenerate_response') {
                $fixture.state=$fixture.state | ConvertTo-Json -Depth 10 | ConvertFrom-Json
                $fixture.state.conversation.messages[1].id='new-assistant-id'
                $fixture.state | Add-Member -NotePropertyName command_requests -NotePropertyValue @(
                    [pscustomobject]@{request_id='retry-id';expected_web_action='regenerate_response'
                        status='succeeded';result=[pscustomobject]@{ok=$true;detail='official_runtime_v1:regenerate_observed'}}
                )
            } elseif ($Action -ne 'chatgpt_open_conversation') {throw "Unexpected smoke write: $Action"}
            [pscustomobject]@{command_receipt=[pscustomobject]@{request_id='retry-id'}}
        }
        function Wait-ChatGptWebSmokeState($Runtime,$TimeoutSec,$Description,$Predicate,[switch]$RequireChatGptForeground) {
            if (!$RequireChatGptForeground) {throw 'Foreground guard missing'}
            if (!(& $Predicate $fixture.state)) {throw 'Fixture does not satisfy smoke predicate'}
            $fixture.state
        }
        function Wait-ChatGptCommandReceipt($InvokeUiState,$RequestId,$ExpectedAction,$TimeoutSec,$PollIntervalSec) {
            & $InvokeUiState | Out-Null
        }
        function Assert-ChatGptWebSmokeAdapterVersion($State,$ExpectedAdapterVersion) {}
        function Normalize-ChatGptProbeReply($Value) {[string]$Value}
        function Stop-ChatGptWebSmokeAwakeLease($Runtime) {$fixture.cleanup++}
        function Register-ChatGptWebVerificationCases($Runtime,$CaseIds,$ExpectedAdapterVersion) {$fixture.recorded++}
        $runtime=@{}; $ReadyTimeoutSec=30; $ReplyTimeoutSec=30; $PollIntervalSec=1; $ExpectedAdapterVersion=312
        $UseCurrentNativeSurface=$true; $UseExistingProbe=$true; $NativeRetry=$false; $RequireOfficialRuntime=$true
        $result=$null; $originPath=''; $originCaptured=$false; $originRestored=$false; $failure=''
        try {. ([scriptblock]::Create($Main.Extent.Text)) | Out-Null} catch {$failure=$_.Exception.Message}
        [pscustomobject]@{result=$result;actions=$fixture.actions;cleanup=$fixture.cleanup;recorded=$fixture.recorded;failure=$failure}
    } $case $definitions $main
    if ($case -eq 'reuse') {
        if ($outcome.failure) {throw "Reuse fixture failed: $($outcome.failure)"}
        Check ($outcome.result.passed -and $outcome.result.sent_messages -eq 0 -and
            $outcome.result.reused_initial_reply -and $outcome.result.original_user_turn_preserved -and
            ($outcome.actions -join ',') -eq 'chatgpt_regenerate_response,chatgpt_open_conversation' -and
            $outcome.recorded -eq 1 -and $outcome.cleanup -eq 1) 'real_orchestration_reuses_without_send'
    } elseif ($case -eq 'background') {
        Check ($outcome.failure -like '*native_chat_not_foreground*' -and $outcome.actions.Count -eq 0 -and
            $outcome.recorded -eq 0 -and $outcome.cleanup -eq 1) 'real_orchestration_stops_before_background_dispatch'
    } else {
        Check ($outcome.failure -eq 'Existing conversation is not an isolated regenerate probe.' -and
            ($outcome.actions -join ',') -eq 'chatgpt_open_conversation' -and $outcome.recorded -eq 0 -and
            $outcome.cleanup -eq 1) 'real_orchestration_rejects_non_probe_without_send'
    }
}
Write-Output "REGENERATE_REPLY_STATE_TESTS=passed count=$passed"
