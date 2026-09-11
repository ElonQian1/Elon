#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateRange(1,9999)][int]$ExpectedAdapterVersion = 357,
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$r = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$report = [ordered]@{schema='elon.canvas_share_read_ui.v1';passed=$false;stage='prepare';error=$null
    native_entry=$false;native_list=$false;selected_matches=$false;confirmation_canceled=$false
    list_restored=$false;clipboard_exact_match=$false;test_draft_cleared=$false;links_unchanged=$false
    conversation_unchanged=$false;draft_unchanged=$false;awake_restored=$false;cleanup_failed=$false
    sent_messages=0;created_links=0;revoked_links=0;content_exported=$false}
$before=$null; $original=$null; $url=''; $opened=$false
function Web { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state }
function Act([string]$Action, [hashtable]$Arguments=@{}) { Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Action -Arguments $Arguments }
function Menu([string]$Step) {
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass ConversationUiAcceptance -Step $Step -ResultPrefix CONVERSATION_UI_RESULT
}
function Ui([string]$Step) {
    $parameters=@{resource='canvas'}
    if ($url) { $parameters.url_b64=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($url)) }
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass SharedLinkUiAcceptance -Step $Step -Parameters $parameters -ResultPrefix SHARED_LINK_UI_RESULT
}
function Open-CanvasList {
    $ids=@((Web).command_requests | ForEach-Object request_id)
    Act 'open_chat_side_menu' | Out-Null
    $script:opened=$true
    Menu 'conversation_actions' | Out-Null
    Menu 'share' | Out-Null
    $report.native_entry=(Ui 'open_canvas_list').native_entry_clicked
    # Wait for the production list's request. Do not supersede it with an MCP refresh.
    $observed=Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 25 -Description 'native canvas list receipt' -Predicate {
        param($state)
        @($state.command_requests | Where-Object { $_.request_id -notin $ids -and
            $_.expected_web_action -eq 'share_conversation' -and $_.status -in @('succeeded','failed','timed_out') }).Count -gt 0
    }.GetNewClosure()
    $receipt=@($observed.command_requests | Where-Object { $_.request_id -notin $ids -and
        $_.expected_web_action -eq 'share_conversation' }) | Select-Object -Last 1
    if ($receipt.status -ne 'succeeded') { throw 'native_canvas_list_unconfirmed' }
    $page=$receipt.result.detail | ConvertFrom-Json
    if ($page.schema -cne 'elon.canvas_shares.v1' -or -not $page.complete -or $page.offset -ne 0 -or
        $null -ne $page.nextOffset -or @($page.items).Count -eq 0 -or @($page.items).Count -gt 5) { throw 'canvas_sample_unavailable' }
    $visible=Ui 'inspect_list'
    if (-not $visible.native_list_visible -or $visible.row_count -ne @($page.items).Count) { throw 'native_canvas_rows_mismatch' }
    $report.native_list=$true
    $report.row_count=$visible.row_count
    return $page
}
try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if (-not (Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready) { throw 'device_locked' }
    $original=Get-ChatGptWebNativeChatState -Runtime $r
    $before=Web
    if ($original.active_surface -ne 'social_ai' -or $original.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        -not $before.authenticated -or $before.streaming -or $before.dictation_active -or $original.input.text -or
        [int]$before.input.official_draft_length -gt 0) { throw 'idle_native_chat_required' }
    Assert-ChatGptWebSmokeAdapterVersion -State $before -ExpectedAdapterVersion $ExpectedAdapterVersion
    Start-ChatGptWebSmokeAwakeLease -Runtime $r | Out-Null
    $report.stage='native_list'
    Write-Host 'CANVAS_SHARE_STAGE=native_list'
    $page=Open-CanvasList
    $id=[string]$page.items[0].id
    if ($id -cnotmatch '^[A-Za-z0-9_-]{1,128}$') { throw 'canvas_id_unconfirmed' }
    # The bounded receipt carries an ID; the native Link model constructs the official URL.
    $url='https://chatgpt.com/canvas/shared/' + $id
    $report.stage='confirmation_cancel'
    Write-Host 'CANVAS_SHARE_STAGE=confirmation_cancel'
    $report.selected_matches=(Ui 'select_first').selected_matches
    $report.confirmation_canceled=(Ui 'cancel_revoke').confirmation_canceled
    $report.list_restored=(Ui 'back_to_list').list_restored
    $restored=Ui 'inspect_list'
    if ($restored.row_count -ne $report.row_count) { throw 'returned_canvas_rows_mismatch' }
    $report.stage='native_copy'
    Write-Host 'CANVAS_SHARE_STAGE=native_copy'
    Ui 'select_first' | Out-Null
    Ui 'copy' | Out-Null
    Act 'close_chat_side_menu' | Out-Null
    $opened=$false
    $copy=Ui 'paste_check'
    $report.clipboard_exact_match=$copy.clipboard_exact_match
    $report.test_draft_cleared=$copy.test_draft_cleared
    $report.stage='readback'
    Write-Host 'CANVAS_SHARE_STAGE=readback'
    $after=Open-CanvasList
    $oldIds=@($page.items | ForEach-Object id)
    $newIds=@($after.items | ForEach-Object id)
    $report.links_unchanged=$oldIds.Count -eq $newIds.Count -and @($oldIds | Where-Object { $_ -cnotin $newIds }).Count -eq 0
    Ui 'close_list' | Out-Null
    Act 'close_chat_side_menu' | Out-Null
    $opened=$false
    $report.stage='complete'
} catch {
    $message=[string]$_.Exception.Message
    $report.error=if ($message -match '^[a-z_]+$') { $message }
        elseif ($message -match '^Semantic UI acceptance failed: ([a-z_]+)$') { $Matches[1] }
        else { 'canvas_share_acceptance_failed' }
} finally {
    try {
        if ($opened) {
            $visible=Ui 'inspect'
            if ($visible.revoke_dialog) { Menu 'back' | Out-Null; $visible=Ui 'inspect' }
            if ($visible.selected_dialog) { Menu 'back' | Out-Null; $visible=Ui 'inspect' }
            if ($visible.canvas_list) { Ui 'close_list' | Out-Null }
            elseif ((Menu 'inspect').share_menu) { Menu 'back' | Out-Null }
            Act 'close_chat_side_menu' | Out-Null
        }
        $main=Get-ChatGptWebNativeChatState -Runtime $r
        if ($url -and $main.input.text -ceq $url) { Act 'set_input_text' @{text=''} | Out-Null }
        $current=Web
        $main=Get-ChatGptWebNativeChatState -Runtime $r
        $report.conversation_unchanged=$before -and $current.conversation.url -ceq $before.conversation.url -and
            $main.social_chat.web_chat_conversation_path -ceq $original.social_chat.web_chat_conversation_path
        $report.draft_unchanged=$original -and $main.input.text -ceq $original.input.text -and
            $current.input.official_draft_length -eq $before.input.official_draft_length
    } catch { $report.cleanup_failed=$true }
    try { $report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r }
    catch { $report.cleanup_failed=$true }
    $report.passed=$report.stage -eq 'complete' -and $report.native_entry -and $report.native_list -and
        $report.selected_matches -and $report.confirmation_canceled -and $report.list_restored -and
        $report.clipboard_exact_match -and $report.test_draft_cleared -and $report.links_unchanged -and
        $report.conversation_unchanged -and $report.draft_unchanged -and $report.awake_restored -and
        -not $report.error -and -not $report.cleanup_failed
    $report | ConvertTo-Json -Depth 4 -Compress
}
if (-not $report.passed) { throw 'canvas_share_native_read_acceptance_failed' }
