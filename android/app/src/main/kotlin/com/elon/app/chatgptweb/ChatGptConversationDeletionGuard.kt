package com.elon.app.chatgptweb

/** A bounded write lease shared by all native voice launch paths. */
internal class ChatGptConversationDeletionGuard(private val nowMs: () -> Long) {
    private data class Lease(val requestId: String, val until: Long, val currentIdentity: String?, val confirmed: Boolean = false)
    private var lease: Lease? = null

    fun begin(requestId: String, voiceActive: Boolean, path: String, currentUrl: String?): String? {
        if (voiceActive) return "delete_voice_active"
        if (isBusy()) return "delete_busy"
        val target = ChatGptWebConversationPath.identity(path)
        val current = ChatGptWebConversationPath.fromUrl(currentUrl)?.let(ChatGptWebConversationPath::identity)
        lease = Lease(requestId, nowMs() + LEASE_MS, target.takeIf { it == current })
        return null
    }

    fun isBusy(): Boolean {
        if (lease?.let { !it.confirmed && nowMs() >= it.until } == true) lease = null
        return lease != null
    }

    fun accept(event: ChatGptWebEvent) {
        val pending = lease ?: return
        if (event is ChatGptWebEvent.CommandResult && event.action == "delete_conversation" &&
            event.requestId == pending.requestId) {
            lease = if (event.ok && pending.currentIdentity != null) pending.copy(confirmed = true) else null
        }
        if (event is ChatGptWebEvent.Snapshot && pending.confirmed && ChatGptWebAccessPolicy.canChat(event.value) &&
            ChatGptWebConversationPath.fromUrl(event.value.url)?.let(ChatGptWebConversationPath::identity) != pending.currentIdentity) {
            lease = null
        }
    }

    fun clear() { lease = null }

    companion object {
        // Auth (7s), one write (9s), one read-only reconciliation (9s), plus delivery.
        const val LEASE_MS = 35_000L

        fun rejection(path: String, snapshot: ChatGptWebSnapshot?, nativeDraft: String, sendPending: Boolean = false): String? {
            val current = snapshot ?: return "delete_context_unavailable"
            val identity = ChatGptWebConversationPath.identity(path) ?: return "invalid_conversation_path"
            if (sendPending) return "delete_conversation_busy"
            if (identity != ChatGptWebConversationPath.fromUrl(current.url)?.let(ChatGptWebConversationPath::identity)) return null
            if (!current.composerReady || current.streaming || current.dictationActive ||
                current.dictationCapturePending || current.dictationCaptureActive) return "delete_conversation_busy"
            if (current.draft.isNotEmpty() || nativeDraft.isNotEmpty() || current.attachments.isNotEmpty()) {
                return "delete_draft_present"
            }
            return null
        }
    }
}
