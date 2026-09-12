#requires -Version 7.0
$ErrorActionPreference='Stop'
$smoke=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'smoke-chatgpt-web-collapsed-composer.ps1') -Raw
$java=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'android/CanvasUiAcceptance.java') -Raw
$visual=Get-Content -LiteralPath (Join-Path $PSScriptRoot '../android/app/src/main/kotlin/com/elon/app/MainSendButtonVisualActions.kt') -Raw
$tokens=$null; $errors=$null
[Management.Automation.Language.Parser]::ParseInput($smoke,[ref]$tokens,[ref]$errors)|Out-Null
if($errors.Count){throw 'collapsed_composer_smoke_syntax'}
foreach($guard in @('[Parameter(Mandatory)][switch]$CreateFixture','Assert-ChatGptWebSmokeTrustedDevice',
    'Assert-ChatGptWebSmokeAdapterVersion','idle_native_chat_required','native_input_state_unavailable',
    '$origin.input.has_text','$after.input.text_length','Test-WebChatNativeChatSurfaceForeground',
    'foreground_changed_skip_cleanup','Stop-ChatGptWebSmokeAwakeLease','Restore-WebChatNativeConversation',
    "Ui 'set_composer_fixture'","Ui 'collapse_composer'","Ui 'send_fixture'","Ui 'clear_fixture_draft'",
    'collapsed_draft_send_missing','native_send_unconfirmed','official_runtime_v1:',
    'message_offset=[int]$anchor.index','fixture_anchor_mismatch','content_exported=$false')){
    if(-not $smoke.Contains($guard)){throw "missing_collapsed_composer_guard:$guard"}
}
foreach($forbidden in @('send_input','chatgpt_send_message','set_input_text','chatgpt_set_draft','message_offset=0')){
    if($smoke.Contains($forbidden)){throw "native_composer_bypass:$forbidden"}
}
foreach($guard in @('resourceId(APP + ":id/inputLayout")','click(composerPreview())','composer_not_empty',
    'composer.isShowingHintText()','composer.isFocused()','click(input())','composer_not_editable','ACTION_SET_TEXT','composer_fixture_required',
    'getUiDevice().pressBack()','composer_not_collapsed','not_owned_draft',
    'UiObject visibleDraft = input.exists() ? input : composerPreview()',
    'click(description("web-chat-send"))','semantic_owner_mismatch')){
    if(-not $java.Contains($guard)){throw "missing_native_composer_guard:$guard"}
}
if($java.Contains('click(fixture.exists() ? fixture : text(')){throw 'unscoped_composer_preview'}
if(-not $visual.Contains('val webChatComposer = binding.modelButton.tag == WEB_CHAT_MODEL_BUTTON_OWNER') -or
    -not $visual.Contains('allowCollapsedDraftSend = webChatComposer')){throw 'web_only_collapsed_send_wiring_missing'}
'COLLAPSED_COMPOSER_NATIVE_SMOKE_CONTRACT=passed'
