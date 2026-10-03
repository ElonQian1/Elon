package com.elon.app.grid.sources

import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.MotionEvent
import android.view.WindowManager
import android.widget.CheckBox
import com.elon.app.esk.platform.EskPlatformSessionStore
import com.elon.app.grid.host.BinanceAccountActivity
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.host.BinanceHostState
import java.util.concurrent.Executors

class GridDeviceSourcesActivity : com.elon.app.MobileActivity() {
    private val handler = Handler(Looper.getMainLooper())
    private val executor = Executors.newSingleThreadExecutor()
    private lateinit var sessions: EskPlatformSessionStore
    private lateinit var page: GridDeviceListView
    private var sources = emptyList<GridDeviceSource>()
    private var reader: GridDeviceHttp? = null
    private var epoch = 0L
    private var active = false
    private var busy = false
    private val expiry = Runnable { render() }
    private var local: GridDeviceSource? = null
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(null)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        sessions = EskPlatformSessionStore(this) { handler.post { clear(); if (active) finish() } }
        page = GridDeviceListView(this) { render() }
        val sync = GridDeviceSync.get(this)
        page.controls.addView(CheckBox(this).apply {
            text = "将本机 APK 网格同步给本人设备"; isChecked = sync.enabled(); isSaveEnabled = false
            setTextColor(com.elon.app.MobileColors(this@GridDeviceSourcesActivity).text)
            minHeight = (48 * resources.displayMetrics.density).toInt(); filterTouchesWhenObscured = true
            setOnCheckedChangeListener { _, enabled ->
                runCatching { BinanceHostRuntime.onMain(this@GridDeviceSourcesActivity) { sync.setEnabled(enabled, it) } }
                    .onFailure { page.status.text = "同步设置未完成，请确认一龙登录后重试。" }
                page.status.text = sync.status
            }
        })
        page.button("刷新所选来源／重试同步") { read(retrySync = true) }
        page.button("登录／管理本机币安") { startActivity(Intent(this, BinanceAccountActivity::class.java)) }
        page.button("返回", ::finish)
        setContentView(page.root)
    }
    private fun read(retrySync: Boolean = false) {
        if (!active || busy) return
        val session = sessions.capture() ?: run { finish(); return }
        busy = true; val ticket = ++epoch
        sources = emptyList(); local = null; render()
        page.status.text = "正在读取所选来源…"
        runCatching { BinanceHostRuntime.onMain(this) { host ->
            if (page.apk.isChecked) host.recoverConnection()
            val sync = GridDeviceSync.get(this)
            if (retrySync || sync.enabled()) sync.retry(host)
            if (host.live() && host.owner() == BinanceHostState.digest(session.userId) && host.state.fresh())
                local = GridDeviceSource("local", "android", sync.deviceId(), host.state.generation,
                    host.state.account, host.state.accountKind, host.state.observed, host.state.observed + 300000,
                    "fresh", host.state.snapshotRows())
        } }
        render()
        val transport = GridDeviceHttp().also { reader = it }
        executor.execute {
            val result = runCatching { GridDeviceContract.parse(transport.request(session, sessions), System.currentTimeMillis()) }
            handler.post {
                if (ticket != epoch || !active || !session.sameAs(sessions.capture())) return@post
                busy = false; reader = null
                result.fold({ sources = it; page.status.text = "本人设备只读快照 · 5 分钟后过期，刷新不会延长原始数据有效期。" },
                    { page.status.text = "远端读取未成功；本机已有数据仍可查看。请检查网络和来源端同步状态。" })
                render()
            }
        }
    }
    private fun render() {
        if (!::page.isInitialized) return
        handler.removeCallbacks(expiry)
        val current = sources.filterNot { local != null && it.platform == "android" && it.device == local?.device } + listOfNotNull(local)
        val now = System.currentTimeMillis()
        page.show(current, now)
        current.filter { it.fresh(now) }.minOfOrNull { it.freshUntil }?.let { handler.postDelayed(expiry, (it - now).coerceAtLeast(1)) }
    }
    private fun clear() {
        epoch++; reader?.cancel(); reader = null; busy = false; sources = emptyList(); local = null
        handler.removeCallbacks(expiry); if (::page.isInitialized) page.show(emptyList(), System.currentTimeMillis())
    }
    override fun onResume() { super.onResume(); active = true; read() }
    override fun onPause() { active = false; clear(); super.onPause() }
    override fun onDestroy() { clear(); sessions.close(); executor.shutdownNow(); super.onDestroy() }
    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        if (event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0) { clear(); return true }
        return super.dispatchTouchEvent(event)
    }
}
