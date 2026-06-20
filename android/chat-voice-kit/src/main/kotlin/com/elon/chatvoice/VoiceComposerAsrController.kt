package com.elon.chatvoice

import android.content.Context
import android.os.Handler
import android.os.Looper
import okhttp3.Call

class VoiceComposerAsrController(
    context: Context,
    private var config: VoiceComposerConfig,
    private val callbacks: Callbacks,
) {
    interface Callbacks {
        fun onReady() {}
        fun onVolume(value: Float) {}
        fun onPartial(transcript: SpeechTranscript) {}
        fun onFinal(transcript: SpeechTranscript) {}
        fun onServerFallbackStarted(reason: ChatVoiceError?) {}
        fun onTooShort() {}
        fun onCanceled() {}
        fun onError(error: ChatVoiceError) {}
    }

    private val appContext = context.applicationContext
    private val main = Handler(Looper.getMainLooper())
    private var transcriber = newTranscriber(config)
    private var recorder = newRecorder(config)
    private var serverClient = newServerClient(config)
    private var serverCall: Call? = null
    private var recordedVoice: RecordedVoice? = null
    private var session = 0
    private var active = false
    private var released = false
    private var completed = false
    private var recorderStarted = false
    private var serverFallbackStarted = false
    private var localFinal: SpeechTranscript? = null
    private var localError: ChatVoiceError? = null

    init {
        prewarmLocalEngine()
    }

    fun applyConfig(next: VoiceComposerConfig) {
        cancel(notify = false)
        config = next
        transcriber = newTranscriber(next)
        recorder = newRecorder(next)
        serverClient = newServerClient(next)
        prewarmLocalEngine()
    }

    fun start(): Boolean {
        val current = ++session
        active = true
        released = false
        completed = false
        recorderStarted = false
        serverFallbackStarted = false
        recordedVoice = null
        localFinal = null
        localError = null
        serverCall?.cancel()
        val recordResult = recorder.start()
        recorderStarted = recordResult.isSuccess
        recordResult.exceptionOrNull()?.let { error ->
            localError = ChatVoiceError("record_start_failed", error.message ?: "录音启动失败", error)
        }
        transcriber.start(
            listener(current),
            preferOffline = config.preferOfflineAsr,
            startTimeoutMs = config.asr.localStartTimeoutMs,
        )
        return true
    }

    fun release() {
        if (!active || completed) return
        released = true
        if (!stopRecorderForRelease()) return
        localFinal?.let {
            completeWithTranscript(it)
            return
        }
        transcriber.stop(finalTimeoutMs = config.asr.localResultTimeoutMs)
        main.postDelayed({ handleLocalTimeout(session) }, config.asr.localResultTimeoutMs)
        if (localError != null) startServerFallbackOrError(localError)
    }

    fun releaseRecording(): RecordedVoice? {
        if (!active || completed) return null
        released = true
        runCatching { transcriber.cancel() }
        if (!stopRecorderForRelease()) return null
        val voice = recordedVoice
        recordedVoice = null
        active = false
        released = false
        completed = true
        localError = null
        localFinal = null
        serverFallbackStarted = false
        serverCall?.cancel()
        serverCall = null
        prewarmLocalEngine()
        return voice
    }

    fun cancel(notify: Boolean = true) {
        session += 1
        active = false
        released = false
        completed = true
        localError = null
        localFinal = null
        serverFallbackStarted = false
        serverCall?.cancel()
        serverCall = null
        runCatching { transcriber.cancel() }
        runCatching { recorder.cancel() }
        cleanupRecordedVoice()
        prewarmLocalEngine()
        if (notify) callbacks.onCanceled()
    }

    fun releaseResources() {
        cancel(notify = false)
        transcriber.release()
    }

    private fun listener(sessionId: Int): SystemSpeechTranscriber.Listener =
        object : SystemSpeechTranscriber.Listener {
            override fun onReady() {
                if (isCurrent(sessionId)) callbacks.onReady()
            }

            override fun onVolume(value: Float) {
                if (isCurrent(sessionId)) callbacks.onVolume(value)
            }

            override fun onPartial(transcript: SpeechTranscript) {
                if (isCurrent(sessionId)) callbacks.onPartial(transcript)
            }

            override fun onFinal(transcript: SpeechTranscript) {
                if (!isCurrent(sessionId)) return
                if (!released) {
                    localFinal = transcript
                    return
                }
                completeWithTranscript(transcript)
            }

            override fun onCanceled() = Unit

            override fun onError(error: ChatVoiceError) {
                if (!isCurrent(sessionId)) return
                localError = error
                if (released) startServerFallbackOrError(error)
            }
        }

    private fun stopRecorderForRelease(): Boolean {
        if (!recorderStarted) return true
        val result = recorder.stop()
        recorderStarted = false
        val voice = result.getOrNull()
        if (voice != null) {
            recordedVoice = voice
            return true
        }
        completeAsTooShort()
        return false
    }

    private fun handleLocalTimeout(sessionId: Int) {
        if (!isCurrent(sessionId) || completed || !released) return
        val timeout = ChatVoiceError("system_asr_timeout", "系统语音识别超时")
        runCatching { transcriber.cancel() }
        startServerFallbackOrError(localError ?: timeout)
    }

    private fun startServerFallbackOrError(reason: ChatVoiceError?) {
        if (completed) return
        if (serverFallbackStarted) return
        val voice = recordedVoice
        val asr = config.asr
        val client = serverClient
        if (!asr.serverFallbackEnabled || asr.serverConfig == null || client == null || voice == null) {
            completeWithError(reason ?: ChatVoiceError("system_asr_no_result", config.copy.recognitionFailed))
            return
        }
        serverFallbackStarted = true
        callbacks.onServerFallbackStarted(reason)
        config.eventSink?.onVoiceEvent(
            ChatVoiceEvent.StateChanged(
                ChatVoiceListeningState.PROCESSING,
                config.copy.serverProcessing,
            )
        )
        val sessionId = session
        serverCall = client.transcribe(voice.file, asr.serverOptions) { result ->
            if (!isCurrent(sessionId) || completed) return@transcribe
            result.fold(
                onSuccess = { serverResult ->
                    val transcript = SpeechTranscript(serverResult.text, isFinal = true, SpeechSource.SERVER_ASR)
                    config.eventSink?.onVoiceEvent(ChatVoiceEvent.FinalResult(transcript))
                    completeWithTranscript(transcript)
                },
                onFailure = { error ->
                    val voiceError = if (error is ServerAsrException) {
                        ChatVoiceError(error.code, error.message, error)
                    } else {
                        ChatVoiceError("server_asr_failed", error.message ?: "云端语音识别失败", error)
                    }
                    completeWithError(voiceError)
                },
            )
        }
    }

    private fun completeWithTranscript(transcript: SpeechTranscript) {
        if (completed) return
        active = false
        released = false
        completed = true
        serverCall?.cancel()
        serverCall = null
        cleanupRecordedVoice()
        prewarmLocalEngine()
        callbacks.onFinal(transcript)
    }

    private fun completeWithError(error: ChatVoiceError) {
        if (completed) return
        active = false
        released = false
        completed = true
        serverCall = null
        cleanupRecordedVoice()
        config.eventSink?.onVoiceEvent(ChatVoiceEvent.Error(error))
        prewarmLocalEngine()
        callbacks.onError(error)
    }

    private fun completeAsTooShort() {
        if (completed) return
        active = false
        released = false
        completed = true
        runCatching { transcriber.cancel() }
        cleanupRecordedVoice()
        prewarmLocalEngine()
        config.eventSink?.onVoiceEvent(ChatVoiceEvent.TooShort(config.holdOptions.minRecordDurationMs, config.holdOptions.minVoiceBytes))
        callbacks.onTooShort()
    }

    private fun cleanupRecordedVoice() {
        val file = recordedVoice?.file
        recordedVoice = null
        if (config.asr.deleteRecordedFileAfterResult) file?.delete()
    }

    private fun isCurrent(sessionId: Int): Boolean =
        sessionId == session

    private fun newTranscriber(next: VoiceComposerConfig): SystemSpeechTranscriber =
        SystemSpeechTranscriber(
            appContext,
            next.languageTag,
            next.eventSink,
            engineFallbackEnabled = next.asr.localEngineFallbackEnabled,
            prewarmEnabled = next.asr.prewarmLocalEngine,
        )

    private fun newRecorder(next: VoiceComposerConfig): ChatVoiceRecorder =
        ChatVoiceRecorder(appContext, next.holdOptions, next.eventSink)

    private fun newServerClient(next: VoiceComposerConfig): ServerAsrClient? =
        next.asr.serverConfig?.let { ServerAsrClient(it) }

    private fun prewarmLocalEngine() {
        if (config.asr.prewarmLocalEngine) runCatching { transcriber.prewarm() }
    }
}
