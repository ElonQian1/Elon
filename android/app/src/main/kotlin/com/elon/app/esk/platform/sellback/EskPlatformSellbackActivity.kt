package com.elon.app.esk.platform.sellback

import android.app.Activity
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.WindowManager
import com.elon.app.BuildConfig
import com.elon.app.esk.platform.EskPlatformRequestGate
import com.elon.app.esk.platform.EskPlatformSession
import com.elon.app.esk.platform.EskPlatformSessionStore
import com.elon.app.esk.platform.eskPlatformEndpoint
import java.util.UUID

/** Independent native requests. No Intent credentials, IPC authority, Paper balance, or settlement. */
class EskPlatformSellbackActivity : Activity() {
    private val gate = EskPlatformRequestGate()
    private val state = EskPlatformSellbackState()
    private val handler = Handler(Looper.getMainLooper())
    private var foreground = false
    private var sessions: EskPlatformSessionStore? = null
    private var owner: EskPlatformSession? = null
    @Volatile private var client: EskPlatformSellbackClient? = null
    private lateinit var view: EskPlatformSellbackView
    private var summary: SellbackSummary? = null
    private var records: List<SellbackRecord> = emptyList()
    private var position: SellbackPage? = null
    private var shownAt = -1L
    // Non-private warning only. A pause can cancel transport, never prove a business rollback.
    private var reviewIdentity: SellbackReviewIdentity? = null
    private val needsReview: Boolean get() = reviewIdentity != null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        view = EskPlatformSellbackView(this, ::finish, { refresh(false) }, ::next,
            ::submit, ::retry, ::reviewed, ::cancel)
    }
    override fun onResume() {
        super.onResume(); foreground = true
        refresh(true)
    }

    private fun session(): EskPlatformSession? {
        if (!foreground || isFinishing || isDestroyed) return null
        // This guard must precede SessionStore construction AND every credential capture.
        if (eskPlatformEndpoint(BuildConfig.SERVER_URL) == null) {
            clearPrivateState()
            view.unavailable("当前服务未提供安全连接。不会通过 HTTP 发送账户凭据，申请与数量暂不可用。")
            return null
        }
        if (sessions == null) sessions = EskPlatformSessionStore(this) {
            gate.invalidate(); client?.cancel()
            runOnUiThread { invalidateSession() }
        }
        val captured = sessions?.capture()
        if (captured == null || (owner != null && owner?.sameAs(captured) != true)) {
            invalidateSession(); return null
        }
        owner = captured
        if (reviewIdentity?.belongsTo(captured) == false) reviewIdentity = null
        if (EskPlatformSellbackRecovery.current(captured) != null) reviewIdentity = SellbackReviewIdentity(captured)
        return captured
    }

    private fun refresh(recover: Boolean) {
        if (client != null) return
        val session = session() ?: return
        val hint = if (recover) EskPlatformSellbackRecovery.current(session) else null
        if (hint != null) {
            request(session, null, { source, token -> source.lookupKey(BuildConfig.SERVER_URL, hint.key, token) }) {
                accept(it, session)
            }
        } else request(session, null, { source, token -> source.page(BuildConfig.SERVER_URL, null, token) }) {
            accept(it, session)
        }
    }

    private fun next() {
        val session = freshDisplay() ?: return
        val previous = position ?: return
        val cursor = previous.nextCursor ?: return
        // Keep only a quota identity, position and one anchor, never accumulate prior pages.
        val expected = previous.summary
        val end = previous.end
        val last = previous.requests.lastOrNull() ?: return
        request(session, null, { source, token -> source.page(BuildConfig.SERVER_URL, cursor, token) }) {
            val first = it.requests.firstOrNull()
            if (it.summary != expected || it.start != end + 1 || first == null ||
                !(last.created > first.created || (last.created == first.created && last.id > first.id))) {
                clearDisplay(); view.unavailable("申请快照已变化，原页已清除。请重新读取本人申请。")
            } else accept(it, session)
        }
    }

    private fun submit(raw: String) {
        val session = freshDisplay() ?: return
        if (blocked(session)) return
        val current = summary ?: return
        val amount = sellbackInput(raw)
        val action = amount?.let { runCatching { SellbackAction.submit(current, it, UUID.randomUUID().toString()) }.getOrNull() }
        if (action == null) { view.message("请输入符合当前条款和可申请量的 ESK 数量，最多六位小数。"); return }
        val draft = state.prepare(action, session, elapsed(), epoch(), foreground) ?: return
        confirm(draft, false)
    }
    private fun cancel(record: SellbackRecord) {
        val session = freshDisplay() ?: return
        if (blocked(session) || records.none { it === record }) return
        val action = runCatching { SellbackAction.cancel(record) }.getOrNull() ?: return
        val draft = state.prepare(action, session, elapsed(), epoch(), foreground) ?: return
        confirm(draft, false)
    }
    private fun retry() {
        if (client != null) return
        val session = session() ?: return
        val draft = state.retry(session, elapsed(), epoch(), foreground) ?: return
        confirm(draft, true)
    }
    private fun confirm(draft: EskPlatformSellbackState.Draft, retrying: Boolean) {
        view.confirm(draft.action, draft.session.displayName, retrying, confirmed = confirmation@{
            val current = session()
            val ticket = state.confirm(draft, current, elapsed(), epoch(), foreground)
            if (ticket == null || current == null) {
                clearDisplay(); view.unavailable("确认已失效。请核对当前账户并重新读取。"); return@confirmation
            }
            reviewIdentity = SellbackReviewIdentity(current)
            EskPlatformSellbackRecovery.remember(draft.action, current)
            request(current, ticket, { source, token -> source.execute(BuildConfig.SERVER_URL, draft.action, token) }) {
                if (state.complete(ticket)) {
                    EskPlatformSellbackRecovery.clear(); reviewIdentity = null
                    accept(it, current)
                }
            }
        }, dismissed = { state.dismiss(draft) })
    }

    private fun reviewed() {
        val current = freshDisplay() ?: return
        if (!blocked(current)) return
        view.confirmReviewed {
            val again = freshDisplay()
            if (again != null && current.sameAs(again)) {
                // Explicit acknowledgement, NOT proof that an earlier request was never executed.
                state.clear(); EskPlatformSellbackRecovery.clear(); reviewIdentity = null
                refresh(false)
            }
        }
    }

    private fun <T> request(session: EskPlatformSession, mutation: EskPlatformSellbackState.Ticket?,
        operation: (EskPlatformSellbackClient, () -> String) -> T, accepted: (T) -> Unit) {
        if (client != null) return
        val adapter = sessions ?: return
        val ticket = gate.begin(session, elapsed(), epoch(), foreground) ?: return
        handler.removeCallbacksAndMessages(null)
        clearDisplay()
        val source = EskPlatformSellbackClient().also { client = it }
        view.loading()
        handler.postDelayed({
            if (client === source) failed(source, mutation, null)
        }, EskPlatformRequestGate.MAX_REQUEST_MS)
        Thread({
            val result = runCatching { operation(source) {
                // Close/account-change invalidates the adapter before a late worker reads its token.
                val current = adapter.capture()
                if (!session.sameAs(current)) throw SellbackNetworkException(SellbackNetworkFailure.SIGN_IN_REQUIRED)
                requireNotNull(current).token
            } }
            runOnUiThread {
                if (client !== source || !foreground || isFinishing || isDestroyed) return@runOnUiThread
                val current = adapter.capture()
                if (!session.sameAs(current) || current?.validAt(epoch()) != true) {
                    invalidateSession(); return@runOnUiThread
                }
                if (!gate.consume(ticket, current, elapsed(), epoch(), foreground)) {
                    failed(source, mutation, null); return@runOnUiThread
                }
                result.fold(onSuccess = {
                    client = null; handler.removeCallbacksAndMessages(null)
                    accepted(it)
                }, onFailure = { failed(source, mutation, it) })
            }
        }, "esk-platform-sellback").start()
    }

    private fun failed(source: EskPlatformSellbackClient, mutation: EskPlatformSellbackState.Ticket?, error: Throwable?) {
        if (client !== source) return
        source.cancel(); client = null; gate.invalidate()
        handler.removeCallbacksAndMessages(null); clearDisplay()
        if (mutation != null) { state.unknown(mutation); owner?.let { reviewIdentity = SellbackReviewIdentity(it) } }
        val failure = (error as? SellbackNetworkException)?.failure
        if (failure == SellbackNetworkFailure.SIGN_IN_REQUIRED) {
            invalidateSession(); return
        }
        view.unavailable(if (mutation != null || needsReview || state.unresolved()) UNKNOWN else
            "未能读取有效的本人申请，原数量已清除。请检查连接后重新读取；申请变化时需从第一页开始。",
            canRetry = state.unresolved())
        armExpiry()
    }

    private fun accept(page: SellbackPage, session: EskPlatformSession) {
        resolve(page.requests, session)
        summary = page.summary; records = page.requests; position = page; shownAt = elapsed()
        view.page(page, session.displayName, blocked(session), state.unresolved())
        armExpiry()
    }
    private fun accept(result: SellbackResult, session: EskPlatformSession) {
        resolve(listOf(result.request), session)
        summary = result.summary; records = listOf(result.request); position = null; shownAt = elapsed()
        view.receipt(result, session.displayName, blocked(session), state.unresolved())
        armExpiry()
    }
    private fun resolve(records: List<SellbackRecord>, session: EskPlatformSession) {
        val exact = state.resolve(records)
        val hint = EskPlatformSellbackRecovery.resolve(session, records)
        if (exact || hint) reviewIdentity = null
    }
    private fun blocked(session: EskPlatformSession) = needsReview || state.unresolved() ||
        EskPlatformSellbackRecovery.current(session) != null
    private fun freshDisplay(): EskPlatformSession? {
        if (client != null) return null
        val current = session() ?: return null
        val now = elapsed()
        if (summary == null || shownAt < 0 || now < shownAt || now - shownAt >= DISPLAY_MS) {
            clearDisplay(); view.unavailable("本次数量已到期，请先重新读取本人记录。"); return null
        }
        return current
    }
    private fun armExpiry() {
        handler.removeCallbacksAndMessages(null)
        val untilExpiry = owner?.expiresAtMillis?.takeIf { it != 0L }?.let { (it - epoch()).coerceAtLeast(0L) } ?: DISPLAY_MS
        handler.postDelayed({
            if (state.clear()) owner?.let { reviewIdentity = SellbackReviewIdentity(it) }
            clearDisplay()
            view.unavailable(if (needsReview) UNKNOWN else "本次查看已到期，数量已清除。请重新读取本人申请。")
        }, minOf(untilExpiry, DISPLAY_MS))
    }
    private fun clearDisplay() {
        summary = null; records = emptyList(); position = null; shownAt = -1L
        if (::view.isInitialized) view.clear()
    }
    private fun invalidateSession() {
        val unknown = needsReview || state.unresolved()
        clearPrivateState(); EskPlatformSellbackRecovery.clear(); reviewIdentity = null
        if (foreground && !isFinishing && !isDestroyed) view.unavailable(
            "登录或账户状态已变化，旧数量和操作权限已清除。请重新登录后核对本人记录。" +
                if (unknown) "先前操作结果未知，不能据此认为未执行。" else "")
    }
    private fun clearPrivateState() {
        gate.invalidate(); client?.cancel(); client = null
        if (state.clear()) owner?.let { reviewIdentity = SellbackReviewIdentity(it) }
        sessions?.close(); sessions = null; owner = null
        handler.removeCallbacksAndMessages(null); clearDisplay()
    }
    override fun onPause() { foreground = false; clearPrivateState(); super.onPause() }
    override fun onStop() { clearPrivateState(); super.onStop() }
    override fun onSaveInstanceState(outState: Bundle) {
        clearPrivateState(); super.onSaveInstanceState(outState); outState.clear()
    }
    override fun onDestroy() { foreground = false; clearPrivateState(); super.onDestroy() }
    private fun elapsed() = SystemClock.elapsedRealtime()
    private fun epoch() = System.currentTimeMillis()
    companion object {
        private const val DISPLAY_MS = 60_000L
        private const val UNKNOWN = "操作结果未知。超时、断线或取消连接不代表服务器未执行；请查回并核对本人记录，不要重复新建。"
    }
}
