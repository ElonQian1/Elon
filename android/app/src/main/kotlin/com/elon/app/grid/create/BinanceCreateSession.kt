package com.elon.app.grid.create

import android.os.SystemClock
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.privateaccess.StrictJson
import java.util.UUID

/** Submission comes from a visible user confirmation: legacy Activity or authenticated native consumer. */
internal class BinanceCreateSession(private val host: BinanceHostRuntime, val attempt: BinanceCreateAttempt,
    private val persist: () -> Boolean, private val changed: () -> Unit,
    reference: BinanceCreateReference = BinanceCreateReference(host)) {
    var message = "填写参数后先检查账号。检查不会下单。"; private set
    var busy = false; private set
    private var id = UUID.randomUUID().toString().replace("-", "")
    private var token = ""
    private var expectedAccount = ""
    private var pending: BinanceGridDraft? = null
    private var closed = false
    private var referenceObserved: Long? = null
    private val ruleCheck = BinanceCreateRuleCheck(reference::start, reference::snapshot, reference::close,
        { task, delay -> host.handler.postDelayed(task, delay); Unit }, { host.handler.removeCallbacks(it) },
        SystemClock::elapsedRealtime, System::currentTimeMillis)
    init {reference.onResult=ruleCheck::changed}

    fun prepare(draft: BinanceGridDraft) {
        require(!busy && !attempt.unresolved && host.live() && host.state.fresh()) { "请先完成币安登录，并等待读取当前账号" }
        cancelPreparation()
        token = host.document.snapshot().documentToken
        expectedAccount = host.state.account ?: error("当前账号尚未确认")
        id = UUID.randomUUID().toString().replace("-", "")
        pending = draft; busy = true; message = "正在检查当前账号、动态格数范围和最低保证金，尚未下单。"; changed()
        val thisId = id
        val referenceRequest = UUID.randomUUID().toString().replace("-", "") + UUID.randomUUID().toString().replace("-", "")
        ruleCheck.begin(referenceRequest, expectedAccount, draft) { result ->
            if (!closed && id == thisId && busy && pending != null) {
                result.fold({ observed ->
                    if (!host.live() || !host.state.fresh() || host.state.account != expectedAccount ||
                        host.document.snapshot().documentToken != token) {
                        cancelPreparation(); message = "账号或连接已变化，请重新检查参数；未下单"; changed()
                    } else {
                        referenceObserved = observed
                        evaluate("prepare", listOf(token, id, expectedAccount, draft.payload())) { accepted ->
                            if (accepted != true && id == thisId && busy) { busy = false; pending = null; message = "币安连接尚未就绪，请重新连接后检查参数。"; changed() }
                        }
                    }
                }, { error ->
                    pending = null; busy = false; referenceObserved = null
                    message = error.message ?: "动态规则检查未完成，请重新检查；未下单"; changed()
                })
            }
        }
        host.handler.postDelayed({ if (!closed && thisId == id && pending != null) {
            cancelPreparation(); message = "检查超时，未提交交易；请重新连接或完成登录验证。"; changed()
        } }, 45_000)
    }
    fun canSubmit() = !busy && host.live() && host.state.fresh() && BinanceCreateRuleCheck.fresh(referenceObserved,System.currentTimeMillis()) &&
        attempt.canSubmit(host.state.account,host.document.snapshot().documentToken)
    fun submit() {
        require(canSubmit()) { "账号、页面或准备结果已变化，请重新检查" }
        attempt.start(host.state.account,host.document.snapshot().documentToken)
        if (!persist()) { attempt.notSent("journal_unavailable"); error("无法记录本机提交状态，未发送请求") }
        busy = true; message = "正在提交本次创建；请勿重复操作。"; changed()
        evaluate("submit", listOf(token,id)) { accepted ->
            if (accepted != true && attempt.status == "submitting") {
                if (accepted == false) attempt.notSent("bridge_not_ready") else attempt.unknown()
                busy = false; persist()
                message = if (accepted == false) "页面未接受提交，请重新检查。" else "网页未返回明确提交回执，请先在官网核对；不会重发。"
                changed()
            }
        }
        host.handler.postDelayed({ if (!closed && attempt.status == "submitting") {
            attempt.unknown(); busy = false; persist(); message = "结果尚未确认；请到官网核对，系统不会自动重发。"; changed()
        } }, 65_000)
    }
    fun detail() {
        require(!busy && host.live() && host.state.account == attempt.account && attempt.strategyId.isNotEmpty()) { "请连接原币安账号后查询" }
        token = host.document.snapshot().documentToken
        busy = true; message = "正在查询刚创建的策略。"; changed()
        evaluate("detail", listOf(token,id,attempt.account,attempt.strategyId)) { accepted ->
            if (accepted != true) { busy = false; message = "查询连接尚未就绪，请重新连接后重试。"; changed() }
        }
        host.handler.postDelayed({ if (!closed && busy && attempt.status != "submitting") {
            busy = false; message = "详情查询尚未完成，可重新查询；不会重新创建。"; changed()
        } }, 45_000)
    }
    fun observed(raw: String) {
        if (closed) return
        runCatching {
            val e = StrictJson.parse(raw, 4096)
            if (e["schema"] != "yilong.binance_create_event.v1" || e["token"] != token || e["attempt"] != id ||
                host.document.accept(token) == null) return
            val base = setOf("schema","token","attempt","kind")
            when (e["kind"]) {
                "prepared" -> {
                    require(e.keys == base + setOf("client_id","window_count"))
                    val d = pending ?: return
                    require(host.live() && host.state.fresh() && expectedAccount == host.state.account)
                    val count = (e["window_count"] as StrictJson.Number).text.toInt()
                    attempt.prepare(expectedAccount,token,d,e["client_id"] as String,count)
                    pending = null; busy = false; message = "检查完成，尚未下单。请核对当前账号和全部参数。"
                    host.handler.postDelayed({ if (!closed) changed() },60_000)
                }
                "prepare_failed" -> {
                    require(e.keys == base + "code"); pending = null; busy = false
                    message = "检查未通过（${safe(e["code"])}），未下单；请在官网处理登录或账户状态。"
                }
                "accepted" -> {
                    require(e.keys == base + setOf("strategy_id","client_id","provider_status") && e["client_id"] == attempt.clientId)
                    attempt.accepted(e["strategy_id"] as String,e["provider_status"] as String)
                    persist(); busy = false; message = "币安已受理创建。请查询策略状态，受理不代表网格已经运行。"
                    // Website owns the list query. Reload invalidates old grants/document and fetches a new list;
                    // never leave the pre-create empty snapshot looking fresh or invent its POST pagination body.
                    host.view?.reload()
                }
                "rejected", "not_sent", "unknown" -> {
                    require(e.keys == base + "code")
                    val code = safe(e["code"])
                    when (e["kind"]) {
                        "rejected" -> { attempt.rejected(code); message = "币安拒绝本次创建（$code）；没有自动重试。" }
                        "not_sent" -> { attempt.notSent(code); message = "未发送创建（$code），请重新检查。" }
                        else -> { attempt.unknown(); message = "创建结果未知，请到官网核对；不会自动重发。" }
                    }
                    persist(); busy = false
                }
                "detail" -> {
                    require(e.keys == base + setOf("strategy_id","provider_status") && host.state.account == attempt.account)
                    attempt.detail(e["strategy_id"] as String,e["provider_status"] as String)
                    persist(); busy = false; message = "已读取币安策略状态；挂单、仓位、费用仍以官网为准。"
                }
                "detail_failed" -> { require(e.keys == base + "code"); busy = false; message = "未取得可验证详情，请在官网核对。" }
                else -> return
            }
            changed()
        }.onFailure {
            attempt.unknown(); pending = null; busy = false; persist()
            message = if (attempt.unresolved) "回执未能验证，请在官网核对；不会自动重发。" else "准备结果未能验证，未发送创建。"
            changed()
        }
    }
    fun cancelPreparation() {
        ruleCheck.cancel(); referenceObserved = null
        pending = null; busy = false; attempt.invalidatePreparation()
        host.view?.evaluateJavascript("window.__elonBinanceCreateV1?.cancel()",null)
    }
    fun close(persistState: Boolean = true) { cancelPreparation(); attempt.unknown(); if (persistState) persist(); closed = true }
    private fun safe(value: Any?): String = (value as? String)?.takeIf { Regex("[A-Za-z0-9_-]{1,64}").matches(it) } ?: "response_unrecognized"
    private fun evaluate(action: String, values: List<Any>, callback: (Boolean?) -> Unit) {
        val args = values.joinToString(",") { StrictJson.encode(it) }
        val view = host.view
        if (view == null) { callback(false); return }
        view.evaluateJavascript("(()=>{const a=window.__elonBinanceCreateV1;return typeof a?.$action==='function'?a.$action($args):false})()") {
            if (!closed) callback(binanceDispatchStarted(it))
        }
    }
}
