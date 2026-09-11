#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$base = Join-Path $root 'android/app/src/main/kotlin/com/elon/app'
$menu = Get-Content -LiteralPath (Join-Path $base 'WebChatConversationShareCoordinator.kt') -Raw
$links = Get-Content -LiteralPath (Join-Path $base 'WebChatConversationSharedLinksCoordinator.kt') -Raw
$port = Get-Content -LiteralPath (Join-Path $base 'chatgptweb/ChatGptWebConsumerPortAdapter.kt') -Raw
$content = Get-Content -LiteralPath (Join-Path $base 'WebChatCanvasContentView.kt') -Raw
if (-not $menu.Contains('3 -> links.show(conversation, allConversations = true, canvas = true)')) {
    throw 'canvas_shares_not_routed_from_existing_production_menu'
}
foreach ($guard in @('port.manageCanvasShares(offset, selection)',
    'it.resource == ChatGptWebSharedLinks.Resource.CANVAS', 'web-chat-canvas-share-links-list',
    'ClipData.newPlainText(', 'link.url', 'confirmRevoke(conversation, index, link, page)',
    'port?.manageCanvasShares(selectionTicket = index.ticket, shareId = link.id, userConfirmed = true)',
    'token != epoch || !active()', 'state?.adapterCurrent != true', '"重新读取") { _, _ -> load(conversation) }')) {
    if (-not $links.Contains($guard)) { throw "missing_canvas_native_guard:$guard" }
}
foreach ($guard in @('override fun manageCanvasShares(', '.put("resource", "canvas")',
    '.put("operation", if (shareId == null) "list_account" else "revoke_account")')) {
    if (-not $port.Contains($guard)) { throw "missing_canvas_native_dispatch:$guard" }
}
foreach ($forbidden in @('evaluateJavascript', 'loadUrl(', 'composerReady', 'shareConversation(')) {
    if ($links.Contains($forbidden)) { throw 'canvas_management_uses_wrong_layer' }
}
'CANVAS_SHARE_UI_CONTRACT=passed'
foreach ($guard in @('consumerPort()?.readCanvasShare(link.id, index.ticket)',
    'it.requestId == result.requestId && it.id == link.id', 'WebChatCanvasContentView.dialog',
    'epoch += 1; showLink(conversation, index, link, page)',
    'web-chat-canvas-content-copy', 'web-chat-canvas-content-back')) {
    if (-not $links.Contains($guard)) { throw "missing_canvas_content_ui_guard:$guard" }
}
foreach ($guard in @('web-chat-canvas-share-view', 'web-chat-canvas-content-body', 'web-chat-canvas-content-title',
    'setTextIsSelectable(true)', 'text = value.content', 'value.isCode', 'web-chat-canvas-content-scroll')) {
    if (-not $content.Contains($guard)) { throw "missing_canvas_content_surface:$guard" }
}
foreach ($forbidden in @('evaluateJavascript', 'WebView(', 'loadUrl(', 'loadData(', 'Html.fromHtml', 'Linkify')) {
    if ($content.Contains($forbidden)) { throw 'canvas_content_must_remain_inert_native_text' }
}
'CANVAS_CONTENT_UI_CONTRACT=passed'
foreach ($guard in @('confirmCanvasUpdate(conversation, index, link, page)',
    'web-chat-canvas-share-update-confirm', 'web-chat-canvas-share-update-cancel',
    'consumerPort()?.updateCanvasShare(link.id, index.ticket, userConfirmed = true)',
    'timeoutCode = "share_canvas_update_unconfirmed"', 'detail != "share_canvas_updated"',
    'showCanvasContent(conversation, result, link, backToList = true) { load(conversation) }')) {
    if (-not $links.Contains($guard)) { throw "missing_canvas_publish_ui_guard:$guard" }
}
if (-not $content.Contains('web-chat-canvas-share-update') -or
    -not $port.Contains('override fun updateCanvasShare(') -or
    -not $port.Contains('.put("operation", "update_account")')) {
    throw 'canvas_publish_missing_production_entry'
}
'CANVAS_PUBLISH_UI_CONTRACT=passed'
