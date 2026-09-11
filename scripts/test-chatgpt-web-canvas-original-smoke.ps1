#requires -Version 7.0
$ErrorActionPreference='Stop'
$original=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'smoke-chatgpt-web-canvas-original-ui.ps1') -Raw
$content=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'smoke-chatgpt-web-canvas-content-ui.ps1') -Raw
$java=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'android/CanvasUiAcceptance.java') -Raw
foreach($source in @($original,$content)){
    $tokens=$null; $errors=$null
    [Management.Automation.Language.Parser]::ParseInput($source,[ref]$tokens,[ref]$errors)|Out-Null
    if($errors.Count){throw 'canvas_smoke_syntax'}
    foreach($guard in @('Assert-ChatGptWebSmokeTrustedDevice','Assert-ChatGptWebSmokeAdapterVersion',
        'idle_native_chat_required','$origin.input.has_text','$after.input.text_length','Stop-ChatGptWebSmokeAwakeLease','content_exported=$false',
        'native_input_state_unavailable','Test-WebChatNativeChatSurfaceForeground','foreground_changed_skip_cleanup')){
        if(-not $source.Contains($guard)){throw "missing_canvas_smoke_guard:$guard"}
    }
    if($source -match '\.input\.text\b'){throw 'obsolete_mcp_input_field'}
}
foreach($guard in @('[Parameter(Mandatory)][switch]$CreateFixture','if ($VerifyFixtureWrites)',
    'canvas_fixture_not_created','canvas_fixture_body_unconfirmed','saved_native_body_mismatch',
    'history_body_mismatch','restore_native_body_mismatch','Restore-WebChatNativeConversation','fixture_prompt_mismatch',
    '$receipt.observed_at_ms -gt $Since','Get-ChatGptWebToolReplyEvidence',
    "Ui 'focus_composer'","Ui 'clear_fixture_draft'","Ui 'cancel_restore'","Ui 'cancel_create'")){
    if(-not $original.Contains($guard)){throw "missing_canvas_fixture_guard:$guard"}
}
foreach($guard in @('elon.canvas_shares.v1','share_canvas_ready','native_canvas_content_mismatch',
    'canvas_content.content_length',"Canvas 'return_link'",'sent_messages=0;writes=0')){
    if(-not $content.Contains($guard)){throw "missing_canvas_read_guard:$guard"}
}
foreach($forbidden in @('send_input','start_new_web_chat_conversation','share_create','update_account','user_confirmed')){
    if($content.Contains($forbidden)){throw "canvas_read_mutation:$forbidden"}
}
foreach($guard in @('fixture_write_not_authorized','not_owned_canvas_fixture','fixture_hash_required','fixture_body_changed',
    'fixture_already_edited','not_owned_draft','canvas_body_not_native','canvas_body_not_visible','semantic_owner_mismatch',
    'restore_clicked_once','save_clicked_once','ACTION_SET_TEXT','ACTION_CLICK')){
    if(-not $java.Contains($guard)){throw "missing_canvas_semantic_guard:$guard"}
}
if($java -match '\.put\("(?:url|text|content|clipboard)",'){throw 'private_canvas_content_export'}
'CANVAS_NATIVE_SMOKE_CONTRACT=passed'
