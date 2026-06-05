package com.elon.app

import android.content.Context
import android.media.AudioAttributes
import android.media.MediaPlayer
import android.os.Handler
import android.os.Looper
import android.util.Log
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
import java.util.concurrent.TimeUnit

/** 优先播放服务器情绪女声 TTS，失败时由调用方回退到 Android 系统 TTS。 */
internal class VoiceServerTtsPlayer(context: Context) {
    companion object {
        private const val TAG = "VoiceServerTts"
        private const val COOLDOWN_MS = 30 * 1000L

        @Volatile
        private var unavailableUntilMs: Long = 0L
    }

    private val appContext = context.applicationContext
    private val mainHandler = Handler(Looper.getMainLooper())
    private val http = OkHttpClient.Builder()
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(90, TimeUnit.SECONDS)
        .writeTimeout(15, TimeUnit.SECONDS)
        .build()

    private var activeCall: Call? = null
    private var player: MediaPlayer? = null
    private var activeFile: File? = null
    private var generation: Int = 0

    val isSpeaking: Boolean
        get() = player?.isPlaying == true || activeCall != null

    fun trySpeak(
        text: String,
        profile: VoiceTtsProfile,
        onDone: () -> Unit,
        onFallback: () -> Unit
    ): Boolean {
        val now = System.currentTimeMillis()
        if (now < unavailableUntilMs) {
            Log.w(TAG, "server TTS still in cooldown: remainingMs=${unavailableUntilMs - now}")
            return false
        }
        val request = buildRequest(text, profile) ?: run {
            Log.w(TAG, "server TTS request build failed")
            return false
        }
        Log.i(
            TAG,
            "request server TTS voice=${profile.serverVoiceId} emotion=${profile.serverEmotionId} intensity=${profile.serverIntensity}"
        )
        val call = http.newCall(request)
        val requestGeneration = begin(call)
        call.enqueue(object : Callback {
            override fun onFailure(call: Call, e: IOException) {
                if (!isCurrent(requestGeneration)) return
                markUnavailable("request failed: ${e.message}")
                clearCall(requestGeneration)
                mainHandler.post {
                    if (isCurrent(requestGeneration)) onFallback()
                }
            }

            override fun onResponse(call: Call, response: Response) {
                response.use { resp ->
                    if (!isCurrent(requestGeneration)) return
                    if (!resp.isSuccessful) {
                        markUnavailable("server returned ${resp.code}")
                        clearCall(requestGeneration)
                        mainHandler.post {
                            if (isCurrent(requestGeneration)) onFallback()
                        }
                        return
                    }
                    val bytes = resp.body?.bytes()
                    if (bytes == null || bytes.isEmpty()) {
                        markUnavailable("empty audio")
                        clearCall(requestGeneration)
                        mainHandler.post {
                            if (isCurrent(requestGeneration)) onFallback()
                        }
                        return
                    }
                    val contentType = resp.header("Content-Type").orEmpty()
                    Log.i(TAG, "server TTS audio ready contentType=$contentType bytes=${bytes.size}")
                    val file = writeTempAudio(bytes, contentType)
                    clearCall(requestGeneration)
                    mainHandler.post {
                        if (!isCurrent(requestGeneration)) {
                            file.delete()
                            return@post
                        }
                        playFile(requestGeneration, file, onDone, onFallback)
                    }
                }
            }
        })
        return true
    }

    @Synchronized
    fun stop() {
        generation += 1
        activeCall?.cancel()
        activeCall = null
        releasePlayerLocked()
    }

    fun release() {
        stop()
        runCatching { http.dispatcher.executorService.shutdown() }
        runCatching { http.connectionPool.evictAll() }
    }

    private fun buildRequest(text: String, profile: VoiceTtsProfile): Request? {
        val url = ServerUrlManager.getActive(appContext).trimEnd('/') + "/api/voice/tts"
        val json = JSONObject().apply {
            put("text", text)
            put("voiceId", profile.serverVoiceId)
            put("emotionId", profile.serverEmotionId)
            put("intensity", profile.serverIntensity)
            put("rewrite", true)
        }
        val body = json.toString().toRequestBody("application/json".toMediaType())
        val builder = Request.Builder().url(url).post(body)
        return AuthManager.applyAuth(appContext, builder).build()
    }

    @Synchronized
    private fun begin(call: Call): Int {
        generation += 1
        activeCall?.cancel()
        releasePlayerLocked()
        activeCall = call
        return generation
    }

    @Synchronized
    private fun clearCall(requestGeneration: Int) {
        if (generation == requestGeneration) activeCall = null
    }

    @Synchronized
    private fun isCurrent(requestGeneration: Int): Boolean = generation == requestGeneration

    private fun writeTempAudio(bytes: ByteArray, contentType: String): File {
        val dir = File(appContext.cacheDir, "server_tts").apply { mkdirs() }
        val file = File.createTempFile("tts_", ".${extensionFor(contentType)}", dir)
        file.writeBytes(bytes)
        return file
    }

    private fun playFile(
        requestGeneration: Int,
        file: File,
        onDone: () -> Unit,
        onFallback: () -> Unit
    ) {
        val mediaPlayer = MediaPlayer()
        synchronized(this) {
            if (generation != requestGeneration) {
                file.delete()
                mediaPlayer.release()
                return
            }
            releasePlayerLocked()
            player = mediaPlayer
            activeFile = file
        }
        mediaPlayer.setAudioAttributes(
            AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_ASSISTANCE_ACCESSIBILITY)
                .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                .build()
        )
        mediaPlayer.setOnPreparedListener {
            if (isCurrent(requestGeneration)) {
                it.start()
            } else {
                runCatching { it.release() }
                runCatching { file.delete() }
            }
        }
        mediaPlayer.setOnCompletionListener {
            if (!isCurrent(requestGeneration)) return@setOnCompletionListener
            cleanupPlayer(requestGeneration)
            onDone()
        }
        mediaPlayer.setOnErrorListener { _, what, extra ->
            if (!isCurrent(requestGeneration)) return@setOnErrorListener true
            Log.w(TAG, "MediaPlayer error what=$what extra=$extra")
            cleanupPlayer(requestGeneration)
            onFallback()
            true
        }
        runCatching {
            mediaPlayer.setDataSource(file.absolutePath)
            mediaPlayer.prepareAsync()
        }.onFailure { error ->
            Log.w(TAG, "prepare server TTS audio failed", error)
            if (isCurrent(requestGeneration)) {
                cleanupPlayer(requestGeneration)
                onFallback()
            }
        }
    }

    @Synchronized
    private fun cleanupPlayer(requestGeneration: Int) {
        if (generation != requestGeneration) return
        releasePlayerLocked()
    }

    private fun releasePlayerLocked() {
        val oldPlayer = player
        val oldFile = activeFile
        player = null
        activeFile = null
        runCatching { oldPlayer?.stop() }
        runCatching { oldPlayer?.release() }
        runCatching { oldFile?.delete() }
    }

    private fun markUnavailable(reason: String) {
        unavailableUntilMs = System.currentTimeMillis() + COOLDOWN_MS
        Log.w(TAG, "server TTS unavailable for cooldown: $reason")
    }

    private fun extensionFor(contentType: String): String = when {
        contentType.contains("mpeg", ignoreCase = true) || contentType.contains("mp3", ignoreCase = true) -> "mp3"
        contentType.contains("ogg", ignoreCase = true) -> "ogg"
        contentType.contains("mp4", ignoreCase = true) || contentType.contains("m4a", ignoreCase = true) -> "m4a"
        else -> "wav"
    }
}
