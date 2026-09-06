package com.elon.app.chatgptweb

internal class ChatGptConversationDeletionCaches(
    private val forget: (Set<String>) -> Unit,
    private val cachedUrl: () -> String?,
    private val clearCached: () -> Unit,
    private val restoredUrl: () -> String,
    private val clearRestored: () -> Unit,
) {
    constructor(
        navigation: ChatGptConversationNavigationCoordinator,
        snapshots: WebChatSnapshotStore,
        restorer: ChatGptWebSessionRestorer,
    ) : this(navigation::forget, { snapshots.restore()?.url }, snapshots::clear,
        restorer::restoreUrl, restorer::clear)

    fun accept(ids: Set<String>) {
        if (ids.isEmpty()) return
        val deleted = ChatGptDeletedConversations().apply { remember(ids) }
        forget(deleted.ids())
        // The last persisted snapshot may lag behind a currently streaming chat.
        if (deleted.containsUrl(cachedUrl())) clearCached()
        if (deleted.containsUrl(restoredUrl())) clearRestored()
    }
}
