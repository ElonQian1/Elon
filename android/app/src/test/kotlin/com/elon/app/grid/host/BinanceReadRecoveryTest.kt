package com.elon.app.grid.host

import com.elon.app.grid.create.BinanceCreateSlot
import org.junit.Assert.*
import org.junit.Test

class BinanceReadRecoveryTest {
    private var now = 1_000L
    private val slot = BinanceCreateSlot()
    private val recovery = BinanceReadRecovery({ now }, slot)
    private var reloads = 0
    private fun attempt(value: String? = "false", eligible: Boolean = true) =
        recovery.recover(value, { eligible }) { reloads++; recovery.pageStarted() }
    private fun stalled() { recovery.pageStarted(); now += 10_000 }

    @Test fun missingContextGetsOneReloadAfterInitializationGrace() {
        recovery.pageStarted()
        assertFalse(attempt())
        now += 9_999
        assertFalse(attempt())
        now++
        assertTrue(attempt())
        assertEquals(1, reloads)
        assertTrue(recovery.reloadUsed)
    }

    @Test fun reloadPageEventsAndRepeatedResumesCannotMakeReloadLoop() {
        stalled(); assertTrue(attempt())
        repeat(10) {
            now += 20_000
            recovery.pageStarted()
            now += 20_000
            assertFalse(attempt())
        }
        assertEquals(1, recovery.reloadCount)
    }

    @Test fun StartedReadAndMissingAdapterDoNotTriggerReload() {
        stalled()
        listOf("true", "null", "undefined", "", null, "\"false\"").forEach { assertFalse(attempt(it)) }
        assertEquals(0, reloads)
        assertTrue(attempt())
    }

    @Test fun LateCallbackOrNonIdleDocumentDoesNotConsumeBudget() {
        stalled()
        assertFalse(attempt(eligible = false))
        assertFalse(recovery.reloadUsed)
        assertTrue(attempt(eligible = true))
    }

    @Test fun ActiveCreateOrManageSlotBlocksReloadWithoutStealingOwnership() {
        stalled()
        val controller = Any()
        assertTrue(slot.acquire(controller))
        assertFalse(attempt())
        assertFalse(slot.acquire(Any()))
        slot.release(controller)
        assertTrue(attempt())
    }

    @Test fun EligibilityIsCheckedWhileHoldingWriteSlotAndSlotIsReleased() {
        stalled()
        assertTrue(recovery.recover("false", {
            assertFalse(slot.acquire(Any()))
            true
        }) { assertFalse(slot.acquire(Any())) })
        val controller = Any()
        assertTrue(slot.acquire(controller))
        slot.release(controller)
    }

    @Test fun OnlyVerifiedListStartsAnotherRecoveryEpisode() {
        stalled(); assertTrue(attempt())
        now += 20_000
        assertFalse(attempt())
        recovery.verifiedList()
        assertTrue(attempt())
        assertEquals(2, reloads)
    }

    @Test fun InvalidatedSessionNeedsNewDocumentAndGrace() {
        stalled(); assertTrue(attempt())
        recovery.reset()
        now += 20_000
        assertFalse(attempt())
        assertEquals(0, recovery.reloadCount)
        recovery.pageStarted()
        assertFalse(attempt())
        now += 10_000
        assertTrue(attempt())
    }

    @Test fun ReloadFailureKeepsBudgetSpentAndReleasesSlot() {
        stalled()
        try {
            recovery.recover("false", { true }) { error("renderer disappeared") }
            fail("expected reload failure")
        } catch (_: IllegalStateException) { }
        assertFalse(attempt())
        val controller = Any()
        assertTrue(slot.acquire(controller))
        slot.release(controller)
    }
}
