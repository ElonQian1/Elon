package com.elon.app.esk.platform

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.WindowManager
import com.elon.app.BuildConfig

/** Foreground-only, read-only platform ledger. Never exports a result to another application. */
class EskPlatformAssetsActivity : Activity() {
    private val gate = EskPlatformRequestGate()
    private val handler = Handler(Looper.getMainLooper())
    private var foreground = false
    private var store: EskPlatformSessionStore? = null
    @Volatile private var reader: EskPlatformAccountReader? = null
    private lateinit var page: EskPlatformAssetsView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        page = EskPlatformAssetsView(this, ::finish, ::refresh) {
            startActivity(Intent(this, EskPlatformHistoryActivity::class.java))
        }
    }

    override fun onResume() {
        super.onResume()
        foreground = true
        refresh()
    }

    private fun refresh() {
        clearPrivateState()
        if (!foreground || isFinishing || isDestroyed) return
        // Must precede construction of the session adapter as well as every credential read.
        if (eskPlatformEndpoint(BuildConfig.SERVER_URL) == null) {
            page.unavailable("当前主服务暂不支持安全资产读取。账户凭据不会通过 HTTP 发送；数量暂不可用，请稍后重试。")
            return
        }
        val sessions = EskPlatformSessionStore(this) {
            gate.invalidate()
            reader?.cancel()
            runOnUiThread { invalidateDisplayedAccount() }
        }.also { store = it }
        val session = sessions.capture()
        if (session == null) {
            clearPrivateState()
            page.unavailable("请先在主项目登录自己的账户，再返回此页刷新正式登记。")
            return
        }
        val ticket = gate.begin(session, SystemClock.elapsedRealtime(), System.currentTimeMillis(), foreground)
            ?: return invalidateDisplayedAccount()
        val source = EskPlatformAccountReader().also { reader = it }
        page.loading()
        handler.postDelayed({
            if (reader === source) {
                clearPrivateState()
                page.unavailable("读取超时，未显示余额。请检查网络后刷新。")
            }
        }, EskPlatformRequestGate.MAX_REQUEST_MS)
        Thread({
            val result = runCatching { source.fetch(BuildConfig.SERVER_URL) { session.token } }
            runOnUiThread {
                if (reader !== source || !foreground || isFinishing || isDestroyed) return@runOnUiThread
                if (!gate.consume(ticket, sessions.capture(), SystemClock.elapsedRealtime(),
                        System.currentTimeMillis(), foreground)) {
                    invalidateDisplayedAccount()
                    return@runOnUiThread
                }
                handler.removeCallbacksAndMessages(null)
                reader = null
                result.fold(onSuccess = { account ->
                    page.show(account, session.displayName)
                    val untilExpiry = if (session.expiresAtMillis == 0L) 60_000L
                        else (session.expiresAtMillis - System.currentTimeMillis()).coerceAtLeast(0L)
                    handler.postDelayed({
                        clearPrivateState()
                        page.unavailable("本次查看已到期，余额已清除。请刷新以确认当前账户的最新登记。")
                    }, minOf(untilExpiry, 60_000L))
                }, onFailure = {
                    clearPrivateState()
                    page.unavailable(if ((it as? EskPlatformReadException)?.failure == EskPlatformReadFailure.SIGN_IN_REQUIRED)
                        "登录状态已失效，余额已清除。请返回主项目重新登录后刷新。"
                        else "未能读取有效的正式登记。没有显示余额；请确认登录和网络后刷新。")
                })
            }
        }, "esk-platform-account").start()
    }

    private fun invalidateDisplayedAccount() {
        clearPrivateState()
        if (foreground && !isFinishing && !isDestroyed) {
            page.unavailable("账户状态已变化，原数量已清除。请确认当前账户后刷新。")
        }
    }

    private fun clearPrivateState() {
        gate.invalidate()
        reader?.cancel()
        reader = null
        store?.close()
        store = null
        handler.removeCallbacksAndMessages(null)
        if (::page.isInitialized) page.clear()
    }

    override fun onPause() {
        foreground = false
        clearPrivateState()
        super.onPause()
    }

    override fun onStop() {
        clearPrivateState()
        super.onStop()
    }

    override fun onSaveInstanceState(outState: Bundle) {
        clearPrivateState()
        super.onSaveInstanceState(outState)
        outState.clear()
    }

    override fun onDestroy() {
        foreground = false
        clearPrivateState()
        super.onDestroy()
    }
}
