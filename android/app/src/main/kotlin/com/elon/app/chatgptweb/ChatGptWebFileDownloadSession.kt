package com.elon.app.chatgptweb

import com.elon.app.WebChatFileDownloadState
import com.elon.app.WebChatFileDownloadState.Stage

internal class ChatGptWebFileDownloadSession(private val nowMs: () -> Long = { System.nanoTime() / 1_000_000 }) {
    private var leaseId: String? = null
    private var value: WebChatFileDownloadState? = null
    private var startedAt = 0L

    fun snapshot(): WebChatFileDownloadState? {
        val current = value ?: return null
        val timeout = if (current.stage == Stage.PREPARING) 25_000 else ChatGptWebFileByteTransfer.COMMAND_TIMEOUT_MS
        if (current.active && nowMs() - startedAt >= timeout) value = current.copy(
            stage = if (current.stage == Stage.PREPARING) Stage.FAILED else Stage.UNCONFIRMED)
        return value
    }

    fun begin(lease: String, requestId: String): Boolean {
        if (lease.isBlank() || requestId.isBlank() || snapshot()?.active == true) return false
        leaseId = lease
        value = WebChatFileDownloadState(requestId, Stage.PREPARING)
        startedAt = nowMs()
        return true
    }

    fun requestCancel(requestId: String): String? {
        val current = snapshot() ?: return null
        if (current.requestId != requestId || !current.active) return null
        value = current.copy(stage = Stage.CANCELLING)
        return leaseId
    }

    fun pageCancelled(lease: String) {
        // A repeated page abort cannot finish a native save/cleanup still in flight.
        if (value?.stage != Stage.CANCELLING) update(lease, Stage.FAILED)
    }

    fun update(lease: String, stage: Stage, received: Long = 0, total: Long = -1) {
        val current = value ?: return
        if (leaseId != lease || !current.active && !(current.stage == Stage.UNCONFIRMED && stage == Stage.SAVED)) return
        if (received !in 0..ChatGptWebFileByteTransfer.MAX_BYTES ||
            total !in -1..ChatGptWebFileByteTransfer.MAX_BYTES) return
        if (current.stage == Stage.CANCELLING && stage !in setOf(Stage.CANCELLED, Stage.SAVED, Stage.FAILED)) return
        val next = when {
            stage == Stage.CANCELLED && current.stage != Stage.CANCELLING -> Stage.FAILED
            stage == Stage.FAILED && current.stage == Stage.CANCELLING -> Stage.CANCELLED
            else -> stage
        }
        value = current.copy(stage = next, receivedBytes = maxOf(current.receivedBytes, received),
            totalBytes = if (total >= 0) total else current.totalBytes)
    }
}
