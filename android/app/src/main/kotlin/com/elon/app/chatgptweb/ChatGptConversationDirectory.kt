package com.elon.app.chatgptweb

import java.time.LocalDate

internal class ChatGptConversationDirectory(
    restored: ChatGptConversationHistoryCache?,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private var conversations = restored?.conversations.orEmpty()
    private var projects = restored?.projects.orEmpty()
    private var collection = restored?.let {
        ChatGptWebConversationCollection.cached(it.conversations.size, it.savedAtMs)
    } ?: ChatGptWebConversationCollection()

    fun index(): ChatGptWebConversationIndexState = ChatGptWebConversationIndexState(
        conversations = conversations,
        projects = ChatGptWebConversationIndex.projects(conversations, projects),
        collection = collection,
    )

    fun beginRefresh(): List<ChatGptWebProject> {
        collection = collection.copy(
            stale = conversations.isNotEmpty(),
            officialLoadState = ChatGptWebConversationCollection.LOAD_LOADING,
        )
        return projects
    }

    fun markRefreshing() {
        collection = collection.copy(
            stale = conversations.isNotEmpty(),
            officialLoadState = ChatGptWebConversationCollection.LOAD_LOADING,
        )
    }

    fun needsOfficialRefresh(): Boolean =
        collection.officialLoadState != ChatGptWebConversationCollection.LOAD_READY

    fun observeCurrent(snapshot: ChatGptWebSnapshot, activityDate: LocalDate) {
        conversations = ChatGptWebConversationIndex.observeCurrent(
            previous = conversations,
            snapshot = snapshot,
            activityDate = activityDate,
            knownProjects = projects,
        )
    }

    fun failRefresh() {
        collection = collection.copy(
            stale = conversations.isNotEmpty(),
            officialLoadState = ChatGptWebConversationCollection.LOAD_FAILED,
        )
    }

    fun accept(event: ChatGptWebEvent.ConversationList) {
        conversations = ChatGptWebConversationIndex.mergeOfficialHistory(
            conversations,
            event.conversations,
            collectionComplete = event.collection.isComplete,
        )
        projects = ChatGptWebConversationIndex.mergeObservedProjects(
            conversations,
            previous = projects,
            observed = event.projects,
        )
        collection = event.collection.copy(
            source = ChatGptWebConversationCollection.SOURCE_OFFICIAL,
            stale = false,
            officialLoadState = ChatGptWebConversationCollection.LOAD_READY,
            cachedAtMs = nowMs(),
        )
    }

    fun save(store: ChatGptConversationHistoryStore) {
        store.save(conversations, projects)
    }

    fun clear() {
        conversations = emptyList()
        projects = emptyList()
        collection = ChatGptWebConversationCollection()
    }
}

internal data class ChatGptWebConversationIndexState(
    val conversations: List<ChatGptWebConversation> = emptyList(),
    val projects: List<ChatGptWebProject> = emptyList(),
    val collection: ChatGptWebConversationCollection = ChatGptWebConversationCollection(),
)
