package com.elon.app.esk.platform

import android.app.Activity
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.WindowManager
import com.elon.app.BuildConfig

/** Authenticated native history only; no saved state, export, wallet, or payment action. */
class EskPlatformHistoryActivity : Activity() {
    private val gate = EskPlatformRequestGate()
    private val history = EskPlatformHistoryPageState()
    private val handler = Handler(Looper.getMainLooper())
    private var foreground = false
    private var store: EskPlatformSessionStore? = null
    @Volatile private var reader: EskPlatformHistoryReader? = null
    private lateinit var page: EskPlatformHistoryView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        page = EskPlatformHistoryView(this, ::finish, ::refresh, ::next)
    }

    override fun onResume() { super.onResume(); foreground = true; refresh() }

    private fun refresh() {
        clearPrivateState()
        if (!foreground || isFinishing || isDestroyed) return
        // Validate the configured origin before constructing an adapter or reading credentials.
        if (eskPlatformEndpoint(BuildConfig.SERVER_URL) == null) {
            page.unavailable("当前主服务暂不支持安全流水读取。账户凭据不会通过 HTTP 发送；请稍后重试。")
            return
        }
        val sessions = EskPlatformSessionStore(this) {
            gate.invalidate()
            history.clear()
            reader?.cancel()
            runOnUiThread { unavailable("账户状态已变化，流水已清除。请确认账户后重新加载。") }
        }.also { store = it }
        val session = sessions.capture() ?: return unavailable("请先在主项目登录自己的账户，再重新加载。")
        load(history.first(session), sessions, session)
    }

    private fun next() {
        if (!foreground || reader != null || isFinishing || isDestroyed) return
        val sessions = store ?: return unavailable("本次查看已失效，请重新加载。")
        val session = sessions.capture()
        val ticket = history.next(session, SystemClock.elapsedRealtime(), System.currentTimeMillis(), foreground)
            ?: return unavailable("本次查看已失效，请重新加载以确认当前账户。")
        load(ticket, sessions, requireNotNull(session))
    }

    private fun load(
        historyTicket: EskPlatformHistoryPageState.Ticket,
        sessions: EskPlatformSessionStore, session: EskPlatformSession,
    ) {
        handler.removeCallbacksAndMessages(null)
        val ticket = gate.begin(session, SystemClock.elapsedRealtime(), System.currentTimeMillis(), foreground)
            ?: return unavailable("登录状态已失效，请重新登录。")
        val source = EskPlatformHistoryReader().also { reader = it }
        page.loading()
        handler.postDelayed({
            if (reader === source) unavailable("读取超时，流水已清除。请重新加载。")
        }, EskPlatformRequestGate.MAX_REQUEST_MS)
        Thread({
            val result = runCatching { source.fetch(BuildConfig.SERVER_URL, historyTicket.cursor) { session.token } }
            runOnUiThread {
                if (reader !== source || !foreground || isFinishing || isDestroyed) return@runOnUiThread
                val current = sessions.capture()
                val elapsed = SystemClock.elapsedRealtime()
                val epoch = System.currentTimeMillis()
                if (!gate.consume(ticket, current, elapsed, epoch, foreground)) {
                    unavailable("账户或请求已失效，流水已清除。请重新加载。")
                    return@runOnUiThread
                }
                handler.removeCallbacksAndMessages(null)
                reader = null
                result.fold(onSuccess = { records ->
                    if (!history.accept(historyTicket, records, current, elapsed, epoch, foreground)) {
                        unavailable("流水上下文已变化，请从第一页重新加载。")
                        return@fold
                    }
                    page.show(records, session.displayName)
                    val untilExpiry = if (session.expiresAtMillis == 0L) EskPlatformHistoryPageState.MAX_DISPLAY_MS
                        else (session.expiresAtMillis - epoch).coerceAtLeast(0L)
                    handler.postDelayed({ unavailable("本次查看已到期，流水已清除。请重新加载。") },
                        minOf(untilExpiry, EskPlatformHistoryPageState.MAX_DISPLAY_MS))
                }, onFailure = {
                    unavailable(when ((it as? EskPlatformHistoryReadException)?.failure) {
                        EskPlatformHistoryReadFailure.HISTORY_CHANGED -> "账本已更新，请重新加载。旧流水已清除。"
                        EskPlatformHistoryReadFailure.SIGN_IN_REQUIRED -> "登录状态已失效，请重新登录后加载。"
                        else -> "未能读取有效的正式流水。请确认登录和网络后重新加载。"
                    })
                })
            }
        }, "esk-platform-history").start()
    }

    private fun unavailable(message: String) {
        clearPrivateState()
        if (foreground && !isFinishing && !isDestroyed) page.unavailable(message)
    }

    private fun clearPrivateState() {
        gate.invalidate(); history.clear()
        reader?.cancel(); reader = null
        store?.close(); store = null
        handler.removeCallbacksAndMessages(null)
        if (::page.isInitialized) page.clear()
    }

    override fun onPause() { foreground = false; clearPrivateState(); super.onPause() }
    override fun onStop() { clearPrivateState(); super.onStop() }
    override fun onSaveInstanceState(outState: Bundle) {
        clearPrivateState(); super.onSaveInstanceState(outState); outState.clear()
    }
    override fun onDestroy() { foreground = false; clearPrivateState(); super.onDestroy() }
}
