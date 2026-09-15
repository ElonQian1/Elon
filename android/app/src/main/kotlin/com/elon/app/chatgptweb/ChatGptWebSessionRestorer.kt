package com.elon.app.chatgptweb

import android.content.Context

internal class ChatGptWebSessionRestorer(context: Context) {
    private val stateStore = ChatGptWebSessionStateStore(context)
    private val startupHistory = ChatGptStartupHistoryRefresh(stateStore.restoreUrl())

    fun restoreUrl(): String = stateStore.restoreUrl()

    fun onPageReady(url: String) {
        stateStore.saveUrl(url)
    }

    fun onSnapshot(snapshot: ChatGptWebSnapshot, refreshHistory: () -> Unit) {
        confirmedConversationUrl(snapshot)?.let(stateStore::saveUrl)
        if (startupHistory.take(snapshot)) refreshHistory()
    }

    fun clear() {
        startupHistory.clear()
        stateStore.clear()
    }

    internal companion object {
        fun confirmedConversationUrl(snapshot: ChatGptWebSnapshot): String? {
            // SPA navigation does not finish a page load. Ignore cached preview ownership.
            if (!snapshot.authenticated || snapshot.contentOnly ||
                ChatGptWebAccessPolicy.requiresLogin(snapshot) ||
                !WebChatSnapshotPrivacyPolicy.canPersist("chatgpt", snapshot.url)
            ) return null
            val path = ChatGptWebConversationPath.fromUrl(snapshot.url) ?: return null
            return ChatGptWebSessionStateStore.normalizeRestorableUrl("https://chatgpt.com$path")
        }
    }
}
