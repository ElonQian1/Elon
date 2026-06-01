// infrastructure/voice/CloudAsrFallback.kt
// module: infrastructure/voice | layer: infrastructure | role: cloud-asr-fallback
// summary: 云端 ASR 兜底 — 本地引擎全部失败时，用 MediaRecorder 录 M4A 上传到服务器 Whisper 转写

package com.elon.app.agent.infrastructure.voice

import android.content.Context
import android.media.MediaRecorder
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import com.elon.app.AsrFallbackSettings
import kotlinx.coroutines.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.asRequestBody
import org.json.JSONObject
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * 云端 ASR 兜底
 *
 * 用 MediaRecorder 录制 AAC/M4A，上传到 `/api/voice/asr` 获取转写文字。
 * 只做语音识别（不触发 AI），结果通过回调返回给调用方。
 *
 * 生命周期：
 *   start(onResult) → [录音中，最长 MAX_RECORD_MS] → stopAndUpload() → onResult(text)
 *   cancel() 可随时取消，不会触发 onResult
 */
class CloudAsrFallback(
    private val context: Context,
    private val serverUrl: String = "http://43.139.149.158:8080"
) {
    companion object {
        private const val TAG = "CloudAsrFallback"
        private const val MAX_RECORD_MS = 8_000L
    }

    private var recorder: MediaRecorder? = null
    private var outputFile: File? = null
    private var pendingResult: ((String?) -> Unit)? = null

    private val mainHandler = Handler(Looper.getMainLooper())
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private val httpClient = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()

    private val timeoutRunnable = Runnable {
        Log.i(TAG, "录音达到 ${MAX_RECORD_MS}ms 上限，自动停止")
        stopAndUpload()
    }

    var isRecording: Boolean = false
        private set

    /** 录音开始时（主线程）回调，可用于更新 UI */
    var onRecordStart: () -> Unit = {}

    // ==================== 公开方法 ====================

    /**
     * 开始录音。
     * @param onResult 主线程回调；null 表示失败或被取消
     */
    fun start(onResult: (String?) -> Unit) {
        if (isRecording) {
            Log.w(TAG, "已在录音中，忽略重复 start()")
            return
        }
        pendingResult = onResult
        val file = File(context.cacheDir, "cloud_asr_${System.currentTimeMillis()}.m4a")
        outputFile = file

        try {
            recorder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                MediaRecorder(context)
            } else {
                @Suppress("DEPRECATION")
                MediaRecorder()
            }
            recorder!!.apply {
                setAudioSource(MediaRecorder.AudioSource.MIC)
                setOutputFormat(MediaRecorder.OutputFormat.MPEG_4)
                setAudioEncoder(MediaRecorder.AudioEncoder.AAC)
                setAudioSamplingRate(16000)
                setAudioChannels(1)
                setAudioEncodingBitRate(64000)
                setOutputFile(file.absolutePath)
                prepare()
                start()
            }
            isRecording = true
            mainHandler.post { onRecordStart() }
            mainHandler.postDelayed(timeoutRunnable, MAX_RECORD_MS)
            Log.i(TAG, "☁️ 云端 ASR 录音开始：${file.name}")
        } catch (e: Exception) {
            Log.e(TAG, "录音启动失败", e)
            cleanup()
            mainHandler.post { onResult(null) }
        }
    }

    /** 手动停止录音并上传（用户触发停止时调用）。 */
    fun stop() {
        if (!isRecording) return
        mainHandler.removeCallbacks(timeoutRunnable)
        stopAndUpload()
    }

    /** 取消录音，不上传，不触发 onResult。 */
    fun cancel() {
        mainHandler.removeCallbacks(timeoutRunnable)
        pendingResult = null
        cleanup()
    }

    /** 释放所有资源（Activity/Fragment onDestroy 时调用）。 */
    fun destroy() {
        cancel()
        scope.cancel()
    }

    // ==================== 私有方法 ====================

    private fun stopAndUpload() {
        val file = outputFile ?: return
        val callback = pendingResult ?: return
        pendingResult = null
        isRecording = false

        try {
            recorder?.stop()
        } catch (e: Exception) {
            // 录音时间极短时 stop() 可能抛异常
            Log.w(TAG, "recorder.stop() 失败（录音太短？）: ${e.message}")
            cleanup()
            mainHandler.post { callback(null) }
            return
        }
        recorder?.release()
        recorder = null

        Log.i(TAG, "☁️ 录音完成 ${file.length()} bytes，上传中...")
        scope.launch {
            val text = runCatching { uploadToServer(file) }.getOrElse {
                Log.e(TAG, "上传/转写失败", it)
                null
            }
            file.delete()
            mainHandler.post { callback(text) }
        }
    }

    private fun cleanup() {
        isRecording = false
        runCatching { recorder?.stop() }
        runCatching { recorder?.release() }
        recorder = null
        outputFile?.delete()
        outputFile = null
    }

    private fun uploadToServer(file: File): String {
        val token = context.getSharedPreferences("auth", Context.MODE_PRIVATE)
            .getString("auth_token", null)
            ?: throw IllegalStateException("未登录，无法使用云端 ASR")

        val requestBody = MultipartBody.Builder()
            .setType(MultipartBody.FORM)
            .addFormDataPart(
                "audio", file.name,
                file.asRequestBody("audio/m4a".toMediaType())
            )
            .addFormDataPart("language", AsrFallbackSettings.getWhisperLanguage(context))
            .addFormDataPart("beam_size", AsrFallbackSettings.getWhisperBeamSize(context).toString())
            .addFormDataPart("vad_filter", AsrFallbackSettings.getWhisperVadFilter(context).toString())
            .build()

        val request = Request.Builder()
            .url("$serverUrl/api/voice/asr")
            .post(requestBody)
            .addHeader("Authorization", "Bearer $token")
            .build()

        httpClient.newCall(request).execute().use { response ->
            val body = response.body?.string() ?: ""
            if (!response.isSuccessful) {
                throw RuntimeException("服务器返回 ${response.code}：$body")
            }
            val transcript = JSONObject(body).optString("transcript")
            if (transcript.isBlank()) throw RuntimeException("转写结果为空")
            Log.i(TAG, "☁️ 转写完成: $transcript")
            return transcript
        }
    }
}
