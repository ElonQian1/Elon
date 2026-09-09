package com.elon.app.grid.manage

import com.elon.app.privateaccess.StrictJson

internal data class BinanceManageSnapshot(val id: String, val symbol: String, val status: String,
    val cps: Boolean, val cos: Boolean, val sharing: Boolean, val trailingLower: Boolean, val trailingUpper: Boolean,
    val investment:BinanceInvestmentSnapshot?=null,val range:BinanceRangeSnapshot?=null) {
    companion object {
        fun parse(raw: Any?): BinanceManageSnapshot {
            val v = raw as? Map<*, *> ?: error("详情格式不完整")
            val base=setOf("strategy_id","symbol","provider_status","cps","cos","sharing","trailingStopLowerLimit","trailingStopUpperLimit")
            require(v.keys.containsAll(base) && v.keys.all{it in base+setOf("investment","range")})
            val id = v["strategy_id"] as String; require(validId(id))
            val symbol = v["symbol"] as String; require(Regex("[A-Z0-9]{1,24}USDT").matches(symbol))
            val status = v["provider_status"] as String; require(Regex("[A-Z][A-Z0-9_]{0,63}").matches(status))
            return BinanceManageSnapshot(id,symbol,status,v["cps"] as Boolean,v["cos"] as Boolean,
                v["sharing"] as Boolean,v["trailingStopLowerLimit"] as Boolean,v["trailingStopUpperLimit"] as Boolean,
                v["investment"]?.let(BinanceInvestmentSnapshot::parse),v["range"]?.let(BinanceRangeSnapshot::parse))
        }
        fun validId(id: String) = Regex("[1-9][0-9]{0,15}").matches(id) && (id.toLongOrNull() ?: Long.MAX_VALUE) <= 9_007_199_254_740_991L
    }
}

/** Only a pending intent identity is persisted. No replayable request, headers or financial balances. */
internal class BinanceManageState(private val elapsed: () -> Long) {
    var status = "idle"; private set
    var account = ""; private set
    var document = ""; private set
    var id = ""; private set
    var action = ""; private set
    var cps = false; private set
    var investmentDelta=""; private set
    private var investmentTarget=""
    var rangeDraft:BinanceRangeDraft?=null;private set
    private var rangeTarget=""
    var snapshot: BinanceManageSnapshot? = null; private set
    var providerStatus = ""; private set
    private var expires = 0L
    val unresolved get() = status in setOf("submitting","unknown","accepted","observed")
    fun prepare(account: String, document: String, action: String, cps: Boolean, snapshot: BinanceManageSnapshot, investmentDelta:String="",rangeDraft:BinanceRangeDraft?=null) {
        require(!unresolved && Regex("[a-f0-9]{64}").matches(account) && Regex("doc_[a-z0-9_]{3,80}").matches(document))
        require(action in setOf("settings","close","investment","range") && snapshot.status == "WORKING")
        require((action=="range")==(rangeDraft!=null))
        require(if (action == "settings") snapshot.cps != cps else snapshot.cps == cps)
        val amount=if(action=="investment")BinanceInvestment.amount(investmentDelta) else "".also{require(investmentDelta.isEmpty())}
        val target=if(action=="investment")(snapshot.investment ?: error("官网尚未返回可核验的投入详情")).target(amount) else ""
        if(action=="range") {
            require(snapshot.investment!=null && snapshot.range!=null){"官网尚未返回可核验的普通网格参数。"}
            require(snapshot.range.target()!=rangeDraft!!.target()){"区间和格数没有变化。"}
        }
        this.account=account;this.document=document;this.action=action;this.cps=cps;this.id=snapshot.id
        this.investmentDelta=amount;investmentTarget=target
        this.rangeDraft=rangeDraft;rangeTarget=rangeDraft?.target().orEmpty()
        this.snapshot=snapshot;providerStatus=snapshot.status;expires=elapsed()+60_000;status="prepared"
    }
    fun canSubmit(account: String?, document: String) = status=="prepared" && snapshot!=null &&
        elapsed()<expires && this.account==account && this.document==document
    fun start(account: String?, document: String) { require(canSubmit(account,document));status="submitting" }
    fun cancel() { if(status=="prepared") {status="idle";snapshot=null} }
    fun unknown() { if(status=="submitting") status="unknown" }
    fun outcome(status: String, raw: String = "") {
        require(this.status in setOf("submitting","unknown"))
        require(status in setOf("unknown","not_sent","rejected","accepted"))
        require(status!="accepted" || raw.isNotEmpty() || action in setOf("investment","range"))
        require(raw.isEmpty() || Regex("[A-Z][A-Z0-9_]{0,63}").matches(raw))
        this.status=status;providerStatus=raw
    }
    fun observe(account: String, value: BinanceManageSnapshot) {
        require(!unresolved || (this.account==account && this.id==value.id))
        snapshot=value;providerStatus=value.status
        if(unresolved) status="observed"
    }
    fun effectObserved() = snapshot?.let {
        when(action) {
            "settings"->it.cps==cps
            "investment"->investmentTarget.isNotEmpty() && it.investment?.target("0")==investmentTarget
            "range"->rangeTarget.isNotEmpty() && it.range?.target()==rangeTarget
            else->action=="close" && it.status in setOf("CANCELED","CANCELLED","CLOSE_WITH_POSITION")
        }
    } == true
    fun investmentPreview()=snapshot?.investment?.invested(investmentDelta.ifEmpty{"0"})
    fun journal() = StrictJson.encode(mapOf("schema" to "yilong.binance_manage_journal.v3","status" to status,
        "account" to account,"strategy_id" to id,"action" to action,"cps" to cps,"provider_status" to providerStatus,"investment_target" to investmentTarget,"range_target" to rangeTarget))
    fun restore(raw: String) {
        val v=StrictJson.parse(raw,2048)
        val v3=v["schema"]=="yilong.binance_manage_journal.v3"
        val v2=v3 || v["schema"]=="yilong.binance_manage_journal.v2"
        require(v.keys==setOf("schema","status","account","strategy_id","action","cps","provider_status")+(if(v2)setOf("investment_target") else emptySet())+(if(v3)setOf("range_target") else emptySet()))
        require((v2 || v["schema"]=="yilong.binance_manage_journal.v1") && v["status"] in setOf("submitting","unknown","accepted","observed"))
        account=v["account"] as String;require(Regex("[a-f0-9]{64}").matches(account))
        id=v["strategy_id"] as String;require(BinanceManageSnapshot.validId(id))
        action=v["action"] as String;require(action in setOf("settings","close") || (v2 && action=="investment") || (v3 && action=="range"));cps=v["cps"] as Boolean
        investmentDelta="";investmentTarget=if(v2)v["investment_target"] as String else ""
        require(if(action=="investment")Regex("[a-f0-9]{64}").matches(investmentTarget) else investmentTarget.isEmpty())
        rangeDraft=null;rangeTarget=if(v3)v["range_target"] as String else ""
        require(if(action=="range")Regex("[a-f0-9]{64}").matches(rangeTarget) else rangeTarget.isEmpty())
        providerStatus=v["provider_status"] as String;require(providerStatus.isEmpty() || Regex("[A-Z][A-Z0-9_]{0,63}").matches(providerStatus))
        status=if(v["status"] in setOf("accepted","observed")) "accepted" else "unknown"
        document="";snapshot=null;expires=0
    }
}
