package com.elon.app.exchange.okx

import com.elon.app.privateaccess.StrictJson

internal class OkxRecordQuery(val kind:String,val id:String,val symbol:String,val after:String) {
    init {
        require(kind in setOf("orders","fills","positions"));OkxReadProtocol.id(id)
        require(Regex("[A-Z0-9]{1,20}-USDT-SWAP").matches(symbol))
        if(after.isNotEmpty())OkxReadProtocol.id(after)
        require(kind!="positions" || after.isEmpty())
    }
    override fun toString()="OkxRecordQuery(private)"
}
internal class OkxRecordsPage(val query:OkxRecordQuery,val rows:List<Map<String,Any?>>,val nextAfter:String?) {
    val complete get()=query.kind=="positions" || rows.isEmpty()
    fun encode(account:OkxAccount,generation:Long,revision:Long,now:Long):String {
        val result=StrictJson.encode(mapOf("schema" to SCHEMA,"environment" to "live","account" to account.reference,"account_kind" to account.kind,
            "generation" to generation,"revision" to revision,"observed_at_ms" to now,"fresh_until_ms" to now+60_000,
            "kind" to query.kind,"strategy_id" to query.id,"symbol" to query.symbol,"after" to query.after,"next_after" to nextAfter,
            "complete" to complete,"rows" to rows))
        if(result.toByteArray(Charsets.UTF_8).size>196_608)okxFail(OkxReadFailure.RESPONSE_LIMIT)
        return result
    }
    companion object {const val SCHEMA="yilong.okx_host_records.v1"}
}
internal class OkxRecordsReader(private val gateway:OkxReadGateway) {
    fun read(credentials:OkxCredentials,query:OkxRecordQuery,now:Long):OkxRecordsPage {
        val parent=OkxReadProtocol.rows(gateway.get(credentials,OkxReadRequest.Detail.of(query.id)),1).singleOrNull()
        require(parent!=null && parent["algoId"]==query.id && parent["instId"]==query.symbol && parent["algoOrdType"]=="contract_grid")
        val source=OkxReadProtocol.rows(gateway.get(credentials,OkxReadRequest.Records.of(query)),if(query.kind=="positions")10 else 50)
        val rows=source.map{OkxRecordsProjection.row(it,query,now)}
        val ids=rows.map{if(query.kind=="positions")"${it["symbol"]}:${it["side"]}" else it["id"] as String}
        require(ids.distinct().size==ids.size)
        if(query.after.isNotEmpty())require(ids.all{OkxReadProtocol.older(it,query.after)})
        val next=if(query.kind=="positions")null else ids.minWithOrNull(compareBy<String>{it.length}.thenBy{it})
        return OkxRecordsPage(query,rows,next)
    }
}
