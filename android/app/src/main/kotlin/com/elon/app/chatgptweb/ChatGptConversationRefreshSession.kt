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

internal class ChatGptConversationRefreshSession(
    private val coordinator: ChatGptConversationRefreshCoordinator,
) {
    private var pendingProjectId: String? = null
    private var suspended = false

    fun request(projectId: String?): Boolean {
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
        return ChatGptConversationRefreshDispatch(pendingProjectId).also {
            pendingProjectId = null
        }
    }

    fun suspend(onSuspended: () -> Unit) {
        if (suspended) return
        suspended = true
        pendingProjectId = null
        coordinator.reset()
        onSuspended()
    }

    fun resume() {
        suspended = false
    }

    fun yieldToUserNavigation() {
        pendingProjectId = null
        coordinator.yieldToUserNavigation()
    }

    fun reset() {
        pendingProjectId = null
        suspended = false
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
