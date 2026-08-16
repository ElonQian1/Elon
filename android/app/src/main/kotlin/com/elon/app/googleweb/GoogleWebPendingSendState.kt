package com.elon.app.googleweb

internal class GoogleWebPendingSendState {
    enum class Phase {
        IDLE,
        SUBMITTING,
        AWAITING_RESPONSE,
        OFFICIAL_CONFIRMATION,
    }

    enum class TimeoutAction {
        IGNORE,
        KEEP_WAITING,
        REQUIRE_OFFICIAL_CONFIRMATION,
        RESTORE,
    }

    data class TimeoutResult(
        val action: TimeoutAction,
        val prompt: String? = null,
    )

    private data class Pending(
        val prompt: String,
        val generation: Long,
        var submissionConfirmed: Boolean = false,
        var confirmationRechecks: Int = 0,
        var requiresOfficialConfirmation: Boolean = false,
    )

    private var generation = 0L
    private var pending: Pending? = null

    fun begin(prompt: String): Long {
        generation += 1
        pending = Pending(prompt = prompt, generation = generation)
        return generation
    }

    fun prompt(): String? = pending?.prompt

    fun requiresOfficialConfirmation(): Boolean =
        pending?.requiresOfficialConfirmation == true

    fun phase(): Phase = pending?.let { current ->
        when {
            current.requiresOfficialConfirmation -> Phase.OFFICIAL_CONFIRMATION
            current.submissionConfirmed -> Phase.AWAITING_RESPONSE
            else -> Phase.SUBMITTING
        }
    } ?: Phase.IDLE

    fun confirmSubmission(): Boolean {
        val current = pending ?: return false
        current.submissionConfirmed = true
        return true
    }

    fun failSubmission(): String? {
        val prompt = pending?.prompt ?: return null
        invalidate()
        return prompt
    }

    fun observeCompletedTurn(content: String?, assistantObserved: Boolean): Boolean {
        val current = pending ?: return false
        if (content?.trim() != current.prompt.trim()) return false
        if (!assistantObserved) return false
        invalidate()
        return true
    }

    fun onConfirmationTimeout(expectedGeneration: Long): TimeoutResult {
        val current = pending
            ?: return TimeoutResult(TimeoutAction.IGNORE)
        if (current.generation != expectedGeneration) {
            return TimeoutResult(TimeoutAction.IGNORE)
        }
        if (current.submissionConfirmed) {
            if (current.requiresOfficialConfirmation) {
                return TimeoutResult(TimeoutAction.IGNORE)
            }
            current.confirmationRechecks += 1
            if (current.confirmationRechecks > MAX_CONFIRMATION_RECHECKS) {
                current.requiresOfficialConfirmation = true
                return TimeoutResult(TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION)
            }
            return TimeoutResult(TimeoutAction.KEEP_WAITING)
        }
        val prompt = current.prompt
        invalidate()
        return TimeoutResult(TimeoutAction.RESTORE, prompt)
    }

    fun clear() {
        invalidate()
    }

    private fun invalidate() {
        generation += 1
        pending = null
    }

    private companion object {
        const val MAX_CONFIRMATION_RECHECKS = 2
    }
}

internal object GoogleWebPendingSendPresentation {
    fun status(phase: GoogleWebPendingSendState.Phase): String? = when (phase) {
        GoogleWebPendingSendState.Phase.IDLE -> null
        GoogleWebPendingSendState.Phase.SUBMITTING -> "发送中…"
        GoogleWebPendingSendState.Phase.AWAITING_RESPONSE -> "已发送 · 等待回复"
        GoogleWebPendingSendState.Phase.OFFICIAL_CONFIRMATION -> "已发送 · 回答同步较慢"
    }
}
