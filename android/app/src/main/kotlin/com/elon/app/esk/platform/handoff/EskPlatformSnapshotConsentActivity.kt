package com.elon.app.esk.platform.handoff

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.WindowManager
import com.elon.app.BuildConfig
import com.elon.app.esk.platform.EskPlatformAccount
import com.elon.app.esk.platform.EskPlatformAccountReader
import com.elon.app.esk.platform.EskPlatformRequestGate
import com.elon.app.esk.platform.EskPlatformSession
import com.elon.app.esk.platform.EskPlatformSessionStore
import com.elon.app.esk.platform.eskPlatformEndpoint
import com.elon.eskcontract.EskPlatformSnapshotContract

/** One native, user-confirmed formal-source disclosure. Not a login or funds authorization. */
class EskPlatformSnapshotConsentActivity : Activity() {
    private enum class Phase { NEW, CONFIRMING, READING, FAILED, FINISHED }
    private var phase = Phase.NEW
    private var foreground = false
    private var startedAt = 0L
    private var nonce: String? = null
    private var session: EskPlatformSession? = null
    private var sessions: EskPlatformSessionStore? = null
    @Volatile private var revoked = false
    @Volatile private var reader: EskPlatformAccountReader? = null
    private val gate = EskPlatformRequestGate()
    private val handler = Handler(Looper.getMainLooper())
    private lateinit var page: EskPlatformSnapshotConsentView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        startedAt = SystemClock.elapsedRealtime()
        if (savedInstanceState != null || !hasOfficialEskPlatformSnapshotCaller()) return cancelAndFinish()
        val request = readEskPlatformSnapshotRequest(intent) ?: return cancelAndFinish()
        nonce = request.getValue("nonce")
        page = EskPlatformSnapshotConsentView(this, ::cancelAndFinish)
        handler.postDelayed({ cancelAndFinish() }, EskPlatformSnapshotContract.REQUEST_WINDOW_MS)
        // Check transport before even constructing the session adapter or reading credentials.
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
        page.show(captured.displayName, ::confirm)
    }

    private fun liveAuthorization(): Boolean {
        val captured = session ?: return false
        return !revoked && foreground && !isFinishing && !isDestroyed &&
            EskPlatformSnapshotContract.validWindow(startedAt, SystemClock.elapsedRealtime()) &&
            captured.validAt(System.currentTimeMillis()) && captured.sameAs(sessions?.capture()) &&
            hasOfficialEskPlatformSnapshotCaller()
    }

    private fun confirm() {
        if (phase != Phase.CONFIRMING) return
        if (!liveAuthorization()) return cancelAndFinish()
        val captured = session ?: return cancelAndFinish()
        val ticket = gate.begin(captured, SystemClock.elapsedRealtime(), System.currentTimeMillis(), foreground)
            ?: return cancelAndFinish()
        phase = Phase.READING
        page.loading()
        val source = EskPlatformAccountReader().also { reader = it }
        handler.postDelayed({
            if (phase == Phase.READING && reader === source) fail("读取超时，没有返回摘要。请返回量化应用重新发起授权。")
        }, EskPlatformRequestGate.MAX_REQUEST_MS)
        Thread({
            val result = runCatching { source.fetch(BuildConfig.SERVER_URL) { captured.token } }
            runOnUiThread {
                if (reader !== source || phase != Phase.READING) return@runOnUiThread
                if (!liveAuthorization() || !gate.consume(ticket, sessions?.capture(), SystemClock.elapsedRealtime(),
                        System.currentTimeMillis(), foreground)) return@runOnUiThread cancelAndFinish()
                result.fold(onSuccess = ::returnSnapshot, onFailure = {
                    fail("未能读取有效的正式登记，没有返回摘要。请确认登录和网络后从量化应用重试。")
                })
            }
        }, "esk-platform-disclosure").start()
    }

    private fun returnSnapshot(account: EskPlatformAccount) {
        if (phase != Phase.READING || !liveAuthorization()) return cancelAndFinish()
        val captured = session ?: return cancelAndFinish()
        val expectedNonce = nonce ?: return cancelAndFinish()
        val now = SystemClock.elapsedRealtime()
        val epoch = System.currentTimeMillis()
        val remaining = if (captured.expiresAtMillis == 0L) EskPlatformSnapshotContract.DISPLAY_WINDOW_MS
            else captured.expiresAtMillis - epoch
        val result = runCatching {
            require(remaining > 0)
            val expires = Math.addExact(now, minOf(remaining, EskPlatformSnapshotContract.DISPLAY_WINDOW_MS))
            val fields = composeEskPlatformSnapshot(account, expectedNonce, startedAt, now, expires)
            eskPlatformSnapshotResult(fields, expectedNonce, startedAt, SystemClock.elapsedRealtime())
        }.getOrNull() ?: return cancelAndFinish()
        // Recheck OS caller and atomic auth revision immediately before the one-shot result.
        if (!liveAuthorization()) return cancelAndFinish()
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

    override fun onResume() {
        super.onResume()
        foreground = true
        if (phase in setOf(Phase.CONFIRMING, Phase.READING) && !liveAuthorization()) cancelAndFinish()
    }

    override fun onPause() {
        foreground = false
        cancelAndFinish()
        super.onPause()
    }

    override fun onStop() {
        cancelAndFinish()
        super.onStop()
    }

    override fun onNewIntent(intent: Intent?) {
        super.onNewIntent(intent)
        cancelAndFinish()
    }

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
