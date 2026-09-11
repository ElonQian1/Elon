package com.elon.app.grid.create

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceCreateFundsResultTest {
    private val request="a".repeat(64)
    private val account="b".repeat(64)
    private val base=mapOf<String,Any>("schema" to BinanceCreateFundsResult.SCHEMA,"request" to request,"account" to account,"status" to "ready")
    private val ready=base+mapOf("asset" to "USDT","available" to "123.45678901234567890123","source" to "futures_transferable","observed_at" to 200000L)
    private fun parse(v:Map<String,Any>,now:Long=200000)=BinanceCreateFundsResult.parse(StrictJson.encode(v),request,account,now)
    @Test fun preservesExactDecimalAndExplicitZero() {
        assertEquals(ready["available"],parse(ready)["available"])
        assertEquals("0",parse(ready+("available" to "0"))["available"])
        assertEquals("portfolio_spot_free",parse(ready+("source" to "portfolio_spot_free"))["source"])
    }
    @Test fun rejectsWrongIdentityAssetSourceAndUnexpectedFields() {
        for(edit in listOf("request" to "c".repeat(64),"account" to "d".repeat(64),"asset" to "BTC","source" to "strategy_margin","cookie" to "CANARY"))
            assertTrue(runCatching {parse(ready+edit)}.isFailure)
    }
    @Test fun rejectsInvalidMoneyAndUnsafeNumericInput() {
        for(value in listOf<Any>("-1","01","1e3","","1."+"1".repeat(21),12.5,0,true))assertTrue(runCatching {parse(ready+("available" to value))}.isFailure)
        assertTrue(runCatching {parse(ready-"available")}.isFailure)
    }
    @Test fun boundedFreshnessAndNonReadyHaveNoAmount() {
        parse(ready,260000);parse(ready,195000)
        assertTrue(runCatching {parse(ready,260001)}.isFailure);assertTrue(runCatching {parse(ready,194999)}.isFailure)
        for(status in listOf("pending","expired","unavailable")) {
            assertFalse(parse(base+("status" to status)).containsKey("available"))
            assertTrue(runCatching {parse(ready+("status" to status))}.isFailure)
        }
    }
}
