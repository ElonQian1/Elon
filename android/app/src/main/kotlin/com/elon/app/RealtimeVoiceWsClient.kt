package com.elon.app

import android.util.Log
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString.Companion.toByteString
import org.json.JSONObject

/**
 * 实时语音 WebSocket 客户端，对应服务端 voice_protocol.rs。
 *
 * 协议：
 *  - 二进制帧 = 裸 PCM16 LE
 *  - 文本帧 = JSON 控制消息 / 转写事件
 *  - 首条消息必须发送 hello（携带 user_id / sample_rate / channels）
 *
 * 模式：
 *  - [Mode.VirtualMic]    → /ws/voice/virtual-mic（方案 A：投喂虚拟麦克风）
 *  - [Mode.Transcribe]    → /ws/voice/transcribe（方案 B：转写为文本）
 */
internal class RealtimeVoiceWsClient(
    private val baseHttpUrl: String,
    private val mode: Mode,
    private val userId: String,
    private val target: String? = null,
    private val projectId: String? = null,
    private val conversationId: String? = null,
    private val listener: Listener,
) {
    enum class Mode(val path: String, val label: String) {
        VirtualMic("/ws/voice/virtual-mic", "virtual_mic"),
        Transcribe("/ws/voice/transcribe", "transcribe"),
    }

    object Target {
        const val SocialAiDirect = "social_ai_direct"
    }

    interface Listener {
        fun onReady(mode: String) {}
        fun onTranscriptDelta(text: String) {}
        fun onTranscriptFinal(text: String) {}
        fun onVirtualMicFed(bytes: Long) {}
        fun onCliDispatched(ok: Boolean, message: String) {}
        /** AI 任务执行中的进度消息（对应 WsMessage.Progress）。 */
        fun onAiProgress(text: String) {}
        /** AI 任务完成（对应 WsMessage.Done）。 */
        fun onAiDone(message: String, apkUrl: String?) {}
        /** AI 任务出错（对应 WsMessage.Error）。 */
        fun onAiError(message: String) {}
        fun onServerError(code: String, message: String) {}
        fun onClosed() {}
    }

    private val client: OkHttpClient = OkHttpClient.Builder()
        .pingInterval(java.time.Duration.ofSeconds(20))
        .build()

    @Volatile
    private var ws: WebSocket? = null

    @Volatile
    var isOpen: Boolean = false
        private set

    fun connect() {
        val wsUrl = baseHttpUrl
            .replaceFirst(Regex("^http://"), "ws://")
            .replaceFirst(Regex("^https://"), "wss://")
            .trimEnd('/') + mode.path
        val req = Request.Builder().url(wsUrl).build()
        ws = client.newWebSocket(req, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                isOpen = true
                val hello = JSONObject().apply {
                    put("type", "hello")
                    put("user_id", userId)
                    if (!target.isNullOrBlank()) put("target", target)
                    if (!projectId.isNullOrBlank()) put("project_id", projectId)
                    if (!conversationId.isNullOrBlank()) put("conversation_id", conversationId)
                    put("sample_rate", RealtimePcmRecorder.SAMPLE_RATE_HZ)
                    put("channels", 1)
                }
                webSocket.send(hello.toString())
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                handleServerText(text)
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                isOpen = false
                webSocket.close(1000, null)
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                isOpen = false
                shutdownOkHttp()
                listener.onClosed()
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                isOpen = false
                shutdownOkHttp()
                Log.w("RealtimeVoiceWs", "ws failure", t)
                val detail = t.message
                    ?: t.javaClass.simpleName.takeIf { it.isNotBlank() }
                    ?: "连接异常"
                listener.onServerError("ws_failure", detail)
                listener.onClosed()
            }
        })
    }

    fun sendPcm(chunk: ByteArray) {
        ws?.takeIf { isOpen }?.send(chunk.toByteString(0, chunk.size))
    }

    fun commit() {
        ws?.takeIf { isOpen }?.send("""{"type":"commit"}""")
    }

    fun close() {
        val current = ws
        ws = null
        isOpen = false
        runCatching { current?.send("""{"type":"close"}""") }
        runCatching { current?.close(1000, "client close") }
        shutdownOkHttp()
    }

    private fun shutdownOkHttp() {
        runCatching { client.dispatcher.executorService.shutdown() }
        runCatching { client.connectionPool.evictAll() }
    }

    private fun handleServerText(text: String) {
        val obj = runCatching { JSONObject(text) }.getOrNull() ?: return
        when (obj.optString("type")) {
            "ready" -> listener.onReady(obj.optString("mode"))
            "transcript_delta" -> listener.onTranscriptDelta(obj.optString("text"))
            "transcript_final" -> listener.onTranscriptFinal(obj.optString("text"))
            "virtual_mic_fed" -> listener.onVirtualMicFed(obj.optLong("bytes"))
            "cli_dispatched" -> listener.onCliDispatched(
                obj.optBoolean("ok"),
                obj.optString("message"),
            )
            // WsMessage JSON 透传：AI 任务进度/完成/错误
            "progress" -> listener.onAiProgress(obj.optString("message"))
            "done" -> listener.onAiDone(
                obj.optString("message"),
                obj.optString("apk_url").takeIf { it.isNotBlank() },
            )
            "error" -> {
                // "error" 既可能来自 ServerEvent（code+message），也可能来自 WsMessage（message）
                val code = obj.optString("code")
                val msg = obj.optString("message")
                if (code.isNotBlank()) {
                    listener.onServerError(code, msg)
                } else {
                    listener.onAiError(msg)
                }
            }
        }
    }
}
