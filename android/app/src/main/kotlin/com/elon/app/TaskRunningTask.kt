package com.elon.app

internal data class RunningTask(
    val traceId: String,
    val projectId: String?,
    val conversationId: String?,
    var payload: String,
    var isDevelopment: Boolean,
    var waitingForReply: Boolean = true,
    var payloadSentForCurrentConnection: Boolean = false,
    var reconnectAttempts: Int = 0,
    var serverUrl: String? = null,
    var clientGeneration: Int = 0,
    var startedAtMs: Long = System.currentTimeMillis(),
    var firstServerEventAtMs: Long = 0L,
    var firstChatReplyAtMs: Long = 0L,
    var wsClient: ElonWsClient? = null,
    var reconnectRunnable: Runnable? = null,
    var lastStep: Int = 0,
    var lastStepTotal: Int = 4,
    var lastPhaseStartMs: Long = 0L
) {
    val requestKind: String
        get() = if (isDevelopment) "development" else "chat"
}
