package com.elon.app.chatgptweb

internal class ChatGptWebObservedState(
    initialConversationHistory: ChatGptConversationHistoryCache? = null,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private var conversations: List<ChatGptWebConversation> =
        initialConversationHistory?.conversations.orEmpty()
    private var projects: List<ChatGptWebProject> =
        initialConversationHistory?.projects.orEmpty()
    private var conversationCollection = initialConversationHistory?.let {
        ChatGptWebConversationCollection.cached(it.conversations.size, it.savedAtMs)
    } ?: ChatGptWebConversationCollection()
    private var features: List<ChatGptWebFeature> = emptyList()
    private var composerSections: Map<String, List<ChatGptWebComposerOption>> = emptyMap()
    private var lastCommand: ChatGptWebEvent.CommandResult? = null
    private var lastCommandObservedAtMs: Long? = null
    private var commandRequests: List<CommandRequest> = emptyList()
    private var nextCommandId = 0L
    private var updatedAtMs: Long = 0L
    private var pageGeneration = 0L
    private var adapterGeneration = 0L

    fun accept(event: ChatGptWebEvent) {
        val observedAtMs = nowMs()
        when (event) {
            is ChatGptWebEvent.ConversationList -> {
                conversations = ChatGptWebConversationIndex.merge(conversations, event.conversations)
                projects = ChatGptWebConversationIndex.projects(
                    conversations,
                    event.projects + projects,
                )
                conversationCollection = event.collection.copy(
                    source = ChatGptWebConversationCollection.SOURCE_OFFICIAL,
                    stale = false,
                    officialLoadState = ChatGptWebConversationCollection.LOAD_READY,
                    cachedAtMs = observedAtMs,
                )
            }
            is ChatGptWebEvent.Snapshot -> updateActiveConversation(event.value.url)
            is ChatGptWebEvent.FeatureNavigation -> features = event.features
            is ChatGptWebEvent.ComposerControls -> {
                composerSections = composerSections + (event.section to event.options)
            }
            is ChatGptWebEvent.CommandResult -> {
                lastCommand = event
                lastCommandObservedAtMs = observedAtMs
                completeRequest(event, observedAtMs)
                if (event.action == "list_conversations" && !event.ok) {
                    conversationCollection = conversationCollection.copy(
                        stale = conversations.isNotEmpty(),
                        officialLoadState = ChatGptWebConversationCollection.LOAD_FAILED,
                    )
                }
            }
            else -> return
        }
        updatedAtMs = observedAtMs
    }

    fun beginComposerRequest(section: String) {
        composerSections = composerSections - section
        updatedAtMs = nowMs()
    }

    fun clearConversationHistory() {
        conversations = emptyList()
        projects = emptyList()
        conversationCollection = ChatGptWebConversationCollection()
        updatedAtMs = nowMs()
    }

    fun updateDocument(document: ChatGptWebDocumentSession.Snapshot) {
        if (document.pageGeneration < pageGeneration) return
        val observedAtMs = nowMs()
        if (document.pageGeneration > pageGeneration) {
            conversationCollection = if (conversations.isEmpty()) {
                ChatGptWebConversationCollection()
            } else {
                conversationCollection.copy(
                    source = ChatGptWebConversationCollection.SOURCE_CACHE,
                    stale = true,
                    officialLoadState = ChatGptWebConversationCollection.LOAD_IDLE,
                )
            }
            features = emptyList()
            composerSections = emptyMap()
            lastCommand = null
            lastCommandObservedAtMs = null
            commandRequests = commandRequests.map { request ->
                if (request.status != CommandRequest.PENDING) return@map request
                request.copy(
                    status = CommandRequest.FAILED,
                    completedAtMs = observedAtMs,
                    result = ChatGptWebEvent.CommandResult(
                        action = request.expectedAction,
                        ok = false,
                        detail = PAGE_GENERATION_CHANGED,
                        requestId = request.id,
                    ),
                )
            }
        }
        pageGeneration = document.pageGeneration
        adapterGeneration = document.adapterGeneration
        updatedAtMs = observedAtMs
    }

    fun beginCommand(expectedAction: String): CommandRequest {
        val startedAt = nowMs()
        val request = CommandRequest(
            id = "mcp_${(++nextCommandId).toString(36)}",
            expectedAction = expectedAction,
            status = CommandRequest.PENDING,
            startedAtMs = startedAt,
        )
        commandRequests = (commandRequests + request).takeLast(MAX_COMMAND_REQUESTS)
        if (expectedAction == "list_conversations") {
            conversationCollection = conversationCollection.copy(
                stale = conversations.isNotEmpty() &&
                    conversationCollection.source != ChatGptWebConversationCollection.SOURCE_OFFICIAL,
                officialLoadState = ChatGptWebConversationCollection.LOAD_LOADING,
            )
        }
        updatedAtMs = startedAt
        return request
    }

    private fun updateActiveConversation(rawUrl: String) {
        val path = ChatGptWebConversationPath.fromUrl(rawUrl) ?: return
        conversations = conversations.map { it.copy(active = it.path == path) }
    }

    fun failCommand(requestId: String, expectedAction: String, detail: String) {
        accept(ChatGptWebEvent.CommandResult(
            action = expectedAction,
            ok = false,
            detail = detail,
            requestId = requestId,
        ))
    }

    fun snapshot(): Snapshot {
        expirePendingCommands()
        return Snapshot(
            conversations = conversations,
            projects = projects,
            features = features,
            composerSections = composerSections,
            lastCommand = lastCommand,
            commandRequests = commandRequests,
            updatedAtMs = updatedAtMs,
            lastCommandObservedAtMs = lastCommandObservedAtMs,
            pageGeneration = pageGeneration,
            adapterGeneration = adapterGeneration,
            conversationCollection = conversationCollection,
        )
    }

    private fun completeRequest(
        event: ChatGptWebEvent.CommandResult,
        observedAtMs: Long,
    ) {
        val requestId = event.requestId ?: return
        val index = commandRequests.indexOfFirst {
            it.status == CommandRequest.PENDING &&
                it.id == requestId &&
                it.expectedAction == event.action
        }
        if (index < 0) return
        commandRequests = commandRequests.toMutableList().apply {
            this[index] = this[index].copy(
                status = if (event.ok) CommandRequest.SUCCEEDED else CommandRequest.FAILED,
                result = event,
                completedAtMs = observedAtMs,
            )
        }
    }

    private fun expirePendingCommands() {
        val now = nowMs()
        var changed = false
        commandRequests = commandRequests.map { request ->
            if (
                request.status == CommandRequest.PENDING &&
                now - request.startedAtMs >= COMMAND_TIMEOUT_MS
            ) {
                changed = true
                request.copy(status = CommandRequest.TIMED_OUT, completedAtMs = now)
            } else {
                request
            }
        }
        if (changed) updatedAtMs = now
    }

    internal data class Snapshot(
        val conversations: List<ChatGptWebConversation>,
        val features: List<ChatGptWebFeature>,
        val composerSections: Map<String, List<ChatGptWebComposerOption>>,
        val lastCommand: ChatGptWebEvent.CommandResult?,
        val commandRequests: List<CommandRequest>,
        val updatedAtMs: Long,
        val lastCommandObservedAtMs: Long? = null,
        val pageGeneration: Long = 0L,
        val adapterGeneration: Long = 0L,
        val conversationCollection: ChatGptWebConversationCollection = ChatGptWebConversationCollection(
            observedCount = conversations.size,
        ),
        val projects: List<ChatGptWebProject> = emptyList(),
    ) {
        val adapterCurrent: Boolean
            get() = pageGeneration > 0 && adapterGeneration == pageGeneration

        companion object {
            val EMPTY = Snapshot(
                conversations = emptyList(),
                projects = emptyList(),
                features = emptyList(),
                composerSections = emptyMap(),
                lastCommand = null,
                commandRequests = emptyList(),
                updatedAtMs = 0L,
            )
        }
    }

    internal data class CommandRequest(
        val id: String,
        val expectedAction: String,
        val status: String,
        val startedAtMs: Long,
        val completedAtMs: Long? = null,
        val result: ChatGptWebEvent.CommandResult? = null,
    ) {
        companion object {
            const val PENDING = "pending"
            const val SUCCEEDED = "succeeded"
            const val FAILED = "failed"
            const val TIMED_OUT = "timed_out"
        }
    }

    private companion object {
        const val MAX_COMMAND_REQUESTS = 20
        const val COMMAND_TIMEOUT_MS = 20_000L
        const val PAGE_GENERATION_CHANGED = "page_generation_changed"
    }
}
