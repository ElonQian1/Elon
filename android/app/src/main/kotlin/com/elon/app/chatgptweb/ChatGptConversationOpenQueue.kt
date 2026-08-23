package com.elon.app.chatgptweb

internal data class ChatGptDeferredConversationOpen(
    val path: String,
    val previousSnapshot: ChatGptWebSnapshot?,
)

/** Keeps local cache presentation independent from temporary WebView bridge readiness. */
internal class ChatGptConversationOpenQueue {
    private var pending: ChatGptDeferredConversationOpen? = null

    fun enqueue(path: String, previousSnapshot: ChatGptWebSnapshot?): ChatGptDeferredConversationOpen {
        val existing = pending
        val originalSnapshot = if (existing == null) previousSnapshot else existing.previousSnapshot
        val request = ChatGptDeferredConversationOpen(
            path = path,
            previousSnapshot = originalSnapshot,
        )
        pending = request
        return request
    }

    fun hasPending(): Boolean = pending != null

    fun take(): ChatGptDeferredConversationOpen? = pending.also { pending = null }

    fun clear() {
        pending = null
    }
}
