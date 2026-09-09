package com.elon.app.chatgptweb

internal class ChatGptConversationRefreshRuntime(
    private val directory: ChatGptConversationDirectory,
    private val pageAdapter: () -> ChatGptWebPageAdapter?,
    private val isReady: () -> Boolean,
    private val onIndexChanged: () -> Unit,
    scheduleRefresh: (Runnable, Long) -> Unit,
    cancelRefresh: (Runnable) -> Unit,
    scheduleComposerRelease: (Runnable, Long) -> Unit,
    cancelComposerRelease: (Runnable) -> Unit,
) {
    private val coordinator = ChatGptConversationRefreshCoordinator(
        dispatch = ::dispatch,
        schedule = scheduleRefresh,
        cancel = cancelRefresh,
    )
    private val session = ChatGptConversationRefreshSession(coordinator)
    private val owner = ChatGptConversationRefreshOwner()
    private val composerInterlock = ChatGptComposerRefreshInterlock(
        suspendRefresh = {
            suspend(
                ChatGptConversationRefreshSuspension.COMPOSER_OPTIONS,
                preserveInterruptedRefresh = true,
            )
        },
        resumeRefresh = {
            if (directory.needsOfficialRefresh()) session.requestDefaultIfMissing()
            session.resume(ChatGptConversationRefreshSuspension.COMPOSER_OPTIONS)
        },
        schedule = scheduleComposerRelease,
        cancel = cancelComposerRelease,
    )

    fun request(projectId: String? = null): Boolean = session.request(projectId)

    fun suspendForConversationAction() = suspend(
        ChatGptConversationRefreshSuspension.CONVERSATION_ACTION,
        preserveInterruptedRefresh = false,
    )

    fun resumeAfterConversationAction() = session.resume(
        ChatGptConversationRefreshSuspension.CONVERSATION_ACTION,
    )

    fun acquireForComposer() = composerInterlock.acquire()

    fun releaseAfterComposerQuietPeriod() = composerInterlock.releaseAfterQuietPeriod()

    fun yieldToUserNavigation() = session.yieldToUserNavigation()

    fun onSnapshot(event: ChatGptWebEvent.ConversationList) {
        if (owner.isStaleNative(event.requestId)) return
        val owned = owner.matches(event)
        val continueRefresh = owned && session.canContinueRefresh(event.continueRefresh, event.collection.steps)
        directory.accept(event.copy(continueRefresh = continueRefresh), settleRefresh = owned)
        if (owned) {
            owner.clear()
            session.onSucceeded(continueRefresh, event.collection.steps)
        }
    }

    fun onFailed(event: ChatGptWebEvent.CommandResult) {
        if (!owner.matches(event.requestId)) return
        owner.clear()
        directory.failRefresh()
        onIndexChanged()
        session.onFailed()
    }

    fun refreshOnReady(
        postVoiceRefresh: Boolean,
        supported: Boolean,
        projectRefreshNeeded: Boolean,
        officialRefreshNeeded: Boolean,
    ): Boolean = session.refreshOnReady(
        postVoiceRefresh,
        supported,
        projectRefreshNeeded,
        officialRefreshNeeded,
    )

    fun reset() {
        owner.clear()
        composerInterlock.abandon()
        session.reset()
    }

    private fun suspend(
        owner: ChatGptConversationRefreshSuspension,
        preserveInterruptedRefresh: Boolean,
    ) {
        val refreshWasBusy = coordinator.isBusy
        session.suspend(owner, preserveInterruptedRefresh) {
            this.owner.clear()
            if (refreshWasBusy) {
                directory.failRefresh()
                onIndexChanged()
            }
            pageAdapter()?.cancelConversationDirectoryWork()
        }
    }

    private fun dispatch(): Boolean {
        val adapter = pageAdapter() ?: return false
        if (!isReady()) return false
        val request = session.beginDispatch() ?: return false
        val refresh = directory.beginRefresh(request.projectId)
        session.bindDispatchedScope(refresh.scopeProjectId)
        onIndexChanged()
        adapter.listConversations(
            projectHints = refresh.projectHints,
            scopeProjectId = refresh.scopeProjectId,
            requestId = owner.begin(refresh.scopeProjectId),
        )
        return true
    }
}
