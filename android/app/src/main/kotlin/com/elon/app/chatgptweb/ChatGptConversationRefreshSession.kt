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
    private val nowMs: () -> Long = { System.nanoTime() / 1_000_000 },
) {
    private var pendingProjectId: String? = null
    private var pendingRefresh = false
    private var lastDispatch: ChatGptConversationRefreshDispatch? = null
    private var cycleStartedAtMs = 0L
    private var lastSteps = 0
    private var continuationCount = 0
    private val suspensions = mutableSetOf<ChatGptConversationRefreshSuspension>()

    private val suspended: Boolean
        get() = suspensions.isNotEmpty()

    fun onSucceeded(continueRefresh: Boolean = false, observedSteps: Int = 0) {
        if (suspended) return
        val canContinue = canContinueRefresh(continueRefresh, observedSteps)
        lastSteps = observedSteps
        if (canContinue) {
            continuationCount += 1
            coordinator.onPartial()
        } else {
            lastDispatch = null
            coordinator.onSucceeded()
        }
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

    fun requestDefaultIfMissing() {
        if (!pendingRefresh && lastDispatch == null) request(null)
    }

    fun beginDispatch(): ChatGptConversationRefreshDispatch? {
        if (suspended) return null
        if (pendingRefresh || lastDispatch == null) {
            lastDispatch = ChatGptConversationRefreshDispatch(pendingProjectId)
            cycleStartedAtMs = nowMs()
            lastSteps = 0
            continuationCount = 0
        }
        pendingRefresh = false
        pendingProjectId = null
        return lastDispatch
    }

    fun canContinueRefresh(requested: Boolean, observedSteps: Int): Boolean =
        !suspended && !pendingRefresh && lastDispatch != null && requested &&
            observedSteps > lastSteps && continuationCount < 10 && nowMs() - cycleStartedAtMs < 60_000

    fun bindDispatchedScope(projectId: String?) {
        lastDispatch = lastDispatch?.copy(projectId = projectId)
    }

    fun suspend(
        owner: ChatGptConversationRefreshSuspension,
        preserveInterruptedRefresh: Boolean = false,
        onSuspended: () -> Unit,
    ) {
        if (!suspensions.add(owner)) return
        if (preserveInterruptedRefresh && coordinator.isBusy && !pendingRefresh) {
            pendingRefresh = true
            pendingProjectId = lastDispatch?.projectId
        }
        if (!preserveInterruptedRefresh) {
            pendingRefresh = false
            pendingProjectId = null
        }
        if (suspensions.size > 1) return
        lastDispatch = null
        coordinator.reset()
        onSuspended()
    }

    fun resume(owner: ChatGptConversationRefreshSuspension) {
        if (!suspensions.remove(owner) || suspended) return
        if (pendingRefresh) coordinator.requestAfterCurrent()
    }

    fun yieldToUserNavigation() {
        pendingProjectId = null
        pendingRefresh = false
        lastDispatch = null
        coordinator.yieldToUserNavigation()
    }

    fun reset() {
        pendingProjectId = null
        pendingRefresh = false
        lastDispatch = null
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
