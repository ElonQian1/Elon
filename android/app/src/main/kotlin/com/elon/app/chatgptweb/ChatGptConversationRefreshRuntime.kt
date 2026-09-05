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
    private val composerInterlock = ChatGptComposerRefreshInterlock(
        suspendRefresh = {
            suspend(
                ChatGptConversationRefreshSuspension.COMPOSER_OPTIONS,
                preserveInterruptedRefresh = true,
            )
        },
        resumeRefresh = {
            if (directory.needsOfficialRefresh()) session.request(null)
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

    fun onSucceeded() = session.onSucceeded()

    fun onFailed() = session.onFailed()

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
        composerInterlock.abandon()
        session.reset()
    }

    private fun suspend(
        owner: ChatGptConversationRefreshSuspension,
        preserveInterruptedRefresh: Boolean,
    ) {
        val refreshWasBusy = coordinator.isBusy
        session.suspend(owner, preserveInterruptedRefresh) {
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
        onIndexChanged()
        adapter.listConversations(
            projectHints = refresh.projectHints,
            scopeProjectId = refresh.scopeProjectId,
        )
        return true
    }
}
