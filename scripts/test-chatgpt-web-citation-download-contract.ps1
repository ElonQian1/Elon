#requires -Version 7.0
$ErrorActionPreference='Stop'
$path=Join-Path $PSScriptRoot 'smoke-chatgpt-web-citation-download.ps1'
$source=Get-Content -LiteralPath $path -Raw
$tokens=$null; $errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($path,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count) {throw 'citation_acceptance_parse_failed'}
foreach ($required in @(
    'Assert-ChatGptWebSmokeTrustedDevice','Get-ChatGptWebSmokeUserReadiness','existing_work_in_progress',
    'Test-WebChatNativeChatSurfaceForeground','Start-ChatGptWebSmokeAwakeLease','Stop-ChatGptWebSmokeAwakeLease',
    "Act 'start_new_web_chat_conversation'",'ordinary_chat_required',"Act 'send_input'",
    'fixed_ascii_text_v1','assistant_fixture_citation_unavailable','saved_fixture_hash_mismatch',
    "Ui 'current_settings'","Ui 'file_row'","Ui 'download' -Download","Ui 'wait_download' -Download",
    'download_save_unconfirmed','saved_fixture_not_unique','operation_still_active','changed_draft_preserved',
    'conversation_path=$origin.social_chat.web_chat_conversation_path',
    '$report.passed -and $report.restored -and $report.awake_restored',
    'synthetic_remote_artifacts_may_remain','content_exported=$false','Select-Object method,path,status'
    'existing_synthetic_fixture_unavailable', '.Contains($prompt,[StringComparison]::Ordinal)', 'fixture_checkpoint_invalid'
    'download_receipt_unavailable', 'download_detail', 'status,responseState,responseFields'
    'ExistingProjectFixture','project_synthetic_fixture_unconfirmed','project_fixture_attachment_unconfirmed'
    'project_fixture_prior_send_unconfirmed','project_citation_reply_unconfirmed','project_fixture_verified=$true'
    'if ($origin.social_chat.web_chat_conversation_path)', 'elseif ($ReuseFixture)'
    'project_control_requires_existing_fixture','if ($ProjectAttachmentControl) {return}'
    '$files[$i].role -eq $selectedRole','project_attachment_control_unavailable'
    '$report.cached_directory_candidate=$page.stale -eq $true'
    'invalid_fresh_project_fixture_mode','fresh_project_membership_unconfirmed'
    "Act 'open_web_chat_project'",'project_fixture_path_unconfirmed'
    'foreach ($offset in @(0,50,100,150))','[regex]::Escape($verifiedProjectId)'
    '$blankRoute -and $w.composer_ready','-not $url.Query -and -not $url.Fragment'
    'project-citation-download-fixture.json','Select-Object state,received_bytes,total_bytes,can_cancel'
    'fresh_fixture_exists_use_reuse','if ($FixtureCheckpoint)',"mode='file_download_source'"
)) {if (-not $source.Contains($required)) {throw "citation_acceptance_guard_missing: $required"}}
if ([regex]::Matches($source,"Act 'send_input'").Count -ne 2 -or
    [regex]::Matches($source,"Ui 'download' -Download").Count -ne 1) {throw 'citation_acceptance_write_replay'}
foreach ($forbidden in @('chatgpt_download_conversation_file','chatgpt_delete_', 'screencap','pm clear','removeAllCookies',"@('pull'")) {
    if ($source.Contains($forbidden)) {throw "citation_acceptance_bypass_or_export: $forbidden"}
}
$java=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'android/ConversationUiAcceptance.java') -Raw
if (-not $java.Contains('fileIndex >= 0 && fileIndex < 160') -or
    -not $java.Contains('click(description("web-chat-conversation-file-" + fileIndex))')) {throw 'citation_native_row_guard_missing'}
$fixture=Get-Content -LiteralPath (Join-Path $PSScriptRoot '../android/app/src/main/kotlin/com/elon/app/ChatGptWebAcceptanceAttachmentFixture.kt') -Raw
$literal=[regex]::Match($fixture,'private const val CONTENT = ("(?:\\.|[^"\\])*")').Groups[1].Value
$bytes=[Text.Encoding]::UTF8.GetBytes(($literal | ConvertFrom-Json))
$hash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
if ($bytes.Length -ne 78 -or -not $source.Contains($hash)) {throw 'citation_fixture_bytes_drifted'}
Write-Output 'CHATGPT_CITATION_DOWNLOAD_CONTRACT=passed'
