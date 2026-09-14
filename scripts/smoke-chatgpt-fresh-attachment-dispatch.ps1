#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory)][ValidateRange(1,9999)][int]$ExpectedAdapterVersion,
    [Parameter(Mandatory)][ValidateRange(1,99999)][int]$ExpectedVersionCode,
    [ValidateRange(30,300)][int]$TimeoutSec=180,
    [string]$Adb='D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-trial-smoke.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-attachment-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-tool-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$r=New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
$r.mcp_bootstrapped=$true
$common=[IO.Path]::GetFullPath((git rev-parse --git-common-dir).Trim())
$root=Join-Path $common 'ai-acceptance-fixtures'
$ledgerPath=Join-Path $root 'fresh-attachment-fixture.json'
$locks=@();$origin=$null;$nativeOrigin=$null;$opened=$false;$navigated=$false;$staged=$false
$awaiting=$false;$trialRequested=$false;$prompt='';$generation=$null;$ledger=$null
$fixtureId='fixed_media_batch_v1'
$report=[ordered]@{schema='elon.fresh_attachment_ui.v1';scope='existing_personal_local_txt_pdf_png';stage='preflight';
    passed=$false;native_send_actions=0;fresh_http=$false;content_read=$false;cards_cleared=$false;
    restored=$false;awake_restored=$false;write_unconfirmed=$false;private_content_exported=$false}
function Web {Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state}
function Main {Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState}
function Act([string]$Action,[hashtable]$Arguments=@{}) {
    $result=Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Action -Arguments $Arguments
    if($result.control_ok -cne $true){throw 'native_action_unconfirmed'}
    return $result
}
function Trial([string]$Mode) {Invoke-ChatGptFreshTrial -Runtime $r -Mode $Mode}
function Stage([string]$Value) {$report.stage=$Value;Write-Host "FRESH_ATTACHMENT_STAGE=$Value"}
function Save-Ledger {
    [IO.File]::WriteAllText($ledgerPath,($ledger|ConvertTo-Json -Depth 14 -Compress),[Text.UTF8Encoding]::new($false))
}
function Assert-Idle($M,$W,[bool]$EmptyDraft=$true) {
    if($M.active_surface -cne 'social_ai' -or $M.social_chat.web_chat_provider_id -cne 'chatgpt_web' -or
        $W.authenticated -cne $true -or $W.streaming -cne $false -or $W.dictation_active -cne $false -or
        $W.private_voice_native_research.phase -cne 'idle'){throw 'chat_not_idle'}
    if($EmptyDraft -and ($M.input.has_text -cne $false -or $W.input.text -cne '' -or $W.input.official_draft_length -ne 0)) {
        throw 'user_draft_preserved'
    }
}
function Refresh-Tools {
    $request=Act chatgpt_list_composer_options @{section='tools'}
    Wait-ChatGptCommandReceipt -InvokeUiState {Web} -RequestId $request.command_receipt.request_id `
        -ExpectedAction list_composer_tools -TimeoutSec 30 -PollIntervalSec 1|Out-Null
    $navigation=Act chatgpt_get_navigation @{section='tools'}
    $request=Act chatgpt_dismiss_composer_options
    Wait-ChatGptCommandReceipt -InvokeUiState {Web} -RequestId $request.command_receipt.request_id `
        -ExpectedAction dismiss_composer_menu -TimeoutSec 15 -PollIntervalSec 1|Out-Null
    return @($navigation.composer_sections.tools)
}
try {
    foreach($name in @('fresh-tool-fixture.lock','fresh-attachment-fixture.lock')) {
        $locks+=,[IO.File]::Open((Join-Path $root $name),[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
    }
    $seed=Get-Content (Join-Path $root 'fresh-text-pending.json') -Raw|ConvertFrom-Json -AsHashtable
    $tools=Get-Content (Join-Path $root 'fresh-tool-fixture.json') -Raw|ConvertFrom-Json -AsHashtable
    if($seed.schema -cne 'elon.fresh_text_pending.v1' -or $seed.source -cne 'native_fixture' -or
        $seed.readback_completed -cne $true -or $seed.replay_allowed -cne $false -or
        $seed.resolved_path -cnotmatch '^/c/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$' -or
        $tools.schema -cne 'elon.fresh_tool_fixture.v1' -or $tools.path -cne $seed.resolved_path -or $null -ne $tools.pending) {
        throw 'owned_fixture_unconfirmed'
    }
    $ledger=if(Test-Path -LiteralPath $ledgerPath){Get-Content $ledgerPath -Raw|ConvertFrom-Json -AsHashtable}
        else{@{schema='elon.fresh_attachment_fixture.v1';path=$seed.resolved_path;accepted=$false;pending=$null}}
    if($ledger.schema -cne 'elon.fresh_attachment_fixture.v1' -or $ledger.path -cne $seed.resolved_path){throw 'ledger_owner_unconfirmed'}
    if($ledger.accepted){throw 'scope_already_verified_do_not_repeat'}
    if($null -ne $ledger.pending){throw 'pending_write_requires_readonly_recovery'}
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if((Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready -cne $true){throw 'device_locked'}
    $package=Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','dumpsys','package','com.elon.app') -TimeoutSec 5 -Label 'installed package version'
    if($package -notmatch '\bversionCode=(\d+)\b' -or [int]$Matches[1] -ne $ExpectedVersionCode){throw 'installed_version_mismatch'}
    $origin=Main
    if($origin.active_surface -cnotin @('conversation_home','project_chat','social_ai') -or $origin.input.has_text -cne $false) {throw 'origin_not_idle'}
    $opened=$true
    $nativeOrigin=Open-ChatGptWebNativeChatSurface -Runtime $r -TimeoutSec 40
    $web=Web; Assert-Idle $nativeOrigin $web
    Assert-ChatGptWebSmokeAdapterVersion -State $web -ExpectedAdapterVersion $ExpectedAdapterVersion
    if(!(Test-ChatGptFreshTextIdle (Trial state)) -or $nativeOrigin.social_chat.web_chat_pending_attachment_count -ne 0){throw 'origin_writer_pending'}
    Start-ChatGptWebSmokeAwakeLease -Runtime $r|Out-Null
    $navigated=$true
    Stage owned_fixture
    Act open_web_chat_conversation @{conversation_path=$seed.resolved_path}|Out-Null
    $web=Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 40 -Description 'owned attachment fixture' -Predicate {
        param($s) $s.conversation.url -ceq ('https://chatgpt.com'+$seed.resolved_path) -and $s.composer_ready -ceq $true -and
            $s.streaming -ceq $false -and @($s.conversation.messages).Count -gt 0
    }.GetNewClosure()
    $main=Main;Assert-Idle $main $web
    if($main.social_chat.web_chat_pending_attachment_count -ne 0 -or $main.chatgpt_web_acceptance_attachment.composer_pending_count -ne 0){throw 'existing_files_preserved'}
    $expected=@(@{id=$seed.user_message_id;content=$seed.prompt})+@($tools.accepted|ForEach-Object {
        if($_.fresh_http -cne $true -or $_.tool_restored -cne $true -or $_.tool -cnotin @('web_search','image_generation')){throw 'tool_fixture_unconfirmed'}
        @{id=$_.user_message_id;content=$_.prompt}
    })
    $users=@($web.conversation.messages|Where-Object role -CEQ user)
    if($users.Count -ne $expected.Count){throw 'fixture_user_count_changed'}
    for($i=0;$i -lt $users.Count;$i++){if($users[$i].id -cne $expected[$i].id -or $users[$i].content -cne $expected[$i].content){throw 'fixture_user_changed'}}
    if(@($web.conversation.messages|Where-Object {Test-ChatGptFreshMediaFacts ([string]$_.content)}).Count){throw 'fixture_contains_answer_facts'}
    Stage transport_admission
    $trialRequested=$true;$before=Trial start
    $report.trial_control=if($before.control -cmatch '^[a-z_]{1,64}$'){$before.control}else{'unknown'}
    if($before.armed -cne $true -or $before.pending -cne $false){throw 'trial_not_armed'}
    $items=@(Refresh-Tools)
    $inherited=@($items|Where-Object selected -EQ $true)
    $report.inherited_tool_count=$inherited.Count
    if($inherited.Count -eq 1){
        $toolId=[string]$inherited[0].semantic
        $owned=@($tools.accepted|Where-Object {$_.tool -ceq $toolId -and $_.fresh_http -ceq $true -and $_.tool_restored -ceq $true})
        if($toolId -cnotin @('web_search','image_generation') -or $owned.Count -ne 1){throw 'tool_selection_preserved'}
        $clearBefore=Web;Assert-Idle (Main) $clearBefore
        Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass ConversationUiAcceptance `
            -Step $(if($toolId -ceq 'web_search'){'clear_search'}else{'clear_image'}) -ResultPrefix CONVERSATION_UI_RESULT|Out-Null
        $until=[DateTimeOffset]::UtcNow.AddSeconds(15)
        do{$clearAfter=Web;if(Test-ChatGptFreshToolClearReceipt -Before $clearBefore -After $clearAfter){break};Start-Sleep -Milliseconds 500}
            while([DateTimeOffset]::UtcNow -lt $until)
        $items=@(Refresh-Tools)
        if(!(Test-ChatGptFreshToolCleared -Before $clearBefore -After (Web) -Items $items -ToolId $toolId)){throw 'owned_tool_clear_unconfirmed'}
        $report.owned_tool_cleared=$true
    }
    if(!$items.Count -or @($items|Where-Object {$_.selected -isnot [bool] -or $_.selected}).Count){throw 'tool_selection_preserved'}
    $generation=$web.page_generation
    Stage stage_files
    $stagedResult=Act stage_chatgpt_web_acceptance_attachment @{fixture_id=$fixtureId}
    $staged=$true
    if($stagedResult.fixture_staged -cne $true -or $stagedResult.composer_pending_count -ne 3){throw 'fixture_not_staged'}
    $stamp=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $prompt="ELON_FRESH_ATTACHMENT_ACCEPTANCE_V1 $stamp. Read all three attached test files. Reply in English: quote the exact first line from each document, then describe the shapes in the image, including their counts and colors. If an attachment is unavailable, say so instead of guessing."
    Act set_input_text @{text=$prompt}|Out-Null
    $baseline=Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 12 -Description 'attachment fixture draft' -Predicate {
        param($s) $s.input.text -ceq $prompt -and $s.page_generation -eq $generation -and $s.streaming -ceq $false
    }.GetNewClosure()
    $prior=@($baseline.command_requests|ForEach-Object request_id)
    $ready=Trial state
    if($ready.armed -cne $true -or $ready.pending -cne $false -or $ready.attempts -ne $before.attempts){throw 'trial_admission_changed'}
    $ledger.pending=@{prompt=$prompt;generation=$generation;before_trial=$before;before_requests=$prior;
        baseline_users=@($users|ForEach-Object {@{id=$_.id;content=$_.content}});replay_allowed=$false}
    Save-Ledger
    Stage native_send
    $since=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $awaiting=$true;$report.native_send_actions=1
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass ConversationUiAcceptance -Step send_fresh_attachment_fixture `
        -Parameters @{prompt_b64=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($prompt))} -ResultPrefix CONVERSATION_UI_RESULT|Out-Null
    $report.native_click_acknowledged=$true
    $deadline=[DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    Stage reply
    do {
        if(!(Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_changed'}
        $main=Main;$web=Web
        if($web.page_generation -ne $generation -or $web.conversation.url -cne $baseline.conversation.url){throw 'send_context_changed'}
        $report.attachment_phase=$main.social_chat.web_chat_attachment_phase
        if($report.attachment_phase -ceq 'failed'){throw 'upload_or_send_failed'}
        $receipts=@($web.command_requests|Where-Object {$_.request_id -cnotin $prior -and $_.expected_web_action -ceq 'send_prompt'})
        $report.send_receipts=$receipts.Count
        if($receipts.Count -gt 1){throw 'multiple_send_receipts'}
        if($receipts.Count -eq 1 -and $receipts[0].status -cin @('failed','timed_out')){throw 'send_not_confirmed'}
        if($receipts.Count -eq 1 -and $receipts[0].status -ceq 'succeeded' -and $web.streaming -ceq $false) {
            $native=Act get_web_chat_context @{message_offset=0;message_limit=40}
            $report.content_read=Test-ChatGptFreshAttachmentNativeEvidence -Baseline $baseline -Web $web -Main $main -Native $native -Prompt $prompt
            if($report.content_read){
                $after=Trial state
                $report.fresh_http=Test-ChatGptFreshSendEvidence -Before $before -After $after -Receipt $receipts[0]
                $upload=$main.chatgpt_web_mcp.last_attachment_upload
                $report.private_upload=$upload.ok -ceq $true -and $upload.detail -ceq 'private_attachment_associated' -and $upload.observed_at_ms -ge $since
                $report.cards_cleared=$main.social_chat.web_chat_pending_attachment_count -eq 0 -and
                    $main.chatgpt_web_acceptance_attachment.composer_pending_count -eq 0 -and $main.input.has_text -ceq $false
                $report.stream_events=$after.stream_events;$report.history=$after.history
                $report.elapsed_ms=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()-$since
                if(!$report.fresh_http -or !$report.private_upload -or !$report.cards_cleared){throw 'private_file_send_not_verified'}
                $awaiting=$false
                $ledger.accepted=$true;$ledger.user_message_id=@($web.conversation.messages|Where-Object {$_.role -ceq 'user' -and $_.content -ceq $prompt})[0].id
                $ledger.prompt=$prompt;$ledger.pending=$null;Save-Ledger
                Stage complete
                break
            }
        }
        Start-Sleep -Seconds 2
    }while([DateTimeOffset]::UtcNow -lt $deadline)
    if($awaiting){throw 'file_reply_unconfirmed'}
}catch{
    $message=[string]$_.Exception.Message
    $report.error=if($message -cmatch '^[a-z_]+$'){$message}else{'attachment_acceptance_failed'}
    if($message -cmatch '^Semantic UI acceptance failed: ([a-z_]+)$'){$report.error=$Matches[1]}
    $report.error_line=$_.InvocationInfo.ScriptLineNumber
    $report.error_file=[IO.Path]::GetFileName($_.InvocationInfo.ScriptName)
}finally{
    try{
        if($awaiting){$report.write_unconfirmed=$true;throw 'pending_write_preserved'}
        if($trialRequested -and !(Test-ChatGptFreshTextIdle (Trial end))){throw 'trial_not_disarmed'}
        if($navigated){
            $web=Web;$main=Main
            if($web.conversation.url -cne ('https://chatgpt.com'+$seed.resolved_path) -or
                $web.streaming -cne $false -or ($null -ne $generation -and $web.page_generation -ne $generation)){throw 'cleanup_context_changed'}
            if($prompt -and $web.input.text -ceq $prompt){Act set_input_text @{text=''}|Out-Null}
            elseif($web.input.text -cne ''){throw 'user_draft_preserved'}
            if($staged -and $main.chatgpt_web_acceptance_attachment.fixture_staged -ceq $true){
                Act remove_chatgpt_web_acceptance_attachment @{fixture_id=$fixtureId}|Out-Null
            }
            if(!(Restore-WebChatNativeConversation -Runtime $r -ProviderId chatgpt_web -ConversationPath $nativeOrigin.social_chat.web_chat_conversation_path -TimeoutSec 35)){throw 'native_route_restore_failed'}
        }
        if($opened -and $origin.active_surface -cin @('conversation_home','project_chat')){
            Act show_conversation_home|Out-Null
            if($origin.active_surface -ceq 'project_chat'){
                Act open_project_chat @{project_id=$origin.active_project.id;conversation_id=$origin.active_conversation.id;reload_if_missing=$false}|Out-Null
            }
            $restored=Main
            if($restored.active_surface -cne $origin.active_surface -or $restored.input.has_text -cne $false){throw 'origin_surface_restore_failed'}
            if($origin.active_surface -ceq 'project_chat' -and ($restored.active_project.id -cne $origin.active_project.id -or
                $restored.active_conversation.id -cne $origin.active_conversation.id)){throw 'origin_project_restore_failed'}
        }
        $report.restored=$true
    }catch{$report.cleanup_failed=$true}
    try{$report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r}catch{$report.awake_restored=$false}
    foreach($lock in $locks){$lock.Dispose()}
    $report.passed=$report.stage -ceq 'complete' -and $report.restored -and $report.awake_restored -and !$report.cleanup_failed
    $report|ConvertTo-Json -Depth 5 -Compress
}
if(!$report.passed){exit 1}
