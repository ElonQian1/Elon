package com.elon.app

internal enum class WebChatDomDictationPhase {
    IDLE,
    STARTING,
    ACTIVE,
    REVIEW,
    SUBMITTING,
    CANCELLING,
    START_FAILED,
}

internal data class WebChatDomDictationState(
    val phase: WebChatDomDictationPhase = WebChatDomDictationPhase.IDLE,
) {
    val controlsActive: Boolean get() = phase != WebChatDomDictationPhase.IDLE
    val canFinish: Boolean get() = phase in setOf(
        WebChatDomDictationPhase.ACTIVE,
        WebChatDomDictationPhase.REVIEW,
    )
    val reviewPending: Boolean get() = phase == WebChatDomDictationPhase.REVIEW
    val finishPending: Boolean get() = phase in setOf(
        WebChatDomDictationPhase.SUBMITTING,
        WebChatDomDictationPhase.CANCELLING,
    )
    val startFailed: Boolean get() = phase == WebChatDomDictationPhase.START_FAILED
}

/** Keeps native review controls stable when the official recorder releases before its draft. */
internal class WebChatDomDictationSession(
    private val nowMs: () -> Long,
    private val startTimeoutMs: Long = DEFAULT_START_TIMEOUT_MS,
    private val finishTimeoutMs: Long = DEFAULT_FINISH_TIMEOUT_MS,
) {
    private var state = WebChatDomDictationState()
    private var originalDraft = ""
    private var startDeadlineMs = 0L
    private var finishDeadlineMs = 0L
    private var officialActiveObserved = false
    private var lastOfficialActive = false
    private var lastCaptureActive = false

    fun state(
        officialActive: Boolean,
        currentDraft: String,
        captureActive: Boolean = officialActive,
    ): WebChatDomDictationState {
        lastOfficialActive = officialActive
        lastCaptureActive = captureActive
        if (state.phase == WebChatDomDictationPhase.STARTING) {
            when {
                officialActive && captureActive -> {
                    officialActiveObserved = true
                    state = WebChatDomDictationState(WebChatDomDictationPhase.ACTIVE)
                }
                nowMs() >= startDeadlineMs -> {
                    if (officialActive) {
                        state = WebChatDomDictationState(WebChatDomDictationPhase.START_FAILED)
                    } else {
                        reset()
                    }
                }
            }
            return state
        }
        if (state.startFailed) {
            if (!officialActive && !captureActive) reset()
            return state
        }
        if (officialActive || captureActive) {
            // Composer geometry can briefly misclassify ordinary controls as the recorder.
            // Only a live audio track may adopt an official session that we did not start.
            if (state.phase == WebChatDomDictationPhase.IDLE && !captureActive) return state
            if (state.phase == WebChatDomDictationPhase.IDLE) originalDraft = currentDraft
            officialActiveObserved = officialActiveObserved || captureActive
            state = when {
                state.finishPending && nowMs() < finishDeadlineMs -> state
                else -> WebChatDomDictationState(WebChatDomDictationPhase.ACTIVE)
            }
            return state
        }
        when {
            state.finishPending -> reset()
            state.phase == WebChatDomDictationPhase.ACTIVE && officialActiveObserved -> {
                state = WebChatDomDictationState(
                    if (currentDraft != originalDraft) {
                        WebChatDomDictationPhase.REVIEW
                    } else {
                        WebChatDomDictationPhase.IDLE
                    },
                )
                if (state.phase == WebChatDomDictationPhase.IDLE) reset()
            }
        }
        return state
    }

    fun startRequested(draft: String): Boolean {
        if (state.controlsActive) return false
        originalDraft = draft
        officialActiveObserved = false
        startDeadlineMs = nowMs() + startTimeoutMs
        state = WebChatDomDictationState(WebChatDomDictationPhase.STARTING)
        return true
    }

    fun finishRequested(action: String): Boolean {
        if (!state.canFinish || action !in FINISH_ACTIONS) return false
        state = WebChatDomDictationState(
            if (action == CANCEL_ACTION) {
                WebChatDomDictationPhase.CANCELLING
            } else {
                WebChatDomDictationPhase.SUBMITTING
            },
        )
        finishDeadlineMs = nowMs() + finishTimeoutMs
        return true
    }

    fun commandResult(action: String, ok: Boolean) {
        when (action) {
            START_ACTION -> if (!ok) reset()
            SUBMIT_ACTION,
            CANCEL_ACTION,
            -> if (!ok && state.finishPending) {
                state = WebChatDomDictationState(
                    if (lastOfficialActive) {
                        WebChatDomDictationPhase.ACTIVE
                    } else {
                        if (lastCaptureActive) {
                            WebChatDomDictationPhase.ACTIVE
                        } else {
                            WebChatDomDictationPhase.IDLE
                        }
                    },
                )
                if (state.phase == WebChatDomDictationPhase.IDLE) reset()
                finishDeadlineMs = 0L
            }
        }
    }

    fun acceptReview(): Boolean {
        if (!state.reviewPending) return false
        reset()
        return true
    }

    fun cancelReview(): String? {
        if (!state.reviewPending) return null
        val restored = originalDraft
        reset()
        return restored
    }

    fun reset() {
        state = WebChatDomDictationState()
        originalDraft = ""
        startDeadlineMs = 0L
        finishDeadlineMs = 0L
        officialActiveObserved = false
        lastOfficialActive = false
        lastCaptureActive = false
    }

    companion object {
        const val START_ACTION = "start_dictation"
        const val SUBMIT_ACTION = "submit_dictation"
        const val CANCEL_ACTION = "cancel_dictation"
        const val DEFAULT_START_TIMEOUT_MS = 8_000L
        const val DEFAULT_FINISH_TIMEOUT_MS = 2_500L
        private val FINISH_ACTIONS = setOf(SUBMIT_ACTION, CANCEL_ACTION)
    }
}
