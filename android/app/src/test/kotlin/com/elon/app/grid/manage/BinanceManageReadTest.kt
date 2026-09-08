package com.elon.app.grid.manage

import org.junit.Assert.*
import org.junit.Test
import com.elon.app.grid.create.BinanceCreateSlot

class BinanceManageReadTest {
    private fun gate(current:Boolean=true,count:Int=1,selected:Boolean=true,unresolved:Boolean=false,
        same:Boolean=true,blocked:Boolean=false,busy:Boolean=false,prepared:Boolean=false) =
        BinanceManageReadGate(current,count,selected,unresolved,same,blocked,busy,prepared)
    @Test fun emptyListAndUnverifiedListAreDifferent() {
        val empty=gate(count=0,selected=false)
        assertEquals("empty",empty.listState);assertFalse(empty.readEnabled);assertFalse(empty.prepareEnabled)
        assertEquals("unverified",gate(current=false,count=0,selected=false).listState)
    }
    @Test fun aVerifiedListStillRequiresSelection() {
        val unselected=gate(selected=false)
        assertEquals("available",unselected.listState);assertFalse(unselected.readEnabled);assertFalse(unselected.prepareEnabled)
        assertTrue(gate().readEnabled);assertTrue(gate().prepareEnabled)
    }
    @Test fun pendingOutcomeCanBeReadWithAnEmptyListOnlyInOriginalAccount() {
        val pending=gate(count=0,selected=false,unresolved=true)
        assertTrue(pending.readEnabled);assertTrue(pending.permits("read"));assertFalse(pending.prepareEnabled)
        assertFalse(pending.permits("select"));assertFalse(pending.permits("reload"))
        assertFalse(gate(count=0,selected=false,unresolved=true,same=false).readEnabled)
    }
    @Test fun busyStaleOrBlockedCannotRead() {
        listOf(gate(busy=true),gate(current=false),gate(blocked=true)).forEach{assertFalse(it.readEnabled)}
    }
    @Test fun debugCommandsNeverInterfereWithPreparedTrade() {
        val prepared=gate(prepared=true)
        listOf("select","read","reload","prepare","submit","resolve").forEach{assertFalse(prepared.permits(it))}
        assertTrue(prepared.permits("status"))
    }
    @Test fun onlyFourActionsAndStrictIndexesAreAccepted() {
        assertEquals("status",BinanceManageReadRequest.parse(emptyMap()).action)
        assertEquals(0,BinanceManageReadRequest.parse(mapOf("action" to "select","index" to 0)).index)
        listOf(mapOf("action" to "submit"),mapOf("action" to "read","strategy_id" to "12"),
            mapOf("action" to "read","index" to 0),mapOf("action" to "select","index" to 0.0),
            mapOf("action" to "select","index" to -1),mapOf("action" to "select","index" to 500),
            mapOf("action" to "select"),mapOf("action" to "status","script" to "x"))
            .forEach { args -> assertThrows(IllegalArgumentException::class.java){BinanceManageReadRequest.parse(args)} }
    }
    @Test fun traceDistinguishesStartedVerifiedAndCancelledReads() {
        val trace=BinanceManageReadTrace()
        assertEquals("idle",trace.outcome);trace.start();assertEquals(1L,trace.sequence)
        trace.finish("verified");trace.cancel();assertEquals("verified",trace.outcome)
        trace.start();trace.cancel();assertEquals(2L,trace.sequence);assertEquals("cancelled",trace.outcome)
        trace.start();trace.finish("failed","settings_unavailable");assertEquals("settings_unavailable",trace.reason)
        trace.start();trace.finish("failed","credential-or-private-content");assertEquals("verification_failed",trace.reason)
    }
    @Test fun readOnlyLeaseDoesNotDisplaceManualController() {
        val slot=BinanceCreateSlot();val manual=Any();val reader=Any()
        assertTrue(slot.acquire(manual));assertFalse(slot.acquire(reader))
        slot.release(reader);assertFalse(slot.acquire(reader))
        slot.release(manual);assertTrue(slot.acquire(reader));assertFalse(slot.acquire(manual))
        slot.release(reader);assertTrue(slot.acquire(manual))
    }
    @Test fun closedReadPagePreservesHistoryWithoutClaimingCurrentDetails() {
        val endpoint=object:BinanceManageReadEndpoint {
            override fun readFacts()=mapOf("page_open" to true,"detail_current" to true,"read_outcome" to "verified")
            override fun readCommand(request:BinanceManageReadRequest)="unused"
        }
        BinanceManageReadBridge.bind(endpoint)
        try {assertEquals(true,BinanceManageReadBridge.facts()["detail_current"])}
        finally {BinanceManageReadBridge.unbind(endpoint)}
        assertEquals("verified",BinanceManageReadBridge.facts()["read_outcome"])
        assertEquals(false,BinanceManageReadBridge.facts()["page_open"])
        assertEquals(false,BinanceManageReadBridge.facts()["detail_current"])
        assertEquals("no_active_page",BinanceManageReadBridge.execute(BinanceManageReadRequest("read"))["result"])
    }
}
