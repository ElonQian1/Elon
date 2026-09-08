package com.elon.app.grid.host

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.ViewGroup
import android.view.View
import android.view.MotionEvent
import android.view.WindowManager
import android.webkit.CookieManager
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.FrameLayout
import android.widget.ScrollView
import com.elon.app.grid.ui.BinanceGridAppearance

/** Official page and explicit read consent; no website session leaves this application. */
class BinanceHostConnectActivity : Activity() {
    private val ui by lazy { BinanceGridAppearance(this) }
    private var host: BinanceHostRuntime? = null
    private var nonce = ""
    private var finished = false
    private var continuous = false
    private lateinit var status: TextView
    private lateinit var authorize: Button
    private lateinit var official: FrameLayout
    private lateinit var toggle: Button
    private var officialRequested = false
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        if (savedInstanceState != null || !BinanceHostCaller.activity(this)) return finish()
        if (intent.data != null || intent.clipData != null || intent.selector != null) return finish()
        if (intent.extras?.keySet() !in listOf(setOf("nonce"), setOf("nonce", "purpose"))) return finish()
        continuous = intent.hasExtra("purpose")
        if (continuous && intent.getStringExtra("purpose") != "continuous_grid_read_v2") return finish()
        nonce = intent.getStringExtra("nonce")?.takeIf { Regex("[0-9a-f]{64}").matches(it) } ?: return finish()
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL; setBackgroundColor(ui.background)
            setPadding(ui.dp(18), ui.dp(12), ui.dp(18), ui.dp(8))
        }
        root.addView(ui.label("连接币安 · 读取网格", 23f))
        val content = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        status = ui.label("正在连接本人币安…", 16f).apply { contentDescription = "binance-host-status" }
        content.addView(status)
        content.addView(ui.label("这是读取授权页。授权后返回量化查看本人网格列表与详情。\n创建网格请返回量化，点击“创建网格（本人确认）”。", 14f))
        content.addView(ui.label(if (continuous) "同意后可持续查看当前币安账户的网格，重新打开量化会自动恢复。你可随时断开；更换账户需要重新确认。此授权仅用于读取，不包含交易权限。" else "本次只读授权有效 15 分钟，可随时撤销。更新量化应用后可启用持续连接。", 13f).apply { setTextColor(ui.muted) })
        authorize = ui.button(if (continuous) "同意并保持只读连接" else "授权量化读取 15 分钟", "binance-host-approve", primary = true) { approve() }.apply { isEnabled = false }
        content.addView(authorize)
        content.addView(ui.button("重新加载币安网格", "binance-host-reload") { attachHost(reload = true) })
        toggle = ui.button("查看币安官网／确认登录", "binance-host-official") {
            officialRequested = !officialRequested; render()
        }
        content.addView(toggle)
        official = FrameLayout(this).apply { setBackgroundColor(ui.surface) }
        content.addView(official, LinearLayout.LayoutParams(-1, ui.dp(520)))
        root.addView(ScrollView(this).apply { isSaveEnabled = false; addView(content) }, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(ui.button("取消并返回量化", "binance-host-return") { finish() })
        setContentView(root)
        attachHost()
    }
    private fun attachHost(reload: Boolean = false) {
        runCatching {
            BinanceHostRuntime.onMain(this) { runtime ->
                host = runtime; runtime.onChanged = ::render
                val previous = runtime.view
                if (runtime.begin()) runtime.view?.let { view ->
                    (view.parent as? ViewGroup)?.removeView(view)
                    official.addView(view, FrameLayout.LayoutParams(-1, -1))
                    if (reload && view === previous) view.loadUrl(BinanceHostRuntime.ENTRY)
                    else if (!runtime.state.fresh() && view === previous && runtime.pagePhase == "finished") view.loadUrl(BinanceHostRuntime.ENTRY)
                }
            }
        }.onFailure {
            host?.fail("当前系统 WebView 暂不支持连接，请更新后重试")
                ?: run { status.text = "当前系统 WebView 暂不支持连接，请更新后重试" }
        }
        render()
    }
    private fun render() {
        val runtime = host ?: return
        val ready = runtime.live() && runtime.state.fresh()
        status.text = "${if (ready) "可以授权读取" else "正在确认读取连接"}\n${runtime.status}"
        authorize.isEnabled = ready
        official.visibility = if (officialRequested || !ready) View.VISIBLE else View.GONE
        toggle.visibility = if (ready) View.VISIBLE else View.GONE
        toggle.text = if (officialRequested) "收起币安官网" else "查看币安官网／切换账户"
    }
    private fun approve() {
        if (finished || !hasWindowFocus() || !BinanceHostCaller.activity(this)) return
        val runtime = host ?: return
        val token = runCatching { runtime.grant(continuous) }.getOrNull() ?: return render()
        CookieManager.getInstance().flush()
        setResult(RESULT_OK, Intent().putExtra("nonce", nonce).putExtra("grant", token).putExtra("schema", if (continuous) "yilong.binance_host_grant.v2" else "yilong.binance_host_grant.v1"))
        finished = true; finish()
    }
    override fun onResume() { super.onResume(); if (::status.isInitialized) render() }
    override fun onNewIntent(intent: Intent?) { super.onNewIntent(intent); finish() }
    override fun onSaveInstanceState(outState: Bundle) { super.onSaveInstanceState(outState); outState.clear() }
    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        if (event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0) return true
        return super.dispatchTouchEvent(event)
    }
    override fun onDestroy() {
        host?.let { runtime ->
            runtime.onChanged = null
            runtime.view?.let { if (it.parent === official) official.removeView(it) }
            if (!finished) runtime.invalidate("连接已取消")
        }
        host = null; super.onDestroy()
    }
}
