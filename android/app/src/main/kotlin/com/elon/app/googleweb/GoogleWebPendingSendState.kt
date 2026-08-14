package com.elon.app.googleweb

internal class GoogleWebPendingSendState {
    enum class TimeoutAction {
        IGNORE,
        KEEP_WAITING,
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
    )

    private var generation = 0L
    private var pending: Pending? = null

    fun begin(prompt: String): Long {
        generation += 1
        pending = Pending(prompt = prompt, generation = generation)
        return generation
    }

    fun prompt(): String? = pending?.prompt

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
}
