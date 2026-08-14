package com.elon.app.googleweb

import com.elon.app.chatgptweb.ChatGptWebSnapshot

internal enum class GoogleWebNewConversationTransition {
    CONTINUE_CURRENT,
    IGNORE_STALE,
    START_NEW,
}

internal object GoogleWebNewConversationPolicy {
    fun transition(
        awaitingBoundary: Boolean,
        previous: ChatGptWebSnapshot?,
        incoming: ChatGptWebSnapshot,
    ): GoogleWebNewConversationTransition {
        if (!awaitingBoundary) return GoogleWebNewConversationTransition.CONTINUE_CURRENT
        val incomingUser = incoming.messages.lastOrNull { it.role == "user" }
            ?: return GoogleWebNewConversationTransition.START_NEW
        val previousUser = previous?.messages?.lastOrNull { it.role == "user" }
            ?: return GoogleWebNewConversationTransition.START_NEW
        val sameUrl = GoogleWebNavigationPolicy.sanitizeRestorableUrl(previous.url) ==
            GoogleWebNavigationPolicy.sanitizeRestorableUrl(incoming.url)
        val sameUser = normalized(previousUser.content) == normalized(incomingUser.content)
        return if (sameUrl && sameUser) {
            GoogleWebNewConversationTransition.IGNORE_STALE
        } else {
            GoogleWebNewConversationTransition.START_NEW
        }
    }

    private fun normalized(value: String): String = value.trim().replace(WHITESPACE, " ")

    private val WHITESPACE = Regex("\\s+")
}
