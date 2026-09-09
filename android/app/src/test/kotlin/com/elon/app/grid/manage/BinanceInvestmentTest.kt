package com.elon.app.grid.manage

import org.junit.Assert.*
import org.junit.Test
import com.elon.app.privateaccess.StrictJson

class BinanceInvestmentTest {
    private val account="a".repeat(64)
    private val document="doc_invest_test"
    private val funding=BinanceInvestmentSnapshot("600",3,"-0.25")
    private fun snapshot()=BinanceManageSnapshot("123","NEARUSDT","WORKING",false,true,false,false,false,funding)
    private fun fails(block:()->Unit)=assertTrue(runCatching(block).isFailure)
    @Test fun investmentIsMarginAndUsesExactDecimals() {
        assertEquals("199.75",funding.invested())
        assertEquals("0.00000001",BinanceInvestment.amount("0.00000001"))
        assertEquals("2000",BinanceInvestment.amount("2000.000"))
        for(v in listOf("0","-1","1e3","NaN","2000.00000001","1.000000001","", " 1", "01"))fails{BinanceInvestment.amount(v)}
    }
    @Test fun typedFundingDoesNotDefaultMissingAdjustmentsToZero() {
        val v=StrictJson.parse("""{"initial_value":"600","initial_leverage":3,"total_adjustment":"-0.25"}""")
        assertEquals(funding,BinanceInvestmentSnapshot.parse(v))
        fails{BinanceInvestmentSnapshot.parse(v-"total_adjustment")}
        fails{BinanceInvestmentSnapshot.parse(v+("initial_value" to 600.1))}
        fails{BinanceInvestmentSnapshot.parse(v+("initial_leverage" to 0))}
    }
    @Test fun investmentHasFreshSnapshotAndSingleAttempt() {
        val s=BinanceManageState{0L}
        fails{s.prepare(account,document,"investment",false,snapshot().copy(investment=null),"20")}
        fails{s.prepare(account,document,"investment",true,snapshot(),"20")}
        s.prepare(account,document,"investment",false,snapshot(),"20.125")
        assertEquals("20.125",s.investmentDelta)
        assertEquals("219.875",s.investmentPreview())
        s.start(account,document);s.outcome("accepted","")
        assertTrue(s.unresolved);assertFalse(s.canSubmit(account,document));assertFalse(s.effectObserved())
    }
    @Test fun restartRetainsOnlyTargetFingerprintAndNeverResends() {
        val s=BinanceManageState{0L};s.prepare(account,document,"investment",false,snapshot(),"20.125")
        s.start(account,document);val raw=s.journal()
        assertFalse(raw.contains("20.125"));assertFalse(raw.contains("600"))
        val restored=BinanceManageState{5L};restored.restore(raw)
        assertEquals("unknown",restored.status);assertEquals("",restored.investmentDelta)
        assertFalse(restored.canSubmit(account,document))
        restored.observe(account,snapshot().copy(investment=funding.copy(totalAdjustment="19.8750")))
        assertTrue(restored.effectObserved());assertTrue(restored.unresolved)
        restored.observe(account,snapshot().copy(investment=funding.copy(totalAdjustment="19.8751")))
        assertFalse(restored.effectObserved())
    }
    @Test fun oldJournalStillRestoresButCannotSmuggleInvestment() {
        val raw="""{"schema":"yilong.binance_manage_journal.v1","status":"unknown","account":"$account","strategy_id":"123","action":"settings","cps":true,"provider_status":""}"""
        val s=BinanceManageState{0L};s.restore(raw);assertTrue(s.unresolved)
        fails{BinanceManageState{0L}.restore(raw.replace("settings","investment"))}
    }
}
