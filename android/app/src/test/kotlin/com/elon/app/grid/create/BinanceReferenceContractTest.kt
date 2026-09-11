package com.elon.app.grid.create

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceReferenceContractTest {
    private val input=mapOf("symbol" to "NEARUSDT","direction" to "LONG","lower" to "1.2","upper" to "2.4","count" to "33","leverage" to "10",
        "spacing" to "ARITH","triggerPrice" to "","trailingUp" to "false","trailingDown" to "false","marginType" to "ISOLATED")
    private val account="a".repeat(64);private val request="b".repeat(64)
    private val result=mapOf("schema" to BinanceReferenceResult.SCHEMA,"request" to request,"account" to account,"symbol" to "NEARUSDT","status" to "ready",
        "minimum_count" to 2,"maximum_count" to 1000,"minimum_margin" to "27.053","observed_at" to 200000,"source" to "binance_create_rules_v1","code" to "")
    @Test fun partialCountIsAllowedButUndocumentedFieldsAndInvalidNumbersAreNot() {
        assertEquals(input,BinanceReferenceInput.parse(StrictJson.encode(input)))
        assertEquals("",BinanceReferenceInput.parse(StrictJson.encode(input+("count" to "")))["count"])
        for(edit in listOf("upper" to "1.1","lower" to "0","leverage" to "126","count" to "1e2","url" to "/trade","trailingUp" to "yes"))
            assertTrue(edit.toString(),runCatching {BinanceReferenceInput.parse(StrictJson.encode(input+edit))}.isFailure)
    }
    @Test fun resultMustMatchAccountSymbolRequestAndActualObservationTime() {
        val raw=StrictJson.encode(result)
        assertEquals("27.053",BinanceReferenceResult.parse(raw,request,account,"NEARUSDT",200000)["minimum_margin"])
        assertTrue(runCatching {BinanceReferenceResult.parse(raw,request,"c".repeat(64),"NEARUSDT",200000)}.isFailure)
        assertTrue(runCatching {BinanceReferenceResult.parse(raw,request,account,"BTCUSDT",200000)}.isFailure)
        assertTrue(runCatching {BinanceReferenceResult.parse(raw,"d".repeat(64),account,"NEARUSDT",200000)}.isFailure)
        assertTrue(runCatching {BinanceReferenceResult.parse(raw,request,account,"NEARUSDT",320001)}.isFailure)
        for(edit in listOf("source" to "guessed","minimum_margin" to "0","maximum_count" to 10001,"code" to "unsupported"))
            assertTrue(runCatching {BinanceReferenceResult.parse(StrictJson.encode(result+edit),request,account,"NEARUSDT",200000)}.isFailure)
    }
}
