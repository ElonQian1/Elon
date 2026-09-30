package com.elon.app.esk.compute

import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.WindowManager
import com.elon.app.BuildConfig
import com.elon.app.esk.platform.EskPlatformAssetsActivity
import com.elon.app.esk.platform.EskPlatformHistoryActivity
import com.elon.app.esk.platform.EskPlatformRequestGate
import com.elon.app.esk.platform.EskPlatformSessionStore
import com.elon.app.esk.platform.eskPlatformEndpoint

/** Native foreground-only account center. No snapshots in saved state. */
class EskComputeActivity : com.elon.app.MobileActivity() {
    private val handler = Handler(Looper.getMainLooper())
    private val gate = EskPlatformRequestGate()
    private var foreground = false
    private var sessions: EskPlatformSessionStore? = null
    private var reader: EskComputeClient? = null
    private var pageNumber = 1
    private lateinit var view: EskComputeView
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        view = EskComputeView(this, ::finish, ::refresh,
            { startActivity(Intent(this, EskPlatformHistoryActivity::class.java)) },
            { startActivity(Intent(this, EskPlatformAssetsActivity::class.java)) },
            { next -> pageNumber = next; refresh() })
    }
    override fun onResume() { super.onResume(); foreground = true; refresh() }
    override fun onPause() { foreground = false; clear(); super.onPause() }
    override fun onDestroy() { clear(); super.onDestroy() }
    private fun clear() {
        gate.invalidate(); reader?.cancel(); reader = null
        sessions?.close(); sessions = null; handler.removeCallbacksAndMessages(null)
        if (::view.isInitialized) view.clear()
    }
    private fun refresh() {
        clear()
        if (!foreground || isFinishing || isDestroyed) return
        if (eskPlatformEndpoint(BuildConfig.ASSET_ACCESS_ORIGIN) == null) {
            view.unavailable("当前主服务暂不支持安全资产读取。请配置 HTTPS 主服务后重试；没有显示账户余额。")
            return
        }
        val store = EskPlatformSessionStore(this) { runOnUiThread {
            pageNumber = 1; clear(); if (foreground) view.unavailable("账户已变化，请确认当前账户后刷新。")
        } }.also { sessions = it }
        val session = store.capture()
        if (session == null) { clear(); view.unavailable("请先登录自己的主项目账户。登录后返回刷新。"); return }
        val ticket = gate.begin(session, SystemClock.elapsedRealtime(), System.currentTimeMillis(), foreground) ?: return
        val source = EskComputeClient().also { reader = it }
        view.loading()
        handler.postDelayed({ if (reader === source) { clear(); view.unavailable("读取超时，请检查网络后重试。") } }, 15_000)
        val requestedPage = pageNumber
        Thread({
            val result = runCatching { source.fetch(BuildConfig.ASSET_ACCESS_ORIGIN, requestedPage) { session.token } }
            runOnUiThread {
                if (reader !== source || !foreground || isFinishing || isDestroyed) return@runOnUiThread
                if (!gate.consume(ticket, store.capture(), SystemClock.elapsedRealtime(), System.currentTimeMillis(), foreground)) {
                    clear(); view.unavailable("本次账户查看已失效，请刷新。"); return@runOnUiThread
                }
                handler.removeCallbacksAndMessages(null); reader = null
                result.fold(onSuccess = { snapshot ->
                    if (snapshot.page != requestedPage) { clear(); view.unavailable("账单页码无效，请刷新。"); return@fold }
                    view.show(snapshot, session.displayName)
                    val expiry = minOf(snapshot.freshUntil, if (session.expiresAtMillis == 0L) Long.MAX_VALUE else session.expiresAtMillis)
                    handler.postDelayed({ clear(); if (foreground) view.unavailable("本次账户资料已到期，已清除余额。请刷新确认最新状态。") },
                        (expiry - System.currentTimeMillis()).coerceAtLeast(0))
                    snapshot.quote?.let { quote -> handler.postDelayed({ view.expireQuote() },
                        (quote.validUntil - System.currentTimeMillis()).coerceAtLeast(0)) }
                }, onFailure = { clear(); view.unavailable("无法读取有效账户资料。请确认登录、安全连接和网络后刷新。") })
            }
        }, "esk-compute-center").start()
    }
}
