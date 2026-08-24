package com.elon.app.chatgptweb

import java.net.URI
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
    private var projectCollections = restoredProjectCollections(restored)
    private var navigationProjectId: String? = null
    private var activeRefreshProjectId: String? = null

    fun index(): ChatGptWebConversationIndexState = ChatGptWebConversationIndexState(
        conversations = conversations,
        projects = ChatGptWebConversationIndex.projects(conversations, projects),
        collection = collection,
        projectCollections = projectCollections,
    )

    fun requestProject(path: String): Boolean {
        val projectId = ChatGptWebConversationPath.projectId(path) ?: return false
        navigationProjectId = projectId
        updateProjectCollection(projectId) { current ->
            current.copy(
                observedCount = conversations.count { it.projectId == projectId },
                stale = current.source != ChatGptWebConversationCollection.SOURCE_NONE ||
                    conversations.any { it.projectId == projectId },
                officialLoadState = ChatGptWebConversationCollection.LOAD_IDLE,
            )
        }
        return true
    }

    fun beginRefresh(requestedProjectId: String? = null): ChatGptConversationRefreshRequest {
        val scopeProjectId = ChatGptWebConversationPath.canonicalProjectId(requestedProjectId)
            ?: navigationProjectId
        activeRefreshProjectId = scopeProjectId
        if (scopeProjectId == null) {
            collection = collection.loading(conversations.isNotEmpty())
        } else {
            updateProjectCollection(scopeProjectId) { current ->
                current.loading(
                    current.source != ChatGptWebConversationCollection.SOURCE_NONE ||
                        conversations.any { it.projectId == scopeProjectId },
                )
            }
        }
        return ChatGptConversationRefreshRequest(projects, scopeProjectId)
    }

    fun markRefreshing() {
        collection = collection.loading(conversations.isNotEmpty())
    }

    fun needsOfficialRefresh(): Boolean =
        collection.officialLoadState != ChatGptWebConversationCollection.LOAD_READY

    fun needsProjectRefresh(snapshotUrl: String): Boolean {
        val projectId = navigationProjectId ?: return false
        if (activeRefreshProjectId == projectId) return false
        val path = runCatching { URI(snapshotUrl).path }.getOrNull()
        return ChatGptWebConversationPath.projectId(path) == projectId
    }

    fun observeCurrent(snapshot: ChatGptWebSnapshot, activityDate: LocalDate) {
        conversations = ChatGptWebConversationIndex.observeCurrent(
            previous = conversations,
            snapshot = snapshot,
            activityDate = activityDate,
            knownProjects = projects,
        )
        ChatGptWebConversationPath.projectId(
            runCatching { URI(snapshot.url).path }.getOrNull(),
        )?.let { projectId ->
            val count = conversations.count { it.projectId == projectId }
            updateProjectCollection(projectId) { current -> current.copy(observedCount = count) }
        }
    }

    fun failRefresh() {
        val scopeProjectId = activeRefreshProjectId
        if (scopeProjectId == null) {
            collection = collection.failed(conversations.isNotEmpty())
        } else {
            updateProjectCollection(scopeProjectId) { current ->
                current.failed(conversations.any { it.projectId == scopeProjectId })
            }
            if (navigationProjectId == scopeProjectId) navigationProjectId = null
        }
        activeRefreshProjectId = null
    }

    fun accept(event: ChatGptWebEvent.ConversationList) {
        val scopeProjectId = event.scopeProjectId
        conversations = if (scopeProjectId == null) {
            ChatGptWebConversationIndex.mergeOfficialHistory(
                conversations,
                event.conversations,
                collectionComplete = event.collection.isComplete,
            )
        } else {
            ChatGptWebConversationIndex.mergeProjectHistory(
                previous = conversations,
                observed = event.conversations,
                projectId = scopeProjectId,
                collectionComplete = event.collection.isComplete,
            )
        }
        projects = ChatGptWebConversationIndex.mergeObservedProjects(
            conversations,
            previous = projects,
            observed = event.projects,
        )
        val acceptedCollection = event.collection.copy(
            source = ChatGptWebConversationCollection.acceptedOfficialSource(event.collection.source),
            stale = false,
            officialLoadState = ChatGptWebConversationCollection.LOAD_READY,
            cachedAtMs = nowMs(),
        )
        if (scopeProjectId == null) {
            collection = acceptedCollection
        } else {
            projectCollections = projectCollections + (scopeProjectId to acceptedCollection.copy(
                observedCount = conversations.count { it.projectId == scopeProjectId },
            ))
            if (navigationProjectId == scopeProjectId) navigationProjectId = null
        }
        if (activeRefreshProjectId == scopeProjectId) activeRefreshProjectId = null
    }

    fun save(store: ChatGptConversationHistoryStore) {
        store.save(
            conversations = conversations,
            projects = projects,
            projectCachedAtMs = projectCollections.mapValues { it.value.cachedAtMs }
                .filterValues { it > 0L },
        )
    }

    fun clear() {
        conversations = emptyList()
        projects = emptyList()
        collection = ChatGptWebConversationCollection()
        projectCollections = emptyMap()
        navigationProjectId = null
        activeRefreshProjectId = null
    }

    private fun updateProjectCollection(
        projectId: String,
        transform: (ChatGptWebConversationCollection) -> ChatGptWebConversationCollection,
    ) {
        val current = projectCollections[projectId] ?: ChatGptWebConversationCollection()
        projectCollections = projectCollections + (projectId to transform(current))
    }

    private fun restoredProjectCollections(
        cache: ChatGptConversationHistoryCache?,
    ): Map<String, ChatGptWebConversationCollection> {
        cache ?: return emptyMap()
        val ids = cache.projectCachedAtMs.keys + cache.conversations.mapNotNull { it.projectId }
        return ids.mapNotNull { rawId ->
            val id = ChatGptWebConversationPath.canonicalProjectId(rawId) ?: return@mapNotNull null
            val count = cache.conversations.count { it.projectId == id }
            val cachedAtMs = cache.projectCachedAtMs[id] ?: cache.savedAtMs
            id to ChatGptWebConversationCollection.cached(count, cachedAtMs)
        }.toMap()
    }

    private fun ChatGptWebConversationCollection.loading(hasCachedData: Boolean) = copy(
        stale = hasCachedData,
        officialLoadState = ChatGptWebConversationCollection.LOAD_LOADING,
    )

    private fun ChatGptWebConversationCollection.failed(hasCachedData: Boolean) = copy(
        stale = hasCachedData,
        officialLoadState = ChatGptWebConversationCollection.LOAD_FAILED,
    )
}

internal data class ChatGptConversationRefreshRequest(
    val projectHints: List<ChatGptWebProject>,
    val scopeProjectId: String? = null,
)

internal data class ChatGptWebConversationIndexState(
    val conversations: List<ChatGptWebConversation> = emptyList(),
    val projects: List<ChatGptWebProject> = emptyList(),
    val collection: ChatGptWebConversationCollection = ChatGptWebConversationCollection(),
    val projectCollections: Map<String, ChatGptWebConversationCollection> = emptyMap(),
)
