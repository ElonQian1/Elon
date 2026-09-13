package com.elon.app.chatgptweb

internal object WebChatSendContextPolicy {
    fun allows(
        sessionReady: Boolean,
        snapshot: ChatGptWebSnapshot?,
        navigationPending: Boolean,
        selectedConversationPath: String?,
        observedConversationPath: String?,
        allowPrivateText: Boolean = false,
    ): Boolean {
        if (!sessionReady || snapshot == null || snapshot.streaming) return false
        if (!(if (allowPrivateText) ChatGptWebAccessPolicy.canSendText(snapshot) else snapshot.composerReady)) return false
        if (navigationPending) return false
        return selectedConversationPath == observedConversationPath
    }
}
