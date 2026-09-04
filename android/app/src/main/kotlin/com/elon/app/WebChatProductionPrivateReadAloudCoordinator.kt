package com.elon.app

import android.view.View

internal class WebChatProductionPrivateReadAloudCoordinator(
    private val host: View,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val onFeedback: (String) -> Unit,
    private val onFailure: () -> Unit,
) {
    private var requestEpoch = 0

    fun toggle(contextId: String): Boolean {
        val port = consumerPort() ?: return reject()
        val before = port.state()
        val wasActive = before.privateReadAloudContextId == contextId &&
            before.privateReadAloudState in ACTIVE_STATES
        val request = port.toggleOfficialReadAloud(contextId)
        if (!request.accepted || request.requestId.isNullOrBlank()) return reject()
        val epoch = ++requestEpoch
        onFeedback(if (wasActive) "正在停止官网朗读" else "正在准备官网声音")
        poll(port, request.requestId, contextId, wasActive, epoch, attempt = 0)
        return true
    }

    fun stopIfActive(contextId: String) {
        val port = consumerPort() ?: return
        val state = port.state()
        if (
            state.privateReadAloudContextId == contextId &&
            state.privateReadAloudState in ACTIVE_STATES
        ) {
            port.toggleOfficialReadAloud(contextId)
        }
    }

    fun release() {
        requestEpoch += 1
    }

    private fun poll(
        port: WebChatConsumerPort,
        requestId: String,
        contextId: String,
        expectedStop: Boolean,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch) return
        val state = port.state()
        val status = state.commandRequests.lastOrNull { it.id == requestId }?.status
        when (status) {
            WebChatConsumerCommandStatus.SUCCEEDED -> {
                when {
                    expectedStop -> onFeedback("已停止官网朗读")
                    state.privateReadAloudContextId == contextId &&
                        state.privateReadAloudState == "playing" -> onFeedback("开始官网朗读")
                    state.privateReadAloudState == "idle" -> onFeedback("官网朗读已完成")
                    attempt < POST_RECEIPT_STATE_ATTEMPTS -> schedule {
                        poll(port, requestId, contextId, expectedStop, epoch, attempt + 1)
                    }
                    else -> onFeedback("官网朗读已执行")
                }
                return
            }
            WebChatConsumerCommandStatus.FAILED,
            WebChatConsumerCommandStatus.TIMED_OUT -> return onFailure()
            else -> Unit
        }
        if (attempt >= MAX_POLL_ATTEMPTS) return onFailure()
        schedule { poll(port, requestId, contextId, expectedStop, epoch, attempt + 1) }
    }

    private fun schedule(action: () -> Unit) {
        host.postDelayed(action, POLL_INTERVAL_MS)
    }

    private fun reject(): Boolean {
        onFailure()
        return false
    }

    private companion object {
        val ACTIVE_STATES = setOf("loading", "playing")
        const val POLL_INTERVAL_MS = 100L
        const val MAX_POLL_ATTEMPTS = 210
        const val POST_RECEIPT_STATE_ATTEMPTS = 12
    }
}
