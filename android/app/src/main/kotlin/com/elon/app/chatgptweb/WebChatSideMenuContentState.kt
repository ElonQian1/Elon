package com.elon.app.chatgptweb

internal enum class WebChatSideMenuContentStatus {
    CONTENT,
    LOADING,
    EMPTY,
    FAILED,
}

internal object WebChatSideMenuContentState {
    fun resolve(
        collection: ChatGptWebConversationCollection,
        availableCount: Int,
        visibleCount: Int,
    ): WebChatSideMenuContentStatus = when {
        visibleCount > 0 -> WebChatSideMenuContentStatus.CONTENT
        availableCount > 0 -> WebChatSideMenuContentStatus.EMPTY
        collection.officialLoadState == ChatGptWebConversationCollection.LOAD_LOADING ->
            WebChatSideMenuContentStatus.LOADING
        collection.officialLoadState == ChatGptWebConversationCollection.LOAD_FAILED ->
            WebChatSideMenuContentStatus.FAILED
        collection.source == ChatGptWebConversationCollection.SOURCE_NONE ->
            WebChatSideMenuContentStatus.LOADING
        collection.officialLoadState == ChatGptWebConversationCollection.LOAD_READY ->
            WebChatSideMenuContentStatus.EMPTY
        collection.stale -> WebChatSideMenuContentStatus.LOADING
        else -> WebChatSideMenuContentStatus.EMPTY
    }
}
