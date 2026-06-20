package com.elon.chatvoice

import android.Manifest
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import java.util.Locale

class SystemSpeechTranscriber(
    context: Context,
    private val languageTag: String = "zh-CN",
    private val eventSink: ChatVoiceEventSink? = null,
    private val engineFallbackEnabled: Boolean = true,
    private val prewarmEnabled: Boolean = true,
) {
    interface Listener {
        fun onReady() {}
        fun onVolume(value: Float) {}
        fun onPartial(transcript: SpeechTranscript) {}
        fun onFinal(transcript: SpeechTranscript) {}
        fun onCanceled() {}
        fun onError(error: ChatVoiceError) {}
    }

    private val appContext = context.applicationContext
    private val main = Handler(Looper.getMainLooper())
    private var recognizer: SpeechRecognizer? = null
    private var activeListener: Listener? = null
    private var sessionId = 0
    private var candidates: List<ChatVoiceRecognitionEngine> = emptyList()
    private var candidateIndex = 0
    private var sawAnyPartial = false
    private var sawReady = false
    private var busyRetryOnSame = 0
    private var coldStartRetryOnSame = 0
    private var sessionConflictRetry = 0
    private var listeningStartedAt = 0L
    private var currentEngine: ChatVoiceRecognitionEngine? = null
    private var prewarmedEngine: ComponentName? = null
    private var currentPreferOffline = false
    private var currentStartTimeoutMs = DEFAULT_START_TIMEOUT_MS

    fun isAvailable(): Boolean = SpeechRecognizer.isRecognitionAvailable(appContext)

    fun prewarm() {
        if (!prewarmEnabled || !isAvailable()) return
        main.post {
            if (activeListener != null || recognizer != null) return@post
            val engine = ChatVoiceRecognitionEngineSelector.listForUse(appContext).firstOrNull() ?: return@post
            recognizer = createRecognizer(engine.component).also {
                prewarmedEngine = engine.component
                it.setRecognitionListener(emptyRecognitionListener())
            }
            eventSink?.onVoiceEvent(
                ChatVoiceEvent.StateChanged(
                    ChatVoiceListeningState.PREPARING,
                    ChatVoiceInteractionContract.stateText(ChatVoiceListeningState.PREPARING),
                )
            )
            main.postDelayed({
                if (activeListener == null && recognizer != null && prewarmedEngine == engine.component) {
                    stopRecognizer(cancelOnly = true)
                    prewarmedEngine = null
                }
            }, PREWARM_WATCHDOG_MS)
        }
    }

    fun start(
        listener: Listener,
        preferOffline: Boolean = false,
        startTimeoutMs: Long = DEFAULT_START_TIMEOUT_MS,
    ) {
        main.post {
            val session = ++sessionId
            if (!isAvailable()) {
                listener.onError(ChatVoiceError("system_asr_unavailable", "手机系统语音识别不可用"))
                return@post
            }
            if (appContext.checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
                listener.onError(ChatVoiceError("record_audio_denied", "缺少麦克风权限"))
                return@post
            }
            if (recognizer != null && prewarmedEngine == null) {
                stopRecognizer(cancelOnly = true)
            }
            activeListener = listener
            currentPreferOffline = preferOffline
            currentStartTimeoutMs = startTimeoutMs
            candidates = if (engineFallbackEnabled) {
                ChatVoiceRecognitionEngineSelector.listForUse(appContext)
            } else {
                listOf(ChatVoiceRecognitionEngine(null, ChatVoiceRecognitionEngine.SYSTEM_DEFAULT_KEY, "系统默认"))
            }
            candidateIndex = 0
            sawAnyPartial = false
            sawReady = false
            busyRetryOnSame = 0
            coldStartRetryOnSame = 0
            sessionConflictRetry = 0
            eventSink?.onVoiceEvent(
                ChatVoiceEvent.StateChanged(
                    ChatVoiceListeningState.PREPARING,
                    ChatVoiceInteractionContract.stateText(ChatVoiceListeningState.PREPARING),
                )
            )
            startWithCurrentCandidate(session, preferOffline)
        }
    }

    fun stop(finalTimeoutMs: Long = DEFAULT_STOP_TIMEOUT_MS) {
        main.post {
            val current = recognizer ?: return@post
            val listener = activeListener ?: return@post
            val session = sessionId
            runCatching { current.stopListening() }.onFailure { error ->
                val voiceError = ChatVoiceError("system_asr_stop_failed", error.message ?: "停止识别失败", error)
                stopRecognizer(cancelOnly = true)
                activeListener = null
                eventSink?.onVoiceEvent(ChatVoiceEvent.Error(voiceError))
                listener.onError(voiceError)
                return@post
            }
            if (finalTimeoutMs <= 0L) return@post
            main.postDelayed({
                if (session != sessionId || activeListener !== listener || recognizer == null) return@postDelayed
                val timeout = ChatVoiceError("system_asr_stop_timeout", "系统语音识别超时")
                stopRecognizer(cancelOnly = true)
                activeListener = null
                eventSink?.onVoiceEvent(ChatVoiceEvent.Error(timeout))
                listener.onError(timeout)
            }, finalTimeoutMs)
        }
    }

    fun cancel() {
        main.post {
            sessionId += 1
            val listener = activeListener
            stopRecognizer(cancelOnly = true)
            activeListener = null
            eventSink?.onVoiceEvent(ChatVoiceEvent.Cancel)
            listener?.onCanceled()
        }
    }

    fun release() {
        main.post {
            sessionId += 1
            stopRecognizer(cancelOnly = true)
            activeListener = null
        }
    }

    private fun stopRecognizer(cancelOnly: Boolean) {
        val current = recognizer ?: return
        if (cancelOnly) runCatching { current.cancel() } else runCatching { current.stopListening() }
        runCatching { current.destroy() }
        recognizer = null
        prewarmedEngine = null
        currentEngine = null
    }

    private fun startWithCurrentCandidate(session: Int, preferOffline: Boolean, reuseCurrent: Boolean = false) {
        val engine = candidates.getOrNull(candidateIndex)
        if (engine == null) {
            failActiveListener(ChatVoiceError("system_asr_no_engine", "没有可用的手机语音识别引擎"))
            return
        }
        currentEngine = engine
        sawReady = false
        sawAnyPartial = false
        val current = if (reuseCurrent && recognizer != null) {
            recognizer!!
        } else if (recognizer != null && prewarmedEngine == engine.component) {
            recognizer!!
        } else {
            stopRecognizer(cancelOnly = true)
            createRecognizer(engine.component)
        }
        prewarmedEngine = null
        recognizer = current.apply {
            setRecognitionListener(createRecognitionListener(session))
        }
        listeningStartedAt = System.currentTimeMillis()
        runCatching { current.startListening(recognizerIntent(preferOffline)) }
            .onFailure { error ->
                handleRecognitionError(session, SpeechRecognizer.ERROR_CLIENT, error.message ?: "启动识别失败", preferOffline)
            }
        scheduleStartWatchdog(session, preferOffline, currentStartTimeoutMs)
    }

    private fun createRecognizer(component: ComponentName?): SpeechRecognizer =
        if (component != null) {
            SpeechRecognizer.createSpeechRecognizer(appContext, component)
        } else {
            SpeechRecognizer.createSpeechRecognizer(appContext)
        }

    private fun recognizerIntent(preferOffline: Boolean): Intent =
        Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_CALLING_PACKAGE, appContext.packageName)
            putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
            putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, preferOffline)
            putExtra(RecognizerIntent.EXTRA_LANGUAGE, languageTag)
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_PREFERENCE, languageTag)
        }

    private fun createRecognitionListener(session: Int): RecognitionListener =
        object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) {
                if (session != sessionId) return
                sawReady = true
                eventSink?.onVoiceEvent(
                    ChatVoiceEvent.StateChanged(
                        ChatVoiceListeningState.LISTENING,
                        ChatVoiceInteractionContract.stateText(ChatVoiceListeningState.LISTENING),
                    )
                )
                activeListener?.onReady()
            }

            override fun onRmsChanged(rmsdB: Float) {
                if (session != sessionId) return
                val normalized = ((rmsdB + 2f) / 12f).coerceIn(0f, 1f)
                activeListener?.onVolume(normalized)
                eventSink?.onVoiceEvent(ChatVoiceEvent.Volume(normalized))
            }

            override fun onPartialResults(partialResults: Bundle?) {
                if (session != sessionId) return
                val text = firstResult(partialResults) ?: return
                sawReady = true
                sawAnyPartial = true
                val transcript = SpeechTranscript(text, isFinal = false, SpeechSource.SYSTEM_ASR)
                activeListener?.onPartial(transcript)
                eventSink?.onVoiceEvent(ChatVoiceEvent.PartialResult(transcript))
            }

            override fun onResults(results: Bundle?) {
                if (session != sessionId) return
                sawReady = true
                val text = firstResult(results).orEmpty()
                val listener = activeListener
                stopRecognizer(cancelOnly = false)
                activeListener = null
                if (text.isBlank()) {
                    val error = ChatVoiceError("system_asr_no_match", "没有听清")
                    eventSink?.onVoiceEvent(ChatVoiceEvent.Error(error))
                    listener?.onError(error)
                } else {
                    currentEngine?.let { ChatVoiceEngineHealthStore.markOk(appContext, it.key()) }
                    val transcript = SpeechTranscript(text.trim(), isFinal = true, SpeechSource.SYSTEM_ASR)
                    eventSink?.onVoiceEvent(ChatVoiceEvent.FinalResult(transcript))
                    listener?.onFinal(transcript)
                }
            }

            override fun onError(error: Int) {
                if (session != sessionId) return
                handleRecognitionError(session, error, speechErrorMessage(error), preferOffline = currentPreferOffline)
            }

            override fun onBeginningOfSpeech() = Unit
            override fun onEndOfSpeech() = Unit
            override fun onBufferReceived(buffer: ByteArray?) = Unit
            override fun onEvent(eventType: Int, params: Bundle?) = Unit
        }

    private fun handleRecognitionError(session: Int, code: Int, message: String, preferOffline: Boolean) {
        if (session != sessionId) return
        val sinceStart = System.currentTimeMillis() - listeningStartedAt
        val engine = currentEngine
        if (code == SpeechRecognizer.ERROR_RECOGNIZER_BUSY && engine != null && busyRetryOnSame < 2 && !sawAnyPartial) {
            busyRetryOnSame += 1
            retrySameEngine(session, preferOffline, 250L, resetEngine = false)
            return
        }
        if (code == SpeechRecognizer.ERROR_CLIENT && sinceStart < 50L && engine != null && sessionConflictRetry < 2 && !sawAnyPartial) {
            sessionConflictRetry += 1
            retrySameEngine(session, preferOffline, if (sessionConflictRetry == 1) 600L else 1_200L, resetEngine = true)
            return
        }
        if (code == ERROR_SERVER_DISCONNECTED && sinceStart in 0..200 && engine != null && coldStartRetryOnSame < 2 && !sawAnyPartial) {
            coldStartRetryOnSame += 1
            retrySameEngine(session, preferOffline, if (coldStartRetryOnSame == 1) 100L else 300L, resetEngine = false)
            return
        }
        if (shouldTryNextEngine(code) && !sawAnyPartial && candidateIndex < candidates.lastIndex) {
            engine?.let { ChatVoiceEngineHealthStore.markFailed(appContext, it.key(), code, message) }
            candidateIndex += 1
            busyRetryOnSame = 0
            coldStartRetryOnSame = 0
            sessionConflictRetry = 0
            main.post { startWithCurrentCandidate(session, preferOffline) }
            return
        }
        engine?.let { ChatVoiceEngineHealthStore.markFailed(appContext, it.key(), code, message) }
        failActiveListener(ChatVoiceError("system_asr_$code", speechErrorMessage(code)))
    }

    private fun retrySameEngine(session: Int, preferOffline: Boolean, delayMs: Long, resetEngine: Boolean) {
        main.postDelayed({
            if (session != sessionId || activeListener == null) return@postDelayed
            if (resetEngine) {
                stopRecognizer(cancelOnly = true)
            }
            startWithCurrentCandidate(session, preferOffline, reuseCurrent = !resetEngine)
        }, delayMs)
    }

    private fun shouldTryNextEngine(code: Int): Boolean =
        when (code) {
            SpeechRecognizer.ERROR_NO_MATCH,
            SpeechRecognizer.ERROR_SPEECH_TIMEOUT,
            SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS,
            SpeechRecognizer.ERROR_AUDIO -> false
            else -> engineFallbackEnabled
        }

    private fun failActiveListener(error: ChatVoiceError) {
        val listener = activeListener
        stopRecognizer(cancelOnly = true)
        activeListener = null
        eventSink?.onVoiceEvent(ChatVoiceEvent.Error(error))
        listener?.onError(error)
    }

    private fun emptyRecognitionListener(): RecognitionListener =
        object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) = Unit
            override fun onBeginningOfSpeech() = Unit
            override fun onRmsChanged(rmsdB: Float) = Unit
            override fun onBufferReceived(buffer: ByteArray?) = Unit
            override fun onEndOfSpeech() = Unit
            override fun onError(error: Int) = Unit
            override fun onResults(results: Bundle?) = Unit
            override fun onPartialResults(partialResults: Bundle?) = Unit
            override fun onEvent(eventType: Int, params: Bundle?) = Unit
        }

    private fun firstResult(bundle: Bundle?): String? =
        bundle
            ?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
            ?.firstOrNull()
            ?.trim()
            ?.takeIf { it.isNotEmpty() }

    private fun speechErrorMessage(error: Int): String =
        when (error) {
            SpeechRecognizer.ERROR_AUDIO -> "麦克风录音失败"
            SpeechRecognizer.ERROR_CLIENT -> "识别服务客户端异常"
            SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS -> "麦克风权限不足"
            SpeechRecognizer.ERROR_NETWORK -> "网络不可用"
            SpeechRecognizer.ERROR_NETWORK_TIMEOUT -> "网络超时"
            SpeechRecognizer.ERROR_NO_MATCH -> "没有听清"
            SpeechRecognizer.ERROR_RECOGNIZER_BUSY -> "识别服务正忙"
            SpeechRecognizer.ERROR_SERVER -> "系统语音服务异常"
            SpeechRecognizer.ERROR_SPEECH_TIMEOUT -> "没有检测到语音"
            ERROR_SERVER_DISCONNECTED -> "语音服务断开"
            ERROR_LANGUAGE_NOT_SUPPORTED -> "系统不支持当前识别语言"
            ERROR_LANGUAGE_UNAVAILABLE -> "当前识别语言不可用"
            ERROR_CANNOT_CHECK_SUPPORT -> "无法检查系统识别能力"
            else -> String.format(Locale.ROOT, "系统语音识别失败(%d)", error)
        }

    private fun scheduleStartWatchdog(session: Int, preferOffline: Boolean, startTimeoutMs: Long) {
        if (startTimeoutMs <= 0L) return
        main.postDelayed({
            if (session != sessionId || activeListener == null || recognizer == null) return@postDelayed
            if (sawReady || sawAnyPartial) return@postDelayed
            handleRecognitionError(
                session,
                SpeechRecognizer.ERROR_CLIENT,
                "系统语音识别启动超时",
                preferOffline,
            )
        }, startTimeoutMs)
    }

    companion object {
        private const val PREWARM_WATCHDOG_MS: Long = 2_500L
        private const val ERROR_SERVER_DISCONNECTED = 11
        private const val ERROR_LANGUAGE_NOT_SUPPORTED = 12
        private const val ERROR_LANGUAGE_UNAVAILABLE = 13
        private const val ERROR_CANNOT_CHECK_SUPPORT = 14
        const val DEFAULT_START_TIMEOUT_MS: Long = 2_500L
        const val DEFAULT_STOP_TIMEOUT_MS: Long = 4_500L
    }
}
