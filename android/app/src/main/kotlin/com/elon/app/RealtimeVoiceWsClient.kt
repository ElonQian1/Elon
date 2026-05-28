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
    private val projectId: String? = null,
    private val conversationId: String? = null,
    private val listener: Listener,
) {
    enum class Mode(val path: String, val label: String) {
        VirtualMic("/ws/voice/virtual-mic", "virtual_mic"),
        Transcribe("/ws/voice/transcribe", "transcribe"),
    }

    interface Listener {
        fun onReady(mode: String) {}
        fun onTranscriptDelta(text: String) {}
        fun onTranscriptFinal(text: String) {}
        fun onVirtualMicFed(bytes: Long) {}
        fun onCliDispatched(ok: Boolean, message: String) {}
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
                listener.onClosed()
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                isOpen = false
                Log.w("RealtimeVoiceWs", "ws failure", t)
                listener.onServerError("ws_failure", t.message ?: "unknown")
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
            "error" -> listener.onServerError(
                obj.optString("code"),
                obj.optString("message"),
            )
        }
    }
}
