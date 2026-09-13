package com.elon.app.exchange.okx

import java.math.BigDecimal
import java.util.Locale

/** Fixed allowlist, original fee/rebate signs and explicit nulls. No inferred net PNL. */
internal object OkxRecordsProjection {
    private fun text(row:Map<String,Any?>,key:String):String? {
        val value=row[key] ?: return null
        require(value is String && value.length<=128)
        return value.takeIf{it.isNotEmpty()}
    }
    private fun decimal(row:Map<String,Any?>,key:String,nonnegative:Boolean=false):String?=text(row,key)?.let{
        require(Regex("-?(0|[1-9][0-9]{0,28})(\\.[0-9]{1,28})?").matches(it))
        val value=BigDecimal(it).stripTrailingZeros();require(value.precision()<=29 && value.scale()<=28)
        if(nonnegative)require(value.signum()>=0)
        value.toPlainString()
    }
    private fun time(row:Map<String,Any?>,key:String,now:Long):String?=text(row,key)?.let{
        require(Regex("[1-9][0-9]{0,18}").matches(it) && it.toLong()<=now+30_000);it
    }
    private fun enum(row:Map<String,Any?>,key:String,allowed:Set<String>?=null):String?=text(row,key)?.let{
        require(Regex("[a-z_]{1,40}").matches(it) && (allowed==null || it in allowed));it.uppercase(Locale.ROOT)
    }
    private fun currency(row:Map<String,Any?>,key:String)=text(row,key)?.also{require(Regex("[A-Z0-9]{1,24}").matches(it))}
    fun row(row:Map<String,Any?>,query:OkxRecordQuery,now:Long):Map<String,Any?> {
        require(row["algoId"]==query.id && row["instId"]==query.symbol && row["instType"]=="SWAP")
        if(row["ccy"]!=null && row["ccy"]!="")require(row["ccy"]=="USDT")
        return if(query.kind=="positions") {
            require(row["posSide"]=="net")
            mapOf("symbol" to query.symbol,"side" to "NET","quantity" to decimal(row,"pos"),"entry" to decimal(row,"avgPx",true),
                "margin_mode" to enum(row,"mgnMode",setOf("cross","isolated")),"initial_margin" to decimal(row,"imr",true),
                "maintenance_margin" to decimal(row,"mmr",true),"unrealized_pnl" to decimal(row,"upl"),"unrealized_ratio" to decimal(row,"uplRatio"),
                "liquidation_price" to decimal(row,"liqPx",true),"mark_price" to decimal(row,"markPx",true),"last_price" to decimal(row,"last",true),
                "notional_usd" to decimal(row,"notionalUsd",true),"leverage" to decimal(row,"lever",true),"updated" to time(row,"uTime",now))
        } else {
            require(row["algoOrdType"]=="contract_grid")
            val id=OkxReadProtocol.id(text(row,"ordId") ?: error("ORDER_ID_MISSING"))
            val state=enum(row,"state",setOf("canceled","live","partially_filled","filled","cancelling")) ?: error("STATE_MISSING")
            if(query.kind=="fills")require(state=="FILLED")
            val side=enum(row,"side",setOf("buy","sell")) ?: error("SIDE_MISSING")
            val quantity=decimal(row,"sz",true);val executed=decimal(row,"accFillSz",true)
            if(quantity!=null && executed!=null)require(BigDecimal(executed)<=BigDecimal(quantity))
            mapOf("id" to id,"side" to side,"status" to state,"type" to enum(row,"ordType"),"position_side" to enum(row,"posSide",setOf("net")),
                "price" to decimal(row,"px",true),"quantity" to quantity,"executed" to executed,"average" to decimal(row,"avgPx",true),
                "fee" to decimal(row,"fee"),"fee_asset" to currency(row,"feeCcy"),"rebate" to decimal(row,"rebate"),"rebate_asset" to currency(row,"rebateCcy"),
                "pnl" to decimal(row,"pnl"),"contract_value" to decimal(row,"ctVal",true),"leverage" to decimal(row,"lever",true),
                "created" to time(row,"cTime",now),"updated" to time(row,"uTime",now))
        }
    }
}
