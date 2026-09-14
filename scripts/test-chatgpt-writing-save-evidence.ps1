#requires -Version 7.0
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'chatgpt-writing-save-evidence.ps1')
$count=0
function Check([bool]$Value,[string]$Name) {
    if (!$Value) { throw "writing_save_evidence_failed:$Name" }
    $script:count++
}
function Fixture {
    $path='/c/11111111-1111-4111-8111-111111111111'
    $receipt=@{request_id='mcp_save1';expected_web_action='writing_block';status='succeeded';result=@{ok=$true;detail='writing_saved'}}
    $web=@{authenticated=$true;page_generation='1';streaming=$false;dictation_active=$false;
        command_requests=@($receipt);conversation=@{url="https://chatgpt.com$path";messages=@(
            @{id='message';role='assistant';state='completed';parts=@(@{type='writing_block'})})}}
    return @{Web=$web;BeforeRequestIds=@('mcp_prepare1');ExpectedPath=$path;ExpectedGeneration='1';MessageId='message';PartIndex=0}
}
$f=Fixture
Check (Get-ChatGptWritingSaveEvidence @f).confirmed 'ordinary_saved'
$f.ExpectedPath='/g/g-p-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/c/11111111-1111-4111-8111-111111111111'
$f.Web.conversation.url='https://chatgpt.com'+$f.ExpectedPath
Check (Get-ChatGptWritingSaveEvidence @f).confirmed 'project_saved'
foreach ($case in @('old_receipt','timeout','failed','pending','sync_pending','prepare_only',
    'missing_receipt','multiple_receipts','wrong_action','missing_ok','string_ok','wrong_generation',
    'wrong_path','wrong_origin','query','fragment','streaming','unknown_streaming','dictation',
    'unknown_dictation','unauthenticated','missing_message','wrong_role','incomplete_message',
    'wrong_part','invalid_request','duplicate_message','concurrent_send')) {
    $f=Fixture
    $receipt=$f.Web.command_requests[0]
    switch ($case) {
        'old_receipt' {$f.BeforeRequestIds+= 'mcp_save1'}
        'timeout' {$receipt.status='timed_out'}
        'failed' {$receipt.status='failed'}
        'pending' {$receipt.result.detail='writing_write_unconfirmed'}
        'sync_pending' {$receipt.result.detail='writing_saved_sync_pending'}
        'prepare_only' {$receipt.result.detail='writing_ready'}
        'missing_receipt' {$f.Web.command_requests=@()}
        'multiple_receipts' {$f.Web.command_requests+=@{request_id='mcp_save2';expected_web_action='writing_block'}}
        'wrong_action' {$receipt.expected_web_action='send_prompt'}
        'missing_ok' {$receipt.result.Remove('ok')}
        'string_ok' {$receipt.result.ok='true'}
        'wrong_generation' {$f.Web.page_generation='2'}
        'wrong_path' {$f.Web.conversation.url='https://chatgpt.com/c/22222222-2222-4222-8222-222222222222'}
        'wrong_origin' {$f.Web.conversation.url='https://example.test'+$f.ExpectedPath}
        'query' {$f.Web.conversation.url+='?temporary-chat=true'}
        'fragment' {$f.Web.conversation.url+='#changed'}
        'streaming' {$f.Web.streaming=$true}
        'unknown_streaming' {$f.Web.Remove('streaming')}
        'dictation' {$f.Web.dictation_active=$true}
        'unknown_dictation' {$f.Web.Remove('dictation_active')}
        'unauthenticated' {$f.Web.authenticated=$false}
        'missing_message' {$f.Web.conversation.messages=@()}
        'wrong_role' {$f.Web.conversation.messages[0].role='user'}
        'incomplete_message' {$f.Web.conversation.messages[0].state='streaming'}
        'wrong_part' {$f.Web.conversation.messages[0].parts[0].type='code'}
        'invalid_request' {$receipt.request_id='other'}
        'duplicate_message' {$f.Web.conversation.messages+=$f.Web.conversation.messages[0]}
        'concurrent_send' {$f.Web.command_requests+=@{request_id='mcp_send';expected_web_action='send_prompt'}}
    }
    $e=Get-ChatGptWritingSaveEvidence @f
    Check (!$e.confirmed) $case
    Check (!(Test-ChatGptWritingRestoreAllowed -WriteAttempted $true -Evidence $e)) "restore_$case"
}
Check (Test-ChatGptWritingRestoreAllowed -WriteAttempted $false -Evidence $null) 'no_write'
Check (!(Test-ChatGptWritingRestoreAllowed -WriteAttempted $true -Evidence $null)) 'missing_evidence'
Check (!(Test-ChatGptWritingRestoreAllowed -WriteAttempted $true -Evidence @{confirmed='true'})) 'string_evidence'
$f=Fixture
Check (Test-ChatGptWritingRestoreAllowed -WriteAttempted $true -Evidence (Get-ChatGptWritingSaveEvidence @f)) 'confirmed_restore'
& {
    Set-StrictMode -Version Latest
    Check (!(Test-ChatGptWritingRestoreAllowed -WriteAttempted $true -Evidence @{})) 'strict_missing_evidence'
    Check (!(Test-ChatGptWritingRestoreAllowed -WriteAttempted $true -Evidence $null)) 'strict_null_evidence'
}
$source=Get-Content (Join-Path $PSScriptRoot 'smoke-chatgpt-writing-block-save-ui.ps1') -Raw
$cleanup=$source.Substring($source.IndexOf('} finally {'))
$guardIndex=$cleanup.IndexOf('Test-ChatGptWritingRestoreAllowed')
Check ($guardIndex -ge 0 -and $guardIndex -lt $cleanup.IndexOf('if($opened)')) 'guard_before_close'
Check ($cleanup.Contains("throw 'cloud_save_unconfirmed_preserve_editor'")) 'unknown_stops_cleanup'
Check ($source.Contains("if (!`$saveEvidence.confirmed){throw 'cloud_save_receipt_unconfirmed'}")) 'receipt_before_pass'
Write-Output "WRITING_SAVE_EVIDENCE_TESTS=passed count=$count"
