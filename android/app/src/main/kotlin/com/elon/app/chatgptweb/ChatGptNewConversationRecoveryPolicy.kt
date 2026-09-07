package com.elon.app.chatgptweb

internal enum class ChatGptNewConversationRecoveryAction {
    NONE,
    REQUEST_SNAPSHOT,
}

internal object ChatGptNewConversationRecoveryPolicy {
    fun action(
        navigationActive: Boolean,
        loading: Boolean,
        composerReady: Boolean,
    ): ChatGptNewConversationRecoveryAction {
        if (!navigationActive || !loading || composerReady) {
            return ChatGptNewConversationRecoveryAction.NONE
        }
        // The active page command owns navigation and any guest confirmation.
        // A missing composer during that transaction only calls for fresh evidence.
        return ChatGptNewConversationRecoveryAction.REQUEST_SNAPSHOT
    }
}
