package com.elon.app.grid.manage

import org.junit.Assert.*
import org.junit.Test

class BinanceProtectionTest {
    private fun draft(mode:String="PNL",lower:String="",upper:String="",tp:String="10",sl:String="5") =
        BinanceProtectionDraft(mode,lower,upper,tp,sl,"CONTRACT_PRICE",true)
    @Test fun roiUsesOriginalLeverageAndAdjustmentsWithAsymmetricRounding() {
        val investment=BinanceInvestmentSnapshot("100",3,"0.01")
        val target=draft("ROI",tp="10",sl="10").normalize(investment)
        assertEquals("3.33",target.tp);assertEquals("3.34",target.sl)
        assertEquals("PNL",target.mode)
    }
    @Test fun exactRationalDoesNotTruncateBeforeFinalRounding() {
        val t=draft("ROI",tp="3",sl="3").normalize(BinanceInvestmentSnapshot("100",3,"0"))
        assertEquals("1",t.tp);assertEquals("1",t.sl)
    }
    @Test fun clearIsExplicitAndEmpty() {
        val t=draft("CLEAR",tp="",sl="").normalize(null)
        assertEquals("CLEAR",t.mode);assertEquals("",t.tp)
        assertThrows(IllegalArgumentException::class.java){draft("PNL",tp="",sl="")}
        assertThrows(IllegalArgumentException::class.java){draft("CLEAR")}
    }
    @Test fun strictDecimalAndModeBoundaries() {
        for(value in listOf("-1","0","1e2","NaN","1.001","01"," 1"))
            assertThrows(IllegalArgumentException::class.java){draft(tp=value)}
        assertThrows(IllegalArgumentException::class.java){draft("PRICE",lower="2",upper="1",tp="",sl="")}
        assertThrows(IllegalArgumentException::class.java){draft("PRICE",lower="1")}
    }
    @Test fun unavailableOrNonpositiveRoiBaseIsRejected() {
        assertThrows(IllegalArgumentException::class.java){draft("ROI").normalize(null)}
        assertThrows(IllegalArgumentException::class.java){draft("ROI").normalize(BinanceInvestmentSnapshot("10",10,"-1"))}
        assertThrows(IllegalArgumentException::class.java){draft("ROI",tp="0.00001").normalize(BinanceInvestmentSnapshot("1",1,"0"))}
    }
    @Test fun fingerprintUsesEffectiveTargetAndPositiveLoss() {
        val a=draft(tp="10.00",sl="5.00").normalize(null)
        val b=draft().normalize(null)
        assertEquals(a.target(),b.target());assertEquals("5",b.sl)
        assertNotEquals(a.target(),draft(sl="6").normalize(null).target())
    }
    private fun protection(tp:String="",sl:String="")=BinanceProtectionSnapshot.parse(mapOf(
        "direction" to "LONG","stop_type" to "CONTRACT_PRICE","lower" to "","upper" to "","tp" to tp,"sl" to sl,
        "tpsl_cps" to true,"trailing_up" to false,"trailing_down" to false,"trigger_type" to "CONTRACT_PRICE","trigger_price" to "",
        "auto_init" to true,"trailing_up_price" to "","trailing_down_price" to ""))
    @Test fun protectionJournalIsNonReplayableAndRequiresSameAccountAndObservedTarget() {
        val account="a".repeat(64);val doc="doc_protection_123"
        val snapshot=BinanceManageSnapshot("123","NEARUSDT","WORKING",false,true,false,false,false,protection=protection())
        val state=BinanceManageState{0L};state.prepare(account,doc,"protection",false,snapshot,protectionInput=draft())
        state.cancel();assertFalse(state.canSubmit(account,doc));assertEquals(snapshot,state.snapshot)
        state.prepare(account,doc,"protection",false,snapshot,protectionInput=draft());state.start(account,doc)
        val raw=state.journal();assertTrue(raw.contains("journal.v4"));assertFalse(raw.contains("stop_type"));assertFalse(raw.contains("\"tp\""))
        val next=BinanceManageState{1L};next.restore(raw);assertEquals("unknown",next.status);assertFalse(next.canSubmit(account,doc))
        assertThrows(IllegalArgumentException::class.java){next.observe("b".repeat(64),snapshot)}
        next.observe(account,snapshot.copy(protection=protection("10","4")));assertFalse(next.effectObserved())
        next.observe(account,snapshot.copy(protection=protection("10","5")));assertTrue(next.effectObserved());assertTrue(next.unresolved)
    }
    @Test fun protectionDraftIsVersionBoundAndDoesNotChangeOrdinaryCloseMode() {
        val raw=com.elon.app.privateaccess.StrictJson.encode(draft().payload().mapValues{it.value.toString()}+mapOf("id" to "123","action" to "protection"))
        assertThrows(IllegalArgumentException::class.java){BinanceManageCommandDraft.parse(raw)}
        assertEquals(draft(),BinanceManageCommandDraft.parse(raw,3).protection)
        val snapshot=BinanceManageSnapshot("123","NEARUSDT","WORKING",false,true,false,false,false,protection=protection())
        assertThrows(IllegalArgumentException::class.java){BinanceManageState{0L}.prepare("a".repeat(64),"doc_protection_123","protection",true,snapshot,protectionInput=draft())}
    }
}
