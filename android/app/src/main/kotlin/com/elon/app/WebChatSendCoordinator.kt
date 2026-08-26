package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebSnapshot

internal enum class WebChatSendAuthority {
    OFFICIAL_PAGE,
}

internal interface WebChatSendTransport {
    val authority: WebChatSendAuthority

    fun isReady(): Boolean

    fun dispatch(prompt: String): Boolean

    fun reconcile()
}

internal class OfficialPageWebChatSendTransport(
    private val ready: () -> Boolean,
    private val sendPrompt: (String) -> Boolean,
    private val requestReconciliation: () -> Unit,
) : WebChatSendTransport {
    override val authority: WebChatSendAuthority = WebChatSendAuthority.OFFICIAL_PAGE

    override fun isReady(): Boolean = ready()

    override fun dispatch(prompt: String): Boolean = sendPrompt(prompt)

    override fun reconcile() = requestReconciliation()
}

internal class WebChatSendCoordinator(
    private val transport: WebChatSendTransport,
    private val postDelayed: (Runnable, Long) -> Unit,
    private val removeCallbacks: (Runnable) -> Unit,
    private val onTerminalTimeout: (WebChatPendingSendState.TimeoutResult) -> Unit,
    private val confirmationTimeoutMs: Long = DEFAULT_CONFIRMATION_TIMEOUT_MS,
) {
    enum class DispatchOutcome {
        DISPATCHED,
        BUSY,
        NOT_READY,
        REJECTED,
    }

    enum class Observation {
        NONE,
        SUBMISSION_CONFIRMED,
        TURN_COMPLETED,
    }

    data class DispatchResult(
        val outcome: DispatchOutcome,
        val prompt: String? = null,
    )

    private val state = WebChatPendingSendState()
    private var watchdog: Runnable? = null
    private var baseline: SnapshotEvidence? = null

    fun authority(): WebChatSendAuthority = transport.authority

    fun isReady(): Boolean = transport.isReady()

    fun prompt(): String? = state.prompt()

    fun phase(): WebChatPendingSendState.Phase = state.phase()

    fun status(): String? = WebChatPendingSendPresentation.status(state.phase())

    fun requiresOfficialConfirmation(): Boolean = state.requiresOfficialConfirmation()

    fun dispatch(
        prompt: String,
        baselineSnapshot: ChatGptWebSnapshot?,
        onPending: () -> Unit,
    ): DispatchResult {
        if (state.prompt() != null) return DispatchResult(DispatchOutcome.BUSY)
        if (!transport.isReady()) return DispatchResult(DispatchOutcome.NOT_READY)

        baseline = baselineSnapshot?.let(::snapshotEvidence)
        val generation = state.begin(prompt)
        onPending()
        val dispatched = runCatching { transport.dispatch(prompt) }.getOrDefault(false)
        if (!dispatched) {
            val failedPrompt = state.failSubmission()
            baseline = null
            cancelWatchdog()
            return DispatchResult(DispatchOutcome.REJECTED, failedPrompt)
        }
        armWatchdog(generation)
        return DispatchResult(DispatchOutcome.DISPATCHED)
    }

    fun acceptCommandResult(ok: Boolean): String? {
        if (ok) {
            state.confirmSubmission()
            return null
        }
        val failedPrompt = state.failSubmission()
        baseline = null
        cancelWatchdog()
        return failedPrompt
    }

    fun observeSnapshot(snapshot: ChatGptWebSnapshot): Observation {
        val evidence = snapshotEvidence(snapshot)
        if (!evidence.isNewerThan(baseline)) return Observation.NONE
        if (state.observeCompletedTurn(evidence.latestUserPrompt, evidence.assistantObserved)) {
            baseline = null
            cancelWatchdog()
            return Observation.TURN_COMPLETED
        }
        if (state.observeSubmission(evidence.latestUserPrompt)) {
            return Observation.SUBMISSION_CONFIRMED
        }
        return Observation.NONE
    }

    fun clear() {
        state.clear()
        baseline = null
        cancelWatchdog()
    }

    fun pauseWatchdog() = cancelWatchdog()

    fun dispose() = clear()

    private fun armWatchdog(generation: Long) {
        cancelWatchdog()
        val task = Runnable {
            watchdog = null
            val result = state.onConfirmationTimeout(generation)
            when (result.action) {
                WebChatPendingSendState.TimeoutAction.IGNORE -> Unit
                WebChatPendingSendState.TimeoutAction.KEEP_WAITING -> {
                    reconcile()
                    armWatchdog(generation)
                }
                WebChatPendingSendState.TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION -> {
                    reconcile()
                    onTerminalTimeout(result)
                }
                WebChatPendingSendState.TimeoutAction.RESTORE -> {
                    baseline = null
                    onTerminalTimeout(result)
                }
            }
        }
        watchdog = task
        postDelayed(task, confirmationTimeoutMs)
    }

    private fun reconcile() {
        runCatching(transport::reconcile)
    }

    private fun cancelWatchdog() {
        watchdog?.let(removeCallbacks)
        watchdog = null
    }

    private fun snapshotEvidence(snapshot: ChatGptWebSnapshot): SnapshotEvidence {
        val latestUserIndex = snapshot.messages.indexOfLast { it.role == "user" }
        val latestUser = snapshot.messages.getOrNull(latestUserIndex)
        return SnapshotEvidence(
            latestUserPrompt = latestUser?.content,
            latestUserMessageId = latestUser?.id,
            observedMessageCount = snapshot.observedMessageCount,
            assistantObserved = latestUserIndex >= 0 && snapshot.messages
                .drop(latestUserIndex + 1)
                .any { it.role == "assistant" },
        )
    }

    private data class SnapshotEvidence(
        val latestUserPrompt: String?,
        val latestUserMessageId: String?,
        val observedMessageCount: Int,
        val assistantObserved: Boolean,
    ) {
        fun isNewerThan(previous: SnapshotEvidence?): Boolean {
            if (previous == null) return true
            if (observedMessageCount > previous.observedMessageCount) return true
            return !latestUserMessageId.isNullOrBlank() &&
                latestUserMessageId != previous.latestUserMessageId
        }
    }

    private companion object {
        const val DEFAULT_CONFIRMATION_TIMEOUT_MS = 12_000L
    }
}
