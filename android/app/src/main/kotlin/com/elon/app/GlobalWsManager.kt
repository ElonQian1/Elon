package com.elon.app

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

class GlobalWsManager(private val serverUrl: String) {
    companion object {
        private const val TAG = "GlobalWs"
        private val BACKOFF_MS = longArrayOf(5_000L, 10_000L, 20_000L, 40_000L, 120_000L)
    }

    interface Listener {
        fun onGlobalWsEvent(event: GlobalWsEvent)
    }

    private val http = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS)
        .pingInterval(25, TimeUnit.SECONDS)
        .build()

    private val listeners = CopyOnWriteArrayList<Listener>()
    private val handler = Handler(Looper.getMainLooper())
    private val running = AtomicBoolean(false)
    private val connected = AtomicBoolean(false)
    private val reconnecting = AtomicBoolean(false)
    private var ws: WebSocket? = null
    private var retryCount = 0
    private var connectedToken: String? = null
    private var appCtx: Context? = null
    @Volatile
    private var latestAppUpdate: GlobalWsEvent.AppUpdateAvailable? = null

    fun start(ctx: Context) {
        appCtx = ctx.applicationContext
        val latestToken = currentToken(ctx)
        if (!running.compareAndSet(false, true)) {
            if (latestToken != connectedToken) {
                reconnectWithNewToken()
            } else if (!connected.get() && ws == null) {
                connect()
            }
            return
        }
        retryCount = 0
        connect()
    }

    fun stop() {
        running.set(false)
        connected.set(false)
        handler.removeCallbacksAndMessages(null)
        ws?.cancel()
        ws = null
        connectedToken = null
    }

    fun reconnectWithNewToken() {
        if (!running.get()) return
        connected.set(false)
        ws?.cancel()
        ws = null
        retryCount = 0
        connect()
    }

    fun addListener(listener: Listener) {
        listeners.addIfAbsent(listener)
        latestAppUpdate?.let { event ->
            handler.post {
                if (listeners.contains(listener)) listener.onGlobalWsEvent(event)
            }
        }
    }

    fun removeListener(listener: Listener) {
        listeners.remove(listener)
    }

    fun send(text: String): Boolean = ws?.send(text) ?: false

    private fun currentToken(ctx: Context): String? {
        val activeUrl = ServerUrlManager.getActive(ctx)
        if (activeUrl == BuildConfig.SERVER_URL) return AuthManager.token(ctx)
        val fallbackToken = ctx.getSharedPreferences("agent_config", Context.MODE_PRIVATE)
            .getString("fallback_server_token", null)
            ?.takeIf { it.isNotBlank() }
        return fallbackToken ?: AuthManager.token(ctx)
    }

    private fun connect() {
        val ctx = appCtx ?: return
        if (!running.get()) return
        connected.set(false)

        val activeUrl = ServerUrlManager.getActive(ctx)
        val wsBase = activeUrl
            .replace("https://", "wss://")
            .replace("http://", "ws://")
        val token = currentToken(ctx)
        connectedToken = token
        val versionCode = BuildConfig.VERSION_CODE
        val url = buildString {
            append("$wsBase/ws/app?version_code=$versionCode")
            if (!token.isNullOrBlank()) append("&token=$token")
        }

        Log.d(TAG, "connect retry=$retryCount")
        ws = http.newWebSocket(Request.Builder().url(url).build(), InnerListener())
    }

    private fun scheduleReconnect() {
        if (!running.get()) return
        if (!reconnecting.compareAndSet(false, true)) return
        val delay = BACKOFF_MS.getOrElse(retryCount) { BACKOFF_MS.last() }
        retryCount = (retryCount + 1).coerceAtMost(BACKOFF_MS.size - 1)
        Log.d(TAG, "reconnect in ${delay / 1000}s")
        handler.postDelayed({
            reconnecting.set(false)
            connect()
        }, delay)
    }

    private fun dispatch(event: GlobalWsEvent) {
        if (event is GlobalWsEvent.AppUpdateAvailable) {
            latestAppUpdate = event
        }
        handler.post { listeners.forEach { it.onGlobalWsEvent(event) } }
    }

    private inner class InnerListener : WebSocketListener() {
        override fun onOpen(webSocket: WebSocket, response: Response) {
            connected.set(true)
            retryCount = 0
            Log.d(TAG, "connected")
            appCtx?.let { ServerUrlManager.reportSuccess(it) }
        }

        override fun onMessage(webSocket: WebSocket, text: String) {
            dispatch(GlobalWsEvent.parse(text))
        }

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            connected.set(false)
            if (ws === webSocket) ws = null
            Log.w(TAG, "connection failed: ${t.message}")
            appCtx?.let { ServerUrlManager.reportFailure(it) }
            scheduleReconnect()
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            connected.set(false)
            if (ws === webSocket) ws = null
            if (running.get()) {
                Log.d(TAG, "closed code=$code, reconnecting")
                scheduleReconnect()
            }
        }
    }
}
