package com.elon.app.chatgptweb

import android.net.Uri
import com.elon.app.OfficialPageWebChatSendTransport
import com.elon.app.PendingAttachment
import com.elon.app.WebChatPendingSendState
import com.elon.app.WebChatSendCommandLedger
import com.elon.app.WebChatSendCoordinator
import com.elon.app.WebChatSendTransport

internal enum class ChatGptWebSendOrigin {
    SOCIAL,
    MCP,
    ATTACHMENT,
}

internal data class ChatGptWebSendReceipt(
    val origin: ChatGptWebSendOrigin,
    val ok: Boolean,
    val failedPrompt: String? = null,
)

internal fun chatGptOfficialPageSendTransport(
    pageAdapter: () -> ChatGptWebPageAdapter?,
    snapshot: () -> ChatGptWebSnapshot?,
    ready: () -> Boolean,
): WebChatSendTransport = OfficialPageWebChatSendTransport(
    ready = ready,
    sendPrompt = send@ { prompt, requestId ->
        val adapter = pageAdapter() ?: return@send false
        val current = snapshot() ?: return@send false
        adapter.sendPrompt(prompt, current.draft, requestId)
        true
    },
    requestReconciliation = { pageAdapter()?.requestSnapshot() },
)

internal class ChatGptWebSendOwner(
    transport: WebChatSendTransport,
    private val snapshot: () -> ChatGptWebSnapshot?,
    private val stageUploads: (List<PendingAttachment>) -> List<Uri>?,
    private val requestAttachmentUpload: () -> Boolean,
    private val removeAttachment: (String) -> Unit,
    private val postDelayed: (Runnable, Long) -> Unit,
    private val removeCallbacks: (Runnable) -> Unit,
    private val onTerminalTimeout: (WebChatPendingSendState.TimeoutResult) -> Unit,
    private val onAttachmentChanged: (ChatGptWebAttachmentSendUpdate) -> Unit,
    private val onSendStateChanged: () -> Unit,
    confirmationTimeoutMs: Long = DEFAULT_CONFIRMATION_TIMEOUT_MS,
    private val attachmentTimeoutMs: Long = DEFAULT_ATTACHMENT_TIMEOUT_MS,
) {
    private val coordinator = WebChatSendCoordinator(
        transport = transport,
        postDelayed = postDelayed,
        removeCallbacks = removeCallbacks,
        onTerminalTimeout = {
            onSendStateChanged()
            onTerminalTimeout(it)
        },
        confirmationTimeoutMs = confirmationTimeoutMs,
    )
    private var origin: ChatGptWebSendOrigin? = null
    private var attachmentTracker: ChatGptWebAttachmentSendTracker? = null
    private var queuedUploadUris = emptyList<Uri>()
    private var attachmentTimeout: Runnable? = null
    private var lastAttachmentPhase = ATTACHMENT_PHASE_IDLE

    fun isReady(): Boolean = coordinator.isReady()

    fun prompt(): String? = coordinator.prompt()

    fun status(): String? = coordinator.status()

    fun requiresOfficialConfirmation(): Boolean = coordinator.requiresOfficialConfirmation()

    fun fallbackDecision(): WebChatSendCommandLedger.FallbackDecision =
        coordinator.fallbackDecision()

    fun dispatchSocial(prompt: String): WebChatSendCoordinator.DispatchResult =
        dispatch(prompt, ChatGptWebSendOrigin.SOCIAL)

    fun dispatchMcp(
        prompt: String,
        requestId: String,
    ): WebChatSendCoordinator.DispatchResult =
        dispatch(prompt, ChatGptWebSendOrigin.MCP, requestId)

    fun beginAttachments(
        prompt: String,
        attachments: List<PendingAttachment>,
    ): Boolean {
        val current = snapshot() ?: return false
        if (attachments.isEmpty() || attachmentTracker != null) return false
        val reserved = coordinator.reserve(
            prompt = prompt,
            baselineSnapshot = current,
            onPending = onSendStateChanged,
        )
        if (reserved.outcome != WebChatSendCoordinator.ReserveOutcome.RESERVED) return false
        val uris = runCatching { stageUploads(attachments) }.getOrNull()
        if (uris == null) {
            requireNotNull(reserved.commandId).let(coordinator::cancelReserved)
            onSendStateChanged()
            return false
        }

        origin = ChatGptWebSendOrigin.ATTACHMENT
        attachmentTracker = ChatGptWebAttachmentSendTracker.begin(
            prompt = prompt,
            localAttachmentCount = attachments.size,
            snapshot = current,
        )
        queuedUploadUris = uris
        publishAttachmentPhase(ChatGptWebAttachmentSendTracker.Phase.UPLOADING)
        scheduleAttachmentTimeout()
        if (requestAttachmentUpload()) return true

        failAttachmentSend("官网附件入口尚未就绪。")
        return false
    }

    fun consumeQueuedUploadUris(): List<Uri> = queuedUploadUris.also {
        queuedUploadUris = emptyList()
    }

    fun attachmentSendPhase(): String =
        attachmentTracker?.phase?.wireValue ?: lastAttachmentPhase

    fun pendingAttachmentCount(): Int = attachmentTracker?.localAttachmentCount ?: 0

    fun hasAttachmentSend(): Boolean = attachmentTracker != null

    fun observeSnapshot(current: ChatGptWebSnapshot) {
        if (coordinator.observeSnapshot(current) == WebChatSendCoordinator.Observation.TURN_COMPLETED) {
            origin = null
        }
        processAttachmentSnapshot(current)
    }

    fun acceptCommandResult(event: ChatGptWebEvent.CommandResult): ChatGptWebSendReceipt? {
        if (event.action == "request_attachment_upload" && !event.ok) {
            failAttachmentSend(event.detail.ifBlank { "官网附件操作失败，请重试。" })
            return null
        }
        if (event.action != "send_prompt") return null
        if (event.requestId.isNullOrBlank() || event.requestId != coordinator.commandId()) return null
        val matchedOrigin = origin ?: return null
        val failedPrompt = coordinator.acceptCommandResult(event.requestId, event.ok)
        if (!event.ok) {
            origin = null
            if (matchedOrigin == ChatGptWebSendOrigin.ATTACHMENT) {
                failAttachmentSend(event.detail.ifBlank { "官网附件操作失败，请重试。" })
            }
        }
        onSendStateChanged()
        return ChatGptWebSendReceipt(matchedOrigin, event.ok, failedPrompt)
    }

    fun failFileChooser(detail: String) = failAttachmentSend(detail)

    fun pauseWatchdog() = coordinator.pauseWatchdog()

    fun clear() {
        cancelAttachmentWorkflow()
        coordinator.clear()
        origin = null
        onSendStateChanged()
    }

    fun dispose() {
        cancelAttachmentWorkflow()
        coordinator.dispose()
        origin = null
    }

    private fun dispatch(
        prompt: String,
        nextOrigin: ChatGptWebSendOrigin,
        requestId: String? = null,
    ): WebChatSendCoordinator.DispatchResult {
        if (nextOrigin != ChatGptWebSendOrigin.ATTACHMENT && attachmentTracker == null) {
            lastAttachmentPhase = ATTACHMENT_PHASE_IDLE
        }
        val result = coordinator.dispatch(
            prompt = prompt,
            baselineSnapshot = snapshot(),
            requestId = requestId,
            onPending = onSendStateChanged,
        )
        if (result.outcome == WebChatSendCoordinator.DispatchOutcome.DISPATCHED) {
            origin = nextOrigin
        }
        return result
    }

    private fun processAttachmentSnapshot(current: ChatGptWebSnapshot) {
        val tracker = attachmentTracker ?: return
        when (val observation = tracker.observe(current)) {
            ChatGptWebAttachmentSendTracker.Observation.Wait -> Unit
            ChatGptWebAttachmentSendTracker.Observation.SendPrompt -> {
                publishAttachmentPhase(tracker.phase)
                val commandId = coordinator.commandId()
                val result = if (origin == ChatGptWebSendOrigin.ATTACHMENT && commandId != null) {
                    coordinator.dispatchReserved(commandId)
                } else {
                    WebChatSendCoordinator.DispatchResult(WebChatSendCoordinator.DispatchOutcome.BUSY)
                }
                if (result.outcome != WebChatSendCoordinator.DispatchOutcome.DISPATCHED) {
                    failAttachmentSend("官网发送入口尚未就绪。")
                }
            }
            is ChatGptWebAttachmentSendTracker.Observation.Complete -> {
                cancelAttachmentTimeout()
                attachmentTracker = null
                queuedUploadUris = emptyList()
                lastAttachmentPhase = ATTACHMENT_PHASE_COMPLETED
                onAttachmentChanged(
                    ChatGptWebAttachmentSendUpdate(
                        phase = ATTACHMENT_PHASE_COMPLETED,
                        attachmentCount = tracker.localAttachmentCount,
                        userMessageId = observation.userMessageId,
                    ),
                )
            }
            is ChatGptWebAttachmentSendTracker.Observation.Failed ->
                failAttachmentSend(observation.detail)
        }
    }

    private fun publishAttachmentPhase(phase: ChatGptWebAttachmentSendTracker.Phase) {
        lastAttachmentPhase = phase.wireValue
        onAttachmentChanged(
            ChatGptWebAttachmentSendUpdate(
                phase = phase.wireValue,
                attachmentCount = attachmentTracker?.localAttachmentCount ?: 0,
            ),
        )
    }

    private fun failAttachmentSend(detail: String) {
        val tracker = attachmentTracker ?: return
        cancelAttachmentTimeout()
        snapshot()?.let(tracker::uploadedAttachmentIds)?.forEach(removeAttachment)
        if (
            origin == ChatGptWebSendOrigin.ATTACHMENT &&
            coordinator.fallbackDecision() == WebChatSendCommandLedger.FallbackDecision.SAFE_BEFORE_DISPATCH
        ) {
            coordinator.commandId()?.let(coordinator::cancelReserved)
            origin = null
        }
        if (coordinator.prompt() == null) origin = null
        tracker.markSendFailed()
        queuedUploadUris = emptyList()
        lastAttachmentPhase = tracker.phase.wireValue
        onAttachmentChanged(
            ChatGptWebAttachmentSendUpdate(
                phase = tracker.phase.wireValue,
                attachmentCount = tracker.localAttachmentCount,
                detail = detail,
            ),
        )
        attachmentTracker = null
        onSendStateChanged()
    }

    private fun cancelAttachmentWorkflow() {
        cancelAttachmentTimeout()
        queuedUploadUris = emptyList()
        attachmentTracker = null
        lastAttachmentPhase = ATTACHMENT_PHASE_IDLE
    }

    private fun scheduleAttachmentTimeout() {
        cancelAttachmentTimeout()
        val expectedTracker = attachmentTracker ?: return
        val task = Runnable {
            attachmentTimeout = null
            if (attachmentTracker === expectedTracker) {
                failAttachmentSend("附件上传超时，请检查网络后重试。")
            }
        }
        attachmentTimeout = task
        postDelayed(task, attachmentTimeoutMs)
    }

    private fun cancelAttachmentTimeout() {
        attachmentTimeout?.let(removeCallbacks)
        attachmentTimeout = null
    }

    private companion object {
        const val ATTACHMENT_PHASE_IDLE = "idle"
        const val ATTACHMENT_PHASE_COMPLETED = "completed"
        const val DEFAULT_CONFIRMATION_TIMEOUT_MS = 12_000L
        const val DEFAULT_ATTACHMENT_TIMEOUT_MS = 120_000L
    }
}
