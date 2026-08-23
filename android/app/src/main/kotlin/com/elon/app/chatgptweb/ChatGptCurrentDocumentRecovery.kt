package com.elon.app.chatgptweb

import android.webkit.WebView

internal fun repairChatGptCurrentDocument(
    webView: WebView?,
    pageAdapter: ChatGptWebPageAdapter?,
    interactionRequested: () -> Unit,
): Boolean {
    val view = webView ?: return false
    val adapter = pageAdapter ?: return false
    if (!ChatGptWebNavigationPolicy.supportsEnhancedMode(view.url)) return false
    interactionRequested()
    adapter.onHostResumed(view.url)
    adapter.requestSnapshot()
    return true
}
