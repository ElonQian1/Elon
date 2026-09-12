package com.elon.app.grid.host

import org.junit.Assert.*
import org.junit.Test

class BinanceEventSubscriptionsTest {
    private val id="a".repeat(64)
    private val operation="b".repeat(64)
    @Test fun onlyTheExactOperationRetainsItsLease() {
        val state=BinanceEventSubscriptions<String>();state.add(id,7,operation,"create","callback")
        assertTrue(state.holds("create",operation));assertFalse(state.holds("manage",operation));assertFalse(state.holds("create",id))
        assertEquals("callback",state.remove(id,7)?.sink);assertFalse(state.holds("create",operation))
    }
    @Test fun anotherUidCannotUnsubscribeOrReplaceAnExistingConsumer() {
        val state=BinanceEventSubscriptions<String>();state.add(id,7,operation,"create","original")
        assertTrue(runCatching {state.remove(id,8)}.isFailure)
        assertTrue(runCatching {state.add(id,8,operation,"create","replacement")}.isFailure)
        assertEquals("original",state.snapshot().getValue(id).sink)
    }
    @Test fun deadConsumersReleaseCapacityAndRepeatedCloseIsSafe() {
        val state=BinanceEventSubscriptions<Int>()
        repeat(16){state.add(it.toString(16).padStart(64,'0'),7,operation,"read",it)}
        assertTrue(runCatching {state.add(id,7,operation,"read",17)}.isFailure)
        val old="0".repeat(64);state.remove(old);assertNull(state.remove(old))
        state.add(id,7,operation,"manage",18);assertEquals(16,state.size)
    }
}
