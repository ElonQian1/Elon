#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$base = Join-Path $root 'android/app/src/main/kotlin/com/elon/app'
$menu = Get-Content -LiteralPath (Join-Path $base 'WebChatConversationShareCoordinator.kt') -Raw
$links = Get-Content -LiteralPath (Join-Path $base 'WebChatConversationSharedLinksCoordinator.kt') -Raw
$port = Get-Content -LiteralPath (Join-Path $base 'chatgptweb/ChatGptWebConsumerPortAdapter.kt') -Raw
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
