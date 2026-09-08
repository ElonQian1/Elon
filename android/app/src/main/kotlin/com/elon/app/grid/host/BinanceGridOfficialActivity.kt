package com.elon.app.grid.host

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.LinearLayout
import com.elon.app.grid.ui.BinanceGridAppearance

/** Full official product in the existing session. The native bridge does not submit any action. */
class BinanceGridOfficialActivity : Activity() {
    private var runtime: BinanceHostRuntime? = null
    private lateinit var frame: FrameLayout
    private var nonce = ""
    private val handler = android.os.Handler(android.os.Looper.getMainLooper())
    private val keepAlive = object : Runnable {
        override fun run() { runtime?.keepAlive(); handler.postDelayed(this, 60000) }
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState); setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        if (savedInstanceState != null || !BinanceHostCaller.activity(this) || intent.data != null ||
            intent.clipData != null || intent.selector != null || intent.extras?.keySet() != setOf("nonce")) return finish()
        nonce = intent.getStringExtra("nonce")?.takeIf { Regex("[a-f0-9]{64}").matches(it) } ?: return finish()
        val ui = BinanceGridAppearance(this)
        val root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setBackgroundColor(ui.background) }
        root.addView(ui.label("币安官网 · 当前登录账户", 18f))
        val status = ui.label("正在连接…", 12f).apply { contentDescription = "binance-official-status" }
        root.addView(status)
        frame = FrameLayout(this); root.addView(frame, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(ui.button("刷新官网", "binance-official-reload") {
            runtime?.let { host ->
                if (host.begin()) host.view?.let { view ->
                    if(view.parent !== frame) { (view.parent as? ViewGroup)?.removeView(view); frame.addView(view, FrameLayout.LayoutParams(-1, -1)) }
                    view.reload()
                }
            }
        })
        root.addView(ui.button("返回量化网格", "binance-official-return") { finish() })
        setContentView(root)
        runCatching {
            BinanceHostRuntime.onMain(this) { host ->
                runtime = host; host.onChanged = { status.text = host.status }
                if (host.begin()) host.view?.let { view ->
                    (view.parent as? ViewGroup)?.removeView(view); frame.addView(view, FrameLayout.LayoutParams(-1, -1))
                }
                status.text = host.status
            }
        }.onFailure { status.text = "官网连接暂不可用，请返回重试" }
        setResult(RESULT_OK, Intent().putExtra("nonce", nonce).putExtra("schema", "yilong.binance_official_return.v1"))
    }
    override fun onResume() { super.onResume(); handler.post(keepAlive) }
    override fun onPause() { handler.removeCallbacks(keepAlive); super.onPause() }
    override fun onDestroy() {
        handler.removeCallbacksAndMessages(null)
        runtime?.let { host ->
            host.onChanged = null
            host.view?.let { if (::frame.isInitialized && it.parent === frame) frame.removeView(it) }
        }
        super.onDestroy()
    }
}
