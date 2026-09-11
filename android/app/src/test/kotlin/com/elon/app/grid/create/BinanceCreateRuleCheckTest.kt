package com.elon.app.grid.create

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceCreateRuleCheckTest {
    private val account="a".repeat(64)
    private val request="b".repeat(64)
    private fun draft(margin:String="27.053",count:String="33")=BinanceGridDraft.parse(mapOf(
        "symbol" to "NEARUSDT","direction" to "LONG","lower" to "1.2","upper" to "2.4",
        "margin" to margin,"count" to count,"leverage" to "10","spacing" to "ARITH",
        "marginType" to "ISOLATED","autoInit" to "true","closeOnStop" to "true"))
    private inner class Fixture {
        var wall=200000L;var elapsed=0L;var starts=0;var reads=0;var clears=0
        var input:Map<String,Any?> = emptyMap()
        var response:Map<String,Any?> = ready()
        var startFailure=false
        val queue=mutableListOf<Runnable>()
        val results=mutableListOf<Result<Long>>()
        val check=BinanceCreateRuleCheck({id,raw->
            assertEquals(request,id);starts++;input=StrictJson.parse(raw,2048)
            if(startFailure)error("transport_down")
        },{reads++;response},{clears++},{job,_->queue.add(job)},{queue.remove(it)},{elapsed},{wall})
        fun ready()=mapOf("schema" to BinanceReferenceResult.SCHEMA,"request" to request,"account" to account,
            "symbol" to "NEARUSDT","status" to "ready","minimum_count" to 3,"maximum_count" to 87,
            "minimum_margin" to "27.053","observed_at" to 200000L,"source" to "binance_create_rules_v1","code" to "")
        fun begin(draft:BinanceGridDraft=draft())=check.begin(request,account,draft){results.add(it)}
        fun run(){queue.removeAt(0).run()}
        fun failure()=results.single().exceptionOrNull()?.message.orEmpty()
    }
    @Test fun equalMinimumAndGreaterAmountsPassWithoutChangingTheDraft() {
        for(amount in listOf("27.053","27.05300000000000000001","2000")) {
            val f=Fixture();f.begin(draft(amount));f.run()
            assertEquals(200000L,f.results.single().getOrThrow());assertEquals(1,f.starts)
            assertEquals(BinanceReferenceInput.keys,f.input.keys)
            assertEquals("false",f.input["trailingUp"]);assertEquals("",f.input["triggerPrice"])
            assertFalse(f.input.containsKey("margin"));assertTrue(f.queue.isEmpty())
        }
    }
    @Test fun belowMinimumUsesExactDecimalComparison() {
        val f=Fixture();f.begin(draft("27.05299999999999999999"));f.run()
        assertTrue(f.failure().contains("至少投入 27.053 USDT"))
    }
    @Test fun actualCountBoundariesAreEnforcedInsteadOfGenericDraftBounds() {
        for(count in listOf("2","88")) {
            val f=Fixture();f.begin(draft(count=count));f.run()
            assertTrue(f.failure().contains("3–87"))
        }
        for(count in listOf("3","87")) {val f=Fixture();f.begin(draft(count=count));f.run();assertTrue(f.results.single().isSuccess)}
    }
    @Test fun impossibleRangesAndIncompleteReferenceCannotAuthorizePreparation() {
        val narrow=Fixture();narrow.response=narrow.ready()+mapOf("maximum_count" to 1,"minimum_margin" to "","code" to "range_too_narrow")
        narrow.begin();narrow.run();assertTrue(narrow.failure().contains("价格区间过窄"))
        for(code in listOf("count_required","count_outside_range")) {
            val f=Fixture();f.response=f.ready()+mapOf("minimum_margin" to "","code" to code);f.begin();f.run()
            assertTrue(f.results.single().isFailure)
        }
    }
    @Test fun responseMustBelongToTheCurrentAccountSymbolRequestAndTime() {
        for(edit in listOf("account" to "c".repeat(64),"request" to "d".repeat(64),"symbol" to "BTCUSDT",
            "observed_at" to 79999L,"observed_at" to 205001L,"minimum_margin" to "0","source" to "guessed")) {
            val f=Fixture();f.response=f.ready()+edit;f.begin();f.run();assertTrue(edit.toString(),f.results.single().isFailure)
        }
    }
    @Test fun pendingPollsAreBoundedAndNeverRestartTheRead() {
        val f=Fixture();f.response=f.ready().filterKeys {it in setOf("schema","request","account","symbol")}+("status" to "pending")
        f.begin();f.run();assertTrue(f.results.isEmpty())
        f.elapsed=30000;f.run()
        assertTrue(f.failure().contains("超时"));assertEquals(1,f.starts);assertEquals(1,f.reads);assertTrue(f.queue.isEmpty())
    }
    @Test fun lateReadyAfterCancellationCannotCompletePreparation() {
        val f=Fixture();f.begin();val old=f.queue.single();f.check.cancel();old.run()
        assertTrue(f.results.isEmpty());assertEquals(0,f.reads);assertTrue(f.queue.isEmpty());assertTrue(f.clears>=1)
    }
    @Test fun replacedJobCannotSatisfyTheNewDraft() {
        val f=Fixture();f.begin();val old=f.queue.single();f.begin(draft("1"));old.run()
        assertTrue(f.results.isEmpty());f.run();assertTrue(f.failure().contains("至少投入"));assertEquals(2,f.starts)
    }
    @Test fun unavailableExpiredAndStartFailureRemainNonSubmittable() {
        for(status in listOf("unavailable","expired")) {
            val f=Fixture();f.response=f.ready().filterKeys {it in setOf("schema","request","account","symbol")}+("status" to status)
            f.begin();f.run();assertTrue(f.results.single().isFailure)
        }
        val f=Fixture();f.startFailure=true;f.begin();assertTrue(f.results.single().isFailure);assertTrue(f.queue.isEmpty())
    }
    @Test fun preparationCannotOutliveItsRuleObservation() {
        assertTrue(BinanceCreateRuleCheck.fresh(200000,320000))
        assertFalse(BinanceCreateRuleCheck.fresh(200000,320001))
        assertFalse(BinanceCreateRuleCheck.fresh(200000,194999))
        assertFalse(BinanceCreateRuleCheck.fresh(null,200000))
    }
}
