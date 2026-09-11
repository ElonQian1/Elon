#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory)][switch]$CreateFixture,
    [switch]$VerifyFixtureWrites,
    [int]$ExpectedAdapterVersion = 359,
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-tool-reply.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$r=New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$report=[ordered]@{schema='elon.canvas_original_ui.v1';passed=$false;stage='prepare';sent_messages=0
    saves=0;restores=0;created_links=0;restored=$false;awake_restored=$false;content_exported=$false}
$origin=$null; $changed=$false; $dialogs=$false; $prompt=''; $fixture=''; $initialHash=''
function Main { Get-ChatGptWebNativeChatState -Runtime $r }
function Web { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state }
function Act([string]$Action,[hashtable]$Arguments=@{}) {
    Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Action -Arguments $Arguments
}
function Menu([string]$Step) {
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass ConversationUiAcceptance -Step $Step -ResultPrefix CONVERSATION_UI_RESULT
}
function Ui([string]$Step,[hashtable]$Arguments=@{}) {
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass CanvasUiAcceptance -Step $Step -Parameters $Arguments -ResultPrefix CANVAS_UI_RESULT
}
function Stage([string]$Name) { $report.stage=$Name; Write-Host "CANVAS_ORIGINAL_STAGE=$Name" }
function Ids { @((Web).command_requests | ForEach-Object request_id) }
function Wait-FixtureReply([long]$Since) {
    $until=[DateTimeOffset]::UtcNow.AddSeconds(150)
    do {
        if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_package_mismatch'}
        $m=Main; $w=Web; $receipt=$m.social_chat.web_chat_last_send_command
        if($receipt.observed_at_ms -gt $Since -and $receipt.ok -ne $true){throw 'fixture_send_unconfirmed'}
        if($receipt.observed_at_ms -gt $Since -and $receipt.ok -eq $true -and
            ([string]$receipt.detail).StartsWith('official_runtime_v1:') -and -not $w.streaming -and
            -not $m.social_chat.web_chat_streaming){
            $anchor=@($w.conversation.messages|Where-Object role -eq user)|Select-Object -Last 1
            if($anchor.id){
                $context=Act 'chatgpt_get_context' @{message_offset=[int]$anchor.index;message_limit=20}
                if($context.control_ok -ne $true -or $context.conversation_url -cne $w.conversation.url){throw 'fixture_context_changed'}
                $user=@($context.messages|Where-Object {$_.role -eq 'user' -and $_.id -eq $anchor.id})
                if($user.Count -ne 1 -or $user[0].content -cne $prompt){throw 'fixture_prompt_mismatch'}
                $evidence=Get-ChatGptWebToolReplyEvidence -Messages @($context.messages) -UserMessageId $anchor.id -ExpectedPartTypes @()
                if($evidence.matched){$report.reply_part_types=$evidence.observed_part_types;return}
            }
        }
        Start-Sleep -Seconds 2
    }while([DateTimeOffset]::UtcNow -lt $until)
    throw 'fixture_reply_timeout'
}
function Receipt([object[]]$Before,[string]$Detail='canvas_ready',[int]$Minimum=1) {
    $until=[DateTimeOffset]::UtcNow.AddSeconds(30)
    do {
        $s=Web
        $commands=@($s.command_requests | Where-Object { $_.request_id -notin $Before -and $_.expected_web_action -eq 'canvas_document' })
        if (@($commands | Where-Object { $_.status -in @('failed','timed_out') }).Count) {
            $failed=$commands | Where-Object { $_.status -in @('failed','timed_out') } | Select-Object -Last 1
            $code=[string]$failed.result.detail
            if ($code -cmatch '^canvas_[a-z0-9_]+$') { throw $code }
            throw 'canvas_command_unconfirmed'
        }
        $last=$commands | Select-Object -Last 1
        if ($commands.Count -ge $Minimum -and $last.status -eq 'succeeded' -and $last.result.detail -ceq $Detail) { return $s }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $until)
    throw 'canvas_receipt_timeout'
}
try {
    if (-not $CreateFixture) { throw 'fixture_creation_required' }
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if (-not (Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready) { throw 'device_locked' }
    $origin=Main; $before=Web
    if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_package_mismatch'}
    if($null -eq $origin.input.has_text -or $null -eq $origin.input.text_length){throw 'native_input_state_unavailable'}
    if ($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        -not $before.authenticated -or $before.streaming -or $before.dictation_active -or $origin.input.has_text -or
        [int]$before.input.official_draft_length -gt 0) { throw 'idle_native_chat_required' }
    Assert-ChatGptWebSmokeAdapterVersion -State $before -ExpectedAdapterVersion $ExpectedAdapterVersion
    Start-ChatGptWebSmokeAwakeLease -Runtime $r | Out-Null
    Stage 'create_fixture'
    $changed=$true
    Act 'start_new_web_chat_conversation' | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $r -MainState -TimeoutSec 45 -Description 'empty native fixture chat' -Predicate {
        param($s) $s.social_chat.web_chat_composer_ready -eq $true -and $s.social_chat.message_count -eq 0
    } | Out-Null
    Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 45 -Description 'empty official fixture chat' -Predicate {
        param($s) $s.composer_ready -eq $true -and $s.conversation.message_count -eq 0 -and ([uri]$s.conversation.url).AbsolutePath -eq '/'
    } | Out-Null
    $fixture='ELON_CANVAS_ACCEPTANCE_V1_' + [Guid]::NewGuid().ToString('N').Substring(0,8)
    $prompt="ELON_EXTENDED_TOOL_ACCEPTANCE_V1: Open a new canvas document titled $fixture. Put exactly these two lines in the canvas: $fixture followed by This is a disposable native editor test document. Use the canvas tool, not a code block in the chat. Do not browse, share, or create other files."
    Ui 'focus_composer' | Out-Null
    Act 'set_input_text' @{text=$prompt} | Out-Null
    $since=[long](Main).social_chat.web_chat_last_send_command.observed_at_ms
    Ui 'send_fixture' | Out-Null
    $report.sent_messages=1
    Wait-FixtureReply $since
    Stage 'native_documents'
    $dialogs=$true
    Menu 'header' | Out-Null
    Menu 'current_settings' | Out-Null
    $ids=Ids
    $report.native_entry=(Ui 'open_documents').native_entry
    $loaded=Receipt $ids
    $report.document_count=$loaded.canvas_documents.document_count
    $visible=Ui 'inspect'
    $report.empty_visible=$visible.empty
    if ($report.document_count -ne 1 -or -not $visible.first_document) { throw 'canvas_fixture_not_created' }
    Ui 'open_first' | Out-Null
    $editor=Ui 'inspect_editor'
    if (-not $editor.body.fixture -or $editor.body.length -le 0 -or $editor.body.length -gt 4096 -or $editor.save_enabled) { throw 'canvas_fixture_body_unconfirmed' }
    $initialHash=$editor.body.sha256
    $report.native_editor=$editor.body.native_body
    $report.initial_length=$editor.body.length
    if ($VerifyFixtureWrites) {
        Stage 'native_save'
        $ids=Ids
        $saved=Ui 'append_fixture_save' @{allow_fixture_write='true';expected_hash=$initialHash}
        $report.saves=1
        Receipt $ids 'canvas_saved' | Out-Null
        $after=Ui 'inspect_editor'
        if ($after.body.sha256 -cne $saved.expected_hash -or $after.save_enabled) { throw 'saved_native_body_mismatch' }
        $report.save_readback=$true
        Stage 'native_history'
        $ids=Ids
        Ui 'history' | Out-Null
        Receipt $ids 'canvas_ready' 2 | Out-Null
        Ui 'preview_first' | Out-Null
        $history=Ui 'inspect_history'
        if ($history.body.sha256 -cne $initialHash) { throw 'history_body_mismatch' }
        $report.native_history=$true
        $report.restore_canceled=(Ui 'cancel_restore').canceled
        Stage 'native_restore'
        $ids=Ids
        Ui 'restore_fixture' @{allow_fixture_write='true';expected_hash=$initialHash} | Out-Null
        $report.restores=1
        Receipt $ids 'canvas_saved' | Out-Null
        $restored=Ui 'inspect_editor'
        if ($restored.body.sha256 -cne $initialHash -or $restored.save_enabled) { throw 'restore_native_body_mismatch' }
        $report.restore_readback=$true
    }
    Stage 'native_share_lookup'
    $ids=Ids
    Ui 'share' | Out-Null
    Receipt $ids 'canvas_share_ready' 2 | Out-Null
    $report.create_canceled=(Ui 'cancel_create').canceled
    Ui 'return_editor' | Out-Null
    Ui 'close_editor' | Out-Null
    $dialogs=$false
    Stage 'complete'
} catch {
    $message=[string]$_.Exception.Message
    $report.error=if ($message -cmatch '^[a-z0-9_]+$') { $message }
        elseif ($message -match '^Semantic UI acceptance failed: ([a-z_]+)$') { $Matches[1] }
        else { 'canvas_native_acceptance_failed' }
} finally {
    try {
        if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_changed_skip_cleanup'}
        if ($dialogs) {
            for ($i=0; $i -lt 4; $i++) {
                $visible=Ui 'inspect'
                if ($visible.editor -and -not $visible.history_list -and -not $visible.history_preview) { Ui 'close_editor' | Out-Null; break }
                Menu 'back' | Out-Null
                if ($visible.empty -or $visible.list_status -or $visible.first_document) { break }
            }
        }
        $current=Main
        if ($prompt -and $current.input.has_text) { Ui 'clear_fixture_draft' | Out-Null }
        if ($changed -and $origin) {
            $report.restored=Restore-WebChatNativeConversation -Runtime $r -ProviderId chatgpt_web -ConversationPath $origin.social_chat.web_chat_conversation_path -TimeoutSec 60
        }
        $after=Main
        $report.restored=$report.restored -and $after.input.text_length -eq $origin.input.text_length -and
            $after.social_chat.web_chat_conversation_path -ceq $origin.social_chat.web_chat_conversation_path
    } catch { $report.cleanup_failed=$true }
    try { $report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r } catch { $report.awake_restored=$false }
    $report.passed=$report.stage -eq 'complete' -and $report.restored -and $report.awake_restored -and -not $report.cleanup_failed
    $report | ConvertTo-Json -Depth 5 -Compress
}
if (-not $report.passed) { exit 1 }
