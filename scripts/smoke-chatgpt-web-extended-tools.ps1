#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateSet('study','canvas')][string[]]$Tools = @('study','canvas'),
    [ValidateRange(0,9999)][int]$ExpectedAdapterVersion = 0,
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-tool-reply.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$report = [ordered]@{schema='elon.extended_tools.v1';passed=$false;stage='prepare';cases=@();sends=0;restored=$false;awake_restored=$false;content_exported=$false}
$origin=$null; $active=''; $changed=$false
function Main { Get-ChatGptWebNativeChatState -Runtime $runtime }
function Web { Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state }
function Act([string]$Action,[hashtable]$Arguments=@{}) {
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action -Arguments $Arguments
}
function Ui([string]$Step) {
    Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass ConversationUiAcceptance -Step $Step -ResultPrefix CONVERSATION_UI_RESULT
}
function Wait-Fresh([object[]]$Ids,[string]$Expected) {
    $until=[DateTimeOffset]::UtcNow.AddSeconds(30)
    do {
        $s=Web
        $receipt=@($s.command_requests | Where-Object { $_.request_id -notin $Ids -and $_.expected_web_action -eq $Expected }) | Select-Object -Last 1
        if ($receipt.status -in @('failed','timed_out')) { throw 'tool_command_unconfirmed' }
        if ($receipt.status -eq 'succeeded') {
            if ($receipt.result.detail -ne 'official_tool_runtime_v1:accepted') { throw 'private_tool_route_unconfirmed' }
            return $receipt
        }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $until)
    throw 'tool_command_timeout'
}
function Clear-Tool([string]$Semantic) {
    $visible=Ui 'inspect'
    if ($visible."${Semantic}_active") {
        $ids=@((Web).command_requests | ForEach-Object request_id)
        Ui "clear_$Semantic" | Out-Null
        Wait-Fresh $ids 'select_composer_tool' | Out-Null
    }
    $visible=Ui 'inspect'
    if ($visible."${Semantic}_active") { throw 'tool_chip_not_cleared' }
}
function Wait-Reply([string]$Prompt,[string]$Semantic,[long]$Since) {
    $until=[DateTimeOffset]::UtcNow.AddSeconds(150)
    do {
        $s=Main; $w=Web
        if ($s.active_surface -ne 'social_ai' -or $s.social_chat.web_chat_provider_id -ne 'chatgpt_web') { throw 'production_surface_lost' }
        $receipt=$s.social_chat.web_chat_last_send_command
        if ($receipt.observed_at_ms -gt $Since -and $receipt.ok -ne $true) { throw 'send_unconfirmed' }
        $users=@($s.social_chat.messages | Where-Object role -eq 'me')
        if ($users.Count -gt 1) { throw 'unexpected_user_turn' }
        $anchor=@($w.conversation.messages | Where-Object role -eq 'user') | Select-Object -Last 1
        if ($receipt.observed_at_ms -gt $Since -and $receipt.ok -eq $true -and
            ([string]$receipt.detail).StartsWith('official_runtime_v1:') -and
            $s.social_chat.web_chat_state -eq 'ready' -and
            @($s.social_chat.messages | Where-Object role -eq 'friend').Count -gt 0 -and
            $s.social_chat.web_chat_streaming -ne $true -and $w.streaming -ne $true -and $anchor.id) {
            $context=Act 'chatgpt_get_context' @{message_offset=[int]$anchor.index;message_limit=20}
            if ($context.control_ok -ne $true -or $context.conversation_url -ne $w.conversation.url) { throw 'reply_scope_changed' }
            $confirmed=@($context.messages | Where-Object { $_.role -eq 'user' -and $_.id -eq $anchor.id })
            if ($confirmed.Count -ne 1 -or $confirmed[0].content -cne $Prompt) { throw 'reply_prompt_mismatch' }
            $expected=if ($Semantic -eq 'canvas') { @('artifact','interactive','code') } else { @() }
            $evidence=Get-ChatGptWebToolReplyEvidence -Messages @($context.messages) -UserMessageId $anchor.id -ExpectedPartTypes $expected
            if ($evidence.matched) { return $evidence }
        }
        Start-Sleep -Seconds 2
    } while ([DateTimeOffset]::UtcNow -lt $until)
    throw 'tool_reply_timeout'
}
try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
    if ((Get-ChatGptWebSmokeUserReadiness -Runtime $runtime).ready -ne $true) { throw 'device_locked' }
    $origin=Main; $w=Web
    if ($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        $w.authenticated -ne $true -or $w.streaming -or $w.dictation_active -or $origin.input.text -or
        [int]$w.input.official_draft_length -gt 0 -or @($w.conversation.attachments | Where-Object { $_ }).Count -gt 0) { throw 'idle_authenticated_native_chat_required' }
    $report.adapter=$w.adapter_version
    if ($w.adapter_version -ne (Resolve-ChatGptWebSmokeExpectedAdapterVersion $ExpectedAdapterVersion)) { throw 'installed_adapter_mismatch' }
    $initialUi=Ui 'inspect'
    if ($initialUi.study_active -or $initialUi.canvas_active -or $initialUi.image_active -or $initialUi.search_active) { throw 'initial_selected_tool_present' }
    Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    foreach ($tool in $Tools | Select-Object -Unique) {
        $report.stage="new_$tool"; $changed=$true
        Act 'start_new_web_chat_conversation' | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -MainState -TimeoutSec 45 -Description 'blank native tool chat' -Predicate {
            param($s) $s.social_chat.web_chat_composer_ready -eq $true -and $s.social_chat.message_count -eq 0
        } | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 45 -Description 'blank official tool context' -Predicate {
            param($s) $s.composer_ready -eq $true -and $s.conversation.message_count -eq 0 -and
                ([uri]$s.conversation.url).AbsolutePath -eq '/' -and -not $s.streaming
        } | Out-Null
        $report.stage="select_$tool"
        $menu=Ui 'tools'
        if (-not $menu."${tool}_option") { throw 'native_tool_option_missing' }
        $ids=@((Web).command_requests | ForEach-Object request_id)
        $active=$tool
        Ui $tool | Out-Null
        Wait-Fresh $ids 'select_composer_tool' | Out-Null
        $visible=Ui 'inspect'
        if (-not $visible."${tool}_active") { throw 'native_selected_chip_missing' }
        $prompt=if ($tool -eq 'study') {
            'ELON_EXTENDED_TOOL_ACCEPTANCE_V1: In study mode, ask one short question about addition. Do not create files or use external tools.'
        } else {
            'ELON_EXTENDED_TOOL_ACCEPTANCE_V1: Create a canvas document with one heading called Test and one sentence saying This is a disposable test document.'
        }
        $since=[long](Main).social_chat.web_chat_last_send_command.observed_at_ms
        Act 'set_input_text' @{text=$prompt} | Out-Null
        $report.stage="send_$tool"; $report.sends++
        Ui 'send_extended_tool_fixture' | Out-Null
        $evidence=Wait-Reply $prompt $tool $since
        $report.stage="clear_$tool"
        Clear-Tool $tool; $active=''
        $report.cases+=@{tool=$tool;private_selection=$true;native_chip=$true;native_send=$true;completed_reply=$true;part_types=$evidence.observed_part_types;cleared=$true}
    }
    $report.passed=$true; $report.stage='complete'
} catch {
    $message=[string]$_.Exception.Message
    $report.error=if ($message -match '^[a-z_]+$') { $message } else { 'acceptance_failed' }
} finally {
    if ($active) { try { Clear-Tool $active } catch { $report.tool_cleanup_failed=$true } }
    if ($changed -and $origin) {
        try {
            $report.restored=Restore-WebChatNativeConversation -Runtime $runtime -ProviderId chatgpt_web -ConversationPath $origin.social_chat.web_chat_conversation_path -TimeoutSec 60
            $after=Main
            $report.restored=$report.restored -and $after.input.text -eq $origin.input.text -and
                $after.social_chat.web_chat_conversation_path -eq $origin.social_chat.web_chat_conversation_path
        } catch { $report.restored=$false }
    }
    try { $report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime } catch { $report.awake_restored=$false }
    $report.passed=$report.passed -and $report.restored -and $report.awake_restored -and -not $report.tool_cleanup_failed
    $report | ConvertTo-Json -Depth 6 -Compress
}
if (-not $report.passed) { exit 1 }
