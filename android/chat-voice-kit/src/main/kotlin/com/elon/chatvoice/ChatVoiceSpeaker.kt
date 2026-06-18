package com.elon.chatvoice

import android.content.Context
import android.media.MediaPlayer
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import okhttp3.Call
import okhttp3.Callback
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import org.json.JSONObject
import java.io.File
import java.io.IOException
import java.util.Locale
import java.util.UUID
import java.util.concurrent.TimeUnit

class ChatVoiceSpeaker(
    context: Context,
    private val config: ChatVoiceConfig,
    private val client: OkHttpClient = defaultClient(),
    private val eventSink: ChatVoiceEventSink? = null,
) : TextToSpeech.OnInitListener {
    private val appContext = context.applicationContext
    private val main = Handler(Looper.getMainLooper())
    private var tts: TextToSpeech? = TextToSpeech(appContext, this)
    private var ttsReady = false
    private var mediaPlayer: MediaPlayer? = null
    private var activeCall: Call? = null
    private var tempAudio: File? = null
    private var pending: PendingSpeak? = null
    private var activeSystemDone: (() -> Unit)? = null

    val isSpeaking: Boolean
        get() = mediaPlayer?.isPlaying == true || tts?.isSpeaking == true

    override fun onInit(status: Int) {
        if (status == TextToSpeech.SUCCESS) {
            ttsReady = true
            tts?.apply {
                language = Locale.SIMPLIFIED_CHINESE
                setOnUtteranceProgressListener(object : UtteranceProgressListener() {
                    override fun onStart(utteranceId: String?) = Unit

                    override fun onDone(utteranceId: String?) {
                        main.post { finishSystemSpeak() }
                    }

                    @Deprecated("Deprecated in API 21", ReplaceWith("onError(utteranceId, errorCode)"))
                    override fun onError(utteranceId: String?) {
                        main.post { finishSystemSpeak() }
                    }
                })
            }
            pending?.let {
                pending = null
                speak(it.request, it.onDone, it.onError)
            }
        }
    }

    fun speak(
        request: TtsRequest,
        onDone: () -> Unit = {},
        onError: (ChatVoiceError) -> Unit = {},
    ) {
        val text = request.text.trim().take(MAX_TTS_CHARS)
        if (text.isEmpty()) {
            onDone()
            return
        }
        if (!ttsReady) {
            pending = PendingSpeak(request.copy(text = text), onDone, onError)
            return
        }
        stop()
        val voiceId = request.voiceId?.trim().orEmpty().ifBlank { config.selectedTtsVoiceProvider().orEmpty() }
        if (!config.preferServerTts || voiceId == ChatVoiceIds.ANDROID_SYSTEM_TTS) {
            speakWithSystem(text, onDone, onError)
            return
        }
        speakWithServer(request.copy(text = text, voiceId = voiceId), onDone) { error ->
            if (config.fallbackToSystemTts) {
                speakWithSystem(text, onDone, onError)
            } else {
                eventSink?.onVoiceEvent(ChatVoiceEvent.Error(error))
                onError(error)
            }
        }
    }

    fun stop() {
        activeCall?.cancel()
        activeCall = null
        activeSystemDone = null
        mediaPlayer?.stopSafely()
        mediaPlayer = null
        tts?.stop()
        cleanupTempAudio()
    }

    fun release() {
        stop()
        tts?.shutdown()
        tts = null
        ttsReady = false
    }

    private fun speakWithServer(
        request: TtsRequest,
        onDone: () -> Unit,
        onError: (ChatVoiceError) -> Unit,
    ) {
        val token = config.bearerTokenProvider()?.trim().orEmpty()
        val bodyJson = JSONObject()
            .put("text", request.text)
            .put("voiceId", request.voiceId?.takeIf { it.isNotBlank() })
            .put("emotionId", request.emotionId)
            .put("intensity", request.intensity)
            .put("agentName", request.agentName)
            .toString()
        val httpRequest = Request.Builder()
            .url("${config.normalizedBaseUrl}/api/voice/tts")
            .apply {
                if (token.isNotEmpty()) header("Authorization", "Bearer $token")
            }
            .post(bodyJson.toRequestBody("application/json; charset=utf-8".toMediaType()))
            .build()
        activeCall = client.newCall(httpRequest).also { call ->
            call.enqueue(object : Callback {
                override fun onFailure(call: Call, e: IOException) {
                    if (!call.isCanceled()) main.post {
                        val error = ChatVoiceError("server_tts_network", "服务器 TTS 网络失败", e)
                        onError(error)
                    }
                }

                override fun onResponse(call: Call, response: Response) {
                    response.use {
                        val body = it.body
                        if (!it.isSuccessful || body == null) {
                            val message = body?.string().orEmpty()
                            main.post {
                                val error = ChatVoiceError("server_tts_${it.code}", message.ifBlank { "服务器 TTS 失败" })
                                onError(error)
                            }
                            return
                        }
                        val file = File.createTempFile("elon_tts_", ".audio", appContext.cacheDir)
                        file.writeBytes(body.bytes())
                        main.post { playServerAudio(file, onDone, onError) }
                    }
                }
            })
        }
    }

    private fun playServerAudio(
        file: File,
        onDone: () -> Unit,
        onError: (ChatVoiceError) -> Unit,
    ) {
        cleanupTempAudio()
        tempAudio = file
        runCatching {
            mediaPlayer = MediaPlayer().apply {
                setDataSource(file.absolutePath)
                setOnCompletionListener {
                    eventSink?.onVoiceEvent(ChatVoiceEvent.TtsEnd)
                    stop()
                    onDone()
                }
                setOnErrorListener { _, _, _ ->
                    stop()
                    val error = ChatVoiceError("server_tts_playback", "服务器 TTS 音频播放失败")
                    onError(error)
                    true
                }
                setOnPreparedListener {
                    eventSink?.onVoiceEvent(ChatVoiceEvent.TtsStart)
                    it.start()
                }
                prepareAsync()
            }
        }.onFailure {
            stop()
            val error = ChatVoiceError("server_tts_playback", "服务器 TTS 音频初始化失败", it)
            onError(error)
        }
    }

    private fun speakWithSystem(
        text: String,
        onDone: () -> Unit,
        onError: (ChatVoiceError) -> Unit,
    ) {
        val engine = tts
        if (engine == null || !ttsReady) {
            onError(ChatVoiceError("system_tts_unavailable", "手机系统 TTS 不可用"))
            return
        }
        engine.language = Locale.SIMPLIFIED_CHINESE
        val params = Bundle()
        activeSystemDone = onDone
        val result = engine.speak(text, TextToSpeech.QUEUE_FLUSH, params, UUID.randomUUID().toString())
        if (result == TextToSpeech.ERROR) {
            activeSystemDone = null
            val error = ChatVoiceError("system_tts_error", "手机系统 TTS 播放失败")
            eventSink?.onVoiceEvent(ChatVoiceEvent.Error(error))
            onError(error)
        } else {
            eventSink?.onVoiceEvent(ChatVoiceEvent.TtsStart)
        }
    }

    private fun finishSystemSpeak() {
        val done = activeSystemDone
        activeSystemDone = null
        eventSink?.onVoiceEvent(ChatVoiceEvent.TtsEnd)
        done?.invoke()
    }

    private fun cleanupTempAudio() {
        tempAudio?.delete()
        tempAudio = null
    }

    private fun MediaPlayer.stopSafely() {
        runCatching { stop() }
        runCatching { release() }
    }

    private data class PendingSpeak(
        val request: TtsRequest,
        val onDone: () -> Unit,
        val onError: (ChatVoiceError) -> Unit,
    )

    companion object {
        const val MAX_TTS_CHARS = 200

        fun defaultClient(): OkHttpClient =
            OkHttpClient.Builder()
                .connectTimeout(10, TimeUnit.SECONDS)
                .readTimeout(60, TimeUnit.SECONDS)
                .writeTimeout(60, TimeUnit.SECONDS)
                .build()
    }
}
