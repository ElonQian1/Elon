package com.elon.app.grid.manage

import com.elon.app.grid.create.BinanceCreatePermit
import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceManageCommandDraftTest {
    private val account="a".repeat(64)
    private val doc="doc_manage_test"
    private val range=BinanceRangeSnapshot("1","2",10,"ARITH","LONG",mapOf("tpslCps" to true,"stopLowerLimit" to "0.8"))
    private val snapshot=BinanceManageSnapshot("123","NEARUSDT","WORKING",false,true,false,false,false,BinanceInvestmentSnapshot("600",3,"0"),range)
    private fun parse(action: String,fields: Map<String,Any>)=BinanceManageCommandDraft.parse(StrictJson.encode(mapOf("id" to "123","action" to action)+fields))
    private fun fails(block:()->Unit)=assertTrue(runCatching(block).isFailure)
    @Test fun onlyFourExactIntentShapesAreAccepted() {
        assertTrue(parse("settings",mapOf("cps" to "true")).cps)
        assertFalse(parse("close",mapOf("cps" to "false")).cps)
        assertEquals("10.25",parse("investment",mapOf("amount" to "10.25")).amount)
        assertEquals(12,parse("range",mapOf("lower" to "1.1","upper" to "2.1","count" to "12","close_positions" to "false","amount" to "0")).range!!.count)
        fails{parse("tpsl",mapOf("cps" to "true"))}
        fails{parse("close",mapOf("cps" to "false","url" to "/trade"))}
        fails{parse("close",mapOf("cps" to false))}
        fails{parse("close",emptyMap())}
    }
    @Test fun InvalidIdsNumbersFlagsAndDuplicateFieldsFailClosed() {
        for(id in listOf("0","01","9007199254740992","1;trade"))fails{parse("close",mapOf("cps" to "true","id" to id))}
        for(flag in listOf("TRUE","1","","yes"))fails{parse("settings",mapOf("cps" to flag))}
        for(amount in listOf("0","-1","1e3","2000.00000001"))fails{parse("investment",mapOf("amount" to amount))}
        fails{BinanceManageCommandDraft.parse("{\"id\":\"123\",\"action\":\"close\",\"cps\":\"false\",\"cps\":\"true\"}")}
        fails{BinanceManageCommandDraft.parse(" ".repeat(4097))}
    }
    @Test fun rangeReusesExistingLimitsAndRequiresExplicitPositionAndAmount() {
        val draft=mapOf("lower" to "1","upper" to "2","count" to "10","close_positions" to "false","amount" to "0")
        for(key in draft.keys)fails{parse("range",draft-key)}
        for(delta in listOf("","-1","2001","0.000000001"))fails{parse("range",draft+("amount" to delta))}
        fails{parse("range",draft+("lower" to "2"))};fails{parse("range",draft+("count" to "1"))}
    }
    @Test fun summaryUsesCurrentSnapshotAndDistinguishesClosingFromChangingSettings() {
        val s=BinanceManageState{0L};s.prepare(account,doc,"close",false,snapshot)
        val summary=BinanceManageCommandView.summary(s)
        assertTrue(summary.contains("NEARUSDT"));assertTrue(summary.contains("123"));assertTrue(summary.contains("不会先修改设置"))
        assertTrue(summary.contains("保留仓位"));assertTrue(summary.contains("0.8"));assertTrue(summary.contains("不证明仓位归零"))
        val settings=BinanceManageState{0L};settings.prepare(account,doc,"settings",true,snapshot)
        assertTrue(BinanceManageCommandView.summary(settings).contains("不立即结束"))
        assertNotEquals(BinanceManageCommandView.digest(s),BinanceManageCommandView.digest(settings))
    }
    @Test fun normalizedDetailHasNoSessionSecretsAndRangePreservesStops() {
        val value=BinanceManageCommandView.detail(snapshot)
        assertEquals(setOf("id","symbol","status","cps","cos","investment","range"),value.keys)
        assertEquals("0.8",(value["range"] as Map<*,*>)["stop_lower"])
        assertEquals("10",(value["range"] as Map<*,*>)["count"])
        assertFalse(StrictJson.encode(value).contains(account))
    }
    @Test fun emptyStopsUseExistingNullableProjectionAndReadableSummary() {
        val current=snapshot.copy(range=range.copy(preserved=mapOf("tpslCps" to true,"stopLowerLimit" to "","stopUpperLimit" to "")))
        val detail=BinanceManageCommandView.detail(current)["range"] as Map<*,*>
        assertNull(detail["stop_lower"]);assertNull(detail["stop_upper"])
        assertEquals("1",detail["lower"]);assertEquals("2",detail["upper"])
        val state=BinanceManageState{0L};state.prepare(account,doc,"close",false,current)
        assertTrue(BinanceManageCommandView.summary(state).contains("原价格止盈止损：未设置 / 未设置"))
        assertEquals("",state.snapshot!!.range!!.preserved["stopLowerLimit"])
    }
    @Test fun permitBindsSummaryAccountDocumentAndOperationOnce() {
        var now=0L;val permit=BinanceCreatePermit{now}
        val s=BinanceManageState{now};s.prepare(account,doc,"investment",false,snapshot,"10")
        val digest=BinanceManageCommandView.digest(s);val op="b".repeat(64);val nonce="c".repeat(64)
        permit.bind(op,account,doc,digest,nonce)
        assertFalse(permit.valid(op,"d".repeat(64),doc,digest,nonce))
        assertFalse(permit.valid(op,account,"doc_other_test",digest,nonce))
        assertFalse(permit.valid("e".repeat(64),account,doc,digest,nonce))
        assertFalse(permit.valid(op,account,doc,"f".repeat(64),nonce))
        permit.consume(op,account,doc,digest,nonce);fails{permit.consume(op,account,doc,digest,nonce)}
        permit.bind(op,account,doc,digest,nonce);now=60_000;assertFalse(permit.valid(op,account,doc,digest,nonce))
    }
}
