package com.elon.app.googleweb

internal class GoogleWebConversationNavigationCoordinator(
    private val sanitizeUrl: (String?) -> String? = GoogleWebNavigationPolicy::sanitizeConversationUrl,
) {
    private var pending: PendingOpen? = null

    fun beginOpen(path: String, restorableUrl: String) {
        pending = PendingOpen(path, restorableUrl)
    }

    fun shouldAccept(snapshotUrl: String?): Boolean {
        val target = pending ?: return true
        if (sanitizeUrl(snapshotUrl) != target.restorableUrl) return false
        pending = null
        return true
    }

    fun hasPending(): Boolean = pending != null

    fun selectedPath(fallback: String?): String? = pending?.path ?: fallback

    fun cancel() {
        pending = null
    }

    private data class PendingOpen(
        val path: String,
        val restorableUrl: String,
    )
}
