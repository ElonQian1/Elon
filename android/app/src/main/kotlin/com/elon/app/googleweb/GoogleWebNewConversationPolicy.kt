package com.elon.app.googleweb

import com.elon.app.chatgptweb.ChatGptWebSnapshot
import com.elon.app.chatgptweb.WebChatNewConversationPolicy
import com.elon.app.chatgptweb.WebChatNewConversationTransition

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
    ): GoogleWebNewConversationTransition = when (WebChatNewConversationPolicy.transition(
        awaitingBoundary = awaitingBoundary,
        previous = previous,
        incoming = incoming,
        canonicalLocation = GoogleWebNavigationPolicy::sanitizeNavigableUrl,
    )) {
        WebChatNewConversationTransition.CONTINUE_CURRENT ->
            GoogleWebNewConversationTransition.CONTINUE_CURRENT
        WebChatNewConversationTransition.IGNORE_STALE ->
            GoogleWebNewConversationTransition.IGNORE_STALE
        WebChatNewConversationTransition.START_NEW -> GoogleWebNewConversationTransition.START_NEW
    }
}
