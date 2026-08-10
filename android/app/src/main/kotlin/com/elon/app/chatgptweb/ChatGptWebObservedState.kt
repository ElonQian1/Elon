package com.elon.app.chatgptweb

internal class ChatGptWebObservedState(
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private var conversations: List<ChatGptWebConversation> = emptyList()
    private var features: List<ChatGptWebFeature> = emptyList()
    private var composerSections: Map<String, List<ChatGptWebComposerOption>> = emptyMap()
    private var lastCommand: ChatGptWebEvent.CommandResult? = null
    private var lastCommandObservedAtMs: Long? = null
    private var commandRequests: List<CommandRequest> = emptyList()
    private var nextCommandId = 0L
    private var updatedAtMs: Long = 0L

    fun accept(event: ChatGptWebEvent) {
        val observedAtMs = nowMs()
        when (event) {
            is ChatGptWebEvent.ConversationList -> conversations = event.conversations
            is ChatGptWebEvent.FeatureNavigation -> features = event.features
            is ChatGptWebEvent.ComposerControls -> {
                composerSections = composerSections + (event.section to event.options)
            }
            is ChatGptWebEvent.CommandResult -> {
                lastCommand = event
                lastCommandObservedAtMs = observedAtMs
                completeOldestRequest(event, observedAtMs)
            }
            else -> return
        }
        updatedAtMs = observedAtMs
    }

    fun beginComposerRequest(section: String) {
        composerSections = composerSections - section
        updatedAtMs = nowMs()
    }

    fun beginCommand(expectedAction: String): CommandRequest {
        val startedAt = nowMs()
        val request = CommandRequest(
            id = "mcp_${(++nextCommandId).toString(36)}",
            expectedAction = expectedAction,
            status = CommandRequest.PENDING,
            startedAtMs = startedAt,
        )
        commandRequests = (commandRequests + request).takeLast(MAX_COMMAND_REQUESTS)
        updatedAtMs = startedAt
        return request
    }

    fun snapshot(): Snapshot {
        expirePendingCommands()
        return Snapshot(
            conversations = conversations,
            features = features,
            composerSections = composerSections,
            lastCommand = lastCommand,
            commandRequests = commandRequests,
            updatedAtMs = updatedAtMs,
            lastCommandObservedAtMs = lastCommandObservedAtMs,
        )
    }

    private fun completeOldestRequest(
        event: ChatGptWebEvent.CommandResult,
        observedAtMs: Long,
    ) {
        val index = commandRequests.indexOfFirst {
            it.status == CommandRequest.PENDING && it.expectedAction == event.action
        }
        if (index < 0) return
        commandRequests = commandRequests.toMutableList().apply {
            this[index] = this[index].copy(
                status = if (event.ok) CommandRequest.SUCCEEDED else CommandRequest.FAILED,
                result = event,
                completedAtMs = observedAtMs,
            )
        }
    }

    private fun expirePendingCommands() {
        val now = nowMs()
        var changed = false
        commandRequests = commandRequests.map { request ->
            if (
                request.status == CommandRequest.PENDING &&
                now - request.startedAtMs >= COMMAND_TIMEOUT_MS
            ) {
                changed = true
                request.copy(status = CommandRequest.TIMED_OUT, completedAtMs = now)
            } else {
                request
            }
        }
        if (changed) updatedAtMs = now
    }

    internal data class Snapshot(
        val conversations: List<ChatGptWebConversation>,
        val features: List<ChatGptWebFeature>,
        val composerSections: Map<String, List<ChatGptWebComposerOption>>,
        val lastCommand: ChatGptWebEvent.CommandResult?,
        val commandRequests: List<CommandRequest>,
        val updatedAtMs: Long,
        val lastCommandObservedAtMs: Long? = null,
    ) {
        companion object {
            val EMPTY = Snapshot(emptyList(), emptyList(), emptyMap(), null, emptyList(), 0L)
        }
    }

    internal data class CommandRequest(
        val id: String,
        val expectedAction: String,
        val status: String,
        val startedAtMs: Long,
        val completedAtMs: Long? = null,
        val result: ChatGptWebEvent.CommandResult? = null,
    ) {
        companion object {
            const val PENDING = "pending"
            const val SUCCEEDED = "succeeded"
            const val FAILED = "failed"
            const val TIMED_OUT = "timed_out"
        }
    }

    private companion object {
        const val MAX_COMMAND_REQUESTS = 20
        const val COMMAND_TIMEOUT_MS = 20_000L
    }
}
