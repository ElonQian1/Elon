package com.elon.app.grid.manage

import com.elon.app.privateaccess.StrictJson

internal data class BinanceManageSnapshot(val id: String, val symbol: String, val status: String,
    val cps: Boolean, val cos: Boolean, val sharing: Boolean, val trailingLower: Boolean, val trailingUpper: Boolean) {
    companion object {
        fun parse(raw: Any?): BinanceManageSnapshot {
            val v = raw as? Map<*, *> ?: error("详情格式不完整")
            require(v.keys == setOf("strategy_id","symbol","provider_status","cps","cos","sharing","trailingStopLowerLimit","trailingStopUpperLimit"))
            val id = v["strategy_id"] as String; require(validId(id))
            val symbol = v["symbol"] as String; require(Regex("[A-Z0-9]{1,24}USDT").matches(symbol))
            val status = v["provider_status"] as String; require(Regex("[A-Z][A-Z0-9_]{0,63}").matches(status))
            return BinanceManageSnapshot(id,symbol,status,v["cps"] as Boolean,v["cos"] as Boolean,
                v["sharing"] as Boolean,v["trailingStopLowerLimit"] as Boolean,v["trailingStopUpperLimit"] as Boolean)
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
    var snapshot: BinanceManageSnapshot? = null; private set
    var providerStatus = ""; private set
    private var expires = 0L
    val unresolved get() = status in setOf("submitting","unknown","accepted","observed")
    fun prepare(account: String, document: String, action: String, cps: Boolean, snapshot: BinanceManageSnapshot) {
        require(!unresolved && Regex("[a-f0-9]{64}").matches(account) && Regex("doc_[a-z0-9_]{3,80}").matches(document))
        require(action in setOf("settings","close") && snapshot.status == "WORKING")
        require(if (action == "settings") snapshot.cps != cps else snapshot.cps == cps)
        this.account=account;this.document=document;this.action=action;this.cps=cps;this.id=snapshot.id
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
        require(status!="accepted" || raw.isNotEmpty())
        require(raw.isEmpty() || Regex("[A-Z][A-Z0-9_]{0,63}").matches(raw))
        this.status=status;providerStatus=raw
    }
    fun observe(account: String, value: BinanceManageSnapshot) {
        require(!unresolved || (this.account==account && this.id==value.id))
        snapshot=value;providerStatus=value.status
        if(unresolved) status="observed"
    }
    fun effectObserved() = snapshot?.let {
        if(action=="settings") it.cps==cps else action=="close" && it.status in setOf("CANCELED","CANCELLED","CLOSE_WITH_POSITION")
    } == true
    fun journal() = StrictJson.encode(mapOf("schema" to "yilong.binance_manage_journal.v1","status" to status,
        "account" to account,"strategy_id" to id,"action" to action,"cps" to cps,"provider_status" to providerStatus))
    fun restore(raw: String) {
        val v=StrictJson.parse(raw,2048)
        require(v.keys==setOf("schema","status","account","strategy_id","action","cps","provider_status"))
        require(v["schema"]=="yilong.binance_manage_journal.v1" && v["status"] in setOf("submitting","unknown","accepted","observed"))
        account=v["account"] as String;require(Regex("[a-f0-9]{64}").matches(account))
        id=v["strategy_id"] as String;require(BinanceManageSnapshot.validId(id))
        action=v["action"] as String;require(action in setOf("settings","close"));cps=v["cps"] as Boolean
        providerStatus=v["provider_status"] as String;require(providerStatus.isEmpty() || Regex("[A-Z][A-Z0-9_]{0,63}").matches(providerStatus))
        status=if(v["status"] in setOf("accepted","observed")) "accepted" else "unknown"
        document="";snapshot=null;expires=0
    }
}
