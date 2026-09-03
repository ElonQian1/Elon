package com.elon.app.chatgptweb

import android.content.Context
import java.net.URI

internal class ChatGptConversationNavigationCoordinator(
    private val snapshotStore: WebChatConversationSnapshotRepository,
) {
    constructor(context: Context) : this(ChatGptConversationSnapshotStore(context))

    private var previousSnapshot: ChatGptWebSnapshot? = null
    private var pendingConversationPath: String? = null
    private var awaitingNewConversationBoundary = false
    private var navigationActive = false

    fun beginNew(previous: ChatGptWebSnapshot?): ChatGptWebSnapshot {
        begin(previous, targetPath = null, newConversation = true)
        return ChatGptWebSnapshotPresentation.newConversation(previousSnapshot)
    }

    fun beginOpen(path: String, previous: ChatGptWebSnapshot?): ChatGptWebSnapshot {
        begin(previous, targetPath = path, newConversation = false)
        return previewOpen(path, previousSnapshot)
    }

    fun previewOpen(path: String, previous: ChatGptWebSnapshot?): ChatGptWebSnapshot {
        return ChatGptWebSnapshotPresentation.loadingConversation(
            cached = snapshotStore.restore(path),
            previous = previous,
            path = path,
        )
    }

    fun shouldAccept(incoming: ChatGptWebSnapshot): Boolean {
        pendingConversationPath?.let { targetPath ->
            if (ChatGptWebConversationPath.fromUrl(incoming.url) == targetPath) {
                if (
                    ChatGptWebAccessPolicy.canChat(incoming) ||
                    ChatGptWebAccessPolicy.requiresLogin(incoming)
                ) {
                    clearBoundary()
                }
                return true
            }
            val transition = boundaryTransition(incoming)
            if (transition == WebChatNewConversationTransition.IGNORE_STALE) return false
            clearBoundary()
            return true
        }
        return when (boundaryTransition(incoming)) {
            WebChatNewConversationTransition.IGNORE_STALE -> false
            WebChatNewConversationTransition.START_NEW -> {
                clearBoundary()
                true
            }
            WebChatNewConversationTransition.CONTINUE_CURRENT -> true
        }
    }

    fun restoreAfterFailure(action: String): ChatGptWebSnapshot? {
        val matches = when (action) {
            "open_conversation" -> pendingConversationPath != null
            "new_conversation" -> awaitingNewConversationBoundary
            else -> false
        }
        if (!matches) return null
        return previousSnapshot.also { clear() }
    }

    fun save(path: String, snapshot: ChatGptWebSnapshot) = snapshotStore.save(path, snapshot)

    fun hasPending(): Boolean = pendingConversationPath != null || awaitingNewConversationBoundary

    fun isNavigating(): Boolean = navigationActive

    fun complete() = clear()

    fun clear() {
        previousSnapshot = null
        pendingConversationPath = null
        awaitingNewConversationBoundary = false
        navigationActive = false
    }

    private fun begin(
        previous: ChatGptWebSnapshot?,
        targetPath: String?,
        newConversation: Boolean,
    ) {
        previousSnapshot = previous
        pendingConversationPath = targetPath
        awaitingNewConversationBoundary = newConversation
        navigationActive = true
    }

    private fun clearBoundary() {
        pendingConversationPath = null
        awaitingNewConversationBoundary = false
    }

    private fun boundaryTransition(incoming: ChatGptWebSnapshot) =
        WebChatNewConversationPolicy.transition(
            awaitingBoundary = hasPending(),
            previous = previousSnapshot,
            incoming = incoming,
            canonicalLocation = ::canonicalLocation,
        )

    private fun canonicalLocation(url: String): String? {
        val uri = runCatching { URI(url) }.getOrNull() ?: return null
        if (!uri.scheme.equals("https", ignoreCase = true)) return null
        if (!uri.host.equals("chatgpt.com", ignoreCase = true)) return null
        if (uri.userInfo != null || (uri.port != -1 && uri.port != 443)) return null
        return uri.path.takeIf { path ->
            path == "/" ||
                ChatGptWebConversationPath.normalize(path) != null ||
                ChatGptWebConversationPath.normalizeProject(path) != null
        }
    }
}
