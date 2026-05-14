package com.elon.app

import android.util.Log
import okhttp3.*
import java.util.concurrent.TimeUnit

class ElonWsClient(
    private val serverUrl: String,
    private val onMessage: (String) -> Unit,
    private val onConnected: () -> Unit,
    private val onDisconnected: () -> Unit,
) {
    private val TAG = "ElonWsClient"
    private val client = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS)  // WebSocket 不设超时
        .build()
    private var ws: WebSocket? = null

    fun connect() {
        val request = Request.Builder().url(serverUrl).build()
        ws = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                Log.i(TAG, "WebSocket 已连接")
                onConnected()
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                Log.d(TAG, "收到消息: $text")
                onMessage(text)
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                Log.e(TAG, "连接失败: ${t.message}")
                onDisconnected()
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                Log.i(TAG, "连接关闭: $reason")
                onDisconnected()
            }
        })
    }

    fun send(message: String) {
        ws?.send(message) ?: Log.w(TAG, "WebSocket 未连接，无法发送")
    }

    fun disconnect() {
        ws?.close(1000, "用户关闭")
        ws = null
    }
}
