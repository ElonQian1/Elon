package com.elon.app.chatgptweb

internal enum class ChatGptNewConversationRecoveryAction {
    NONE,
    RELOAD_HOME,
    LOAD_HOME,
}

internal object ChatGptNewConversationRecoveryPolicy {
    fun action(
        navigationActive: Boolean,
        loading: Boolean,
        composerReady: Boolean,
        webViewAtHome: Boolean,
    ): ChatGptNewConversationRecoveryAction {
        if (!navigationActive || !loading || composerReady) {
            return ChatGptNewConversationRecoveryAction.NONE
        }
        return if (webViewAtHome) {
            ChatGptNewConversationRecoveryAction.RELOAD_HOME
        } else {
            ChatGptNewConversationRecoveryAction.LOAD_HOME
        }
    }
}
