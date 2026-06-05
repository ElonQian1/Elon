// infrastructure/ai/ProjectChatWsClient.kt
// module: infrastructure/ai | layer: infrastructure | role: ws-project-chat
// summary: 通过 /ws/projects/{id} WebSocket 实现全双工项目对话
//   相比 HTTP SSE：连接一次保持长连、服务器可随时推送、无需每轮建连接

package com.elon.app.agent.infrastructure.ai

import android.util.Log
import kotlinx.coroutines.CancellableContinuation
import kotlinx.coroutines.suspendCancellableCoroutine
import okhttp3.*
import org.json.JSONObject
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/**
 * 🔌 项目 WebSocket 对话客户端
 *
 * 连接到 `/ws/projects/{projectId}?token=xxx`，发送用户消息后服务器实时流式推送：
 *   - `progress`        → AI 思考/执行中（步骤提示）
 *   - `assistant_message` → AI 正在说话（中间发言，实时显示）
 *   - `done`            → 最终回复
 *   - `error`           → 出错
 *
 * 与 HTTP/SSE 的区别：
 *   - 连接建立后**复用**，多轮对话不需要每次握手
 *   - 服务器可主动 push（未来可支持"服务器主动打断 AI"等能力）
 *   - 真正全双工：发送消息和接收回复同时进行
 */
class ProjectChatWsClient(
    private val serverUrl: String,
    private val projectId: String,
    private val token: String,
) {
    companion object {
        private const val TAG = "ProjectChatWsClient"
        private const val CONNECT_TIMEOUT_MS = 10_000L
        private const val PING_INTERVAL_SEC = 20L
    }

    interface Listener {
        /** AI 思考/执行中的进度提示 */
        fun onProgress(message: String) {}
        /** AI 中间发言（实时流式展示） */
        fun onAssistantMessage(text: String) {}
        /** 最终回复，调用后本次对话结束 */
        fun onDone(reply: String)
        /** 出错 */
        fun onError(message: String)
    }

    private val http = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS)   // WS 不设读超时
        .connectTimeout(CONNECT_TIMEOUT_MS, TimeUnit.MILLISECONDS)
        .pingInterval(PING_INTERVAL_SEC, TimeUnit.SECONDS)
        .build()

    private var ws: WebSocket? = null
    private val connected = AtomicBoolean(false)

    @Volatile private var currentListener: Listener? = null

    // ─── 公开方法 ───────────────────────────────────────────

    /**
     * 确保 WS 已连接（幂等，重复调用安全）。
     * 如果已连接则直接返回；连接断开时自动触发 [onError]。
     */
    fun ensureConnected() {
        if (connected.get()) return
        val wsUrl = buildWsUrl()
        Log.i(TAG, "→ 连接 $wsUrl")
        val request = Request.Builder().url(wsUrl).build()
        ws = http.newWebSocket(request, wsListener)
    }

    /**
     * 发送用户消息，并在回调中接收 AI 流式回复。
     * 每次调用对应服务器的一轮对话（可在同一 WS 连接上多次调用）。
     *
     * @param message    用户输入
     * @param conversationId 对话 id（null = 让服务器创建新会话）
     * @param listener   本次对话的回调
     */
    fun sendMessage(
        message: String,
        conversationId: String? = null,
        listener: Listener
    ) {
        if (!connected.get()) {
            // 连接还没好，先注册 listener、触发连接，等 onOpen 后自动重发
            currentListener = listener
            ensureConnected()
            // onOpen 后会把待发消息记录并发送
            pendingMessage = message
            pendingConversationId = conversationId
            return
        }
        currentListener = listener
        val payload = JSONObject().apply {
            put("message", message)
            if (conversationId != null) put("conversation_id", conversationId)
        }.toString()
        Log.d(TAG, "→ send: ${message.take(40)}")
        ws?.send(payload)
    }

    /** 主动断开连接（页面销毁时调用） */
    fun disconnect() {
        connected.set(false)
        ws?.close(1000, "正常关闭")
        ws = null
        pendingMessage = null
        pendingConversationId = null
    }

    val isConnected: Boolean get() = connected.get()

    /**
     * suspend 版一轮对话：发消息，等待 done/error，返回最终文本。
     * 全程 WS 全双工，progress/assistant_message 实时回调 [onProgress]。
     *
     * @throws Exception 连接失败 / 服务器返回 error / 被取消
     */
    suspend fun chat(
        message: String,
        conversationId: String? = null,
        onProgress: ((String) -> Unit)? = null,
    ): String = suspendCancellableCoroutine { cont: CancellableContinuation<String> ->
        ensureConnected()
        sendMessage(message, conversationId, object : Listener {
            override fun onProgress(message: String) {
                onProgress?.invoke(message)
            }
            override fun onAssistantMessage(text: String) {
                // 实时流式文字，同样通过 onProgress 通道推给 UI
                onProgress?.invoke(text)
            }
            override fun onDone(reply: String) {
                if (cont.isActive) cont.resume(reply)
            }
            override fun onError(message: String) {
                if (cont.isActive) cont.resumeWithException(Exception(message))
            }
        })
        cont.invokeOnCancellation { /* WS 保持连接，不断开；让调用方决定是否 disconnect */ }
    }

    // ─── 内部状态 ───────────────────────────────────────────

    @Volatile private var pendingMessage: String? = null
    @Volatile private var pendingConversationId: String? = null

    private val wsListener = object : WebSocketListener() {

        override fun onOpen(webSocket: WebSocket, response: Response) {
            connected.set(true)
            Log.i(TAG, "✅ WS 已连接")

            // 如果连接前已有待发消息，现在发出去
            val pending = pendingMessage
            if (pending != null) {
                val payload = JSONObject().apply {
                    put("message", pending)
                    pendingConversationId?.let { put("conversation_id", it) }
                }.toString()
                webSocket.send(payload)
                Log.d(TAG, "→ 重发待发消息: ${pending.take(40)}")
                pendingMessage = null
                pendingConversationId = null
            }
        }

        override fun onMessage(webSocket: WebSocket, text: String) {
            Log.d(TAG, "← ${text.take(120)}")
            val listener = currentListener ?: return
            try {
                val json = JSONObject(text)
                when (val type = json.optString("type")) {
                    "progress" -> listener.onProgress(json.optString("message"))
                    "assistant_message" -> listener.onAssistantMessage(json.optString("text"))
                    "done" -> {
                        currentListener = null
                        listener.onDone(json.optString("message"))
                    }
                    "error" -> {
                        currentListener = null
                        listener.onError(json.optString("message", "服务器返回错误"))
                    }
                    "protocol_hello" -> {
                        // 握手帧，忽略即可
                        Log.d(TAG, "← protocol_hello server_v=${json.optInt("server_version")}")
                    }
                    else -> Log.d(TAG, "← 未知消息类型: $type")
                }
            } catch (e: Exception) {
                Log.w(TAG, "解析消息失败: ${e.message}")
            }
        }

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            connected.set(false)
            ws = null
            Log.e(TAG, "WS 连接失败: ${t.message}")
            val listener = currentListener
            currentListener = null
            listener?.onError("连接断开: ${t.message ?: "网络错误"}")
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            connected.set(false)
            ws = null
            Log.i(TAG, "WS 关闭: code=$code reason=$reason")
            // 如果还有未完成的监听器，通知错误
            val listener = currentListener
            currentListener = null
            if (listener != null && code != 1000) {
                listener.onError("连接关闭: $reason")
            }
        }
    }

    private fun buildWsUrl(): String {
        val base = serverUrl
            .trimEnd('/')
            .replace("http://", "ws://")
            .replace("https://", "wss://")
        return "$base/ws/projects/$projectId?token=${token}"
    }
}
