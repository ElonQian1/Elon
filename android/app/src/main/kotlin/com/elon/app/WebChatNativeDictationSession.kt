package com.elon.app

import android.os.Handler
import android.os.Looper

internal enum class WebChatNativeDictationPhase {
    IDLE,
    STARTING,
    LISTENING,
    PROCESSING,
}

internal data class WebChatNativeDictationState(
    val phase: WebChatNativeDictationPhase = WebChatNativeDictationPhase.IDLE,
    val active: Boolean = phase != WebChatNativeDictationPhase.IDLE,
)

internal interface WebChatNativeDictationEngine {
    var onReady: () -> Unit
    var onStart: () -> Unit
    var onPartial: (String) -> Unit
    var onFinal: (String) -> Unit
    var onEnd: () -> Unit
    var onError: (String) -> Unit
    var onVolume: (Float) -> Unit
    val isRunning: Boolean
    val resultTimeoutMs: Long get() = DEFAULT_RESULT_TIMEOUT_MS
    fun start()
    fun stop()
    fun cancel()
    fun prewarm()
    fun destroy() = cancel()

    companion object {
        const val DEFAULT_RESULT_TIMEOUT_MS = 2_500L
    }
}

internal class AgentVoiceDictationEngine(
    private val delegate: AgentVoiceBridge,
) : WebChatNativeDictationEngine {
    override var onReady: () -> Unit
        get() = delegate.onReady
        set(value) { delegate.onReady = value }
    override var onStart: () -> Unit
        get() = delegate.onStart
        set(value) { delegate.onStart = value }
    override var onPartial: (String) -> Unit
        get() = delegate.onPartial
        set(value) { delegate.onPartial = value }
    override var onFinal: (String) -> Unit
        get() = delegate.onFinal
        set(value) { delegate.onFinal = value }
    override var onEnd: () -> Unit
        get() = delegate.onEnd
        set(value) { delegate.onEnd = value }
    override var onError: (String) -> Unit
        get() = delegate.onError
        set(value) { delegate.onError = value }
    override var onVolume: (Float) -> Unit
        get() = delegate.onVolume
        set(value) { delegate.onVolume = value }
    override val isRunning: Boolean get() = delegate.isRunning
    override fun start() = delegate.start()
    override fun stop() = delegate.stop()
    override fun cancel() = delegate.cancel()
    override fun prewarm() = delegate.prewarm()
    override fun destroy() = delegate.destroy()
}

internal interface WebChatNativeDictationScheduler {
    fun postDelayed(task: Runnable, delayMs: Long)
    fun remove(task: Runnable)
}

internal class MainWebChatNativeDictationScheduler : WebChatNativeDictationScheduler {
    private val handler = Handler(Looper.getMainLooper())
    override fun postDelayed(task: Runnable, delayMs: Long) {
        handler.postDelayed(task, delayMs)
    }
    override fun remove(task: Runnable) {
        handler.removeCallbacks(task)
    }
}

/**
 * Tap-to-dictate session for the production Web AI composer.
 *
 * Recognition remains owned by the injected voice engine. This class only
 * applies partial/final text to the native draft and restores the original draft
 * on cancellation. No recognized text is persisted or logged here.
 */
internal class WebChatNativeDictationSession(
    private val bridge: () -> WebChatNativeDictationEngine,
    private val readDraft: () -> String,
    private val writeDraft: (String) -> Unit,
    private val onStateChanged: (WebChatNativeDictationState) -> Unit,
    private val onUnavailable: (String) -> Unit,
    private val scheduler: WebChatNativeDictationScheduler = MainWebChatNativeDictationScheduler(),
) {
    private var state = WebChatNativeDictationState()
    private var generation = 0
    private var originalDraft = ""
    private var transcript = ""
    private var settleTask: Runnable? = null
    private var activeEngine: WebChatNativeDictationEngine? = null
    private var retainedEngine: WebChatNativeDictationEngine? = null

    fun state(): WebChatNativeDictationState = state

    fun start(): Boolean {
        if (state.active) return true
        val engine = bridge()
        if (engine.isRunning) return false
        if (retainedEngine !== engine) {
            retainedEngine?.destroy()
            retainedEngine = engine
        }

        generation += 1
        val token = generation
        activeEngine = engine
        originalDraft = readDraft()
        transcript = ""
        cancelSettleTask()
        configure(engine, token)
        updateState(WebChatNativeDictationPhase.STARTING)
        engine.start()
        return true
    }

    fun submit(): Boolean {
        if (!state.active) return false
        val token = generation
        updateState(WebChatNativeDictationPhase.PROCESSING)
        scheduleSettlement(token)
        activeEngine?.stop()
        return true
    }

    fun cancel(): Boolean {
        if (!state.active) return false
        generation += 1
        cancelSettleTask()
        val engine = activeEngine
        engine?.cancel()
        writeDraft(originalDraft)
        clearState()
        activeEngine = null
        engine?.prewarm()
        return true
    }

    fun destroy() {
        generation += 1
        cancelSettleTask()
        (activeEngine ?: retainedEngine)?.destroy()
        clearState()
        activeEngine = null
        retainedEngine = null
    }

    private fun configure(activeBridge: WebChatNativeDictationEngine, token: Int) {
        activeBridge.onReady = {
            if (isCurrent(token)) updateState(WebChatNativeDictationPhase.LISTENING)
        }
        activeBridge.onStart = {
            if (isCurrent(token)) updateState(WebChatNativeDictationPhase.LISTENING)
        }
        activeBridge.onPartial = partial@{ value ->
            if (!isCurrent(token) || value.isBlank()) return@partial
            transcript = value.trim()
            renderTranscript()
        }
        activeBridge.onFinal = final@{ value ->
            if (!isCurrent(token)) return@final
            if (value.isNotBlank()) {
                transcript = value.trim()
                renderTranscript()
            }
            settle(token, reportEmpty = true)
        }
        activeBridge.onEnd = end@{
            if (!isCurrent(token)) return@end
            // SmartVAD reports speech end before SpeechRecognizer delivers its
            // final result. Keep the partial draft visible while allowing that
            // final callback (or its 2 s fallback) to arrive.
            updateState(WebChatNativeDictationPhase.PROCESSING)
            scheduleSettlement(token)
        }
        activeBridge.onError = error@{ message ->
            if (!isCurrent(token)) return@error
            val hadTranscript = transcript.isNotBlank()
            if (!hadTranscript) writeDraft(originalDraft)
            settle(token, reportEmpty = false)
            if (!hadTranscript) onUnavailable(message)
        }
        activeBridge.onVolume = {}
    }

    private fun renderTranscript() {
        writeDraft(
            when {
                originalDraft.isBlank() -> transcript
                transcript.isBlank() -> originalDraft
                originalDraft.lastOrNull()?.isWhitespace() == true -> originalDraft + transcript
                else -> "$originalDraft $transcript"
            },
        )
    }

    private fun scheduleSettlement(token: Int) {
        cancelSettleTask()
        settleTask = Runnable { settle(token, reportEmpty = true) }.also {
            scheduler.postDelayed(
                it,
                activeEngine?.resultTimeoutMs ?: WebChatNativeDictationEngine.DEFAULT_RESULT_TIMEOUT_MS,
            )
        }
    }

    private fun settle(token: Int, reportEmpty: Boolean) {
        if (!isCurrent(token)) return
        cancelSettleTask()
        val hadTranscript = transcript.isNotBlank()
        if (hadTranscript) renderTranscript()
        val engine = activeEngine
        generation += 1
        if (engine?.isRunning == true) engine.cancel()
        clearState()
        activeEngine = null
        engine?.prewarm()
        if (reportEmpty && !hadTranscript) onUnavailable("没有识别到语音")
    }

    private fun clearState() {
        transcript = ""
        originalDraft = ""
        updateState(WebChatNativeDictationPhase.IDLE)
    }

    private fun updateState(phase: WebChatNativeDictationPhase) {
        val next = WebChatNativeDictationState(phase)
        if (next == state) return
        state = next
        onStateChanged(next)
    }

    private fun isCurrent(token: Int): Boolean = token == generation && state.active

    private fun cancelSettleTask() {
        settleTask?.let(scheduler::remove)
        settleTask = null
    }
}
