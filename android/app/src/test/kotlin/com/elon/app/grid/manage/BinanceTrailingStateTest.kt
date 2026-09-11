package com.elon.app.grid.manage

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceTrailingStateTest {
    private val account="a".repeat(64)
    private val doc="doc_trailing_test"
    private val input=BinanceTrailingDraft("21","0.5")
    private fun trailing(up:String="20",down:String="1")=BinanceTrailingSnapshot.parse(StrictJson.parse(
        """{"up":true,"down":true,"up_price":"$up","down_price":"$down","lower":"5","upper":"10","count":10,"type":"ARITH"}"""))
    private fun snapshot()=BinanceManageSnapshot("123","NEARUSDT","WORKING",false,true,false,true,true,
        trailing=trailing(),trailingRules=BinanceTrailingRules("0.01","100","0.01"))
    private fun prepare(state:BinanceManageState)=state.prepare(account,doc,"trailing",false,snapshot(),trailingInput=input)
    @Test fun restoredAttemptCannotReplayAndNeedsSameAccountAndExactObservedTarget() {
        val s=BinanceManageState{0L};prepare(s);s.start(account,doc)
        val raw=s.journal();val saved=StrictJson.parse(raw)
        assertEquals("yilong.binance_manage_journal.v5",saved["schema"])
        assertFalse(raw.contains("up_price"));assertFalse(raw.contains("\"21\""));assertFalse(raw.contains(doc))
        val next=BinanceManageState{1L};next.restore(raw)
        assertTrue(next.unresolved);assertEquals("unknown",next.status);assertNull(next.trailingDraft)
        assertFalse(next.canSubmit(account,doc))
        assertTrue(runCatching{next.observe("b".repeat(64),snapshot())}.isFailure)
        next.observe(account,snapshot().copy(trailing=trailing("21","1"),trailingRules=null));assertFalse(next.effectObserved())
        next.observe(account,snapshot().copy(trailing=trailing("21","0.5"),trailingRules=null));assertTrue(next.effectObserved())
        assertTrue(next.unresolved)
    }
    @Test fun prepareRequiresCompleteRulesAndExactCpsAndExpires() {
        var now=0L;val s=BinanceManageState{now};prepare(s)
        assertTrue(s.canSubmit(account,doc));assertFalse(s.canSubmit("b".repeat(64),doc));assertFalse(s.canSubmit(account,"doc_other_test"))
        now=60000;assertFalse(s.canSubmit(account,doc))
        for(before in listOf(snapshot().copy(trailingRules=null),snapshot().copy(trailing=null),snapshot().copy(cps=true)))
            assertTrue(runCatching{s.prepare(account,doc,"trailing",false,before,trailingInput=input)}.isFailure)
    }
    @Test fun journalRejectsDowngradeMissingOrCrossActionTargets() {
        val s=BinanceManageState{0L};prepare(s);s.start(account,doc);val v=StrictJson.parse(s.journal())
        for(bad in listOf(v-"trailing_target",v+("trailing_target" to ""),v+("protection_target" to "b".repeat(64)),
            v+("schema" to "yilong.binance_manage_journal.v4"),v+("action" to "settings")))
            assertTrue(runCatching{BinanceManageState{0L}.restore(StrictJson.encode(bad))}.isFailure)
        for(version in 1..4) {
            val old=linkedMapOf<String,Any>("schema" to "yilong.binance_manage_journal.v$version","status" to "unknown",
                "account" to account,"strategy_id" to "123","action" to "settings","cps" to false,"provider_status" to "")
            if(version>=2)old["investment_target"]=""
            if(version>=3)old["range_target"]=""
            if(version>=4)old["protection_target"]=""
            val restored=BinanceManageState{0L};restored.restore(StrictJson.encode(old));assertTrue(restored.unresolved)
        }
    }
    @Test fun v4DraftAndPublicDetailDoNotChangeV2OrV3Shapes() {
        val raw=StrictJson.encode(input.payload()+mapOf("id" to "123","action" to "trailing"))
        assertEquals(input,BinanceManageCommandDraft.parse(raw,4).trailing)
        for(version in 2..3)assertTrue(runCatching{BinanceManageCommandDraft.parse(raw,version)}.isFailure)
        assertTrue(runCatching{BinanceManageCommandDraft.parse(raw.dropLast(1)+",\"trailingUp\":\"true\"}",4)}.isFailure)
        val old=BinanceManageCommandView.detail(snapshot(),2)
        assertEquals(old.keys+"protection",BinanceManageCommandView.detail(snapshot(),3).keys)
        assertEquals(old.keys+setOf("protection","trailing"),BinanceManageCommandView.detail(snapshot(),4).keys)
        assertNotNull(BinanceManageCommandView.detail(snapshot(),4)["trailing"])
        assertNull(BinanceManageCommandView.detail(snapshot().copy(trailingRules=null),4)["trailing"])
        val s=BinanceManageState{0L};prepare(s)
        val summary=BinanceManageCommandView.summary(s)
        assertTrue(summary.contains("20 → 21"));assertTrue(summary.contains("1 → 0.5"));assertTrue(summary.contains("停止追踪不等于结束网格"))
    }
}
