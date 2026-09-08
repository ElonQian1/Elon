package com.elon.app.grid.manage

import org.junit.Assert.*
import org.junit.Test
import com.elon.app.grid.create.binanceDispatchStarted

class BinanceManageStateTest {
    private val account="a".repeat(64)
    private val document="doc_test_123"
    private fun snapshot(cps:Boolean=false)=BinanceManageSnapshot("123","NEARUSDT","WORKING",cps,true,false,false,false)
    private fun prepared(clock:()->Long={0L})=BinanceManageState(clock).also{it.prepare(account,document,"settings",true,snapshot())}
    private fun fails(action:()->Unit) {assertTrue(runCatching(action).isFailure)}
    @Test fun ambiguousWebViewAcknowledgementCannotProveNotSent() {
        assertEquals(true,binanceDispatchStarted("true"));assertEquals(false,binanceDispatchStarted("false"))
        for(raw in listOf(null,"null","undefined","","\"false\"","{}")) assertNull(binanceDispatchStarted(raw))
    }
    @Test fun accountDocumentAndExpiryAreBound() {
        var now=0L;val s=prepared{now}
        assertTrue(s.canSubmit(account,document));assertFalse(s.canSubmit("b".repeat(64),document));assertFalse(s.canSubmit(account,"doc_next_123"))
        now=60_000;assertFalse(s.canSubmit(account,document))
    }
    @Test fun closeCannotSilentlyChangeCps() {
        val s=BinanceManageState{0L}
        fails{s.prepare(account,document,"close",true,snapshot(false))}
        s.prepare(account,document,"close",false,snapshot(false));assertTrue(s.canSubmit(account,document))
    }
    @Test fun settingsMustChangeAndGridMustWork() {
        val s=BinanceManageState{0L}
        fails{s.prepare(account,document,"settings",false,snapshot(false))}
        fails{s.prepare(account,document,"settings",true,snapshot().copy(status="CANCELED"))}
    }
    @Test fun unknownNeverAutomaticallyRetriesAfterRestart() {
        val s=prepared();s.start(account,document);val raw=s.journal()
        val restored=BinanceManageState{1L};restored.restore(raw)
        assertEquals("unknown",restored.status);assertTrue(restored.unresolved);assertNull(restored.snapshot)
        assertFalse(restored.canSubmit(account,document));fails{restored.prepare(account,document,"settings",true,snapshot())}
    }
    @Test fun observedTargetDoesNotClearPendingRecord() {
        val s=prepared();s.start(account,document);s.outcome("accepted","WORKING")
        assertFalse(s.effectObserved());s.observe(account,snapshot(true));assertTrue(s.effectObserved());assertTrue(s.unresolved)
        fails{s.observe("b".repeat(64),snapshot())};fails{s.observe(account,snapshot().copy(id="999"))}
    }
    @Test fun restoreRequiresValidTypedIdentity() {
        val s=prepared();s.start(account,document)
        val raw=s.journal();val next=BinanceManageState{0L}
        fails{next.restore(raw.replace("123","9007199254740992"))}
        fails{next.restore(raw.replace("\"cps\":true","\"cps\":\"true\""))}
        fails{next.restore(raw.replace("\"settings\"","\"arbitrary\""))}
    }
    @Test fun snapshotRejectsIncompleteOrImpreciseContract() {
        val v=mapOf("strategy_id" to "123","symbol" to "NEARUSDT","provider_status" to "WORKING","cps" to false,
            "cos" to true,"sharing" to false,"trailingStopLowerLimit" to false,"trailingStopUpperLimit" to false)
        assertEquals(snapshot(),BinanceManageSnapshot.parse(v))
        fails{BinanceManageSnapshot.parse(v-"sharing")};fails{BinanceManageSnapshot.parse(v+("cps" to 1))}
        fails{BinanceManageSnapshot.parse(v+("strategy_id" to "9007199254740992"))}
    }
    @Test fun cancelAndTerminalOutcomesDoNotArmSubmission() {
        val s=prepared();s.cancel();assertFalse(s.canSubmit(account,document))
        val n=prepared();n.start(account,document);n.outcome("not_sent");assertFalse(n.unresolved);assertFalse(n.canSubmit(account,document))
        val a=prepared();a.start(account,document);fails{a.outcome("accepted","")}
    }
    @Test fun closeStateStillDoesNotMeanPositionZero() {
        val s=BinanceManageState{0L};s.prepare(account,document,"close",false,snapshot());s.start(account,document);s.outcome("accepted","CANCELED")
        s.observe(account,snapshot().copy(status="CLOSE_WITH_POSITION"));assertTrue(s.effectObserved());assertTrue(s.unresolved)
    }
}
