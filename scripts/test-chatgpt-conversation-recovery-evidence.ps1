#requires -Version 7.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-conversation-recovery-evidence.ps1')
$path = '/c/11111111-1111-4111-8111-111111111111'
$messages = @(@{id='u';role='user';content='synthetic request'},@{id='a';role='assistant';content='synthetic answer'})
$main = @{active_surface='social_ai';input=@{has_text=$false};social_chat=@{
    interaction_mode='chat';web_chat_provider_id='chatgpt_web';web_chat_conversation_path=$path;
    web_chat_streaming=$false;messages=$messages}}
$web = @{adapter_version=417;authenticated=$true;input=@{text=''};streaming=$false;dictation_active=$false;
    private_voice_native_research=@{phase='idle'};conversation=@{url=('https://chatgpt.com'+$path);messages=$messages;
    context_complete=$true;messages_truncated=$false;message_window_start=0;available_message_count=2;message_count=2}}
$before = Get-ChatGptConversationRecoveryEvidence $main $web $path 417
if (!(Test-ChatGptConversationRecoveryMatch $before $before)) { throw 'matching_body_rejected' }
$checks = 1
foreach ($change in @(@{authenticated=$false},@{authenticated='true'},@{authenticated='True'},
    @{adapter_version='417'},@{adapter_version=418},@{streaming=$true},@{streaming='False'},
    @{dictation_active=$true},@{input=@{text='draft'}},@{private_voice_native_research=@{phase='active'}},
    @{conversation=@{url='https://chatgpt.com/';messages=$messages}})) {
    $bad = $web.Clone()
    foreach ($key in $change.Keys) { $bad[$key] = $change[$key] }
    if ($null -ne (Get-ChatGptConversationRecoveryEvidence $main $bad $path 417)) { throw 'unsafe_baseline_accepted' }
    $checks++
}
foreach ($content in @('changed','synthetic ans','',"synthetic answer`nextra")) {
    $bad = $web.Clone(); $bad.conversation = $web.conversation.Clone()
    $bad.conversation.messages = @($messages[0],@{id='a';role='assistant';content=$content})
    if (Test-ChatGptConversationRecoveryMatch $before (Get-ChatGptConversationRecoveryEvidence $main $bad $path 417)) { throw 'changed_body_accepted' }
    $checks++
}
foreach ($list in @(@($messages[1],$messages[0]),@($messages[0]),@($messages[0],$messages[1],$messages[1]))) {
    $bad = $web.Clone(); $bad.conversation = $web.conversation.Clone(); $bad.conversation.messages = $list
    if (Test-ChatGptConversationRecoveryMatch $before (Get-ChatGptConversationRecoveryEvidence $main $bad $path 417)) { throw 'changed_order_or_count_accepted' }
    $checks++
}
foreach ($change in @(@{context_complete=$false},@{context_complete='true'},@{messages_truncated=$true},
    @{message_window_start=1},@{available_message_count=3},@{message_count=3})) {
    $bad = $web.Clone(); $bad.conversation = $web.conversation.Clone()
    foreach ($key in $change.Keys) { $bad.conversation[$key] = $change[$key] }
    if ($null -ne (Get-ChatGptConversationRecoveryEvidence $main $bad $path 417)) { throw 'partial_context_accepted' }
    $checks++
}
if ($null -ne (Get-ChatGptRecoveryBodyDigest @(@{role='assistant';content=@{text='wrong'}}))) { throw 'nontext_body_accepted' }
$checks++
$main.chatgpt_web_mcp = $web
$diagnostic = Get-ChatGptConversationRecoveryDiagnostic $main $path
if ($diagnostic.native_message_count -ne 2 -or $diagnostic.web_message_count -ne 2 -or
    !$diagnostic.native_route_equal -or !$diagnostic.web_route_equal -or
    $diagnostic.failure_layer -cne 'body_or_readiness') { throw 'ready_diagnostic_invalid' }
$checks++
$absent = Get-ChatGptConversationRecoveryDiagnostic @{active_surface='project_chat'} $path
if ($null -ne $absent.native_message_count -or $null -ne $absent.web_message_count -or
    $absent.web_projection_present -or $absent.failure_layer -cne 'native_surface') { throw 'missing_projection_counted_as_message' }
$checks++
$bad = $main.Clone(); $bad.Remove('chatgpt_web_mcp')
if ((Get-ChatGptConversationRecoveryDiagnostic $bad $path).failure_layer -cne 'web_projection') { throw 'missing_web_projection_misclassified' }
$checks++
$bad = $main.Clone(); $bad.chatgpt_web_mcp = $web.Clone()
$bad.chatgpt_web_mcp.conversation = $web.conversation.Clone()
$bad.chatgpt_web_mcp.conversation.messages = @()
$empty = Get-ChatGptConversationRecoveryDiagnostic $bad $path
if ($empty.web_message_count -cne 0 -or !$empty.web_projection_present) { throw 'empty_history_reported_as_missing' }
$checks++
$bad.chatgpt_web_mcp.conversation.messages = @{content='not an array'}
if ($null -ne (Get-ChatGptConversationRecoveryDiagnostic $bad $path).web_message_count) { throw 'malformed_history_counted' }
$checks++
$bad = $main.Clone(); $bad.social_chat = $main.social_chat.Clone()
$bad.social_chat.web_chat_conversation_path = '/'
if ((Get-ChatGptConversationRecoveryDiagnostic $bad $path).failure_layer -cne 'conversation_route') { throw 'route_failure_misclassified' }
$checks++
$bad = $main.Clone(); $bad.active_surface = 'untrusted private label'
$safe = Get-ChatGptConversationRecoveryDiagnostic $bad $path | ConvertTo-Json -Depth 4 -Compress
foreach ($private in @($path, 'untrusted private label', 'synthetic request', 'synthetic answer', $before.native_digest)) {
    if ($safe.Contains($private)) { throw 'diagnostic_leaked_private_evidence' }
}
$checks++
Write-Output "CONVERSATION_RECOVERY_EVIDENCE_TESTS=$checks passed"
