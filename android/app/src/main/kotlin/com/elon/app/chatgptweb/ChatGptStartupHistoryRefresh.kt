package com.elon.app.chatgptweb

internal class ChatGptStartupHistoryRefresh(restoredUrl: String) {
    private var target = ChatGptWebConversationPath.fromUrl(restoredUrl)
    private var consumed = false
    private var generation = 0L
    private var awaitingDocumentRoute = false

    fun onDocument(pageGeneration: Long) {
        if (pageGeneration <= generation) return
        generation = pageGeneration
        target = null
        consumed = false
        awaitingDocumentRoute = true
    }

    fun take(snapshot: ChatGptWebSnapshot): Boolean {
        if (consumed || target == null && !awaitingDocumentRoute) return false
        val confirmed = ChatGptWebSessionRestorer.confirmedConversationUrl(snapshot) ?: return false
        val path = ChatGptWebConversationPath.fromUrl(confirmed)
        if (awaitingDocumentRoute) {
            target = path
            awaitingDocumentRoute = false
        }
        if (path != target) {
            consumed = true
            return false
        }
        // Private identity/editor readiness is independent of usable composer DOM.
        if (!snapshot.privateSendReady || snapshot.streaming || snapshot.dictationActive ||
            snapshot.dictationCaptureActive || snapshot.dictationCapturePending
        ) return false
        consumed = true
        return true
    }

    fun clear() {
        consumed = true
        target = null
        awaitingDocumentRoute = false
    }
}
