package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebSnapshot

internal class WebChatPendingSendState {
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
        if (current.submissionConfirmed) return false
        current.submissionConfirmed = true
        return true
    }

    fun observeSubmission(content: String?): Boolean {
        val current = pending ?: return false
        if (normalize(content) != normalize(current.prompt)) return false
        return confirmSubmission()
    }

    fun failSubmission(): String? {
        val prompt = pending?.prompt ?: return null
        invalidate()
        return prompt
    }

    fun observeCompletedTurn(content: String?, assistantObserved: Boolean): Boolean {
        val current = pending ?: return false
        if (normalize(content) != normalize(current.prompt)) return false
        if (!assistantObserved) return false
        invalidate()
        return true
    }

    fun onConfirmationTimeout(expectedGeneration: Long): TimeoutResult {
        val current = pending ?: return TimeoutResult(TimeoutAction.IGNORE)
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

    private fun normalize(value: String?): String = value.orEmpty().trim().replace(WHITESPACE, " ")

    private companion object {
        const val MAX_CONFIRMATION_RECHECKS = 2
        val WHITESPACE = Regex("\\s+")
    }
}

internal object WebChatPendingSendPresentation {
    fun status(phase: WebChatPendingSendState.Phase): String? = when (phase) {
        WebChatPendingSendState.Phase.IDLE -> null
        WebChatPendingSendState.Phase.SUBMITTING -> "发送中…"
        WebChatPendingSendState.Phase.AWAITING_RESPONSE -> "已发送 · 等待回复"
        WebChatPendingSendState.Phase.OFFICIAL_CONFIRMATION -> "已发送 · 回答同步较慢"
    }
}

internal object WebChatPendingSendSnapshotPresentation {
    fun resolve(
        previous: ChatGptWebSnapshot?,
        incoming: ChatGptWebSnapshot,
        pending: Boolean,
    ): ChatGptWebSnapshot {
        if (!pending || incoming.messages.isNotEmpty()) {
            return incoming
        }
        val retained = previous?.takeIf { it.messages.isNotEmpty() } ?: return incoming
        return incoming.copy(
            messages = retained.messages,
            messageWindowStart = retained.messageWindowStart,
            observedMessageCount = retained.observedMessageCount,
        )
    }
}
