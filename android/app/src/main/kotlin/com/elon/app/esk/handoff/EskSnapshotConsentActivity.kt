package com.elon.app.esk.handoff

import android.app.Activity
import android.content.Intent
import android.content.SharedPreferences
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.WindowManager
import com.elon.app.AuthManager
import com.elon.app.BuildConfig
import com.elon.eskcontract.EskSnapshotContract

/** Ephemeral, explicitly consented Activity result; never a background authorization service. */
class EskSnapshotConsentActivity : Activity() {
    private enum class Phase { NEW, CONFIRMING, READING, FAILED, FINISHED }
    private var phase = Phase.NEW
    private var foreground = false
    private var startedAt = 0L
    private var nonce: String? = null
    private var capturedToken: String? = null
    private var capturedUserId: String? = null
    private var capturedExpiry = 0L
    private var prefs: SharedPreferences? = null
    private var reader: EskSnapshotHttpsReader? = null
    private val handler = Handler(Looper.getMainLooper())
    private val timeout = Runnable { cancelAndFinish() }
    private lateinit var page: EskSnapshotConsentView
    private val sessionListener = SharedPreferences.OnSharedPreferenceChangeListener { _, key ->
        if (key == null || key.startsWith("auth_")) runOnUiThread { cancelAndFinish() }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        startedAt = SystemClock.elapsedRealtime()
        if (savedInstanceState != null || !hasOfficialEskSnapshotCaller()) return cancelAndFinish()
        val request = readEskSnapshotRequest(intent) ?: return cancelAndFinish()
        nonce = request.getValue("nonce")
        page = EskSnapshotConsentView(this, ::cancelAndFinish)
        handler.postDelayed(timeout, EskSnapshotContract.REQUEST_WINDOW_MS)
        // This guard MUST precede every session/token read. Existing HTTP main APIs are not reused.
        if (eskSnapshotEndpoint(BuildConfig.SERVER_URL) == null) {
            return showFailure("当前主服务暂不支持安全资产读取。不会通过 HTTP 发送账户凭据；请返回量化应用，稍后重试。")
        }
        try {
            prefs = AuthManager.prefs(this).also { it.registerOnSharedPreferenceChangeListener(sessionListener) }
            capturedToken = AuthManager.token(this)
            capturedUserId = AuthManager.userId(this)
            capturedExpiry = prefs!!.getLong("auth_expires_at", 0L)
            if (!sameSession()) return showFailure("请先在主项目登录自己的账户，再从量化应用重新发起授权。")
            phase = Phase.CONFIRMING
            page.show(AuthManager.displayName(this), confirm = ::confirm)
        } catch (_: Exception) {
            showFailure("当前账户无法确认。请返回量化应用重新发起授权。")
        }
    }

    private fun sameSession(): Boolean = runCatching {
        val token = capturedToken ?: return false
        val userId = capturedUserId ?: return false
        val source = prefs ?: return false
        token == AuthManager.token(this) && userId == AuthManager.userId(this) &&
            capturedExpiry == source.getLong("auth_expires_at", 0L) &&
            (capturedExpiry == 0L || capturedExpiry > System.currentTimeMillis())
    }.getOrDefault(false)

    private fun liveAuthorization(): Boolean = foreground &&
        EskSnapshotContract.validWindow(startedAt, SystemClock.elapsedRealtime()) &&
        hasOfficialEskSnapshotCaller() && sameSession()

    private fun confirm() {
        if (phase != Phase.CONFIRMING) return
        if (!liveAuthorization()) return cancelAndFinish()
        phase = Phase.READING
        page.loading()
        val token = capturedToken ?: return cancelAndFinish()
        val source = EskSnapshotHttpsReader().also { reader = it }
        Thread({
            val result = runCatching { source.fetch(BuildConfig.SERVER_URL) { token } }
            runOnUiThread {
                if (phase != Phase.READING || !liveAuthorization()) {
                    if (phase != Phase.FINISHED) cancelAndFinish()
                    return@runOnUiThread
                }
                val fields = result.getOrNull()
                if (fields == null) showFailure("未能安全读取有效的资产快照。没有返回余额，请返回量化应用重试。")
                else returnSnapshot(fields)
            }
        }, "esk-snapshot-read").start()
    }

    private fun returnSnapshot(account: Map<String, String>) {
        if (phase != Phase.READING || !liveAuthorization()) return cancelAndFinish()
        val expectedNonce = nonce ?: return cancelAndFinish()
        val now = SystemClock.elapsedRealtime()
        val result = runCatching {
            val fields = account + mapOf("protocol" to EskSnapshotContract.PROTOCOL, "nonce" to expectedNonce,
                "observed_elapsed_ms" to now.toString(),
                "expires_elapsed_ms" to Math.addExact(now, EskSnapshotContract.DISPLAY_WINDOW_MS).toString())
            eskSnapshotResult(fields, expectedNonce, startedAt, now)
        }.getOrNull() ?: return cancelAndFinish()
        // Re-read current OS identity and login immediately before issuing the one-shot result.
        if (!liveAuthorization()) return cancelAndFinish()
        phase = Phase.FINISHED
        clearPrivateState()
        setResult(RESULT_OK, result)
        finish()
    }

    private fun showFailure(message: String) {
        phase = Phase.FAILED
        clearPrivateState()
        if (::page.isInitialized) page.show(null, error = message)
    }

    private fun clearPrivateState() {
        reader?.cancel()
        reader = null
        prefs?.unregisterOnSharedPreferenceChangeListener(sessionListener)
        prefs = null
        capturedToken = null
        capturedUserId = null
        capturedExpiry = 0L
        nonce = null
    }

    private fun cancelAndFinish() {
        if (phase == Phase.FINISHED) return
        phase = Phase.FINISHED
        setResult(RESULT_CANCELED)
        clearPrivateState()
        handler.removeCallbacks(timeout)
        finish()
    }

    override fun onResume() {
        super.onResume()
        foreground = true
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
        handler.removeCallbacksAndMessages(null)
        super.onDestroy()
    }
}
