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

/**
 * 应用级全局 WS 管理器（由 ElonApplication 持有）
 *
 * 连接服务器 /ws/app 作为统一实时通道：
 *  - APK 更新推送
 *  - 好友消息（服务端上线后自动生效）
 *  - 未来：通知、在线状态等
 *
 * 设计原则：
 *  - 单例由 ElonApplication 持有，不绑定 Activity
 *  - 所有事件回调均在主线程，监听者无需切线程
 *  - 断线按指数退避重连（5 → 10 → 20 → 40 → 120 秒封顶）
 *  - 认证可选：有 token 传给服务端以接收个人事件，匿名也能收更新推送
 *
 * 生命周期：
 *  Application 启动后保持连接；Activity 只增删自己的前台 UI 监听者。
 */
class GlobalWsManager(private val serverUrl: String) {

    companion object {
        private const val TAG = "GlobalWs"
        /** 指数退避重连间隔，单位毫秒 */
        private val BACKOFF_MS = longArrayOf(5_000L, 10_000L, 20_000L, 40_000L, 120_000L)
    }

    /** 事件监听者接口，在主线程回调 */
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
    private val reconnecting = AtomicBoolean(false)
    private var ws: WebSocket? = null
    private var retryCount = 0
    private var connectedToken: String? = null
    private var appCtx: Context? = null // 始终持有 applicationContext，不持有 Activity

    // ── 公共 API ─────────────────────────────────────────────────────────────

    /** 开始保活连接。ctx 会提取 applicationContext，不会持有 Activity。 */
    fun start(ctx: Context) {
        appCtx = ctx.applicationContext
        val latestToken = AuthManager.token(ctx)
        if (!running.compareAndSet(false, true)) {
            if (latestToken != connectedToken) reconnectWithNewToken()
            return
        }
        retryCount = 0
        connect()
    }

    /** 停止连接，取消所有重连定时器。 */
    fun stop() {
        running.set(false)
        handler.removeCallbacksAndMessages(null)
        ws?.cancel()
        ws = null
        connectedToken = null
    }

    /** 当用户登录/退出后调用，立即重连以刷新 token。 */
    fun reconnectWithNewToken() {
        if (!running.get()) return
        ws?.cancel()
        ws = null
        retryCount = 0
        connect()
    }

    fun addListener(l: Listener) { listeners.add(l) }
    fun removeListener(l: Listener) { listeners.remove(l) }

    // ── 内部实现 ──────────────────────────────────────────────────────────────

    private fun connect() {
        val ctx = appCtx ?: return
        if (!running.get()) return

        val activeUrl = ServerUrlManager.getActive(ctx)
        val wsBase = activeUrl
            .replace("https://", "wss://")
            .replace("http://", "ws://")
        val token = AuthManager.token(ctx)
        connectedToken = token
        val versionCode = BuildConfig.VERSION_CODE

        val url = buildString {
            append("$wsBase/ws/app?version_code=$versionCode")
            if (!token.isNullOrBlank()) append("&token=$token")
        }
        Log.d(TAG, "连接 $url (retry=$retryCount)")
        ws = http.newWebSocket(Request.Builder().url(url).build(), InnerListener())
    }

    private fun scheduleReconnect() {
        if (!running.get()) return
        if (!reconnecting.compareAndSet(false, true)) return
        val delay = BACKOFF_MS.getOrElse(retryCount) { BACKOFF_MS.last() }
        retryCount = (retryCount + 1).coerceAtMost(BACKOFF_MS.size - 1)
        Log.d(TAG, "将在 ${delay / 1000}s 后重连")
        handler.postDelayed({
            reconnecting.set(false)
            connect()
        }, delay)
    }

    private fun dispatch(event: GlobalWsEvent) {
        handler.post { listeners.forEach { it.onGlobalWsEvent(event) } }
    }

    private inner class InnerListener : WebSocketListener() {
        override fun onOpen(webSocket: WebSocket, response: Response) {
            Log.d(TAG, "已连接")
            retryCount = 0
            appCtx?.let { ServerUrlManager.reportSuccess(it) }
        }

        override fun onMessage(webSocket: WebSocket, text: String) {
            dispatch(GlobalWsEvent.parse(text))
        }

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            Log.w(TAG, "连接失败: ${t.message}")
            appCtx?.let { ServerUrlManager.reportFailure(it) }
            scheduleReconnect()
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            if (running.get()) {
                Log.d(TAG, "连接关闭 code=$code, 将重连")
                scheduleReconnect()
            }
        }
    }
}
