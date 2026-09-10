#requires -Version 5.1

$ErrorActionPreference = "Stop"
$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-regenerate.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
)
if (@($errors).Count -gt 0) {
    throw "PowerShell parse failed for $path`: $($errors[0].Message)"
}

foreach ($required in @(
    "ExpectedHardwareSerial",
    "Assert-ChatGptWebSmokeTrustedDevice",
    "Assert-ChatGptWebSmokeAdapterVersion",
    "function Wait-RegeneratedReply",
    'Invoke-ReceiptAction -Action "chatgpt_new_conversation"',
    'isolated blank regenerate conversation',
    '$state.page_kind -eq "home"',
    '[string]$state.conversation.url -notlike "*$originPath*"',
    'CHATGPT_REGENERATE_PROGRESS phase=create_isolated_conversation',
    'CHATGPT_REGENERATE_PROGRESS phase=send_probe',
    'Invoke-ChatGptWebSmokeAction -Runtime $runtime',
    '-TimeoutSec $ReadyTimeoutSec',
    'regenerate probe draft synchronization',
    '[string]$state.input.text -eq $prompt',
    'CHATGPT_REGENERATE_PROGRESS phase=initial_reply_complete',
    'CHATGPT_REGENERATE_PROGRESS phase=regenerate_dispatched',
    'CHATGPT_REGENERATE_PROGRESS phase=await_regenerated_reply',
    'CHATGPT_REGENERATE_PROGRESS phase=restore_origin',
    'Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "send_input"',
    '-Action "chatgpt_regenerate_response"',
    'expected_web_action -eq "regenerate_response"',
    'assistant_identity_changed',
    'assistant_content_changed',
    'NativeRetry',
    'RequireOfficialRuntime',
    'Test-ChatGptRegeneratedReplyIdentity -Receipt $lastReceipt',
    'function Assert-RetryDocument',
    "-Arguments @{mode='model_runtime_context'}",
    'Get-ChatGptRegenerateDocumentContinuity -Baseline $Baseline -Current $live.state',
    'Regenerate acceptance deferred before write:',
    'CHATGPT_REGENERATE_PROTOCOL=',
    'Native regenerate acceptance did not restore the original conversation.',
    'web_chat_last_send_command',
    'Production regenerate-probe send was not confirmed.',
    '${function:Get-ChatGptRegenerateReplyState}',
    '& $readReplyState -State $s -Prompt $prompt -Marker $marker',
    'Get-ChatGptExistingRegenerateProbe -State $origin',
    'Assert-ChatGptRegenerateForeground -Runtime $runtime',
    '-RequireChatGptForeground',
    'CHATGPT_REGENERATE_INITIAL_REPLY_STATE=',
    'Original ChatGPT view restoration deferred: native_chat_not_foreground.',
    '-Step regenerate -Selector',
    'official_runtime_v1:regenerate_observed',
    'Retry changed the original user turn.',
    'Retry conversation changed.',
    'original_user_turn_preserved = $true',
    'regenerated_assistant_completed = $true',
    'original_conversation_restored = $true',
    'production_surface_preserved = Test-ChatGptWebSmokeActivityForeground',
    'sent_messages = if ($UseExistingProbe) { 0 } else { 1 }',
    'reused_initial_reply = [bool]$UseExistingProbe',
    'regenerated_messages = 1',
    'Register-ChatGptWebVerificationCases -Runtime $runtime',
    'reversible/regenerate_response',
    'private_content_emitted = $false',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    "CHATGPT_WEB_REGENERATE_ACCEPTANCE=passed"
)) {
    if (-not $source.Contains($required)) {
        throw "ChatGPT regenerate smoke contract is missing: $required"
    }
}

foreach ($forbidden in @(
    "chatgpt_remove_attachment",
    "chatgpt_delete",
    "removeAllCookies",
    "pm clear",
    "password",
    "access_token"
)) {
    if ($source.Contains($forbidden)) {
        throw "ChatGPT regenerate smoke contains a forbidden action: $forbidden"
    }
}

Write-Output "CHATGPT_WEB_REGENERATE_SMOKE_CONTRACT=passed"

. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-reply-state.ps1')
$receipt = [pscustomobject]@{ expected_web_action = 'regenerate_response'; status = 'succeeded';
    result = [pscustomobject]@{ ok = $true; detail = 'official_runtime_v1:regenerate_observed' } }
if (!(Test-ChatGptRegeneratedReplyIdentity -Receipt $receipt -IdentityChanged $false -ContentChanged $true -RequireOfficialRuntime $true)) {
    throw 'An owned new variant may reuse the native turn row.'
}
if (Test-ChatGptRegeneratedReplyIdentity -Receipt $receipt -IdentityChanged $false -ContentChanged $false -RequireOfficialRuntime $true) {
    throw 'An unchanged native reply is not a passing UI acceptance.'
}
$receipt.result.detail = 'legacy_success'
if (Test-ChatGptRegeneratedReplyIdentity -Receipt $receipt -IdentityChanged $false -ContentChanged $true -RequireOfficialRuntime $false) {
    throw 'Without official variant evidence, changed text alone must not validate a reused row.'
}
if (Test-ChatGptRegeneratedReplyIdentity -Receipt $receipt -IdentityChanged $true -ContentChanged $true -RequireOfficialRuntime $true) {
    throw 'A different row does not prove the requested official runtime.'
}
$receipt.result.detail = 'official_runtime_v1:regenerate_observed'
$receipt.status = 'failed'
if (Test-ChatGptRegeneratedReplyIdentity -Receipt $receipt -IdentityChanged $true -ContentChanged $true -RequireOfficialRuntime $true) {
    throw 'A failed command cannot pass from a changed row or text.'
}
Write-Output 'CHATGPT_WEB_REGENERATE_IDENTITY_CONTRACT=passed'

$nativeFence=$source.IndexOf('Assert-RetryDocument -Baseline $initialReply')
$nativeClick=$source.IndexOf('-Step regenerate -Selector')
$directFence=$source.IndexOf('Assert-RetryDocument -Baseline $initialReply', $nativeFence + 1)
$directWrite=$source.IndexOf('-Action "chatgpt_regenerate_response"', $directFence)
if ($nativeFence -lt 0 -or $nativeClick -le $nativeFence -or $directFence -le $nativeClick -or $directWrite -le $directFence) {
    throw 'Both native and direct retry must acknowledge the current document before any regeneration write.'
}

function New-RegenerateDocumentFixture {
    [pscustomobject]@{
        surface='chatgpt_web'; bridge_state='ready'; adapter_current=$true; adapter_version=323
        page_generation=2; streaming=$false; input=[pscustomobject]@{text_length=0}
        conversation=[pscustomobject]@{
            url='https://chatgpt.com/c/test-fixture'; attachments=@()
            messages=@(
                [pscustomobject]@{id='user-row';role='user';state='completed';content='synthetic prompt'},
                [pscustomobject]@{id='assistant-row';role='assistant';state='completed';content='synthetic reply'}
            )
        }
    }
}
$baseline=New-RegenerateDocumentFixture
$cases=@(
    @{reason='ready'; change={param($s)}},
    @{reason='document_changed'; change={param($s) $s.page_generation=3}},
    @{reason='document_unknown'; change={param($s) $s.page_generation=0}},
    @{reason='document_unknown'; change={param($s) $s.page_generation='2'}},
    @{reason='bridge_unavailable'; change={param($s) $s.bridge_state='connecting'}},
    @{reason='adapter_changed'; change={param($s) $s.adapter_version=322}},
    @{reason='adapter_changed'; change={param($s) $s.adapter_current=$false}},
    @{reason='conversation_changed'; change={param($s) $s.conversation.url='https://chatgpt.com/c/other-fixture'}},
    @{reason='reply_active'; change={param($s) $s.streaming=$true}},
    @{reason='reply_active'; change={param($s) $s.streaming=$null}},
    @{reason='draft_present'; change={param($s) $s.input.text_length=1}},
    @{reason='draft_present'; change={param($s) $s.input.text_length=$null}},
    @{reason='attachment_present'; change={param($s) $s.conversation.attachments=@([pscustomobject]@{name='fixture.txt'})}},
    @{reason='turn_changed'; change={param($s) $s.conversation.messages[1].content='changed reply'}},
    @{reason='turn_changed'; change={param($s) $s.conversation.messages[0].id='other-user-row'}},
    @{reason='turn_changed'; change={param($s) $s.conversation.messages[1].state='streaming'}},
    @{reason='turn_changed'; change={param($s) $s.conversation.messages=@()}},
    @{reason='surface_unavailable'; change={param($s) $s.surface='google_web'}}
)
foreach($case in $cases){
    $current=New-RegenerateDocumentFixture
    & $case.change $current
    $reason=Get-ChatGptRegenerateDocumentContinuity -Baseline $baseline -Current $current
    if($reason -cne $case.reason){throw "Regenerate document admission expected $($case.reason), got $reason."}
}
Write-Output "CHATGPT_WEB_REGENERATE_DOCUMENT_CONTRACT=passed cases=$($cases.Count)"
