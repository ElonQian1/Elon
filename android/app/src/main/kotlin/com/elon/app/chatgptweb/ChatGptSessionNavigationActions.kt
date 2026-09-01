package com.elon.app.chatgptweb

internal class ChatGptSessionNavigationActions(
    private val sessionReady: () -> Boolean,
    private val sessionCanDefer: () -> Boolean,
    private val bridgeReady: () -> Boolean,
    private val commandAvailable: () -> Boolean,
    private val startNewConversationCommand: () -> Unit,
    private val openConversationCommand: (String) -> Unit,
    private val openProjectCommand: (String) -> Boolean,
    private val latestSnapshot: () -> ChatGptWebSnapshot?,
    private val presentSnapshot: (ChatGptWebSnapshot) -> Unit,
    private val updateLoading: () -> Unit,
    private val ensureInitialized: () -> Unit,
    private val prioritizeUserNavigation: () -> Unit,
    private val cancelNewConversationRecovery: () -> Unit,
    private val scheduleNewConversationRecovery: () -> Unit,
    private val conversationNavigation: ChatGptConversationNavigationCoordinator,
) {
    private val conversationOpenQueue = ChatGptConversationOpenQueue()
    private var pendingNewConversation = false

    fun startNewConversation(): Boolean {
        if (pendingNewConversation || conversationNavigation.hasPending()) return false
        if (!sessionReady() && !sessionCanDefer()) return false
        prioritizeUserNavigation()
        ensureInitialized()
        conversationOpenQueue.clear()
        cancelNewConversationRecovery()
        presentSnapshot(conversationNavigation.beginNew(latestSnapshot()))
        pendingNewConversation = true
        dispatchPendingNewConversation()
        return true
    }

    fun openConversation(path: String): Boolean {
        val normalized = ChatGptWebConversationPath.normalize(path) ?: return false
        prioritizeUserNavigation()
        ensureInitialized()
        if (conversationNavigation.hasPending()) return false
        if (conversationOpenQueue.hasPending()) {
            previewDeferredConversationOpen(normalized)
            dispatchDeferredConversationOpen()
            return true
        }
        if (canDispatch()) return dispatchConversationOpen(normalized, latestSnapshot())
        if (!sessionCanDefer()) return false
        previewDeferredConversationOpen(normalized)
        return true
    }

    fun openProject(path: String): Boolean {
        val normalized = ChatGptWebConversationPath.normalizeProject(path) ?: return false
        prioritizeUserNavigation()
        if (!canDispatch()) return false
        if (!commandAvailable()) return false
        conversationOpenQueue.clear()
        return openProjectCommand(normalized)
    }

    fun onBridgeReady() {
        dispatchPendingNewConversation()
        dispatchDeferredConversationOpen()
    }

    fun onSessionReady() = onBridgeReady()

    fun clearDeferred() {
        conversationOpenQueue.clear()
        pendingNewConversation = false
        conversationNavigation.clear()
    }

    private fun canDispatch(): Boolean =
        !conversationNavigation.hasPending() &&
            (sessionReady() || sessionCanDefer() && bridgeReady())

    private fun dispatchConversationOpen(
        path: String,
        previousSnapshot: ChatGptWebSnapshot?,
    ): Boolean {
        if (!commandAvailable()) return false
        if (conversationNavigation.hasPending()) return false
        cancelNewConversationRecovery()
        presentSnapshot(conversationNavigation.beginOpen(path, previousSnapshot))
        updateLoading()
        openConversationCommand(path)
        return true
    }

    private fun dispatchDeferredConversationOpen() {
        if (pendingNewConversation) return
        if (!canDispatch()) return
        val request = conversationOpenQueue.take() ?: return
        if (!dispatchConversationOpen(request.path, request.previousSnapshot)) {
            conversationOpenQueue.enqueue(request.path, request.previousSnapshot)
        }
    }

    private fun previewDeferredConversationOpen(path: String) {
        val request = conversationOpenQueue.enqueue(path, latestSnapshot())
        presentSnapshot(conversationNavigation.previewOpen(path, request.previousSnapshot))
    }

    private fun dispatchPendingNewConversation() {
        if (!pendingNewConversation) return
        if (!commandAvailable()) return
        if (!sessionReady() && !(sessionCanDefer() && bridgeReady())) return
        pendingNewConversation = false
        updateLoading()
        startNewConversationCommand()
        scheduleNewConversationRecovery()
    }
}
