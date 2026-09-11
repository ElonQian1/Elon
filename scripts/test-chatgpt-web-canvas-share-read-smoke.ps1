#requires -Version 7.0
$ErrorActionPreference='Stop'
$source=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'smoke-chatgpt-web-canvas-share-read-ui.ps1') -Raw
$java=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'android/SharedLinkUiAcceptance.java') -Raw
$tokens=$null; $errors=$null
[Management.Automation.Language.Parser]::ParseInput($source,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count) { throw 'canvas_read_smoke_syntax' }
foreach ($guard in @('idle_native_chat_required','Assert-ChatGptWebSmokeTrustedDevice',
    'Assert-ChatGptWebSmokeAdapterVersion',"resource='canvas'",'elon.canvas_shares.v1',
    'native_canvas_rows_mismatch',"Ui 'cancel_revoke'","Ui 'back_to_list'","Ui 'paste_check'",
    'clipboard_exact_match','links_unchanged','conversation_unchanged','draft_unchanged',
    'Stop-ChatGptWebSmokeAwakeLease','$_.request_id -notin $ids',
    'created_links=0;revoked_links=0;content_exported=$false')) {
    if (-not $source.Contains($guard)) { throw "missing_canvas_read_guard:$guard" }
}
foreach ($forbidden in @("Ui 'revoke'",'user_confirmed','send_input','start_new_web_chat_conversation',
    'chatgpt_share_conversation','chatgpt_open_conversation','Remove-Item','WriteAllText')) {
    if ($source.Contains($forbidden)) { throw "canvas_read_mutation:$forbidden" }
}
foreach ($guard in @('canvas_write_not_permitted','canvas_resource_required','canvas_retention_notice_missing',
    'web-chat-canvas-share-links-list','native_list_wrong_type','native_share_row_not_visible',
    'revoke_confirmation_not_closed','share_list_not_restored','copied_share_mismatch')) {
    if (-not $java.Contains($guard)) { throw "missing_canvas_ui_guard:$guard" }
}
if ($java -match '\.put\("(?:url|text|content|clipboard)",') { throw 'private_content_export' }
'CANVAS_SHARE_READ_SMOKE_CONTRACT=passed'
