#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateSet('web_search','image_generation')][string]$ToolId='web_search',
    [ValidateRange(30,600)][int]$TimeoutSec=180,
    [switch]$PreflightOnly,
    [string]$Adb='D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-trial-smoke.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-tool-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$r=New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
$r.mcp_bootstrapped=$true
$common=[IO.Path]::GetFullPath((git rev-parse --git-common-dir).Trim())
$root=Join-Path $common 'ai-acceptance-fixtures'
$seed=Get-Content (Join-Path $root 'fresh-text-pending.json') -Raw|ConvertFrom-Json
$ledgerPath=Join-Path $root 'fresh-tool-fixture.json'
$lock=$null; $origin=$null; $generation=$null; $navigated=$false; $selected=$false
$awaiting=$false; $trialRequested=$false; $prompt=''; $ledger=$null
$report=[ordered]@{schema='elon.fresh_tool_ui.v1';tool=$ToolId;stage='preflight';passed=$false;
    native_send_actions=0;native_click_acknowledged=$false;seed_sends=0;fresh_http=$false;native_output=$false;restored=$false;
    awake_restored=$false;write_unconfirmed=$false;private_content_exported=$false}
function Web { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state }
function Main { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState }
function Act([string]$Action,[hashtable]$Arguments=@{}) { Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Action -Arguments $Arguments }
function Ui([string]$Step,[hashtable]$Parameters=@{}) {
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass ConversationUiAcceptance -Step $Step -Parameters $Parameters -ResultPrefix CONVERSATION_UI_RESULT
}
function Trial([string]$Mode) { Invoke-ChatGptFreshTrial -Runtime $r -Mode $Mode }
function Stage([string]$Name) { $report.stage=$Name; Write-Host "FRESH_TOOL_STAGE=$Name" }
function Save-Ledger {
    [IO.File]::WriteAllText($ledgerPath,($ledger|ConvertTo-Json -Depth 12 -Compress),[Text.UTF8Encoding]::new($false))
}
function Idle-Reason($Main,$Web) {
    if($Main.active_surface -cne 'social_ai' -or $Main.social_chat.web_chat_provider_id -cne 'chatgpt_web'){return 'surface'}
    if($Main.input.has_text -isnot [bool] -or $Main.input.has_text){return 'native_draft'}
    if($Web.authenticated -isnot [bool] -or !$Web.authenticated){return 'identity'}
    if($Web.streaming -isnot [bool] -or $Web.streaming){return 'streaming'}
    if($Web.dictation_active -isnot [bool] -or $Web.dictation_active){return 'dictation'}
    if($Web.private_voice_native_research.phase -cne 'idle'){return 'voice'}
    if($Web.input.text -isnot [string] -or $Web.input.text -cne ''){return 'web_draft'}
    if($Web.input.official_draft_length -ne 0){return 'official_draft'}
    if($Main.social_chat.web_chat_pending_attachment_count -ne 0){return 'attachments'}
    return 'ready'
}
function Idle($Main,$Web) { return (Idle-Reason $Main $Web) -ceq 'ready' }
function Tools {
    $n=Act chatgpt_get_navigation @{section='tools'}
    return @($n.composer_sections.tools|Where-Object {$null -ne $_})
}
function Refresh-Tools {
    $d=Act chatgpt_list_composer_options @{section='tools'}
    Wait-ChatGptCommandReceipt -InvokeUiState { Web } -RequestId $d.command_receipt.request_id `
        -ExpectedAction list_composer_tools -TimeoutSec 30 -PollIntervalSec 1|Out-Null
    $items=@(Tools)
    $d=Act chatgpt_dismiss_composer_options
    Wait-ChatGptCommandReceipt -InvokeUiState { Web } -RequestId $d.command_receipt.request_id `
        -ExpectedAction dismiss_composer_menu -TimeoutSec 15 -PollIntervalSec 1|Out-Null
    return $items
}
function Clear-NativeTool([string]$Semantic) {
    if($Semantic -cnotin @('web_search','image_generation')){throw 'unknown_tool_preserved'}
    $clearBefore=Web
    if(!(Idle (Main) $clearBefore) -or !(Test-WebChatNativeChatSurfaceForeground -Runtime $r) -or
        $clearBefore.conversation.url -cne ('https://chatgpt.com'+$seed.resolved_path)){throw 'clear_context_changed'}
    Ui $(if($Semantic -ceq 'web_search'){'clear_search'}else{'clear_image'})|Out-Null
    $clearDeadline=[DateTimeOffset]::UtcNow.AddSeconds(20)
    do {
        $clearAfter=Web
        if(Test-ChatGptFreshToolClearReceipt -Before $clearBefore -After $clearAfter){break}
        Start-Sleep -Milliseconds 600
    }while([DateTimeOffset]::UtcNow -lt $clearDeadline)
    if(!(Test-ChatGptFreshToolClearReceipt -Before $clearBefore -After $clearAfter)){throw 'tool_clear_unconfirmed'}
    $clearItems=@(Refresh-Tools)
    if(!(Test-ChatGptFreshToolCleared -Before $clearBefore -After (Web) -Items $clearItems -ToolId $Semantic)){
        throw 'tool_clear_unconfirmed'
    }
}
function Assert-OwnedFixture($Web,$Main) {
    if ($Web.conversation.url -cne ('https://chatgpt.com'+$seed.resolved_path) -or
        $Main.social_chat.web_chat_conversation_path -cne $seed.resolved_path) {throw 'fixture_route_changed'}
    if (@($ledger.accepted).Count -eq 0) {
        if (!(Test-ChatGptFreshPendingReadback -Pending $seed -Web $Web -Main $Main)) {throw 'fixture_seed_not_verified'}
        return
    }
    $expected=@(@{id=$seed.user_message_id;content=$seed.prompt})+@($ledger.accepted|ForEach-Object {
        if ($_.fresh_http -isnot [bool] -or !$_.fresh_http -or $_.tool -cnotin @('web_search','image_generation')) {
            throw 'fixture_record_unverified'
        }
        @{id=$_.user_message_id;content=$_.prompt}
    })
    $users=@($Web.conversation.messages|Where-Object role -CEQ user)
    if ($users.Count -ne $expected.Count) {throw 'fixture_user_count_changed'}
    for($i=0;$i -lt $expected.Count;$i++) {
        if ($users[$i].id -cne $expected[$i].id -or $users[$i].content -cne $expected[$i].content) {
            throw 'fixture_user_changed'
        }
    }
}
try {
    $lock=[IO.File]::Open((Join-Path $root 'fresh-tool-fixture.lock'),[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
    if ($seed.schema -cne 'elon.fresh_text_pending.v1' -or $seed.source -cne 'native_fixture' -or
        $seed.readback_completed -isnot [bool] -or !$seed.readback_completed -or
        $seed.replay_allowed -isnot [bool] -or $seed.replay_allowed -or
        $seed.resolved_path -cnotmatch '^/c/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$') {throw 'resolved_seed_required'}
    $ledger=if(Test-Path -LiteralPath $ledgerPath){Get-Content $ledgerPath -Raw|ConvertFrom-Json -AsHashtable}
        else{@{schema='elon.fresh_tool_fixture.v1';path=$seed.resolved_path;accepted=@();pending=$null}}
    if ($ledger.schema -cne 'elon.fresh_tool_fixture.v1' -or $ledger.path -cne $seed.resolved_path -or
        $null -ne $ledger.pending) {throw 'tool_fixture_requires_readonly_recovery'}
    if (@($ledger.accepted|Where-Object tool -CEQ $ToolId).Count) {throw 'scope_already_verified_do_not_repeat'}
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if (!(Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready) {throw 'device_locked'}
    $origin=Main; $web=Web
    Assert-ChatGptWebSmokeAdapterVersion -State $web -ExpectedAdapterVersion (Resolve-ChatGptWebSmokeExpectedAdapterVersion)
    if (!(Test-WebChatNativeChatSurfaceForeground -Runtime $r) -or !(Idle $origin $web) -or
        !(Test-ChatGptFreshTextIdle (Trial state))) {throw 'origin_not_idle'}
    Start-ChatGptWebSmokeAwakeLease -Runtime $r|Out-Null
    $navigated=$true
    Act open_web_chat_conversation @{conversation_path=$seed.resolved_path}|Out-Null
    $web=Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 40 -Description 'owned tool fixture' -Predicate {
        param($s) $s.conversation.url -ceq ('https://chatgpt.com'+$seed.resolved_path) -and !$s.streaming -and @($s.conversation.messages).Count -gt 0
    }.GetNewClosure()
    $idleDeadline=[DateTimeOffset]::UtcNow.AddSeconds(25)
    do {
        $main=Main; $web=Web
        $report.idle_reason=Idle-Reason $main $web
        if ($report.idle_reason -ceq 'ready'){break}
        Start-Sleep -Seconds 1
    }while([DateTimeOffset]::UtcNow -lt $idleDeadline)
    if (!(Idle $main $web)) {throw 'fixture_not_idle'}
    Assert-OwnedFixture $web $main
    Stage tool_context_preparation
    $web=Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 40 -Description 'tool selection host' -Predicate {
        param($s) $s.composer_ready -ceq $true -and $s.adapter_current -ceq $true -and
            $s.conversation.url -ceq ('https://chatgpt.com'+$seed.resolved_path) -and !$s.streaming
    }.GetNewClosure()
    $main=Main
    if (!(Idle $main $web)) {throw 'fixture_not_idle'}
    Assert-OwnedFixture $web $main
    $generation=$web.page_generation
    $items=@(Refresh-Tools)
    if (@($items|Where-Object semantic -CEQ $ToolId).Count -ne 1) {throw 'tool_option_unconfirmed'}
    $inherited=@($items|Where-Object selected -EQ $true)
    if($inherited.Count){
        $owned=@($ledger.accepted|Where-Object {$_.tool -ceq $inherited[0].semantic -and
            $_.fresh_http -ceq $true -and $_.tool_restored -ceq $true})
        if($PreflightOnly -or $inherited.Count -ne 1 -or $owned.Count -ne 1){throw 'existing_tool_selection_preserved'}
        Clear-NativeTool $inherited[0].semantic
        $report.fixture_inherited_tool_cleared=$true
    }
    if ($PreflightOnly) {Stage preflight_complete}
    else {
        Stage native_tool_selection
        Ui tools|Out-Null
        Ui $(if($ToolId -ceq 'web_search'){'search'}else{'image'})|Out-Null
        $selected=$true
        $deadline=[DateTimeOffset]::UtcNow.AddSeconds(20)
        do {
            $items=@(Tools)
            $selection=@($items|Where-Object { $_.semantic -ceq $ToolId -and $_.selected -eq $true })
            if($selection.Count -eq 1){break}
            Start-Sleep -Milliseconds 600
        }while([DateTimeOffset]::UtcNow -lt $deadline)
        if($selection.Count -ne 1){throw 'native_tool_not_committed'}
        $stamp=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        $prefix='ELON_EXTENDED_TOOL_ACCEPTANCE_V1 '+$(if($ToolId -ceq 'web_search'){'SEARCH_'}else{'IMAGE_'})+$stamp
        $prompt=if($ToolId -ceq 'web_search'){
            "ELON_EXTENDED_TOOL_ACCEPTANCE_V1 SEARCH_$stamp. Use web search to find the official OpenAI homepage. Give one sentence and a clickable source citation."
        }else{
            "ELON_EXTENDED_TOOL_ACCEPTANCE_V1 IMAGE_$stamp. Create one simple image of a solid black circle centered on a white background. No text."
        }
        Act set_input_text @{text=$prompt}|Out-Null
        $baseline=Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 10 -Description 'owned tool prompt' -Predicate {
            param($s) $s.input.text -ceq $prompt -and !$s.streaming -and $s.page_generation -eq $generation
        }.GetNewClosure()
        Ui prepare_extended_tool_fixture @{fixture_prefix=$prefix}|Out-Null
        $prior=@($baseline.command_requests|ForEach-Object request_id)
        $trialRequested=$true
        $before=Trial start
        if ($before.armed -cne $true -or $before.pending -cne $false){throw 'trial_not_armed'}
        $ledger.pending=@{tool=$ToolId;prompt=$prompt;origin_path=$origin.social_chat.web_chat_conversation_path;
            generation=$generation;before_trial=$before;before_requests=$prior;replay_allowed=$false;
            baseline_users=@($baseline.conversation.messages|Where-Object role -CEQ user|ForEach-Object {@{id=$_.id;content=$_.content}})}
        Save-Ledger
        Stage native_send
        $awaiting=$true; $report.native_send_actions=1
        Ui send_extended_tool_fixture @{fixture_prefix=$prefix}|Out-Null
        $report.native_click_acknowledged=$true
        $started=[DateTimeOffset]::UtcNow
        $deadline=$started.AddSeconds($TimeoutSec)
        $partType=if($ToolId -ceq 'web_search'){'citation'}else{'image'}
        do {
            if (!(Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_changed'}
            $main=Main; $web=Web
            if ($web.page_generation -ne $generation -or $web.conversation.url -cne $baseline.conversation.url) {throw 'send_context_changed'}
            $receipts=@($web.command_requests|Where-Object { $_.request_id -cnotin $prior -and $_.expected_web_action -ceq 'send_prompt' })
            $report.send_receipts=$receipts.Count
            if($receipts.Count -eq 1 -and $receipts[0].status -cin @('failed','timed_out')) {throw 'send_receipt_not_successful'}
            if ($receipts.Count -eq 1 -and $receipts[0].status -ceq 'succeeded' -and $web.streaming -ceq $false) {
                $native=Act get_web_chat_context @{message_offset=0;message_limit=40}
                $e=Get-ChatGptFreshToolNativeEvidence -Baseline $baseline -Web $web -Main $main -Native $native -Prompt $prompt -PartType $partType
                $report.output_reason=$e.reason
                if($e.ready){
                    $after=Trial state
                    $report.fresh_http=Test-ChatGptFreshSendEvidence -Before $before -After $after -Receipt $receipts[0]
                    $report.native_output=$true; $report.stream_events=$after.stream_events; $report.history=$after.history
                    $report.elapsed_ms=[long]([DateTimeOffset]::UtcNow-$started).TotalMilliseconds
                    if (!$report.fresh_http){throw 'independent_transport_not_verified'}
                    $awaiting=$false
                    $user=@($web.conversation.messages|Where-Object {$_.role -ceq 'user' -and $_.content -ceq $prompt})[0]
                    $ledger.accepted+=@{tool=$ToolId;prompt=$prompt;user_message_id=$user.id;fresh_http=$true}
                    $ledger.pending=$null; Save-Ledger
                    Stage complete
                    break
                }
            }
            Start-Sleep -Seconds 2
        }while([DateTimeOffset]::UtcNow -lt $deadline)
        if($awaiting){throw 'tool_reply_unconfirmed'}
    }
}catch{
    $message=[string]$_.Exception.Message
    $report.error=if($message -cmatch '^[a-z_]+$'){$message}else{'tool_acceptance_failed'}
    if($message -cmatch '^Semantic UI acceptance failed: ([a-z_]+)$'){$report.error=$Matches[1]}
    $report.error_line=$_.InvocationInfo.ScriptLineNumber
    if($navigated -and !$awaiting){
        try{
            $web=Web
            $report.failure_context=@{adapter_current=$web.adapter_current;composer_ready=$web.composer_ready;
                bridge_state=$web.bridge_state;same_generation=($web.page_generation -eq $generation)}
            $d=Act chatgpt_private_protocol_probe @{mode='composer_tool_context'}
            $v=Wait-ChatGptCommandReceipt -InvokeUiState { Web } -RequestId $d.command_receipt.request_id `
                -ExpectedAction private_protocol_probe -TimeoutSec 10 -PollIntervalSec 1
            $detail=[string]$v.receipt.result.detail
            if($detail -cmatch '^composer_tool_context:[a-z_]+$'){$report.failure_context.tool_context=$detail}
        }catch{$report.failure_probe_failed=$true}
    }
}finally{
    try{
        if($awaiting){$report.write_unconfirmed=$true;throw 'pending_write_preserved'}
        if($trialRequested -and !(Test-ChatGptFreshTextIdle (Trial end))){throw 'trial_not_disarmed'}
        if($navigated){
            if(!(Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_changed_skip_restore'}
            $web=Web
            if($web.conversation.url -cne ('https://chatgpt.com'+$seed.resolved_path) -or
                $web.streaming -cne $false -or ($null -ne $generation -and $web.page_generation -ne $generation)) {throw 'restore_context_changed'}
            if($prompt -and $web.input.text -ceq $prompt){Act set_input_text @{text=''}|Out-Null}
            elseif($web.input.text -cne ''){throw 'user_draft_preserved'}
            if($selected){
                Clear-NativeTool $ToolId
                $report.tool_restored=$true
                $accepted=@($ledger.accepted|Where-Object {$_.tool -ceq $ToolId -and $_.prompt -ceq $prompt})
                if($accepted.Count -eq 1){$accepted[0].tool_restored=$true; Save-Ledger}
            }
            $report.restored=Restore-WebChatNativeConversation -Runtime $r -ProviderId chatgpt_web `
                -ConversationPath $origin.social_chat.web_chat_conversation_path -TimeoutSec 35
        }else{$report.restored=$true}
    }catch{$report.cleanup_failed=$true}
    try{$report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r}catch{$report.awake_restored=$false}
    if($lock){$lock.Dispose()}
    $report.passed=$report.stage -cin @('complete','preflight_complete') -and $report.restored -and $report.awake_restored -and !$report.cleanup_failed
    $report|ConvertTo-Json -Depth 5 -Compress
}
if(!$report.passed){exit 1}
