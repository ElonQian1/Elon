package com.elon.app.googleweb

internal class GoogleWebPendingSendState {
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

    fun observeUserPrompt(content: String?): Boolean {
        val current = pending ?: return false
        if (content?.trim() != current.prompt.trim()) return false
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
