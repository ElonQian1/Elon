package com.elon.app.chatgptweb

internal data class ChatGptConversationRefreshDispatch(
    val projectId: String?,
)

internal data class ChatGptConversationAutoRefreshDecision(
    val action: Action,
    val consumePostVoiceRefresh: Boolean = false,
) {
    enum class Action { NONE, AFTER_CURRENT, IF_IDLE }
}

internal enum class ChatGptConversationRefreshSuspension {
    CONVERSATION_ACTION,
    COMPOSER_OPTIONS,
}

internal class ChatGptConversationRefreshSession(
    private val coordinator: ChatGptConversationRefreshCoordinator,
) {
    private var pendingProjectId: String? = null
    private var pendingRefresh = false
    private val suspensions = mutableSetOf<ChatGptConversationRefreshSuspension>()

    private val suspended: Boolean
        get() = suspensions.isNotEmpty()

    fun onSucceeded() {
        if (!suspended) coordinator.onSucceeded()
    }

    fun onFailed() {
        if (!suspended) coordinator.onFailed()
    }

    fun request(projectId: String?): Boolean {
        pendingRefresh = true
        pendingProjectId = ChatGptConversationRefreshScopePolicy.select(
            pendingProjectId = pendingProjectId,
            requestedProjectId = ChatGptWebConversationPath.canonicalProjectId(projectId),
            refreshBusy = coordinator.isBusy,
        )
        if (suspended) return true
        return coordinator.requestAfterCurrent()
    }

    fun beginDispatch(): ChatGptConversationRefreshDispatch? {
        if (suspended) return null
        pendingRefresh = false
        return ChatGptConversationRefreshDispatch(pendingProjectId).also {
            pendingProjectId = null
        }
    }

    fun suspend(
        owner: ChatGptConversationRefreshSuspension,
        preserveInterruptedRefresh: Boolean = false,
        onSuspended: () -> Unit,
    ) {
        if (!suspensions.add(owner)) return
        if (preserveInterruptedRefresh && coordinator.isBusy) pendingRefresh = true
        if (!preserveInterruptedRefresh) {
            pendingRefresh = false
            pendingProjectId = null
        }
        if (suspensions.size > 1) return
        coordinator.reset()
        onSuspended()
    }

    fun resume(owner: ChatGptConversationRefreshSuspension) {
        if (!suspensions.remove(owner) || suspended) return
        if (pendingRefresh) coordinator.requestAfterCurrent()
    }

    fun yieldToUserNavigation() {
        pendingProjectId = null
        coordinator.yieldToUserNavigation()
    }

    fun reset() {
        pendingProjectId = null
        pendingRefresh = false
        suspensions.clear()
        coordinator.reset()
    }

    fun autoRefreshDecision(
        postVoiceRefresh: Boolean,
        supported: Boolean,
        projectRefreshNeeded: Boolean,
        officialRefreshNeeded: Boolean,
    ): ChatGptConversationAutoRefreshDecision = when {
        suspended || !supported -> ChatGptConversationAutoRefreshDecision(
            ChatGptConversationAutoRefreshDecision.Action.NONE,
        )
        postVoiceRefresh -> ChatGptConversationAutoRefreshDecision(
            ChatGptConversationAutoRefreshDecision.Action.AFTER_CURRENT,
            consumePostVoiceRefresh = true,
        )
        projectRefreshNeeded -> ChatGptConversationAutoRefreshDecision(
            ChatGptConversationAutoRefreshDecision.Action.AFTER_CURRENT,
        )
        officialRefreshNeeded -> ChatGptConversationAutoRefreshDecision(
            ChatGptConversationAutoRefreshDecision.Action.IF_IDLE,
        )
        else -> ChatGptConversationAutoRefreshDecision(
            ChatGptConversationAutoRefreshDecision.Action.NONE,
        )
    }

    fun refreshOnReady(
        postVoiceRefresh: Boolean,
        supported: Boolean,
        projectRefreshNeeded: Boolean,
        officialRefreshNeeded: Boolean,
    ): Boolean {
        val decision = autoRefreshDecision(
            postVoiceRefresh,
            supported,
            projectRefreshNeeded,
            officialRefreshNeeded,
        )
        when (decision.action) {
            ChatGptConversationAutoRefreshDecision.Action.AFTER_CURRENT ->
                coordinator.requestAfterCurrent()
            ChatGptConversationAutoRefreshDecision.Action.IF_IDLE -> coordinator.requestIfIdle()
            ChatGptConversationAutoRefreshDecision.Action.NONE -> Unit
        }
        return postVoiceRefresh && !decision.consumePostVoiceRefresh
    }
}
