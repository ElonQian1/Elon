package com.elon.app.grid.manage

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceTrailingTest {
    private fun snapshot()=BinanceTrailingSnapshot.parse(StrictJson.parse("""{"up":true,"down":true,"up_price":"20","down_price":"1","lower":"5","upper":"10","count":10,"type":"ARITH"}"""))
    private val rules=BinanceTrailingRules("0.01","1000","0.01")
    @Test fun boundariesUseUpperForUpAndLowerForDownAndNeverRoundInput() {
        BinanceTrailingDraft("1000","0.01").validate(snapshot(),rules)
        for(v in listOf(BinanceTrailingDraft("10","1"),BinanceTrailingDraft("6","1"),BinanceTrailingDraft("21","5"),
            BinanceTrailingDraft("21","6"),BinanceTrailingDraft("1000.01","1"),BinanceTrailingDraft("21.001","1"),BinanceTrailingDraft("20","1")))
            assertTrue(runCatching{v.validate(snapshot(),rules)}.isFailure)
    }
    @Test fun draftIsCanonicalStrictAndTargetIncludesDirection() {
        val v=BinanceTrailingDraft.parse(mapOf("up_price" to "21.00","down_price" to "0.50"))
        assertEquals("21",v.upPrice);assertEquals("0.5",v.downPrice)
        for(input in listOf(mapOf("up_price" to "1e2","down_price" to "1"),mapOf("up_price" to "21","down_price" to "1","extra" to "true")))
            assertTrue(runCatching{BinanceTrailingDraft.parse(input)}.isFailure)
        assertNotEquals(BinanceTrailingDraft.fingerprint(true,false,"21",""),BinanceTrailingDraft.fingerprint(true,true,"21",""))
    }
    @Test fun publicDetailDoesNotExposePrivateCalculationInputs() {
        val v=snapshot().publicDetail(rules)
        assertEquals(setOf("up","down","up_price","down_price","lower","upper","minimum","maximum","tick"),v.keys)
    }
}
