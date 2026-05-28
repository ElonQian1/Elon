package com.elon.app

import android.content.Context
import kotlinx.coroutines.CoroutineScope

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
    private val projectId: String? = null,
    private val conversationId: String? = null,
    private val onTranscriptDelta: (String) -> Unit = {},
    private val onTranscriptFinal: (String) -> Unit = {},
    private val onVirtualMicFed: (Long) -> Unit = {},
    private val onCliDispatched: (Boolean, String) -> Unit = { _, _ -> },
    private val onError: (String) -> Unit = {},
) {
    private val recorder = RealtimePcmRecorder(
        onChunk = { chunk -> ws?.sendPcm(chunk) },
        onError = { msg -> onError(msg) },
    )

    private var ws: RealtimeVoiceWsClient? = null

    fun start(scope: CoroutineScope) {
        if (ws != null) return
        val client = RealtimeVoiceWsClient(
            baseHttpUrl = baseHttpUrl,
            mode = mode,
            userId = userId,
            projectId = projectId,
            conversationId = conversationId,
            listener = object : RealtimeVoiceWsClient.Listener {
                override fun onReady(mode: String) {
                    // WS 握手成功后再启动麦克风
                    if (!recorder.start(scope)) {
                        onError("无法启动麦克风采集")
                    }
                }
                override fun onTranscriptDelta(text: String): Unit = onTranscriptDelta(text)
                override fun onTranscriptFinal(text: String): Unit = onTranscriptFinal(text)
                override fun onVirtualMicFed(bytes: Long): Unit = onVirtualMicFed(bytes)
                override fun onCliDispatched(ok: Boolean, message: String): Unit =
                    onCliDispatched(ok, message)

                override fun onServerError(code: String, message: String) {
                    onError("[$code] $message")
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
        ws?.commit()
    }

    fun shutdown() {
        recorder.stop()
        ws?.close()
        ws = null
    }

    /** 暴露给外部探查的状态。 */
    val isRecording: Boolean get() = recorder.isRecording
}
