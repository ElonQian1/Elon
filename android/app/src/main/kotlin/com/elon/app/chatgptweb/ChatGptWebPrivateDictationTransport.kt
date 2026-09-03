package com.elon.app.chatgptweb

import com.elon.app.DebugTraceStore
import com.elon.app.MainWebChatNativeDictationScheduler
import com.elon.app.WebChatNativeDictationPhase
import com.elon.app.WebChatNativeDictationScheduler
import com.elon.app.WebChatNativeDictationState
import com.elon.app.WebChatPrivateDictationPort

/** Coordinates page-local private transcription without moving credentials out of the WebView. */
internal class ChatGptWebPrivateDictationTransport(
    private val enabled: Boolean,
    private val readyCheck: () -> Boolean,
    private val currentOfficialDraft: () -> String?,
    private val readDraft: () -> String,
    private val writeDraft: (String) -> Unit,
    private val dispatchStart: (String, String, () -> Unit) -> Boolean,
    private val dispatchSubmit: () -> Boolean,
    private val dispatchCancel: () -> Boolean,
    private val onFailure: (String) -> Unit,
    private val scheduler: WebChatNativeDictationScheduler =
        MainWebChatNativeDictationScheduler(),
    private val trace: (String, Map<String, Any?>) -> Unit = DebugTraceStore::record,
) : WebChatPrivateDictationPort {
    private var state = WebChatNativeDictationState()
    private var generation = 0
    private var originalDraft = ""
    private var stateListener: (WebChatNativeDictationState) -> Unit = {}
    private var fallbackBeforeCapture: () -> Unit = {}
    private var transcriptLength = 0
    private var awaitingDraft = false
    private var timeoutTask: Runnable? = null

    override fun ready(): Boolean = enabled && !state.active && readyCheck() &&
        currentOfficialDraft() != null

    override fun start(
        onStateChanged: (WebChatNativeDictationState) -> Unit,
        onUnavailableBeforeCapture: () -> Unit,
    ): Boolean {
        if (state.active) return true
        if (!ready()) return false
        val expectedOfficialDraft = currentOfficialDraft() ?: return false
        stateListener = onStateChanged
        fallbackBeforeCapture = onUnavailableBeforeCapture
        originalDraft = readDraft()
        generation += 1
        val token = generation
        updateState(WebChatNativeDictationPhase.STARTING)
        scheduleTimeout(token, START_TIMEOUT_MS, "start_timeout")
        val accepted = dispatchStart(originalDraft, expectedOfficialDraft) {
            if (!isCurrent(token, WebChatNativeDictationPhase.STARTING)) return@dispatchStart
            reset()
            onFailure("需要麦克风权限才能使用语音输入")
        }
        if (!accepted) {
            reset()
            return false
        }
        trace("web_chat_dictation_private_start", mapOf("accepted" to true))
        return true
    }

    override fun submit(): Boolean {
        if (state.phase != WebChatNativeDictationPhase.LISTENING) return false
        val token = generation
        if (!dispatchSubmit()) return false
        updateState(WebChatNativeDictationPhase.PROCESSING)
        scheduleTimeout(token, SUBMIT_TIMEOUT_MS, "submit_timeout")
        return true
    }

    override fun cancel(): Boolean {
        if (!state.active) return false
        val token = generation
        if (!dispatchCancel()) return false
        updateState(WebChatNativeDictationPhase.PROCESSING)
        scheduleTimeout(token, CANCEL_TIMEOUT_MS, "cancel_timeout")
        return true
    }

    override fun state(): WebChatNativeDictationState = state

    override fun onCommandResult(action: String, ok: Boolean, detail: String) {
        when (action) {
            START_ACTION -> handleStartResult(ok, detail)
            SUBMIT_ACTION -> handleSubmitResult(ok, detail)
            CANCEL_ACTION -> handleCancelResult(ok)
        }
    }

    override fun observeOfficialDraft(draft: String) {
        if (!awaitingDraft || state.phase != WebChatNativeDictationPhase.PROCESSING) return
        if (transcriptLength > 0 && draft == originalDraft) return
        writeDraft(draft)
        trace(
            "web_chat_dictation_private_draft",
            mapOf("accepted" to true, "length" to draft.length),
        )
        reset()
    }

    override fun destroy() {
        if (state.active) dispatchCancel()
        reset()
        stateListener = {}
        fallbackBeforeCapture = {}
    }

    private fun handleStartResult(ok: Boolean, detail: String) {
        if (state.phase != WebChatNativeDictationPhase.STARTING) return
        cancelTimeout()
        if (ok && detail == CAPTURE_STARTED) {
            updateState(WebChatNativeDictationPhase.LISTENING)
            return
        }
        val fallback = detail.startsWith(BEFORE_CAPTURE_PREFIX)
        val callback = fallbackBeforeCapture
        reset()
        trace(
            "web_chat_dictation_private_unavailable",
            mapOf("before_capture" to fallback),
        )
        if (fallback) callback() else onFailure("语音输入未能启动，请重试")
    }

    private fun handleSubmitResult(ok: Boolean, detail: String) {
        if (state.phase != WebChatNativeDictationPhase.PROCESSING) return
        cancelTimeout()
        transcriptLength = detail.substringAfter(TRANSCRIPT_READY_PREFIX, "")
            .toIntOrNull()
            ?.coerceIn(0, MAX_TRANSCRIPT_LENGTH)
            ?: 0
        if (!ok || !detail.startsWith(TRANSCRIPT_READY_PREFIX) || transcriptLength <= 0) {
            reset()
            onFailure("语音识别失败，请重试")
            return
        }
        awaitingDraft = true
        val token = generation
        scheduleTimeout(token, DRAFT_TIMEOUT_MS, "draft_timeout")
    }

    private fun handleCancelResult(ok: Boolean) {
        if (!state.active) return
        cancelTimeout()
        writeDraft(originalDraft)
        reset()
        if (!ok) onFailure("语音输入未完全关闭，请重试")
    }

    private fun scheduleTimeout(token: Int, delayMs: Long, stage: String) {
        cancelTimeout()
        timeoutTask = Runnable {
            if (token != generation || !state.active) return@Runnable
            dispatchCancel()
            reset()
            trace("web_chat_dictation_private_timeout", mapOf("stage" to stage))
            onFailure("语音输入连接超时，请重试")
        }.also { scheduler.postDelayed(it, delayMs) }
    }

    private fun cancelTimeout() {
        timeoutTask?.let(scheduler::remove)
        timeoutTask = null
    }

    private fun reset() {
        cancelTimeout()
        generation += 1
        originalDraft = ""
        transcriptLength = 0
        awaitingDraft = false
        fallbackBeforeCapture = {}
        updateState(WebChatNativeDictationPhase.IDLE)
    }

    private fun updateState(phase: WebChatNativeDictationPhase) {
        val next = WebChatNativeDictationState(phase)
        if (state == next) return
        state = next
        stateListener(next)
    }

    private fun isCurrent(token: Int, phase: WebChatNativeDictationPhase): Boolean =
        generation == token && state.phase == phase

    companion object {
        const val START_ACTION = "private_start_dictation"
        const val SUBMIT_ACTION = "private_submit_dictation"
        const val CANCEL_ACTION = "private_cancel_dictation"
        private const val CAPTURE_STARTED = "capture_started"
        private const val BEFORE_CAPTURE_PREFIX = "before_capture:"
        private const val TRANSCRIPT_READY_PREFIX = "transcript_ready:"
        private const val MAX_TRANSCRIPT_LENGTH = 20_000
        private const val START_TIMEOUT_MS = 12_000L
        private const val SUBMIT_TIMEOUT_MS = 35_000L
        private const val CANCEL_TIMEOUT_MS = 5_000L
        private const val DRAFT_TIMEOUT_MS = 5_000L
    }
}
