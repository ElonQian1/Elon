package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession

/** The live document owns private reads independently of the rendered chat UI. */
internal object ChatGptWebConversationReadAdmission {
    fun rejection(url: String?, document: WebBridgeDocumentSession.Snapshot): String? {
        if (url != null && ChatGptWebNavigationPolicy.isAuthenticationPage(url)) return "login_required"
        if (!ChatGptWebNavigationPolicy.supportsEnhancedMode(url)) return "reader_unavailable"
        if (document.pageGeneration <= 0 ||
            !WebBridgeDocumentSession.DOCUMENT_TOKEN.matches(document.documentToken)) return "reader_unavailable"
        return null
    }
}
