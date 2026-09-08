package com.elon.app.grid.create

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.os.Bundle
import android.os.SystemClock
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.*
import com.elon.app.grid.host.BinanceHostCaller
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.ui.BinanceGridAppearance

/** The only native create submission entry: authenticated caller, visible final confirmation, one attempt. */
class BinanceGridCreateActivity : Activity() {
    private val ui by lazy { BinanceGridAppearance(this) }
    private var host: BinanceHostRuntime? = null
    private var session: BinanceCreateSession? = null
    private val attempt = BinanceCreateAttempt(SystemClock::elapsedRealtime)
    private var nonce = ""
    private var recoveryBlocked = false
    private var managementPending = false
    private var recordResolved = false
    private var ready = false
    private var ownsSlot = false
    private lateinit var status: TextView
    private lateinit var summary: TextView
    private lateinit var form: BinanceCreateForm
    private lateinit var confirmed: CheckBox
    private lateinit var check: Button
    private lateinit var submit: Button
    private lateinit var detail: Button
    private lateinit var resolved: Button
    private lateinit var official: FrameLayout
    private val journal by lazy { BinanceCreateJournal(this) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState); setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        if (savedInstanceState != null || !BinanceHostCaller.activity(this) || intent.data != null || intent.clipData != null ||
            intent.selector != null || intent.extras?.keySet() != setOf("nonce")) return finish()
        nonce = intent.getStringExtra("nonce")?.takeIf { Regex("[0-9a-f]{64}").matches(it) } ?: return finish()
        if (!slot.acquire(this)) return finish()
        ownsSlot = true
        runCatching { journal.read()?.let(attempt::restore) }.onFailure { recoveryBlocked = true }
        managementPending = runCatching { BinanceCreateJournal(this,"binance-manage-attempt-v1.json").read() != null }.getOrDefault(true)
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL; setPadding(ui.dp(18),ui.dp(12),ui.dp(18),ui.dp(8))
            setBackgroundColor(ui.background)
        }
        root.addView(label("创建本人币安 U 本位网格",21f))
        status = label("正在连接本人币安",15f).apply { contentDescription = "binance-create-status" }; root.addView(status)
        val content = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; isSaveEnabled = false }
        content.addView(button("重新加载官网／确认登录", "binance-create-reload") {
            if (attempt.status == "submitting") return@button
            session?.cancelPreparation(); confirmed.isChecked = false
            official.visibility = View.VISIBLE; attachHost(reload = true)
        })
        content.addView(button("查看／收起币安官网", "binance-create-official") {
            official.visibility = if (official.visibility == View.VISIBLE) View.GONE else View.VISIBLE
            if (official.visibility == View.VISIBLE && host?.view == null) attachHost()
        })
        official = FrameLayout(this).apply { visibility = View.GONE; setBackgroundColor(ui.surface) }
        content.addView(official, LinearLayout.LayoutParams(-1, ui.dp(520)))
        content.addView(label("填写网格参数", 19f))
        form = BinanceCreateForm(this) { if (ready && !attempt.unresolved) { session?.cancelPreparation(); confirmed.isChecked = false; render() } }
        content.addView(form.root)
        check = button("检查参数与当前账号", "binance-create-prepare") {
            confirmed.isChecked = false
            runCatching { session?.prepare(form.draft()) ?: error("官网连接尚未就绪") }.onFailure { status.text = it.message ?: "检查未完成" }
        }; content.addView(check)
        summary = label("",16f); content.addView(summary)
        confirmed = CheckBox(this).apply {
            text = "我已核对本次账号和全部参数，使用本人资金进行真实测试；本次未设置止损。"
            isSaveEnabled = false; filterTouchesWhenObscured = true
            setTextColor(ui.text); buttonTintList = android.content.res.ColorStateList.valueOf(ui.accent)
            contentDescription = "binance-create-confirm-parameters"
            setOnCheckedChangeListener { _, _ -> if (ready) render() }
        }; content.addView(confirmed)
        submit = button("确认并创建真实网格", "binance-create-submit") {
            if (!hasWindowFocus() || !BinanceHostCaller.activity(this) || !confirmed.isChecked) return@button
            runCatching { session?.submit() ?: error("连接未就绪") }.onFailure { status.text = it.message ?: "提交未完成" }
        }; content.addView(submit)
        detail = button("查询本次创建结果", "binance-create-result") {
            runCatching { session?.detail() ?: error("连接未就绪") }.onFailure { status.text = it.message ?: "读取未完成" }
        }; content.addView(detail)
        resolved = button("我已在官网核对，结束本次记录", "binance-create-resolve") { acknowledge() }; content.addView(resolved)
        root.addView(ScrollView(this).apply { isSaveEnabled = false; addView(content) },LinearLayout.LayoutParams(-1,0,1f))
        root.addView(button("返回量化应用", "binance-create-return") { returnResult() })
        setContentView(root); ready = true; attachHost(); render()
    }
    private fun attachHost(reload: Boolean = false) {
        runCatching {
            BinanceHostRuntime.onMain(this) { runtime ->
                host = runtime
                val previous = runtime.view
                runtime.onChanged = ::render
                if (!runtime.begin()) return@onMain
                if (session == null) session = BinanceCreateSession(runtime,attempt,::persist,::render)
                runtime.onCreateObservation = { session?.observed(it) }
                runtime.view?.let { view ->
                    (view.parent as? ViewGroup)?.removeView(view); official.addView(view,FrameLayout.LayoutParams(-1,-1))
                    if (reload && view === previous) view.loadUrl(BinanceHostRuntime.ENTRY)
                }
            }
        }.onFailure { status.text = "官网连接暂不可用，请更新系统 WebView 后重试。" }
    }
    private fun persist(): Boolean = if (recordResolved) true else if (attempt.unresolved) journal.save(attempt.journal())
        else if (!recoveryBlocked) journal.save(null) else false
    private fun render() {
        if (!ready) return
        val runtime = host
        val same = runtime?.state?.account == attempt.account
        val kind = when (runtime?.state?.accountKind) { "sub" -> "币安子账户"; "primary" -> "币安主账户"; else -> "币安账户尚未确认" }
        val fingerprint = runtime?.state?.account?.takeLast(6)?.let { " · 本机账号标记 $it" } ?: ""
        status.text = "$kind$fingerprint\n${runtime?.status ?: "请连接官网"}\n${session?.message ?: "请先确认登录"}"
        val blocked = recoveryBlocked || attempt.unresolved || managementPending
        form.root.visibility = if (blocked) View.GONE else View.VISIBLE
        check.isEnabled = !blocked && session?.busy == false && runtime?.state?.fresh() == true
        confirmed.visibility = if (attempt.status == "prepared") View.VISIBLE else View.GONE
        submit.visibility = confirmed.visibility
        submit.isEnabled = !recoveryBlocked && confirmed.isChecked && session?.canSubmit() == true
        summary.text = when {
            managementPending -> "存在尚未核对的管理操作，请返回量化并进入管理网格，先核对本机记录。"
            recoveryBlocked -> "上次本机记录未能恢复，请先到官网核对。不会自动重发。"
            attempt.status == "prepared" -> attempt.draft!!.summary() + "\n动态挂单：${if (attempt.draft!!.input.getValue("count").toInt() > attempt.windowCount) "启用" else "未启用"}"
            attempt.unresolved && !same -> "存在需要核对的创建记录。请先连接原币安账号；不会在新账号重新提交。"
            attempt.unresolved -> "本次状态：${statusLabel()}\n客户端单号：${attempt.clientId}\n策略编号：${attempt.strategyId.ifEmpty { "尚未确认" }}\n币安策略状态：${attempt.providerStatus.ifEmpty { "未知" }}"
            else -> ""
        }
        detail.visibility = if (same && attempt.strategyId.isNotEmpty()) View.VISIBLE else View.GONE
        detail.isEnabled = session?.busy == false
        resolved.visibility = if (blocked && !managementPending && attempt.status != "submitting") View.VISIBLE else View.GONE
        resolved.isEnabled = recoveryBlocked || same
    }
    private fun statusLabel() = when (attempt.status) {
        "submitting" -> "正在提交"; "accepted" -> "已受理，待查询"; "observed" -> "已读取策略详情"; else -> "结果未知，请到官网核对"
    }
    private fun acknowledge() {
        if (!hasWindowFocus() || attempt.status == "submitting" || (!recoveryBlocked && host?.state?.account != attempt.account)) return
        AlertDialog.Builder(this).setTitle("结束本机核对记录")
            .setMessage("这不会撤单、结束网格或平仓，也不证明币安没有创建策略。请先在官网确认本次策略及资金状态。")
            .setNegativeButton("继续核对",null).setPositiveButton("我已在官网核对") { _, _ ->
                if (journal.save(null)) { recordResolved = true; recoveryBlocked = false; returnResult(clearRecord = true) }
            }.show().window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
    }
    private fun returnResult(clearRecord: Boolean = false) {
        session?.close()
        if (clearRecord) journal.save(null)
        val same = host?.live() == true && host?.state?.account == attempt.account
        val state = when { recoveryBlocked || (!same && attempt.unresolved) -> "unknown"; attempt.status == "submitting" -> "unknown";
            attempt.status in setOf("accepted","observed","unknown","rejected","not_sent") -> attempt.status; else -> "not_sent" }
        if (BinanceHostCaller.activity(this)) setResult(RESULT_OK, Intent().putExtra("schema","yilong.binance_create_result.v1")
            .putExtra("nonce",nonce).putExtra("status",state).putExtra("strategy_id",if (same) attempt.strategyId else "")
            .putExtra("provider_status",if (same) attempt.providerStatus else ""))
        finish()
    }
    @Deprecated("Deprecated in Java") override fun onBackPressed() = returnResult()
    override fun onResume() { super.onResume(); if (ready) render() }
    override fun onPause() { if (ready && attempt.status != "submitting") { session?.cancelPreparation(); confirmed.isChecked = false }; super.onPause() }
    override fun onNewIntent(intent: Intent?) { super.onNewIntent(intent); returnResult() }
    override fun onSaveInstanceState(outState: Bundle) { super.onSaveInstanceState(outState); outState.clear() }
    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        if (event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0) return true
        return super.dispatchTouchEvent(event)
    }
    override fun onDestroy() {
        if (ownsSlot) {
            session?.close()
            host?.let { it.onChanged = null; it.onCreateObservation = null; it.view?.let { view -> if (view.parent === official) official.removeView(view) } }
            slot.release(this); ownsSlot = false
        }
        super.onDestroy()
    }
    private fun label(value: String, size: Float) = ui.label(value, size)
    private fun button(value: String, id: String, action: () -> Unit) = ui.button(value, id,
        primary = id == "binance-create-reload" || id == "binance-create-prepare", action = action)
    companion object { private val slot = BinanceCreateSlot.shared }
}
