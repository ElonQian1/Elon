package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VoiceSpeakerCompletionLedgerTest {
    @Test
    fun staleCompletionCannotFinishTheCurrentUtterance() {
        val ledger = VoiceSpeakerCompletionLedger()
        var firstDone = 0
        var secondDone = 0

        ledger.begin("first", { firstDone += 1 }, null)
        ledger.begin("second", { secondDone += 1 }, null)

        assertFalse(ledger.complete("first", succeeded = true))
        assertTrue(ledger.complete("second", succeeded = true))
        assertEquals(0, firstDone)
        assertEquals(1, secondDone)
    }

    @Test
    fun failureUsesDedicatedCallbackAndSettlesOnce() {
        val ledger = VoiceSpeakerCompletionLedger()
        var done = 0
        var failed = 0
        ledger.begin("request", { done += 1 }, { failed += 1 })

        assertTrue(ledger.complete("request", succeeded = false))
        assertFalse(ledger.complete("request", succeeded = true))
        assertEquals(0, done)
        assertEquals(1, failed)
    }

    @Test
    fun cancelMakesLateCallbacksHarmless() {
        val ledger = VoiceSpeakerCompletionLedger()
        var callbackCount = 0
        ledger.begin("request", { callbackCount += 1 }, { callbackCount += 1 })

        ledger.cancel()

        assertFalse(ledger.complete("request", succeeded = true))
        assertEquals(0, callbackCount)
    }
}
