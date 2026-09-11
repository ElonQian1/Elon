package com.elon.app.grid.create

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceReferenceEconomicsTest {
    private val id="a".repeat(64);private val account="b".repeat(64)
    private val ready=mapOf("schema" to "yilong.binance_create_reference.v2","request" to id,"account" to account,"symbol" to "NEARUSDT","status" to "ready",
        "minimum_count" to 2,"maximum_count" to 1000,"minimum_margin" to "12.34","observed_at" to 200000,"source" to "binance_create_rules_v2","code" to "",
        "profit_min" to "0.25","profit_max" to "0.5","quantity" to "12.1","quantity_unit" to "NEAR","quantity_status" to "ready")
    private fun parse(value:Map<String,Any?>)=BinanceReferenceResult.parse(StrictJson.encode(value),id,account,"NEARUSDT",200000,2)
    @Test fun strictV2ProjectionAndV1Compatibility() {
        assertEquals("12.1",parse(ready)["quantity"])
        assertTrue(runCatching {BinanceReferenceResult.parse(StrictJson.encode(ready),id,account,"NEARUSDT",200000)}.isFailure)
        for(edit in listOf("profit_min" to "NaN","profit_min" to "1","profit_max" to "","quantity" to "0","quantity_unit" to "BTC","quantity_status" to "filled","cookie" to "extra","account" to "c".repeat(64)))
            assertTrue(edit.toString(),runCatching {parse(ready+edit)}.isFailure)
        assertEquals("-0.5",parse(ready+mapOf("profit_min" to "-0.5","profit_max" to "-0.1"))["profit_min"])
        for(status in listOf("margin_required","margin_below_minimum"))assertEquals("",parse(ready+mapOf("quantity" to "","quantity_status" to status))["quantity"])
        val unknown=ready+mapOf("code" to "count_required","minimum_margin" to "","profit_min" to "","profit_max" to "","quantity" to "","quantity_status" to "count_unavailable")
        assertEquals("",parse(unknown)["profit_min"]);assertTrue(runCatching {parse(unknown+("quantity" to "1"))}.isFailure)
    }
    @Test fun marginIsV2OnlyAndCannotCarryArbitraryRequestInputs() {
        val v=mapOf("symbol" to "NEARUSDT","direction" to "LONG","lower" to "1.2","upper" to "2.4","count" to "33","leverage" to "10",
            "spacing" to "ARITH","triggerPrice" to "","trailingUp" to "false","trailingDown" to "false","marginType" to "ISOLATED","margin" to "200")
        assertEquals(v,BinanceReferenceInput.parse(StrictJson.encode(v),2))
        assertTrue(runCatching {BinanceReferenceInput.parse(StrictJson.encode(v))}.isFailure)
        assertEquals("",BinanceReferenceInput.parse(StrictJson.encode(v+("margin" to "")),2)["margin"])
        for(edit in listOf("margin" to "-1","margin" to "1e3","margin" to "0","url" to "https://example.com"))assertTrue(runCatching {BinanceReferenceInput.parse(StrictJson.encode(v+edit),2)}.isFailure)
    }
}
