package com.elon.app.chatgptweb

internal object WebChatSendContextPolicy {
    fun allows(
        sessionReady: Boolean,
        snapshot: ChatGptWebSnapshot?,
        navigationPending: Boolean,
        selectedConversationPath: String?,
        observedConversationPath: String?,
    ): Boolean {
        if (!sessionReady || snapshot?.composerReady != true || snapshot.streaming) return false
        if (navigationPending) return false
        return selectedConversationPath == observedConversationPath
    }
}
