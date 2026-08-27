package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebSnapshot

internal class WebChatPendingSendState {
    enum class Phase {
        IDLE,
        SUBMITTING,
        AWAITING_RESPONSE,
        RESULT_UNKNOWN,
        OFFICIAL_CONFIRMATION,
    }

    enum class TimeoutAction {
        IGNORE,
        KEEP_WAITING,
        REQUIRE_RECONCILIATION,
        REQUIRE_OFFICIAL_CONFIRMATION,
        RESTORE,
    }

    data class TimeoutResult(
        val action: TimeoutAction,
        val prompt: String? = null,
    )

    private val ledger = WebChatSendCommandLedger()

    fun begin(prompt: String): Long {
        if (ledger.current() != null) ledger.clear()
        val command = requireNotNull(ledger.begin(prompt, WebChatSendAuthority.OFFICIAL_PAGE))
        ledger.markDispatched(command.id)
        return command.generation
    }

    fun prompt(): String? = ledger.prompt()

    fun requiresOfficialConfirmation(): Boolean =
        ledger.current()?.requiresOfficialConfirmation == true

    fun phase(): Phase = ledger.phase()

    fun confirmSubmission(): Boolean {
        val command = ledger.current() ?: return false
        return ledger.acceptReceipt(command.id, ok = true) ==
            WebChatSendCommandLedger.ReceiptResult.ACCEPTED
    }

    fun observeSubmission(content: String?): Boolean = ledger.observeSubmission(content)

    fun failSubmission(): String? {
        val command = ledger.current() ?: return null
        return ledger.failBeforeDispatch(command.id)
    }

    fun observeCompletedTurn(content: String?, assistantObserved: Boolean): Boolean =
        ledger.observeCompletedTurn(content, assistantObserved)

    fun onConfirmationTimeout(expectedGeneration: Long): TimeoutResult =
        ledger.onConfirmationTimeout(expectedGeneration)

    fun clear() = ledger.clear()
}

internal object WebChatPendingSendPresentation {
    fun status(phase: WebChatPendingSendState.Phase): String? = when (phase) {
        WebChatPendingSendState.Phase.IDLE -> null
        WebChatPendingSendState.Phase.SUBMITTING -> "发送中…"
        WebChatPendingSendState.Phase.AWAITING_RESPONSE -> "已发送 · 等待回复"
        WebChatPendingSendState.Phase.RESULT_UNKNOWN -> "发送结果待确认"
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
