package com.elon.app

internal enum class WebChatSendAuthority {
    OFFICIAL_PAGE,
    SAME_ORIGIN_PRIVATE,
}

internal enum class WebChatSendAcceptance {
    DISPATCHING,
    DISPATCHED_UNCONFIRMED,
    ACCEPTED,
    SETTLED,
    FAILED,
    UNKNOWN,
}

internal enum class WebChatPageSyncState {
    CLEAN,
    DIRTY,
    RECONCILING,
}

internal data class WebChatSendCommand(
    val id: String,
    val prompt: String,
    val authority: WebChatSendAuthority,
    val generation: Long,
    val privateTextTransactionAllowed: Boolean = true,
    val acceptance: WebChatSendAcceptance = WebChatSendAcceptance.DISPATCHING,
    val pageSyncState: WebChatPageSyncState = WebChatPageSyncState.CLEAN,
    val confirmationRechecks: Int = 0,
    val requiresOfficialConfirmation: Boolean = false,
)

internal class WebChatSendCommandLedger(
    private val requestIdFactory: (Long) -> String = ::defaultRequestId,
) {
    enum class ReceiptResult {
        IGNORED,
        ACCEPTED,
        FAILED,
        UNKNOWN,
    }

    enum class FallbackDecision {
        SAFE_BEFORE_DISPATCH,
        RECONCILE_ONLY,
        FORBIDDEN_AFTER_ACCEPTANCE,
        NOT_APPLICABLE,
    }

    private var generation = 0L
    private var active: WebChatSendCommand? = null
    private val completed = ArrayDeque<WebChatSendCommand>()

    fun begin(
        prompt: String,
        authority: WebChatSendAuthority,
        requestId: String? = null,
        privateTextTransactionAllowed: Boolean = true,
    ): WebChatSendCommand? {
        if (active != null) return null
        val nextGeneration = generation + 1
        val commandId = when {
            requestId == null -> requestIdFactory(nextGeneration)
            REQUEST_ID.matches(requestId) -> requestId
            else -> return null
        }
        generation = nextGeneration
        val command = WebChatSendCommand(
            id = commandId,
            prompt = prompt,
            authority = authority,
            generation = generation,
            privateTextTransactionAllowed = privateTextTransactionAllowed,
        )
        active = command
        return command
    }

    fun current(): WebChatSendCommand? = active

    fun prompt(): String? = active?.prompt

    fun commandId(): String? = active?.id

    fun phase(): WebChatPendingSendState.Phase = active?.let { command ->
        when {
            command.acceptance == WebChatSendAcceptance.UNKNOWN ->
                WebChatPendingSendState.Phase.RESULT_UNKNOWN
            command.requiresOfficialConfirmation -> WebChatPendingSendState.Phase.OFFICIAL_CONFIRMATION
            command.acceptance == WebChatSendAcceptance.ACCEPTED ->
                WebChatPendingSendState.Phase.AWAITING_RESPONSE
            else -> WebChatPendingSendState.Phase.SUBMITTING
        }
    } ?: WebChatPendingSendState.Phase.IDLE

    fun markDispatched(commandId: String): Boolean = update(commandId) { command ->
        if (command.acceptance != WebChatSendAcceptance.DISPATCHING) return@update command
        command.copy(acceptance = WebChatSendAcceptance.DISPATCHED_UNCONFIRMED)
    }

    fun acceptReceipt(
        commandId: String?,
        ok: Boolean,
        authority: WebChatSendAuthority? = null,
        indeterminate: Boolean = false,
    ): ReceiptResult {
        val command = active ?: return ReceiptResult.IGNORED
        if (commandId.isNullOrBlank() || command.id != commandId) return ReceiptResult.IGNORED
        val resolvedAuthority = authority ?: command.authority
        if (indeterminate) {
            if (command.acceptance == WebChatSendAcceptance.ACCEPTED) return ReceiptResult.IGNORED
            active = command.copy(
                authority = resolvedAuthority,
                acceptance = WebChatSendAcceptance.UNKNOWN,
                pageSyncState = WebChatPageSyncState.RECONCILING,
            )
            return ReceiptResult.UNKNOWN
        }
        if (!ok) {
            archive(command.copy(
                authority = resolvedAuthority,
                acceptance = WebChatSendAcceptance.FAILED,
            ))
            return ReceiptResult.FAILED
        }
        active = command.copy(
            authority = resolvedAuthority,
            acceptance = WebChatSendAcceptance.ACCEPTED,
            pageSyncState = pageSyncStateAfterAcceptance(resolvedAuthority),
        )
        return ReceiptResult.ACCEPTED
    }

    fun observeSubmission(content: String?): Boolean {
        val command = active ?: return false
        if (normalize(content) != normalize(command.prompt)) return false
        if (command.acceptance == WebChatSendAcceptance.ACCEPTED) return false
        active = command.copy(
            acceptance = WebChatSendAcceptance.ACCEPTED,
            pageSyncState = pageSyncStateAfterAcceptance(command.authority),
        )
        return true
    }

    fun observeCompletedTurn(content: String?, assistantObserved: Boolean): Boolean {
        val command = active ?: return false
        if (normalize(content) != normalize(command.prompt) || !assistantObserved) return false
        archive(command.copy(
            acceptance = WebChatSendAcceptance.SETTLED,
            pageSyncState = WebChatPageSyncState.CLEAN,
        ))
        return true
    }

    fun failBeforeDispatch(commandId: String): String? {
        val command = active?.takeIf {
            it.id == commandId && it.acceptance == WebChatSendAcceptance.DISPATCHING
        } ?: return null
        archive(command.copy(acceptance = WebChatSendAcceptance.FAILED))
        return command.prompt
    }

    fun onConfirmationTimeout(expectedGeneration: Long): WebChatPendingSendState.TimeoutResult {
        val command = active ?: return ignoredTimeout()
        if (command.generation != expectedGeneration) return ignoredTimeout()
        if (command.acceptance == WebChatSendAcceptance.ACCEPTED) {
            if (command.requiresOfficialConfirmation) return ignoredTimeout()
            val rechecks = command.confirmationRechecks + 1
            if (rechecks > MAX_CONFIRMATION_RECHECKS) {
                active = command.copy(
                    confirmationRechecks = rechecks,
                    requiresOfficialConfirmation = true,
                )
                return WebChatPendingSendState.TimeoutResult(
                    WebChatPendingSendState.TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION,
                )
            }
            active = command.copy(confirmationRechecks = rechecks)
            return WebChatPendingSendState.TimeoutResult(
                WebChatPendingSendState.TimeoutAction.KEEP_WAITING,
            )
        }
        val rechecks = command.confirmationRechecks + 1
        val unknown = command.copy(
            acceptance = WebChatSendAcceptance.UNKNOWN,
            pageSyncState = WebChatPageSyncState.RECONCILING,
            confirmationRechecks = rechecks,
            requiresOfficialConfirmation = rechecks > MAX_CONFIRMATION_RECHECKS,
        )
        active = unknown
        return if (unknown.requiresOfficialConfirmation) {
            WebChatPendingSendState.TimeoutResult(
                WebChatPendingSendState.TimeoutAction.REQUIRE_RECONCILIATION,
            )
        } else {
            WebChatPendingSendState.TimeoutResult(
                WebChatPendingSendState.TimeoutAction.KEEP_WAITING,
            )
        }
    }

    fun fallbackDecision(): FallbackDecision = when (active?.acceptance) {
        WebChatSendAcceptance.DISPATCHING -> FallbackDecision.SAFE_BEFORE_DISPATCH
        WebChatSendAcceptance.DISPATCHED_UNCONFIRMED,
        WebChatSendAcceptance.UNKNOWN -> FallbackDecision.RECONCILE_ONLY
        WebChatSendAcceptance.ACCEPTED -> FallbackDecision.FORBIDDEN_AFTER_ACCEPTANCE
        WebChatSendAcceptance.SETTLED,
        WebChatSendAcceptance.FAILED,
        null -> FallbackDecision.NOT_APPLICABLE
    }

    fun markPageReconciliationStarted(commandId: String): Boolean = update(commandId) { command ->
        command.copy(pageSyncState = WebChatPageSyncState.RECONCILING)
    }

    fun markPageReconciled(commandId: String): Boolean = update(commandId) { command ->
        command.copy(pageSyncState = WebChatPageSyncState.CLEAN)
    }

    fun history(): List<WebChatSendCommand> = completed.toList()

    fun clear() {
        generation += 1
        active = null
    }

    private fun update(
        commandId: String,
        transform: (WebChatSendCommand) -> WebChatSendCommand,
    ): Boolean {
        val command = active?.takeIf { it.id == commandId } ?: return false
        active = transform(command)
        return true
    }

    private fun archive(command: WebChatSendCommand) {
        completed.addLast(command)
        while (completed.size > MAX_HISTORY) completed.removeFirst()
        generation += 1
        active = null
    }

    private fun normalize(value: String?): String = value.orEmpty().trim().replace(WHITESPACE, " ")

    private fun pageSyncStateAfterAcceptance(authority: WebChatSendAuthority): WebChatPageSyncState =
        if (authority == WebChatSendAuthority.OFFICIAL_PAGE) {
            WebChatPageSyncState.CLEAN
        } else {
            WebChatPageSyncState.DIRTY
        }

    private fun ignoredTimeout() = WebChatPendingSendState.TimeoutResult(
        WebChatPendingSendState.TimeoutAction.IGNORE,
    )

    private companion object {
        const val MAX_CONFIRMATION_RECHECKS = 2
        const val MAX_HISTORY = 24
        val WHITESPACE = Regex("\\s+")
        val REQUEST_ID = Regex("mcp_[a-z0-9]{1,32}")

        fun defaultRequestId(generation: Long): String = "mcp_s${generation.toString(36)}"
    }
}
