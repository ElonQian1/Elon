package com.elon.app

import android.util.Log
import okhttp3.*
import org.json.JSONObject
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

class ElonWsClient(
    private val serverUrl: String,
    private val debugTraceId: String? = null,
    private val debugKind: String? = null,
    private val onMessage: (String) -> Unit,
    private val onConnected: () -> Unit,
    private val onDisconnected: () -> Unit,
    private val onAuthRequired: () -> Unit = {},
) {
    private val TAG = "ElonWsClient"
    private val client = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS)  // WebSocket 不设超时
        .pingInterval(20, TimeUnit.SECONDS)
        .build()
    private var ws: WebSocket? = null
    private val connected = AtomicBoolean(false)

    fun connect() {
        if (connected.get()) return
        ws?.cancel()

        DebugTraceStore.record(
            "ws_connect_start",
            mapOf(
                "trace_id" to debugTraceId,
                "kind" to debugKind,
                "url" to serverUrl
            )
        )
        val request = Request.Builder().url(serverUrl).build()
        ws = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                connected.set(true)
                Log.i(TAG, "WebSocket 已连接")
                DebugTraceStore.record(
                    "ws_connected",
                    mapOf(
                        "trace_id" to debugTraceId,
                        "kind" to debugKind,
                        "url" to serverUrl,
                        "http_code" to response.code
                    )
                )
                onConnected()
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                Log.d(TAG, "收到消息: $text")
                DebugTraceStore.record(
                    "ws_message",
                    mapOf(
                        "trace_id" to debugTraceId,
                        "kind" to debugKind,
                        "type" to messageType(text),
                        "bytes" to text.toByteArray().size
                    )
                )
                onMessage(text)
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                connected.set(false)
                if (ws == webSocket) ws = null
                Log.e(TAG, "连接失败: ${t.message}")
                val httpCode = response?.code
                DebugTraceStore.record(
                    "ws_failure",
                    mapOf(
                        "trace_id" to debugTraceId,
                        "kind" to debugKind,
                        "url" to serverUrl,
                        "error" to t.message,
                        "http_code" to httpCode
                    )
                )
                if (httpCode == 401) {
                    onAuthRequired()
                } else {
                    onDisconnected()
                }
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                connected.set(false)
                if (ws == webSocket) ws = null
                Log.i(TAG, "连接关闭: $reason")
                DebugTraceStore.record(
                    "ws_closed",
                    mapOf(
                        "trace_id" to debugTraceId,
                        "kind" to debugKind,
                        "url" to serverUrl,
                        "code" to code,
                        "reason" to reason
                    )
                )
                onDisconnected()
            }
        })
    }

    fun isConnected(): Boolean = connected.get()

    fun send(message: String): Boolean {
        val socket = ws
        if (!connected.get() || socket == null) {
            Log.w(TAG, "WebSocket 未连接，无法发送")
            return false
        }

        val sent = socket.send(message)
        DebugTraceStore.record(
            if (sent) "ws_send" else "ws_send_failed",
            mapOf("trace_id" to traceIdFromPayload(message), "bytes" to message.toByteArray().size)
        )
        if (!sent) {
            connected.set(false)
        }
        return sent
    }

    fun disconnect() {
        connected.set(false)
        ws?.close(1000, "用户关闭")
        ws = null
        shutdownOkHttp()
    }

    private fun shutdownOkHttp() {
        runCatching { client.dispatcher.executorService.shutdown() }
        runCatching { client.connectionPool.evictAll() }
    }

    private fun traceIdFromPayload(message: String): String? {
        return runCatching { JSONObject(message).optString("trace_id") }
            .getOrNull()
            ?.takeIf { it.isNotBlank() }
    }

    private fun messageType(message: String): String? {
        return runCatching { JSONObject(message).optString("type") }
            .getOrNull()
            ?.takeIf { it.isNotBlank() }
    }
}
