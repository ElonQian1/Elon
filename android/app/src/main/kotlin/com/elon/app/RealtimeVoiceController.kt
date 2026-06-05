package com.elon.app

import android.content.Context
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * 协调 [RealtimePcmRecorder] 和 [RealtimeVoiceWsClient]，
 * 对外暴露最小化的"开始/结束一句话/关闭"API。
 *
 * UI 层使用方式（伪代码）：
 *
 *   val controller = RealtimeVoiceController(
 *       context, baseUrl, userId,
 *       mode = RealtimeVoiceWsClient.Mode.Transcribe,
 *       projectId = currentProjectId,
 *       onTranscript = { finalText -> sendToChatInput(finalText) },
 *       onError = { msg -> toast(msg) },
 *   )
 *   controller.start(scope)        // 按下按钮
 *   controller.commitUtterance()   // 松开按钮 = 一句话结束
 *   controller.shutdown()          // 退出页面
 *
 * 这一层不接 MainActivity，避免膨胀已有的 Main*.kt；
 * MainVoiceModeActions.kt 后续可以在内部 new 一个本类来接入 UI。
 */
internal class RealtimeVoiceController(
    private val context: Context,
    private val baseHttpUrl: String,
    private val userId: String,
    private val mode: RealtimeVoiceWsClient.Mode,
    private val target: String? = null,
    private val projectId: String? = null,
    private val conversationId: String? = null,
    private val continuousAutoCommit: Boolean = false,
    private val onTranscriptDelta: (String) -> Unit = {},
    private val onTranscriptFinal: (String) -> Unit = {},
    private val onVirtualMicFed: (Long) -> Unit = {},
    private val onCliDispatched: (Boolean, String) -> Unit = { _, _ -> },
    private val onAiProgress: (String) -> Unit = {},
    private val onAiDone: (String, String?) -> Unit = { _, _ -> },
    private val onAiError: (String) -> Unit = {},
    private val onRealtimeAudio: (ByteArray) -> Unit = {},
    private val onRealtimeSpeechStarted: () -> Unit = {},
    private val onRealtimeSpeechStopped: () -> Unit = {},
    private val onRealtimeResponseDone: () -> Unit = {},
    private val onError: (String) -> Unit = {},
) {
    private val recorder = RealtimePcmRecorder(
        onChunk = { chunk -> handlePcmChunk(chunk) },
        onError = { msg -> onError(msg) },
    )

    private var ws: RealtimeVoiceWsClient? = null
    private var autoScope: CoroutineScope? = null
    private var autoResumeJob: Job? = null
    private var autoHasSpeech = false
    private var autoSpeechMs = 0
    private var autoSilenceMs = 0
    private var autoTurnMs = 0

    @Volatile
    private var autoPaused = false

    fun start(scope: CoroutineScope) {
        if (ws != null) return
        autoScope = scope
        resetAutoTurn()
        autoPaused = false
        val client = RealtimeVoiceWsClient(
            baseHttpUrl = baseHttpUrl,
            mode = mode,
            userId = userId,
            target = target,
            projectId = projectId,
            conversationId = conversationId,
            listener = object : RealtimeVoiceWsClient.Listener {
                override fun onReady(mode: String) {
                    // WS 握手成功后再启动麦克风
                    if (!recorder.start(scope)) {
                        onError("无法启动麦克风采集")
                    }
                }
                override fun onTranscriptDelta(text: String): Unit =
                    this@RealtimeVoiceController.onTranscriptDelta(text)
                override fun onTranscriptFinal(text: String): Unit =
                    this@RealtimeVoiceController.onTranscriptFinal(text)
                override fun onVirtualMicFed(bytes: Long): Unit =
                    this@RealtimeVoiceController.onVirtualMicFed(bytes)
                override fun onCliDispatched(ok: Boolean, message: String): Unit =
                    this@RealtimeVoiceController.onCliDispatched(ok, message).also {
                        if (!ok) resumeAutoListening()
                    }
                override fun onAiProgress(text: String): Unit =
                    this@RealtimeVoiceController.onAiProgress(text)
                override fun onAiDone(message: String, apkUrl: String?): Unit {
                    cancelAutoResumeFallback()
                    this@RealtimeVoiceController.onAiDone(message, apkUrl)
                }
                override fun onAiError(message: String): Unit =
                    this@RealtimeVoiceController.onAiError(message).also {
                        resumeAutoListening()
                    }
                override fun onRealtimeAudio(chunk: ByteArray): Unit =
                    this@RealtimeVoiceController.onRealtimeAudio(chunk)
                override fun onRealtimeSpeechStarted(): Unit =
                    this@RealtimeVoiceController.onRealtimeSpeechStarted()
                override fun onRealtimeSpeechStopped(): Unit =
                    this@RealtimeVoiceController.onRealtimeSpeechStopped()
                override fun onRealtimeResponseDone(): Unit =
                    this@RealtimeVoiceController.onRealtimeResponseDone()

                override fun onServerError(code: String, message: String) {
                    onError("[$code] $message")
                    resumeAutoListening()
                }

                override fun onClosed() {
                    recorder.stop()
                }
            },
        )
        ws = client
        client.connect()
    }

    /** 一句话结束 → 触发 commit（转写完成 / 静音补尾）。 */
    fun commitUtterance() {
        if (mode == RealtimeVoiceWsClient.Mode.RealtimeChat) {
            ws?.commit()
            return
        }
        if (continuousAutoCommit) {
            commitAutoTurn()
            return
        }
        recorder.stop()
        ws?.commit()
    }

    fun resumeAutoListening() {
        if (!continuousAutoCommit) return
        autoResumeJob?.cancel()
        autoResumeJob = null
        resetAutoTurn()
        autoPaused = false
    }

    fun shutdown() {
        autoResumeJob?.cancel()
        autoResumeJob = null
        autoScope = null
        recorder.stop()
        ws?.close()
        ws = null
    }

    /** 暴露给外部探查的状态。 */
    val isRecording: Boolean get() = recorder.isRecording

    private fun handlePcmChunk(chunk: ByteArray) {
        if (mode == RealtimeVoiceWsClient.Mode.RealtimeChat) {
            ws?.sendPcm(chunk)
            return
        }
        if (!continuousAutoCommit) {
            ws?.sendPcm(chunk)
            return
        }
        handleContinuousPcmChunk(chunk)
    }

    private fun handleContinuousPcmChunk(chunk: ByteArray) {
        if (autoPaused) return
        val isSpeech = pcmRms(chunk) >= AUTO_SPEECH_RMS
        if (!autoHasSpeech && !isSpeech) return

        if (isSpeech) {
            autoHasSpeech = true
            autoSpeechMs += RealtimePcmRecorder.FRAME_MS
            autoSilenceMs = 0
        } else if (autoHasSpeech) {
            autoSilenceMs += RealtimePcmRecorder.FRAME_MS
        }
        autoTurnMs += RealtimePcmRecorder.FRAME_MS
        ws?.sendPcm(chunk)

        val enoughSpeech = autoSpeechMs >= AUTO_MIN_SPEECH_MS
        val silenceEnded = enoughSpeech && autoSilenceMs >= AUTO_END_SILENCE_MS
        val tooLong = enoughSpeech && autoTurnMs >= AUTO_MAX_TURN_MS
        if (silenceEnded || tooLong) commitAutoTurn()
    }

    private fun commitAutoTurn() {
        if (!continuousAutoCommit || autoPaused || !autoHasSpeech) return
        autoPaused = true
        resetAutoTurn()
        ws?.commit()
        scheduleAutoResumeFallback()
    }

    private fun scheduleAutoResumeFallback() {
        cancelAutoResumeFallback()
        autoResumeJob = autoScope?.launch {
            delay(AUTO_RESUME_FALLBACK_MS)
            resumeAutoListening()
        }
    }

    private fun cancelAutoResumeFallback() {
        autoResumeJob?.cancel()
        autoResumeJob = null
    }

    private fun resetAutoTurn() {
        autoHasSpeech = false
        autoSpeechMs = 0
        autoSilenceMs = 0
        autoTurnMs = 0
    }

    private fun pcmRms(chunk: ByteArray): Double {
        if (chunk.size < 2) return 0.0
        var sumSquares = 0.0
        var samples = 0
        var i = 0
        while (i + 1 < chunk.size) {
            val lo = chunk[i].toInt() and 0xff
            val hi = chunk[i + 1].toInt()
            val sample = (hi shl 8) or lo
            val normalized = sample / 32768.0
            sumSquares += normalized * normalized
            samples += 1
            i += 2
        }
        if (samples == 0) return 0.0
        return kotlin.math.sqrt(sumSquares / samples)
    }

    companion object {
        private const val AUTO_SPEECH_RMS = 0.020
        private const val AUTO_MIN_SPEECH_MS = 240
        private const val AUTO_END_SILENCE_MS = 880
        private const val AUTO_MAX_TURN_MS = 12_000
        private const val AUTO_RESUME_FALLBACK_MS = 20_000L
    }
}
