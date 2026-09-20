package com.elon.app.grid.host

import android.app.Activity
import android.app.AlertDialog
import android.os.Bundle
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.AuthManager
import com.elon.app.grid.ui.BinanceGridAppearance

/** Internal account management uses the very same host as the quant APK. */
class BinanceAccountActivity : Activity() {
    private var host: BinanceHostRuntime? = null
    private lateinit var frame: FrameLayout
    private lateinit var status: TextView
    private val handler = android.os.Handler(android.os.Looper.getMainLooper())
    private val heartbeat = object : Runnable {
        override fun run() { host?.keepAlive(); handler.postDelayed(this, 60_000) }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (!AuthManager.isLoggedIn(this)) { finish(); return }
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        val ui = BinanceGridAppearance(this)
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL; setBackgroundColor(ui.background)
        }
        root.addView(ui.label("币安账户", 22f))
        status = ui.label("正在确认账户…", 13f).apply { contentDescription = "binance-account-status" }
        root.addView(status)
        root.addView(ui.label("本机账户供一龙量化 APK 使用。切换后，请回量化重新连接并确认授权。", 13f))
        root.addView(ui.button("切换币安账户", "binance-account-switch") {
            AlertDialog.Builder(this).setTitle("切换币安账户")
                .setMessage("将撤销量化的网格与资产读取授权。请在官网退出当前账户并登录另一账户，再点击核对账户。")
                .setNegativeButton("取消", null).setPositiveButton("开始切换") { _, _ ->
                    if (host?.startAccountSwitch() == true) attach()
                    else status.text = "请先点击核对账户，确认当前登录身份后再切换。"
                }.show()
        })
        root.addView(ui.button("核对账户／刷新连接", "binance-account-verify") {
            host?.let { if (it.begin()) { attach(); it.view?.loadUrl(BinanceHostRuntime.ENTRY) } }
        })
        root.addView(ui.button("取消切换，保留当前官网账户", "binance-account-cancel-switch") {
            if (host?.switchingAccount == true && host?.cancelAccountSwitch() == true) attach()
        })
        frame = FrameLayout(this)
        root.addView(frame, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(ui.button("返回账号与安全", "binance-account-return") { finish() })
        setContentView(root)
        runCatching {
            BinanceHostRuntime.onMain(this) { runtime ->
                host = runtime; runtime.onChanged = ::render
                if (runtime.begin()) attach()
                render()
            }
        }.onFailure { status.text = "币安连接暂不可用，请更新系统 WebView 后重试" }
    }

    private fun attach() {
        host?.view?.let { view ->
            if (view.parent !== frame) {
                (view.parent as? ViewGroup)?.removeView(view)
                frame.addView(view, FrameLayout.LayoutParams(-1, -1))
            }
        }
    }
    private fun render() { host?.let { status.text = "${it.accountSummary()}\n${it.status}"; attach() } }
    override fun onResume() { super.onResume(); handler.post(heartbeat) }
    override fun onPause() { handler.removeCallbacks(heartbeat); super.onPause() }
    override fun dispatchTouchEvent(event: android.view.MotionEvent): Boolean {
        if (event.flags and (android.view.MotionEvent.FLAG_WINDOW_IS_OBSCURED or
                android.view.MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0) return true
        return super.dispatchTouchEvent(event)
    }
    override fun onDestroy() {
        handler.removeCallbacksAndMessages(null)
        host?.let { runtime ->
            runtime.onChanged = null
            runtime.view?.let { if (::frame.isInitialized && it.parent === frame) frame.removeView(it) }
        }
        super.onDestroy()
    }
}
