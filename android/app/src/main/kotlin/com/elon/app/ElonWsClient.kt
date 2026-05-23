package com.elon.app

import android.util.Log
import okhttp3.*
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

class ElonWsClient(
    private val serverUrl: String,
    private val onMessage: (String) -> Unit,
    private val onConnected: () -> Unit,
    private val onDisconnected: () -> Unit,
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

        val request = Request.Builder().url(serverUrl).build()
        ws = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                connected.set(true)
                Log.i(TAG, "WebSocket 已连接")
                onConnected()
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                Log.d(TAG, "收到消息: $text")
                onMessage(text)
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                connected.set(false)
                if (ws == webSocket) ws = null
                Log.e(TAG, "连接失败: ${t.message}")
                onDisconnected()
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                connected.set(false)
                if (ws == webSocket) ws = null
                Log.i(TAG, "连接关闭: $reason")
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
        if (!sent) {
            connected.set(false)
        }
        return sent
    }

    fun disconnect() {
        connected.set(false)
        ws?.close(1000, "用户关闭")
        ws = null
    }
}
