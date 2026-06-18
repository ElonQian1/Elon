package com.elon.chatvoice

import android.Manifest
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

    fun isAvailable(): Boolean = SpeechRecognizer.isRecognitionAvailable(appContext)

    fun start(listener: Listener, preferOffline: Boolean = false) {
        main.post {
            if (!isAvailable()) {
                listener.onError(ChatVoiceError("system_asr_unavailable", "手机系统语音识别不可用"))
                return@post
            }
            if (appContext.checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
                listener.onError(ChatVoiceError("record_audio_denied", "缺少麦克风权限"))
                return@post
            }
            stopRecognizer(cancelOnly = true)
            activeListener = listener
            eventSink?.onVoiceEvent(
                ChatVoiceEvent.StateChanged(
                    ChatVoiceListeningState.PREPARING,
                    ChatVoiceInteractionContract.stateText(ChatVoiceListeningState.PREPARING),
                )
            )
            recognizer = SpeechRecognizer.createSpeechRecognizer(appContext).apply {
                setRecognitionListener(createRecognitionListener())
                startListening(recognizerIntent(preferOffline))
            }
        }
    }

    fun stop() {
        main.post { recognizer?.stopListening() }
    }

    fun cancel() {
        main.post {
            val listener = activeListener
            stopRecognizer(cancelOnly = true)
            activeListener = null
            eventSink?.onVoiceEvent(ChatVoiceEvent.Cancel)
            listener?.onCanceled()
        }
    }

    fun release() {
        main.post {
            stopRecognizer(cancelOnly = true)
            activeListener = null
        }
    }

    private fun stopRecognizer(cancelOnly: Boolean) {
        val current = recognizer ?: return
        if (cancelOnly) runCatching { current.cancel() } else runCatching { current.stopListening() }
        runCatching { current.destroy() }
        recognizer = null
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

    private fun createRecognitionListener(): RecognitionListener =
        object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) {
                eventSink?.onVoiceEvent(
                    ChatVoiceEvent.StateChanged(
                        ChatVoiceListeningState.LISTENING,
                        ChatVoiceInteractionContract.stateText(ChatVoiceListeningState.LISTENING),
                    )
                )
                activeListener?.onReady()
            }

            override fun onRmsChanged(rmsdB: Float) {
                val normalized = ((rmsdB + 2f) / 12f).coerceIn(0f, 1f)
                activeListener?.onVolume(normalized)
                eventSink?.onVoiceEvent(ChatVoiceEvent.Volume(normalized))
            }

            override fun onPartialResults(partialResults: Bundle?) {
                val text = firstResult(partialResults) ?: return
                val transcript = SpeechTranscript(text, isFinal = false, SpeechSource.SYSTEM_ASR)
                activeListener?.onPartial(transcript)
                eventSink?.onVoiceEvent(ChatVoiceEvent.PartialResult(transcript))
            }

            override fun onResults(results: Bundle?) {
                val text = firstResult(results).orEmpty()
                val listener = activeListener
                stopRecognizer(cancelOnly = false)
                activeListener = null
                if (text.isBlank()) {
                    val error = ChatVoiceError("system_asr_no_match", "没有听清")
                    eventSink?.onVoiceEvent(ChatVoiceEvent.Error(error))
                    listener?.onError(error)
                } else {
                    val transcript = SpeechTranscript(text.trim(), isFinal = true, SpeechSource.SYSTEM_ASR)
                    eventSink?.onVoiceEvent(ChatVoiceEvent.FinalResult(transcript))
                    listener?.onFinal(transcript)
                }
            }

            override fun onError(error: Int) {
                val listener = activeListener
                stopRecognizer(cancelOnly = true)
                activeListener = null
                val voiceError = ChatVoiceError("system_asr_$error", speechErrorMessage(error))
                eventSink?.onVoiceEvent(ChatVoiceEvent.Error(voiceError))
                listener?.onError(voiceError)
            }

            override fun onBeginningOfSpeech() = Unit
            override fun onEndOfSpeech() = Unit
            override fun onBufferReceived(buffer: ByteArray?) = Unit
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
            else -> String.format(Locale.ROOT, "系统语音识别失败(%d)", error)
        }
}
