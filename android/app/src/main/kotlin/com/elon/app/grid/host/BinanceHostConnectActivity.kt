package com.elon.app.grid.host

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.ViewGroup
import android.view.MotionEvent
import android.view.WindowManager
import android.webkit.CookieManager
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

/** Official page and explicit read consent; no website session leaves this application. */
class BinanceHostConnectActivity : Activity() {
    private var host: BinanceHostRuntime? = null
    private var nonce = ""
    private var finished = false
    private lateinit var status: TextView
    private lateinit var authorize: Button
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        if (savedInstanceState != null || !BinanceHostCaller.activity(this)) return finish()
        if (intent.data != null || intent.clipData != null || intent.selector != null || intent.extras?.keySet() != setOf("nonce")) return finish()
        nonce = intent.getStringExtra("nonce")?.takeIf { Regex("[0-9a-f]{64}").matches(it) } ?: return finish()
        val root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(12, 12, 12, 12) }
        status = TextView(this).apply { textSize = 16f; isSaveEnabled = false }
        authorize = Button(this).apply {
            text = "授权量化读取 15 分钟"; isEnabled = false; filterTouchesWhenObscured = true
            setOnClickListener { approve() }
        }
        root.addView(status); root.addView(authorize)
        root.addView(Button(this).apply { text = "重新加载币安网格"; setOnClickListener { host?.view?.loadUrl(BinanceHostRuntime.ENTRY) } })
        root.addView(Button(this).apply { text = "取消并返回"; setOnClickListener { finish() } })
        setContentView(root)
        runCatching {
            BinanceHostRuntime.onMain(this) { runtime ->
                host = runtime; runtime.onChanged = ::render
                if (runtime.begin()) runtime.view?.let { view ->
                    (view.parent as? ViewGroup)?.removeView(view)
                    root.addView(view, LinearLayout.LayoutParams(-1, 0, 1f))
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
        status.text = "连接本人币安 · ${runtime.status}\n授权仅供量化读取列表和详情，可撤销，不包含交易权限。"
        authorize.isEnabled = runtime.live() && runtime.state.fresh()
    }
    private fun approve() {
        if (finished || !hasWindowFocus() || !BinanceHostCaller.activity(this)) return
        val runtime = host ?: return
        val token = runCatching { runtime.grant() }.getOrNull() ?: return render()
        CookieManager.getInstance().flush()
        setResult(RESULT_OK, Intent().putExtra("nonce", nonce).putExtra("grant", token).putExtra("schema", "yilong.binance_host_grant.v1"))
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
            runtime.view?.let { (it.parent as? ViewGroup)?.removeView(it) }
            if (!finished) runtime.invalidate("连接已取消")
        }
        host = null; super.onDestroy()
    }
}
