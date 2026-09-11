package com.elon.app.grid.create

import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.privateaccess.StrictJson
import java.math.BigDecimal
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

/** Read-only references owned by a create operation. Never changes preparation or submits a trade. */
internal class BinanceCreateReference(private val host:BinanceHostRuntime,private val version:Int=1) {
    init {require(version in 1..2)}
    private val market=BinanceGridMarket()
    private var generation=0L
    private var account=""
    private var document=""
    private var request=""
    private var symbol=""
    private var started=0L
    private var reading=false
    private var result:Map<String,Any?> = emptyMap()

    fun start(id:String,raw:String) {
        val input=BinanceReferenceInput.parse(raw,version)
        require(Regex("[a-f0-9]{64}").matches(id))
        require(host.live() && host.state.fresh())
        val ticket=++generation
        account=host.state.account ?: error("ACCOUNT_MISSING")
        document=host.document.snapshot().documentToken;request=id;symbol=input.getValue("symbol")
        started=System.currentTimeMillis();reading=false;result=base("pending")
        val expectedAccount=account;val expectedDocument=document
        try {worker.execute {
            val loaded=runCatching {
                val rule=market.rules().single {it.symbol==input.getValue("symbol")}
                val quote=market.quote(rule.symbol)
                val data=mapOf("tick" to rule.tick.toPlainString(),"minQty" to rule.minimumQuantity,
                    "minNotional" to (rule.minimumNotional ?: error("NOTIONAL_MISSING")),
                    "qtyPrecision" to (rule.quantityStep ?: error("QUANTITY_STEP_MISSING")).toBigDecimal().stripTrailingZeros().scale().coerceAtLeast(0),
                    "mark" to quote.mark,"observedAt" to quote.observed)
                if(version==1)data else {
                    val trailing=input["trailingUp"]=="true" || input["trailingDown"]=="true"
                    val ticker=if(trailing && !input["margin"].isNullOrEmpty())market.ticker(rule.symbol) else null
                    data+mapOf("pricePrecision" to (rule.pricePrecision ?: error("PRICE_PRECISION_MISSING")),
                        "last" to (ticker?.last?.toPlainString() ?: ""),
                        "observedAt" to minOf(quote.observed,ticker?.closeTime ?: quote.observed))
                }
            }
            host.handler.post {
                if(ticket!=generation)return@post
                if(!current()){result=base("expired");return@post}
                loaded.fold({data->
                    val args=listOf(expectedDocument,id,expectedAccount,input,data).joinToString(","){StrictJson.encode(it)}
                    host.view?.evaluateJavascript("window.__elonBinanceCreateReferenceV$version?.start($args)") {
                        if(ticket==generation && it!="true")result=base("unavailable")
                    } ?: run {result=base("unavailable")}
                },{result=base("unavailable")})
            }
        }}catch(_:java.util.concurrent.RejectedExecutionException){result=base("unavailable")}
    }

    fun snapshot(id:String):Map<String,Any?> {
        require(id==request && Regex("[a-f0-9]{64}").matches(id))
        if(!current())return base("expired")
        val now=System.currentTimeMillis()
        if(result["status"]=="pending" && now-started !in 0..45_000)result=base("unavailable")
        if(result["status"]=="ready") {
            val observed=result["observed_at"] as Long
            if(now-observed !in -5000..120000)result=base("expired")
        }
        if(!reading && result["status"]=="pending") {
            reading=true;val ticket=generation
            val args=listOf(document,request,account).joinToString(","){StrictJson.encode(it)}
            host.view?.evaluateJavascript("JSON.stringify(window.__elonBinanceCreateReferenceV$version?.read($args) ?? null)") {raw->
                if(ticket!=generation)return@evaluateJavascript
                reading=false
                if(!current()){result=base("expired");return@evaluateJavascript}
                runCatching {
                    val json=StrictJson.parse("{\"value\":$raw}",8192)["value"] as? String ?: return@runCatching
                    if(json=="null")return@runCatching
                    val value=BinanceReferenceResult.parse(json,request,account,symbol,System.currentTimeMillis(),version)
                    result=value
                }.onFailure {result=base("unavailable")}
            } ?: run {reading=false;result=base("unavailable")}
        }
        return result
    }
    fun close(){generation++;result=emptyMap();reading=false;host.view?.evaluateJavascript("window.__elonBinanceCreateReferenceV$version?.cancel()",null)}
    private fun current()=host.live() && host.state.fresh() && host.state.account==account && host.document.snapshot().documentToken==document
    private fun base(status:String):Map<String,Any?> = mapOf("schema" to "yilong.binance_create_reference.v$version","request" to request,"account" to account,"symbol" to symbol,"status" to status)
    companion object {private val worker=ThreadPoolExecutor(1,1,0,TimeUnit.MILLISECONDS,ArrayBlockingQueue(1))}
}

internal object BinanceReferenceInput {
    val keys=setOf("symbol","direction","lower","upper","count","leverage","spacing","triggerPrice","trailingUp","trailingDown","marginType")
    fun parse(raw:String,version:Int=1):Map<String,String> {
        require(version in 1..2)
        val value=StrictJson.parse(raw,2048)
        require(value.keys==(if(version==2)keys+"margin" else keys) && value.values.all {it is String})
        val input=value.mapValues {it.value as String}
        require(Regex("[A-Z0-9]{1,24}USDT").matches(input.getValue("symbol")))
        require(input["direction"] in setOf("LONG","SHORT","NEUTRAL") && input["spacing"] in setOf("ARITH","GEO"))
        require(input["marginType"] in setOf("ISOLATED","CROSSED"))
        require(listOf("trailingUp","trailingDown").all {input[it] in setOf("true","false")})
        for(key in listOf("lower","upper","triggerPrice")+(if(version==2)listOf("margin") else emptyList())) {
            val text=input.getValue(key)
            if(key in setOf("triggerPrice","margin") && text.isEmpty())continue
            require(Regex("(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(text) && text.toBigDecimal()>BigDecimal.ZERO)
        }
        require(input.getValue("upper").toBigDecimal()>input.getValue("lower").toBigDecimal())
        require(Regex("[1-9][0-9]{0,2}").matches(input.getValue("leverage")) && input.getValue("leverage").toInt() in 1..125)
        val count=input.getValue("count")
        require(count.isEmpty() || Regex("[1-9][0-9]{0,4}").matches(count) && count.toInt()<=10000)
        return input
    }
}

internal object BinanceReferenceResult {
    const val SCHEMA="yilong.binance_create_reference.v1"
    fun parse(raw:String,request:String,account:String,symbol:String,now:Long,version:Int=1):Map<String,Any?> {
        require(version in 1..2)
        val data=StrictJson.parse(raw,4096).toMutableMap()
        val base=setOf("schema","request","account","symbol","status")
        require(data["schema"]=="yilong.binance_create_reference.v$version" && data["request"]==request && data["account"]==account && data["symbol"]==symbol)
        require(data["status"] in setOf("pending","ready","unavailable","expired"))
        if(data["status"]!="ready"){require(data.keys==base);return data}
        require(data.keys==base+setOf("minimum_count","maximum_count","minimum_margin","observed_at","source","code")+
            (if(version==2)BinanceReferenceEconomics.keys else emptySet()))
        fun number(key:String)=(data[key] as StrictJson.Number).text.toLong()
        val minimum=number("minimum_count");val maximum=number("maximum_count");val observed=number("observed_at")
        require(minimum in 2..10000 && maximum in 0..10000 && now-observed in -5000..120000)
        val margin=data["minimum_margin"] as String
        require(margin.isEmpty() || Regex("(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(margin) && margin.toBigDecimal()>BigDecimal.ZERO)
        require(data["source"]=="binance_create_rules_v$version" && data["code"] in setOf("","range_too_narrow","count_required","count_outside_range"))
        require((data["code"]=="")==margin.isNotEmpty())
        if(version==2)BinanceReferenceEconomics.validate(data,symbol)
        data["minimum_count"]=minimum;data["maximum_count"]=maximum;data["observed_at"]=observed
        return data
    }
}
