package com.elon.app.grid.create

import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.privateaccess.StrictJson

/** One bounded private read belonging to the current native create operation. */
internal class BinanceCreateFunds(private val host:BinanceHostRuntime) {
    private var generation=0L
    private var account=""
    private var document=""
    private var request=""
    private var started=0L
    private var reading=false
    private var result:Map<String,Any?> = emptyMap()
        set(value) {val changed=field!=value;field=value;if(changed && value["status"] in setOf("ready","unavailable","expired"))host.events.changed("funds")}
    fun start(id:String) {
        require(Regex("[a-f0-9]{64}").matches(id) && host.live() && host.state.fresh())
        val ticket=++generation
        host.pendingReferenceReads.remove(request)
        account=host.state.account ?: error("ACCOUNT_MISSING")
        document=host.document.snapshot().documentToken;request=id
        started=System.currentTimeMillis();reading=false;result=base("pending")
        host.pendingReferenceReads[id]={if(ticket==generation)collectResult()}
        host.handler.postDelayed({if(ticket==generation && result["status"]=="pending")result=base("unavailable")},30_000)
        val args=listOf(document,request,account).joinToString(","){StrictJson.encode(it)}
        host.view?.evaluateJavascript("window.__elonBinanceCreateFundsV1?.start($args)") {
            if(ticket==generation && it!="true")result=base("unavailable")
        } ?: run {result=base("unavailable")}
    }
    fun snapshot(id:String):Map<String,Any?> {
        require(id==request && Regex("[a-f0-9]{64}").matches(id))
        if(!current())return base("expired")
        val now=System.currentTimeMillis()
        if(result["status"]=="pending" && now-started !in 0..30_000)result=base("unavailable")
        if(result["status"]=="ready" && now-(result["observed_at"] as Long) !in -5000..60_000)result=base("expired")
        return result
    }
    private fun collectResult() {
        if(!reading && result["status"]=="pending") {
            reading=true;val ticket=generation
            val args=listOf(document,request,account).joinToString(","){StrictJson.encode(it)}
            host.view?.evaluateJavascript("JSON.stringify(window.__elonBinanceCreateFundsV1?.read($args) ?? null)") {raw->
                if(ticket!=generation)return@evaluateJavascript
                reading=false
                if(!current()){result=base("expired");return@evaluateJavascript}
                if(System.currentTimeMillis()-started !in 0..30_000){result=base("unavailable");return@evaluateJavascript}
                runCatching {
                    val json=StrictJson.parse("{\"value\":$raw}",8192)["value"] as? String ?: return@runCatching
                    if(json!="null")result=BinanceCreateFundsResult.parse(json,request,account,System.currentTimeMillis())
                }.onFailure {result=base("unavailable")}
            } ?: run {reading=false;result=base("unavailable")}
        }
    }
    fun close(){generation++;host.pendingReferenceReads.remove(request);result=emptyMap();reading=false;host.view?.evaluateJavascript("window.__elonBinanceCreateFundsV1?.cancel()",null)}
    private fun current()=host.live() && host.state.fresh() && host.state.account==account && host.document.snapshot().documentToken==document
    private fun base(status:String):Map<String,Any?> = mapOf("schema" to BinanceCreateFundsResult.SCHEMA,"request" to request,"account" to account,"status" to status)
}

internal object BinanceCreateFundsResult {
    const val SCHEMA="yilong.binance_create_funds.v1"
    fun parse(raw:String,request:String,account:String,now:Long):Map<String,Any?> {
        val data=StrictJson.parse(raw,4096).toMutableMap()
        val base=setOf("schema","request","account","status")
        require(data["schema"]==SCHEMA && data["request"]==request && data["account"]==account)
        require(data["status"] in setOf("pending","ready","unavailable","expired"))
        if(data["status"]!="ready"){require(data.keys==base);return data}
        require(data.keys==base+setOf("asset","available","source","observed_at"))
        require(data["asset"]=="USDT" && data["source"] in setOf("futures_transferable","portfolio_spot_free"))
        val amount=data["available"] as? String ?: error("FUNDS_AMOUNT")
        require(Regex("(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(amount))
        val observed=(data["observed_at"] as StrictJson.Number).text.toLong()
        require(observed>0 && now-observed in -5000..60_000)
        data["observed_at"]=observed
        return data
    }
}
