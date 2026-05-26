package com.elon.app

import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.update.AppUpdateManager
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.json.JSONObject
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

/**
 * 首页保活通知 WS（连接 /ws/notify）
 *
 * 功能：
 *  - 不依赖项目会话，只要 APP 在前台就保持一条轻量长连接
 *  - 服务器广播 `app_update_available` 时立即弹出更新提示
 *  - 断线自动重连（30 秒冷却）
 *
 * 生命周期：
 *  onStart → start()
 *  onStop  → stop()
 */
internal class AppNotifyWsClient(
    private val activity: AppCompatActivity,
    private val serverUrl: String
) {
    companion object {
        private const val TAG = "NotifyWs"
        private const val RECONNECT_DELAY_MS = 30_000L
    }

    private val http = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS)
        .pingInterval(25, TimeUnit.SECONDS)
        .build()

    private var ws: WebSocket? = null
    private val started = AtomicBoolean(false)
    private val reconnecting = AtomicBoolean(false)
    private val handler = Handler(Looper.getMainLooper())

    fun start() {
        if (!started.compareAndSet(false, true)) return
        connect()
    }

    fun stop() {
        started.set(false)
        handler.removeCallbacksAndMessages(null)
        ws?.cancel()
        ws = null
    }

    private fun connect() {
        if (!started.get()) return
        val versionCode = BuildConfig.VERSION_CODE
        val wsUrl = serverUrl.replace("http://", "ws://").replace("https://", "wss://")
        val url = "$wsUrl/ws/notify?version_code=$versionCode"
        val request = Request.Builder().url(url).build()
        Log.d(TAG, "连接 $url")
        ws = http.newWebSocket(request, Listener())
    }

    private fun scheduleReconnect() {
        if (!started.get()) return
        if (!reconnecting.compareAndSet(false, true)) return
        handler.postDelayed({
            reconnecting.set(false)
            connect()
        }, RECONNECT_DELAY_MS)
    }

    private inner class Listener : WebSocketListener() {
        override fun onOpen(webSocket: WebSocket, response: Response) {
            Log.d(TAG, "已连接")
        }

        override fun onMessage(webSocket: WebSocket, text: String) {
            try {
                val json = JSONObject(text)
                if (json.optString("type") == "app_update_available") {
                    val code = json.optInt("versionCode", 0)
                    Log.d(TAG, "收到更新推送 versionCode=$code")
                    activity.runOnUiThread {
                        AppUpdateManager(activity).realtimeCheck(code)
                    }
                }
            } catch (_: Exception) { /* 忽略解析错误 */ }
        }

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            Log.w(TAG, "连接失败: ${t.message}")
            scheduleReconnect()
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            Log.d(TAG, "连接关闭: $code $reason")
            scheduleReconnect()
        }
    }
}
