package com.elon.app.esk.platform.progress

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.MotionEvent
import android.view.WindowManager
import com.elon.app.BuildConfig
import com.elon.app.esk.platform.EskPlatformRequestGate
import com.elon.app.esk.platform.EskPlatformSession
import com.elon.app.esk.platform.EskPlatformSessionStore
import com.elon.app.esk.platform.eskPlatformEndpoint
import com.elon.app.esk.platform.sellback.EskPlatformSellbackClient
import com.elon.app.esk.platform.sellback.SellbackNetworkException
import com.elon.app.esk.platform.sellback.SellbackNetworkFailure
import com.elon.app.esk.platform.sellback.SellbackPage
import com.elon.eskcontract.EskPlatformProgressContract

/** One explicitly confirmed read-only page, not a login or financial operation. */
class EskPlatformProgressConsentActivity : Activity() {
    private enum class Phase { NEW, CONFIRMING, READING, FAILED, FINISHED }
    private var phase = Phase.NEW
    private var foreground = false
    private var startedAt = 0L
    private var nonce: String? = null
    private var cursor: String? = null
    private var session: EskPlatformSession? = null
    private var sessions: EskPlatformSessionStore? = null
    @Volatile private var revoked = false
    @Volatile private var reader: EskPlatformSellbackClient? = null
    private val gate = EskPlatformRequestGate()
    private val handler = Handler(Looper.getMainLooper())
    private lateinit var page: EskPlatformProgressConsentView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        startedAt = SystemClock.elapsedRealtime()
        if (savedInstanceState != null || !hasOfficialEskPlatformProgressCaller()) return cancelAndFinish()
        val request = readEskPlatformProgressRequest(intent) ?: return cancelAndFinish()
        nonce = request.getValue("nonce")
        cursor = request.getValue("cursor")
        page = EskPlatformProgressConsentView(this, ::cancelAndFinish)
        handler.postDelayed({ cancelAndFinish() }, EskPlatformProgressContract.REQUEST_WINDOW_MS)
        // Origin verification must precede even constructing the credential/session adapter.
        if (eskPlatformEndpoint(BuildConfig.SERVER_URL) == null) {
            return fail("当前主服务暂不支持安全资产读取。不会通过 HTTP 发送账户凭据；请返回量化应用稍后重试。")
        }
        val source = EskPlatformSessionStore(this) {
            revoked = true
            gate.invalidate()
            reader?.cancel()
            runOnUiThread { cancelAndFinish() }
        }.also { sessions = it }
        val captured = source.capture() ?: return fail("请先在主项目登录自己的账户，再从量化应用重新发起授权。")
        session = captured
        if (revoked) return cancelAndFinish()
        phase = Phase.CONFIRMING
        page.show(captured.displayName, request.getValue("cursor").isNotEmpty(), ::confirm)
    }

    private fun liveAuthorization(): Boolean {
        if (revoked || !foreground || isFinishing || isDestroyed || !hasOfficialEskPlatformProgressCaller()) return false
        val captured = session ?: return false
        val current = sessions?.capture()
        // Read clocks after potentially slow package-manager and session checks.
        return !revoked && captured.sameAs(current) && captured.validAt(System.currentTimeMillis()) &&
            EskPlatformProgressContract.validWindow(startedAt, SystemClock.elapsedRealtime())
    }

    private fun confirm() {
        if (phase != Phase.CONFIRMING) return
        if (!liveAuthorization()) return cancelAndFinish()
        val captured = session ?: return cancelAndFinish()
        val requestedCursor = cursor ?: return cancelAndFinish()
        val ticket = gate.begin(captured, SystemClock.elapsedRealtime(), System.currentTimeMillis(), foreground)
            ?: return cancelAndFinish()
        phase = Phase.READING
        page.loading()
        val source = EskPlatformSellbackClient().also { reader = it }
        handler.postDelayed({
            if (phase == Phase.READING && reader === source) fail("读取超时，没有返回进度。请返回量化应用重新发起授权。")
        }, EskPlatformRequestGate.MAX_REQUEST_MS)
        Thread({
            val result = runCatching { source.page(BuildConfig.SERVER_URL, requestedCursor.takeIf { it.isNotEmpty() }) { captured.token } }
            runOnUiThread {
                if (reader !== source || phase != Phase.READING) return@runOnUiThread
                if (!liveAuthorization() || !gate.consume(ticket, sessions?.capture(), SystemClock.elapsedRealtime(),
                        System.currentTimeMillis(), foreground)) return@runOnUiThread cancelAndFinish()
                result.fold(onSuccess = ::returnProgress, onFailure = {
                    if ((it as? SellbackNetworkException)?.failure == SellbackNetworkFailure.CONFLICT)
                        fail("额度或申请页已变化，没有返回进度。请返回量化应用，明确重新发起首页授权。")
                    else fail("未能读取有效的正式进度，没有返回数据。请核对登录和网络后从量化应用重新授权。")
                })
            }
        }, "esk-platform-progress-disclosure").start()
    }

    private fun returnProgress(value: SellbackPage) {
        if (phase != Phase.READING || !liveAuthorization()) return cancelAndFinish()
        val captured = session ?: return cancelAndFinish()
        val expectedNonce = nonce ?: return cancelAndFinish()
        val requestedCursor = cursor ?: return cancelAndFinish()
        val now = SystemClock.elapsedRealtime()
        val remaining = if (captured.expiresAtMillis == 0L) EskPlatformProgressContract.DISPLAY_WINDOW_MS
            else captured.expiresAtMillis - System.currentTimeMillis()
        val fields = runCatching {
            require(remaining > 0)
            val expires = Math.addExact(now, minOf(remaining, EskPlatformProgressContract.DISPLAY_WINDOW_MS))
            composeEskPlatformProgress(value, expectedNonce, requestedCursor, startedAt, now, expires)
        }.getOrNull() ?: return cancelAndFinish()
        if (!liveAuthorization()) return cancelAndFinish()
        // A slow final identity check must not turn an expired short-lived page into RESULT_OK.
        val result = runCatching {
            eskPlatformProgressResult(fields, expectedNonce, requestedCursor, startedAt, SystemClock.elapsedRealtime())
        }.getOrNull() ?: return cancelAndFinish()
        phase = Phase.FINISHED
        clearPrivateState()
        setResult(RESULT_OK, result)
        finish()
    }

    private fun fail(message: String) {
        if (phase == Phase.FINISHED) return
        phase = Phase.FAILED
        clearPrivateState()
        if (::page.isInitialized) page.unavailable(message)
    }

    private fun clearPrivateState() {
        revoked = true
        gate.invalidate()
        reader?.cancel()
        reader = null
        sessions?.close()
        sessions = null
        session = null
        nonce = null
        cursor = null
        handler.removeCallbacksAndMessages(null)
        if (::page.isInitialized) page.clear()
    }

    private fun cancelAndFinish() {
        if (phase == Phase.FINISHED) return
        phase = Phase.FINISHED
        clearPrivateState()
        setResult(RESULT_CANCELED)
        finish()
    }

    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        if (event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0) {
            cancelAndFinish()
            return true
        }
        return super.dispatchTouchEvent(event)
    }
    override fun onResume() {
        super.onResume()
        foreground = true
        if (phase in setOf(Phase.CONFIRMING, Phase.READING) && !liveAuthorization()) cancelAndFinish()
    }
    override fun onPause() { foreground = false; cancelAndFinish(); super.onPause() }
    override fun onStop() { cancelAndFinish(); super.onStop() }
    override fun onNewIntent(intent: Intent?) { super.onNewIntent(intent); cancelAndFinish() }
    override fun onSaveInstanceState(outState: Bundle) {
        cancelAndFinish()
        super.onSaveInstanceState(outState)
        outState.clear()
    }
    override fun onDestroy() {
        foreground = false
        phase = Phase.FINISHED
        clearPrivateState()
        super.onDestroy()
    }
}
