package com.elon.app.esk.platform.access

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.MotionEvent
import android.view.WindowManager
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.BuildConfig
import com.elon.app.R
import com.elon.app.esk.platform.EskPlatformSession
import com.elon.app.esk.platform.EskPlatformSessionStore
import com.elon.app.esk.platform.eskPlatformEndpoint

/** Explicit native approval returns one PKCE code, never the main account credential. */
class AssetAccessConsentActivity : Activity() {
    private val handler = Handler(Looper.getMainLooper())
    private var started = 0L
    private var foreground = false
    private var finished = false
    private var confirming = false
    @Volatile private var invalidated = false
    private var sessions: EskPlatformSessionStore? = null
    private var session: EskPlatformSession? = null
    private var input: AssetAccessRequest? = null
    private var client: AssetAccessApprovalClient? = null
    private lateinit var text: TextView
    private lateinit var approve: Button

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        if (savedInstanceState != null || !hasOfficialAssetAccessCaller()) return cancel()
        input = AssetAccessRequest.parse(runCatching { intent.getStringExtra(AssetAccessRequest.INPUT) }.getOrNull())
            ?: return cancel()
        started = SystemClock.elapsedRealtime()
        createPage()
        handler.postDelayed({ cancel() }, 120_000)
        if (eskPlatformEndpoint(BuildConfig.ASSET_ACCESS_ORIGIN) == null) return unavailable("主服务尚未开通安全资产授权，请稍后重试。")
        val source = EskPlatformSessionStore(this) {
            invalidated = true
            client?.cancel()
            runOnUiThread { cancel() }
        }.also { sessions = it }
        val captured = source.capture() ?: return unavailable("请先在一龙主项目登录本人账户，再从量化应用重新发起。")
        session = captured
        if (invalidated) return cancel()
        text.text = "授权量化读取本人资产\n\n${captured.displayName}\n\n" +
            "允许量化应用在 15 分钟内查看正式 ESK 总额、可用额、占用额和申请进度，可连续刷新与翻页。\n\n" +
            "此授权不能修改资产或发起交易。你可以在量化应用撤销，15 分钟后会自动失效。"
        approve.isEnabled = true
    }

    private fun createPage() {
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            val padding = (24 * resources.displayMetrics.density).toInt()
            setPadding(padding, padding * 2, padding, padding)
            setBackgroundColor(getColor(R.color.elon_bg_app))
            isSaveEnabled = false
            importantForAutofill = android.view.View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        }
        text = TextView(this).apply { textSize = 17f; setTextColor(getColor(R.color.elon_text_primary)); isSaveEnabled = false }
        approve = Button(this).apply {
            text = "同意，只读 15 分钟"; isEnabled = false; isSaveEnabled = false; filterTouchesWhenObscured = true
            setOnClickListener { confirm() }
        }
        root.addView(text, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(approve)
        root.addView(Button(this).apply { text = "取消"; isSaveEnabled = false; setOnClickListener { cancel() } })
        setContentView(root)
    }

    private fun live(): Boolean = !finished && !invalidated && foreground && !isFinishing &&
        hasOfficialAssetAccessCaller() && session?.let { it.sameAs(sessions?.capture()) && it.validAt(System.currentTimeMillis()) } == true &&
        SystemClock.elapsedRealtime() - started in 0 until 120_000

    private fun confirm() {
        if (confirming || !live()) return cancel()
        val captured = session ?: return cancel()
        val request = input ?: return cancel()
        confirming = true
        approve.isEnabled = false
        val reader = AssetAccessApprovalClient().also { client = it }
        Thread({
            val response = runCatching { reader.authorize(BuildConfig.ASSET_ACCESS_ORIGIN, captured.token, request) }
            runOnUiThread {
                if (!live() || client !== reader) return@runOnUiThread cancel()
                response.fold(onSuccess = { result ->
                    if (!request.validateResult(result, System.currentTimeMillis()) || !live()) return@fold cancel()
                    setResult(RESULT_OK, Intent().putExtra(AssetAccessRequest.OUTPUT, result))
                    finished = true
                    clear()
                    finish()
                }, onFailure = { unavailable("授权未完成。请检查登录和网络后重新发起。") })
            }
        }, "asset-access-approval").start()
    }

    private fun unavailable(message: String) { clear(); if (::text.isInitialized) text.text = message }
    private fun clear() {
        invalidated = true
        client?.cancel(); client = null
        sessions?.close(); sessions = null; session = null; input = null
        handler.removeCallbacksAndMessages(null)
        if (::approve.isInitialized) { approve.isEnabled = false; approve.setOnClickListener(null) }
        if (::text.isInitialized) text.text = ""
    }
    private fun cancel() { if (finished) return; finished = true; clear(); setResult(RESULT_CANCELED); finish() }
    override fun onResume() { super.onResume(); foreground = true; if (!invalidated && session != null && !live()) cancel() }
    override fun onPause() { foreground = false; cancel(); super.onPause() }
    override fun onStop() { cancel(); super.onStop() }
    override fun onNewIntent(intent: Intent?) { super.onNewIntent(intent); cancel() }
    override fun onSaveInstanceState(outState: Bundle) { cancel(); super.onSaveInstanceState(outState); outState.clear() }
    override fun onDestroy() { clear(); super.onDestroy() }
    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        if (event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0) {
            cancel(); return true
        }
        return super.dispatchTouchEvent(event)
    }
}
