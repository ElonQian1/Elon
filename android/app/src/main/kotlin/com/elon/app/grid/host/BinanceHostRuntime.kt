package com.elon.app.grid.host

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.webkit.WebView
import com.elon.app.WebBridgeDocumentSession
import com.elon.app.esk.platform.EskPlatformSession
import com.elon.app.esk.platform.EskPlatformSessionStore
import com.elon.app.privateaccess.StrictJson
import java.util.concurrent.FutureTask
import java.util.concurrent.TimeUnit

/** One main-process host; a new process requires a new grant, while website storage stays local. */
internal class BinanceHostRuntime private constructor(private val context: Context) {
    val handler = Handler(Looper.getMainLooper())
    val state = BinanceHostState(SystemClock::elapsedRealtime, System::currentTimeMillis)
    val reports = BinanceGridReports(SystemClock::elapsedRealtime, System::currentTimeMillis)
    val document = WebBridgeDocumentSession()
    val diagnostics = BinanceHostDiagnostics()
    var view: WebView? = null; private set
    var status = "请先连接币安"; private set
    var pagePhase = "not_started"; private set
    var adapterBound = false; private set
    var onChanged: (() -> Unit)? = null
    var onCreateObservation: ((String) -> Unit)? = null
    private var captured: EskPlatformSession? = null
    private val consent = BinanceHostConsent(context)
    private val sessions = EskPlatformSessionStore(context) { handler.post { consent.clear(); invalidate("主账号已变化，请重新连接") } }
    private val preferences = context.getSharedPreferences("binance_host_owner_v1", Context.MODE_PRIVATE)
    private var deadline = 0L
    private var resumeToken: String? = null
    private var lastResumeRefresh = 0L
    private val expiry = Runnable { invalidate("连接已结束，请重新授权") }

    fun begin(): Boolean {
        check(Looper.myLooper() == Looper.getMainLooper())
        val next = sessions.capture() ?: return reject("请先在主应用登录本人账户")
        val owner = BinanceHostState.digest(next.userId)
        val bound = preferences.getString("owner", null)
        if (bound != null && bound != owner) return reject("币安会话绑定了另一个一龙账号，请切回原账号使用")
        if (captured?.sameAs(next) != true) invalidate("正在准备本人币安连接")
        captured = next
        preferences.edit().putString("owner", owner).apply()
        deadline = SystemClock.elapsedRealtime() + 900_000
        armExpiry()
        if (view == null) view = createBinanceHostWebView(context, this).also { it.loadUrl(ENTRY) }
        return true
    }
    fun live(): Boolean = captured?.let { it.sameAs(sessions.capture()) && it.validAt(System.currentTimeMillis()) } == true &&
        SystemClock.elapsedRealtime() < deadline
    fun keepAlive(): Boolean {
        if (!live()) return false
        deadline = SystemClock.elapsedRealtime() + 900000; armExpiry(); return true
    }
    /** Restore the internal document without requiring a product Activity or minting a read grant. */
    fun recoverConnection() {
        if (!live() && !begin()) return
        if (state.fresh() || !adapterBound) return
        val now = SystemClock.elapsedRealtime()
        if (now - lastResumeRefresh > 10_000) {
            lastResumeRefresh = now
            view?.evaluateJavascript("window.__elonBinanceReadV1?.refresh()", null)
        }
    }
    private fun owner() = captured?.userId?.let(BinanceHostState::digest)
    fun grant(continuous: Boolean = false): String {
        require(live())
        require(state.fresh())
        if (continuous) consent.approve(owner() ?: error("OWNER_MISSING"), state.account ?: error("ACCOUNT_MISSING"), state.accountKind)
        val token = state.grant(continuous)
        if (continuous) resumeToken = token
        deadline = SystemClock.elapsedRealtime() + 900_000
        armExpiry(); return token
    }
    fun resume(): android.os.Bundle {
        fun status(value: String) = android.os.Bundle().apply { putString("status", value) }
        if (!consent.recorded()) return status("consent_required")
        if (!begin()) return status("login_required")
        if (state.account == null || !state.fresh()) {
            val now = SystemClock.elapsedRealtime()
            if (adapterBound && now - lastResumeRefresh > 10_000) {
                lastResumeRefresh = now
                view?.evaluateJavascript("window.__elonBinanceReadV1?.refresh()", null)
            }
            return status("pending")
        }
        if (!consent.permits(owner(), state.account, state.accountKind)) {
            consent.clear(); return status("consent_required")
        }
        val token = resumeToken?.takeIf(state::authorized) ?: state.grant(continuous = true).also { resumeToken = it }
        return status("ready").apply { putString("grant", token) }
    }
    fun readContinuous(token: String): String {
        require(live() && consent.permits(owner(), state.account, state.accountKind))
        state.renew(token)
        deadline = SystemClock.elapsedRealtime() + 900_000; armExpiry()
        return state.reply(token, v2 = true)
    }
    fun disconnect() { consent.clear(); invalidate("已断开量化只读连接，币安登录资料保留") }
    private fun armExpiry() {
        handler.removeCallbacks(expiry)
        handler.postDelayed(expiry, (deadline - SystemClock.elapsedRealtime()).coerceAtLeast(0))
    }
    fun pageStarted(url: String) {
        document.beginPage()
        state.unavailable()
        reports.clear()
        diagnostics.clear()
        pagePhase = "loading"; adapterBound = false
        status = if (url.startsWith("https://accounts.binance.com/")) "请在币安官方页面登录或验证" else "正在等待币安网格响应"
        onChanged?.invoke()
    }
    fun pageReady(url: String, finished: Boolean = true) {
        if (!live() || !url.startsWith("$ORIGIN/")) return
        pagePhase = if (finished) "finished" else "visible"
        val token = document.ensurePage().documentToken
        view?.evaluateJavascript("window.__elonBinanceReadV1?.bind(${StrictJson.encode(token)})") { value ->
            if (value == "true" && live() && document.snapshot().documentToken == token) {
                adapterBound = true; inspectPage()
            }
        }
    }
    fun inspectPage() {
        if (!live() || !adapterBound) return
        val token = document.snapshot().documentToken
        view?.evaluateJavascript("window.__elonBinanceDiagnosticsV1?.inspect(${StrictJson.encode(token)})", null)
    }
    fun observed(raw: String) {
        if (!live()) return invalidate("登录或授权已失效")
        val event = runCatching { StrictJson.parse(raw) }.getOrNull() ?: return fail("响应格式暂不支持")
        if (event["schema"] == "yilong.binance_report_observation.v1") {
            if (document.accept(event["token"] as? String ?: "") != null) {
                runCatching { reports.accept(event) }.onFailure { reports.fail(event["request"] as? String ?: "") }
            }
            return
        }
        if (event["schema"] == "yilong.binance_diagnostic.v1") {
            if (document.accept(event["token"] as? String ?: "") != null) diagnostics.accept(event)
            return
        }
        if (event["schema"] != "yilong.binance_observation.v1" || document.accept(event["token"] as? String ?: "") == null) return
        val previousAccount = state.account
        runCatching { state.accept(raw) }.onFailure { fail("未取得可验证的本人网格响应，请在官网打开网格列表") }
            .onSuccess {
                if(previousAccount != state.account) reports.clear()
                if (state.account != null && consent.recorded() && !consent.permits(owner(), state.account, state.accountKind)) consent.clear()
                val label = when (state.accountKind) { "sub" -> "币安子账户"; "primary" -> "币安主账户"; else -> "当前币安账户" }
                status = if (state.ready) "$label · 已读取 ${state.count} 条网格；仅代表本次页面响应"
                    else if (state.account != null) "已确认$label，等待网格列表"
                    else "币安响应暂不可用，请重新连接"
                onChanged?.invoke()
            }
    }
    fun read(token: String): String { require(live()); return state.reply(token) }
    fun refresh(token: String) {
        require(live() && state.authorized(token))
        val documentToken = document.ensurePage().documentToken
        view?.evaluateJavascript("window.__elonBinanceReadV1?.refresh()") { value ->
            if (document.accept(documentToken) != null && live() && value != "true") {
                status = "尚未取得官网列表请求，请打开官网网格列表后重试"; onChanged?.invoke()
            }
        }
    }
    fun detail(token: String, id: String) {
        require(live() && state.authorized(token) && state.contains(id))
        val page = view ?: error("HOST_MISSING")
        val documentToken = document.ensurePage().documentToken
        page.evaluateJavascript("window.__elonBinanceReadV1?.detail(${StrictJson.encode(id)})") { value ->
            if (document.accept(documentToken) != null && live() && state.authorized(token) && value != "true") {
                fail("详情连接尚未就绪，请重新连接币安")
            }
        }
    }
    fun revoke(token: String) { state.revoke(token) }
    fun reportRequest(token: String, raw: String) {
        readContinuous(token)
        require(live() && state.authorized(token) && state.fresh())
        val q = BinanceReportQuery.parse(raw)
        require(q.kind == "history" || state.contains(q.id) || reports.contains(q.id, q.symbol))
        reports.start(q, state.account ?: error("ACCOUNT_MISSING"), state.accountKind)
        val doc = document.ensurePage().documentToken
        view?.evaluateJavascript("window.__elonBinanceReadV1?.report(${q.json()})") { result ->
            if (document.accept(doc) != null && result != "true") reports.fail(q.request)
        } ?: reports.fail(q.request)
    }
    fun reportRead(token: String, request: String): String {
        readContinuous(token)
        return reports.reply(request, state.account, state.accountKind)
    }
    fun fail(message: String) { state.unavailable(); status = message; onChanged?.invoke() }
    fun invalidate(message: String) {
        captured = null; state.unavailable(); reports.clear(); handler.removeCallbacks(expiry)
        diagnostics.clear()
        pagePhase = "closed"; adapterBound = false
        view?.let { (it.parent as? android.view.ViewGroup)?.removeView(it); it.stopLoading(); it.destroy() }
        view = null; status = message; onChanged?.invoke()
    }
    private fun reject(message: String): Boolean { invalidate(message); return false }
    companion object {
        const val ORIGIN = "https://www.binance.com"
        const val ENTRY = "$ORIGIN/zh-CN/trading-bots/futures/grid/NEARUSDT"
        @Volatile private var instance: BinanceHostRuntime? = null
        fun <T> onMain(context: Context, action: (BinanceHostRuntime) -> T): T {
            val work = { action(instance ?: BinanceHostRuntime(context.applicationContext).also { instance = it }) }
            if (Looper.myLooper() == Looper.getMainLooper()) return work()
            val task = FutureTask(work)
            Handler(Looper.getMainLooper()).post(task)
            return try { task.get(3, TimeUnit.SECONDS) } finally { if (!task.isDone) task.cancel(false) }
        }
    }
}
