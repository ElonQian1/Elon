package com.elon.app.chatgptweb

internal enum class WebChatNewConversationTransition {
    CONTINUE_CURRENT,
    IGNORE_STALE,
    START_NEW,
}

internal object WebChatNewConversationPolicy {
    fun transition(
        awaitingBoundary: Boolean,
        previous: ChatGptWebSnapshot?,
        incoming: ChatGptWebSnapshot,
        canonicalLocation: (String) -> String?,
    ): WebChatNewConversationTransition {
        if (!awaitingBoundary) return WebChatNewConversationTransition.CONTINUE_CURRENT
        val incomingUser = incoming.messages.lastOrNull { it.role == "user" }
            ?: return WebChatNewConversationTransition.START_NEW
        val previousUser = previous?.messages?.lastOrNull { it.role == "user" }
            ?: return WebChatNewConversationTransition.START_NEW
        val sameLocation = canonicalLocation(previous.url) == canonicalLocation(incoming.url)
        val sameUser = normalized(previousUser.content) == normalized(incomingUser.content)
        return if (sameLocation && sameUser) {
            WebChatNewConversationTransition.IGNORE_STALE
        } else {
            WebChatNewConversationTransition.START_NEW
        }
    }

    private fun normalized(value: String): String = value.trim().replace(WHITESPACE, " ")

    private val WHITESPACE = Regex("\\s+")
}
