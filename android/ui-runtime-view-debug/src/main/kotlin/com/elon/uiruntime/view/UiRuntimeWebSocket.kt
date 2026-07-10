package com.elon.uiruntime.view

import android.os.Handler
import android.os.Looper
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import java.util.concurrent.TimeUnit

internal class UiRuntimeWebSocket(
    private val config: UiRuntimeSessionConfig,
    private val onMessage: (String) -> Unit,
    private val onConnectionChanged: (Boolean, String?) -> Unit,
) {
    private val mainHandler = Handler(Looper.getMainLooper())
    private val client = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS)
        .pingInterval(10, TimeUnit.SECONDS)
        .build()
    private var socket: WebSocket? = null
    private var active = false
    private var retry = 0

    fun connect() {
        active = true
        open()
    }

    fun send(text: String): Boolean = socket?.send(text) == true

    fun close() {
        active = false
        mainHandler.removeCallbacksAndMessages(null)
        socket?.close(1000, "Live UI session stopped")
        socket = null
        client.dispatcher.executorService.shutdown()
    }

    private fun open() {
        if (!active) return
        val url = "ws://127.0.0.1:${config.devicePort}/api/android-live/runtime" +
            "?sessionId=${config.sessionId}&token=${config.token}"
        socket = client.newWebSocket(Request.Builder().url(url).build(), Listener())
    }

    private fun reconnect(error: String) {
        if (!active) return
        onConnectionChanged(false, error)
        retry = (retry + 1).coerceAtMost(6)
        val delay = (1L shl retry).coerceAtMost(20L) * 500L
        mainHandler.postDelayed(::open, delay)
    }

    private inner class Listener : WebSocketListener() {
        override fun onOpen(webSocket: WebSocket, response: Response) {
            retry = 0
            onConnectionChanged(true, null)
        }

        override fun onMessage(webSocket: WebSocket, text: String) {
            onMessage(text)
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            socket = null
            reconnect("连接已关闭: $code $reason")
        }

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            socket = null
            reconnect(t.message ?: "WebSocket 连接失败")
        }
    }
}
