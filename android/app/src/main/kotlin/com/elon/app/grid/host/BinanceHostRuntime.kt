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
    val document = WebBridgeDocumentSession()
    var view: WebView? = null; private set
    var status = "请先连接币安"; private set
    var onChanged: (() -> Unit)? = null
    private var captured: EskPlatformSession? = null
    private val sessions = EskPlatformSessionStore(context) { handler.post { invalidate("主账号已变化，请重新连接") } }
    private val preferences = context.getSharedPreferences("binance_host_owner_v1", Context.MODE_PRIVATE)
    private var deadline = 0L
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
    fun grant(): String {
        require(live())
        val token = state.grant()
        deadline = SystemClock.elapsedRealtime() + 900_000
        armExpiry(); return token
    }
    private fun armExpiry() {
        handler.removeCallbacks(expiry)
        handler.postDelayed(expiry, (deadline - SystemClock.elapsedRealtime()).coerceAtLeast(0))
    }
    fun pageStarted(url: String) {
        document.beginPage()
        state.unavailable()
        status = if (url.startsWith("https://accounts.binance.com/")) "请在币安官方页面登录或验证" else "正在等待币安网格响应"
        onChanged?.invoke()
    }
    fun pageReady(url: String) {
        if (!live() || !url.startsWith("$ORIGIN/")) return
        val token = document.ensurePage().documentToken
        view?.evaluateJavascript("window.__elonBinanceReadV1?.bind(${StrictJson.encode(token)})", null)
    }
    fun observed(raw: String) {
        if (!live()) return invalidate("登录或授权已失效")
        val event = runCatching { StrictJson.parse(raw) }.getOrNull() ?: return fail("响应格式暂不支持")
        if (event["schema"] != "yilong.binance_observation.v1" || document.accept(event["token"] as? String ?: "") == null) return
        runCatching { state.accept(raw) }.onFailure { fail("未取得可验证的本人网格响应，请在官网打开网格列表") }
            .onSuccess {
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
        // Do not guess the undocumented list POST body. The website owns its query.
        // A reload changes document generation and requires a fresh authorization.
        view?.reload()
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
    fun fail(message: String) { state.unavailable(); status = message; onChanged?.invoke() }
    fun invalidate(message: String) {
        captured = null; state.unavailable(); handler.removeCallbacks(expiry)
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
