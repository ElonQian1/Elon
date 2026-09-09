package com.elon.app.grid.manage

import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.create.binanceDispatchStarted
import com.elon.app.privateaccess.StrictJson
import java.util.UUID

internal class BinanceManageSession(private val host: BinanceHostRuntime, val state: BinanceManageState,
    private val persist: () -> Boolean, private val changed: () -> Unit) {
    var message="先选择策略并读取当前设置。"; private set
    var busy=false; private set
    val readTrace=BinanceManageReadTrace()
    private var closed=false
    private var ticket=""
    private var token=""
    private var account=""
    private var target=""
    private var pendingAction=""
    private var pendingCps=false
    private var pendingInvestment=""
    private var pendingRange:BinanceRangeDraft?=null
    fun detailCurrent(id:String?)=readTrace.outcome=="verified" && host.live() && host.state.fresh() &&
        host.state.account==account && host.document.snapshot().documentToken==token && state.snapshot?.id==id
    fun cancel() {
        readTrace.cancel()
        ticket="";busy=false;state.cancel()
        host.view?.evaluateJavascript("window.__elonBinanceManageV1?.cancel()",null)
    }
    private fun begin(id: String) {
        require(!busy && host.live() && host.state.fresh() && BinanceManageSnapshot.validId(id)) { "请重新加载官网，等当前账号核验完成。" }
        require(if(state.unresolved) state.id==id && state.account==host.state.account else host.state.contains(id)) { "策略不属于当前已验证列表或原账号。" }
        cancel();ticket=UUID.randomUUID().toString().replace("-","")
        token=host.document.snapshot().documentToken;account=host.state.account ?: error("账号未确认");target=id
        busy=true
    }
    fun read(id: String) {
        begin(id);readTrace.start();message="正在读取当前策略；不会发送交易。";changed()
        execute("inspect",listOf(token,ticket,account,id),false)
    }
    fun prepare(id: String, action: String, cps: Boolean, investmentDelta:String="",rangeDraft:BinanceRangeDraft?=null) {
        require(!state.unresolved && action in setOf("settings","close","investment","range")) { "请先核对并结束上次本机记录。" }
        require((action=="range")==(rangeDraft!=null))
        val amount=if(action=="investment")BinanceInvestment.amount(investmentDelta) else "".also{require(investmentDelta.isEmpty())}
        begin(id);pendingAction=action;pendingCps=cps
        pendingInvestment=amount
        pendingRange=rangeDraft
        message="正在核对账号和当前设置，尚未提交。";changed()
        if(action=="range")execute("prepareRange",listOf(token,ticket,account,id,rangeDraft!!.payload()),false)
        else if(action=="investment")execute("prepareInvestment",listOf(token,ticket,account,id,amount),false)
        else execute("prepare",listOf(token,ticket,account,id,action,cps),false)
    }
    fun canSubmit() = !busy && host.live() && host.state.fresh() && host.state.contains(state.id) &&
        state.canSubmit(host.state.account,host.document.snapshot().documentToken)
    fun submit() {
        require(canSubmit()) { "准备已过期或账号变化，请重新检查。" }
        state.start(host.state.account,host.document.snapshot().documentToken)
        if(!persist()) {state.outcome("not_sent");error("无法记录本机状态，未发送操作。")}
        busy=true;message="正在提交本次操作，不会自动重试。";changed()
        execute("submit",listOf(token,ticket),true)
    }
    private fun execute(action: String, args: List<Any>, writing: Boolean) {
        val expected=ticket
        val script="(()=>{const a=window.__elonBinanceManageV1;return typeof a?.$action==='function'?a.$action(${args.joinToString(","){StrictJson.encode(it)}}):false})()"
        val view=host.view
        if(view==null) {failedToStart(writing);return}
        view.evaluateJavascript(script) { raw -> if(!closed && ticket==expected) {
            val ack=binanceDispatchStarted(raw)
            if(ack!=true && (!writing || state.status=="submitting")) {
                if(writing && ack==null) {
                    state.unknown();persist();busy=false
                    message="网页未返回明确提交回执，请到官网核对；不会重发。";changed()
                } else failedToStart(writing)
            }
        }}
        host.handler.postDelayed({if(!closed && ticket==expected && busy) {
            if(writing) {state.unknown();persist()} else cancel()
            if(!writing && action=="inspect")readTrace.finish("timeout")
            busy=false;message=if(writing || state.unresolved) "结果查询超时，此前操作仍须到官网核对；不会自动补发。" else "读取超时，未提交操作。"
            changed()
        }},if(writing) 90_000 else 70_000)
    }
    private fun failedToStart(writing: Boolean) {
        if(!writing && readTrace.outcome=="pending")readTrace.finish("unavailable")
        if(writing) {state.outcome("not_sent");persist()};busy=false
        message=if(state.unresolved) "结果查询尚未就绪，此前操作仍须核对；不会重发。" else "官网连接未就绪，未提交操作。";changed()
    }
    fun observed(raw: String) {
        if(closed) return
        runCatching {
            val v=StrictJson.parse(raw,4096)
            if(v["schema"]!="yilong.binance_manage_event.v1" || v["token"]!=token || v["attempt"]!=ticket || host.document.accept(token)==null) return
            val base=setOf("schema","token","attempt","kind")
            when(v["kind"]) {
                "prepared","detail" -> {
                    val investing=v["kind"]=="prepared" && pendingAction=="investment"
                    val ranging=v["kind"]=="prepared" && pendingAction=="range"
                    require(v.keys==base+(if(v["kind"]=="prepared") setOf("snapshot","action","cps") else setOf("snapshot"))+
                        (if(investing)setOf("investment_delta") else emptySet())+(if(ranging)setOf("range_draft") else emptySet()))
                    require(host.live() && host.state.account==account)
                    val snapshot=BinanceManageSnapshot.parse(v["snapshot"]);require(snapshot.id==target)
                    if(v["kind"]=="prepared") {
                        require(host.state.contains(target) && v["action"]==pendingAction && v["cps"]==if(investing || ranging)snapshot.cps else pendingCps)
                        require(!investing || v["investment_delta"]==pendingInvestment)
                        require(!ranging || BinanceRangeDraft.parse(v["range_draft"])==pendingRange)
                        state.prepare(account,token,pendingAction,if(investing || ranging)snapshot.cps else pendingCps,snapshot,pendingInvestment,pendingRange)
                        message="检查完成，尚未提交。请核对下方本次操作摘要。"
                        host.handler.postDelayed({if(!closed) changed()},60_000)
                    } else {
                        state.observe(account,snapshot);persist()
                        readTrace.finish("verified")
                        message=if(state.unresolved && state.effectObserved()) "详情已观察到目标值；仍须核对本次操作与资金、挂单和仓位。" else "已读取当前详情；不代表操作完成或仓位已归零。"
                    }
                }
                "accepted" -> {
                    require(v.keys==base+setOf("strategy_id","provider_status") && v["strategy_id"]==state.id)
                    state.outcome("accepted",v["provider_status"] as String);persist()
                    message="币安已受理。请重新查询详情并核对挂单和仓位。";host.view?.reload()
                }
                "unknown","not_sent","rejected" -> {
                    require(v.keys==base+"code");state.outcome(v["kind"] as String);persist()
                    message=when(v["kind"]) {"unknown"->"结果未知，请在官网核对，不要重复提交。";"rejected"->"币安拒绝本次操作，没有自动重试。";else->"提交前核验未通过，没有发送操作。"}
                }
                "prepare_failed","read_failed" -> {
                    require(v.keys==base+"code")
                    if(v["kind"]=="read_failed")readTrace.finish("failed",v["code"] as? String ?: "verification_failed")
                    message=when(v["code"]) {"no_change"->"所选设置没有变化。";"close_mode_changed"->"当前终止处理与选择不同，请先单独修改设置，再重新检查结束。";
                        "not_working"->"策略不是运行状态，请使用官网核对。";"settings_unavailable"->"详情缺少可验证设置，请使用官网。";
                        "range_unavailable"->"当前仅支持参数完整的普通非追踪网格；收益金额止盈止损或未识别参数请在官网修改。";
                        "investment_unavailable"->"官网尚未返回完整投入详情，请在官网追加并核对结果。";else->"账号或详情核验未通过，未提交操作。"}
                }
                else -> return
            }
            busy=false;changed()
        }.onFailure {if(readTrace.outcome=="pending")readTrace.finish("failed","malformed_reply");state.unknown();persist();busy=false;message="回执未能验证，请在官网核对；不会自动重试。";changed()}
    }
    fun close() {cancel();state.unknown();persist();closed=true}
}
