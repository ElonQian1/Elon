package com.elon.app.grid.manage

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceRangeTest {
    private val account="a".repeat(64)
    private val document="doc_range_test"
    private val range=BinanceRangeSnapshot("1","2",10,"ARITH","LONG",mapOf("tpslCps" to true,"stopLowerLimit" to "0.8"))
    private val snapshot=BinanceManageSnapshot("123","NEARUSDT","WORKING",false,true,false,false,false,BinanceInvestmentSnapshot("600",3,"0"),range)
    private val draft=BinanceRangeDraft("1.1","2.1",12,false,"0")
    private fun fails(block:()->Unit)=assertTrue(runCatching(block).isFailure)
    private fun prepared(clock:()->Long={0L})=BinanceManageState(clock).also{it.prepare(account,document,"range",false,snapshot,rangeDraft=draft)}
    @Test fun exactPriceAndExplicitAmountValidation() {
        assertEquals("1.000000000000000001",BinanceRange.price("1.000000000000000001"))
        for(value in listOf("-1","1e2","NaN","","01"))fails{BinanceRange.price(value)}
        for(amount in listOf("","-1","2000.00000001","1.000000001","1e3"))fails{draft.copy(investmentDelta=amount)}
        fails{draft.copy(lower="2.2")};fails{draft.copy(count=1)};fails{draft.copy(count=10001)}
        val parsed=StrictJson.parse(StrictJson.encode(draft.payload()),2048)
        assertEquals(draft,BinanceRangeDraft.parse(parsed));fails{BinanceRangeDraft.parse(parsed-"close_positions")}
    }
    @Test fun incompleteSnapshotAndExtraPreservedFieldsFailClosed() {
        val value=mapOf("lower" to "1","upper" to "2","count" to 10,"grid_type" to "ARITH","direction" to "LONG","preserved" to range.preserved)
        val parsed=StrictJson.parse(StrictJson.encode(value),2048)
        assertEquals(range,BinanceRangeSnapshot.parse(parsed))
        fails{BinanceRangeSnapshot.parse(parsed+("preserved" to mapOf("tpslCps" to "true")))}
        fails{BinanceRangeSnapshot.parse(parsed+("preserved" to mapOf("tpslCps" to true,"strategyId" to "999")))}
    }
    @Test fun requiresChangedRangeVerifiedFundingAndMatchingAccount() {
        val s=BinanceManageState{0L}
        fails{s.prepare(account,document,"range",false,snapshot.copy(range=null),rangeDraft=draft)}
        fails{s.prepare(account,document,"range",false,snapshot.copy(investment=null),rangeDraft=draft)}
        fails{s.prepare(account,document,"range",false,snapshot,rangeDraft=draft.copy(lower="1",upper="2",count=10))}
        fails{s.prepare(account,document,"range",true,snapshot,rangeDraft=draft)}
        fails{s.prepare(account,document,"settings",true,snapshot,rangeDraft=draft)}
        val p=prepared();assertFalse(p.canSubmit("b".repeat(64),document));assertFalse(p.canSubmit(account,"doc_other_test"))
    }
    @Test fun journalStoresOnlyFingerprintAndDoesNotRearmAfterRestart() {
        val s=prepared();s.start(account,document);val raw=s.journal()
        assertFalse(raw.contains("1.1"));assertFalse(raw.contains("close_positions"));assertFalse(raw.contains("investment_delta"))
        val restored=BinanceManageState{0L};restored.restore(raw)
        assertTrue(restored.unresolved);assertEquals("unknown",restored.status);assertNull(restored.rangeDraft)
        assertFalse(restored.canSubmit(account,document));fails{restored.prepare(account,document,"range",false,snapshot,rangeDraft=draft)}
    }
    @Test fun acceptedAndObservedRangesStillRequireReconciliation() {
        val s=prepared();s.start(account,document);s.outcome("accepted","")
        assertTrue(s.unresolved);assertFalse(s.effectObserved())
        s.observe(account,snapshot.copy(range=range.copy(lower=draft.lower,upper=draft.upper,count=draft.count)))
        assertTrue(s.effectObserved());assertTrue(s.unresolved)
        fails{s.observe("b".repeat(64),snapshot)}
    }
    @Test fun oldJournalsRemainCompatibleAndCannotClaimRangeAction() {
        val v2="""{"schema":"yilong.binance_manage_journal.v2","status":"unknown","account":"$account","strategy_id":"123","action":"settings","cps":false,"provider_status":"","investment_target":""}"""
        val s=BinanceManageState{0L};s.restore(v2);assertTrue(s.unresolved)
        fails{BinanceManageState{0L}.restore(v2.replace("settings","range"))}
    }
    @Test fun expiredOrCancelledRangeCannotSubmit() {
        var now=0L;val s=prepared{now};now=60_000;assertFalse(s.canSubmit(account,document))
        val c=prepared();c.cancel();assertFalse(c.canSubmit(account,document))
    }
}
